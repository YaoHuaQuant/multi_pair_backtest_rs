//! StrategyMk2
//! 等仓位占比策略 + 策略交易对
//! 只做多策略

use std::cmp;
use std::collections::HashSet;
use log::{debug, error, info};
use rust_decimal::{
    Decimal,
    prelude::FromPrimitive,
};
use uuid::Uuid;

use crate::{
    config::TRADDING_PAIR_BTC_USDT_MIN_QUANTITY,
    data_runtime::{
        asset::{
            asset::SAsset,
            asset_map::SAssetMap,
        },
        order::{
            EOrderAction,
            trading_pair_order_manager_map::STradingPairOrderManagerMap,
        },
    },
    data_source::trading_pair::ETradingPairType,
    protocol::{ERunnerParseOrderResult, ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult, SStrategyOrderAdd},
    strategy::{
        order::trading_pair_order_map::SStrategyTradingPairOrderMap,
        TStrategy,
    },
};
use crate::config::{MAKER_ORDER_FEE, TRADDING_PAIR_USDT_MIN_QUANTITY};
use crate::data_runtime::order::{EOrderDirection, EOrderPosition};
use crate::strategy::order::order::{EStrategyOrderState, RStrategyOrderResult, SStrategyOrder};
use crate::strategy::order::order_manager::{RStrategyOrderManagerResult, SStrategyOrderManager};
/// 订单管理器异常
#[derive(Debug)]
pub enum EStrategyMk2Error {
    /// 缺少已开仓订单 无法生成平仓单
    LackOpenedOrderError,
}

pub struct SStrategyMk2 {
    /// 目标仓位占比
    pub target_position_ratio: Decimal,
    /// 记录所有已挂单未成交的open订单
    pub opening_orders: HashSet<Uuid>,
    /// 策略订单管理器
    pub strategy_order_map: SStrategyTradingPairOrderMap,
    /// 当订单价格与盘口价格相差一定百分比时，需要停止挂单。
    pub cut_off_price_percentage: Decimal,
    /// 最低盈利百分比（不包括手续费）
    pub minimum_profit_percentage: Decimal,
}

/// 打包get_next_order_with_static_position的返回值
struct SNextOrderFormat {
    pub id: Option<Uuid>,
    pub price: Decimal,
    pub quantity: Decimal,
    pub base_quantity: Decimal,
    pub quote_quantity: Decimal,
}

impl SStrategyMk2 {
    pub fn new(
        target_position_ratio: Decimal,
        cut_off_price_percentage: Decimal,
        minimum_profit_percentage: Decimal,
    ) -> Self {
        let mut strategy_order_map = SStrategyTradingPairOrderMap::default();
        strategy_order_map.inner.insert(ETradingPairType::BtcUsdt, SStrategyOrderManager::new());
        strategy_order_map.inner.insert(ETradingPairType::BtcUsdtFuture, SStrategyOrderManager::new());
        strategy_order_map.inner.insert(ETradingPairType::BtcUsdCmFuture, SStrategyOrderManager::new());
        Self {
            target_position_ratio,
            opening_orders: Default::default(),
            strategy_order_map,
            cut_off_price_percentage,
            minimum_profit_percentage,
        }
    }

    pub fn default() -> Self {
        // 默认仓位50%
        // 只在盘口价的±0.1%挂单
        // 每单至少盈利0.2%
        Self::new(
            Decimal::from_f64(0.5).unwrap(),
            Decimal::from_f64(0.001).unwrap(),
            Decimal::from_f64(0.0002).unwrap(),
        )
    }

    /// 以静态仓位为目标
    /// 计算下一个订单的价格
    /// 以及下一个订单成交后的状态
    pub fn get_next_order_with_static_position(
        &self,
        direction: EOrderDirection,
        position: EOrderPosition,
        price: Decimal, // 收盘价
        base_quantity: Decimal,
        quote_quantity: Decimal,
        // strategy_order_manager: &SStrategyOrderManager,
        opened_strategy_order: Option<&SStrategyOrder>, // 平仓单对应的strategy order
    ) -> Option<SNextOrderFormat>
    {
        // 实际仓位占比
        let position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
        // 目标仓位占比
        let target_position_ratio = self.target_position_ratio;
        // 固定下单量
        // let const_quantity: Decimal = Decimal::from_f64(TRADDING_PAIR_BTC_USDT_MIN_QUANTITY).unwrap();
        let const_quantity: Decimal = Decimal::from_f64(TRADDING_PAIR_USDT_MIN_QUANTITY).unwrap() / price;
        // 最小订单价格间隙
        let const_delta_price_min = Decimal::from_f64(0.00001).unwrap() * price;
        // 最大订单价格间隙 (todo 是否需要？)
        let const_delta_price_max = Decimal::from_f64(0.00002).unwrap() * price;
        // 订单买卖操作
        let action = match direction {
            EOrderDirection::Long => {
                match position {
                    EOrderPosition::Open => { EOrderAction::Buy }
                    EOrderPosition::Close => { EOrderAction::Sell }
                }
            }
            EOrderDirection::Short => {
                match position {
                    EOrderPosition::Open => { EOrderAction::Sell }
                    EOrderPosition::Close => { EOrderAction::Buy }
                }
            }
        };

        // debug!("Mk2 仓位:{:.2?}%\t当前价格:{:?}\tbase:{:?}\tquote:{:?}\tquantity:{:?}",position_ratio*Decimal::from(100), price, base_quantity, quote_quantity,const_quantity);
        let order_quantity = match action {
            EOrderAction::Buy => {
                const_quantity
            }
            EOrderAction::Sell => {
                -const_quantity
            }
        };
        // 计算订单价格（不考虑策略订单对的情况）
        let tmp_order_price =
            if position_ratio == target_position_ratio {
                // 实际仓位等于目标仓位 均匀挂单
                base_quantity * quote_quantity * price / (
                    base_quantity * quote_quantity
                        + base_quantity * order_quantity * price
                        + order_quantity * quote_quantity
                )
            } else {
                match (action, position_ratio > target_position_ratio) {
                    (EOrderAction::Buy, true) | (EOrderAction::Sell, false) => {
                        // 实际仓位大于目标仓位 挂买单：等待下跌 降低仓位
                        // 实际仓位小于目标仓位 挂卖单：等待上涨 提升仓位
                        target_position_ratio * quote_quantity / (base_quantity + order_quantity - target_position_ratio * base_quantity)
                    }
                    (EOrderAction::Sell, true) | (EOrderAction::Buy, false) => {
                        // 实际仓位大于目标仓位 挂卖单：上涨减仓 降低仓位
                        // 实际仓位小于目标仓位 挂买单：下跌加仓 提升仓位
                        // 选择可挂单价格范围内 距离盘口最近的价格
                        price + match action {
                            EOrderAction::Buy => { -const_delta_price_min }
                            EOrderAction::Sell => { const_delta_price_min }
                        }
                    }
                }
            };
        // 计算订单价格（考虑策略订单对的情况）
        let order_price_and_id = if let EOrderPosition::Close = position {
            // 平仓情况 考虑当前已开仓的订单 平仓单价格不能使开仓单发生亏损
            if let Some(strategy_order) = opened_strategy_order {
                // 判断当前价格能否平仓？若不能平仓，则按照当开仓订单的价格进行调整。
                (
                    match direction {
                        EOrderDirection::Long => {
                            let min_sell_price = strategy_order.get_open_price() * (Decimal::from(1) + self.minimum_profit_percentage + Decimal::from_f64(MAKER_ORDER_FEE * 2.0).unwrap());
                            Some(cmp::min(min_sell_price, tmp_order_price))
                        }
                        EOrderDirection::Short => {
                            let max_buy_price = strategy_order.get_open_price() * (Decimal::from(1) - self.minimum_profit_percentage - Decimal::from_f64(MAKER_ORDER_FEE * 2.0).unwrap());
                            Some(cmp::max(max_buy_price, tmp_order_price))
                        }
                    },
                    Some(strategy_order.get_id()
                    )
                )
            } else {
                (None, None)
            }
            // let r = match direction {
            //     // todo strategy_order_manager 没有发生状态转移 导致每次peek都是同一个strategy order
            //     EOrderDirection::Long => { strategy_order_manager.peek_highest_long_opened_order() }
            //     EOrderDirection::Short => { strategy_order_manager.peek_lowest_short_opened_order() }
            // };
            // match r {
            //     Err(e) => {
            //         // 策略订单管理器异常
            //         error!("{:?}", e);
            //         (None, None)
            //     }
            //     Ok(r) => {
            //         match r {
            //             None => {
            //                 // 缺少已开仓订单 无法生成平仓单
            //                 error!("{:?}", EStrategyMk2Error::LackOpenedOrderError);
            //                 (None, None)
            //             }
            //             Some(strategy_order) => {
            //                 // 找到对应的已开仓订单
            //                 // 判断当前价格能否平仓？若不能平仓，则按照当开仓订单的价格进行调整。
            //                 (
            //                     match direction {
            //                         EOrderDirection::Long => {
            //                             let min_sell_price = strategy_order.get_open_price() * (Decimal::from(1) + self.minimum_profit_percentage + Decimal::from_f64(MAKER_ORDER_FEE * 2.0).unwrap());
            //                             Some(cmp::min(min_sell_price, tmp_order_price))
            //                         }
            //                         EOrderDirection::Short => {
            //                             let max_buy_price = strategy_order.get_open_price() * (Decimal::from(1) - self.minimum_profit_percentage - Decimal::from_f64(MAKER_ORDER_FEE * 2.0).unwrap());
            //                             Some(cmp::max(max_buy_price, tmp_order_price))
            //                         }
            //                     },
            //                     Some(strategy_order.get_id()
            //                     )
            //                 )
            //             }
            //         }
            //     }
            // }
        } else {
            // 开仓情况
            let order_price =
                match action {
                    EOrderAction::Buy => { cmp::min(price - const_delta_price_min, tmp_order_price) }
                    EOrderAction::Sell => { cmp::max(price + const_delta_price_min, tmp_order_price) }
                };
            (Some(order_price), None)
        };

        match order_price_and_id {
            (None, _) => { None }
            (Some(order_price), id) => {
                // info!("Strategy Mk2 挂单\t-\tAction:{:?}\tprice:{:?}\tquantity:{:?}", action, order_price, const_quantity);
                // 重新计算仓位、资产
                let new_base_quantity = base_quantity + match action {
                    EOrderAction::Buy => { order_quantity }
                    EOrderAction::Sell => { -order_quantity }
                };
                let new_quote_quantity = quote_quantity + match action {
                    EOrderAction::Buy => { -order_quantity * price }
                    EOrderAction::Sell => { order_quantity * price }
                };
                Some(SNextOrderFormat {
                    id,
                    price: order_price,
                    quantity: const_quantity,
                    base_quantity: new_base_quantity,
                    quote_quantity: new_quote_quantity,
                })
            }
        }
    }
}

impl TStrategy for SStrategyMk2 {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
    ) -> Vec<EStrategyAction> {
        let mut result = Vec::new();
        // 1. 从runner获取order的执行情况，将成功执行的order进行记录。
        let SRunnerParseKlineResult {
            tp_type,
            new_kline,
            new_funding_rate: _,
            order_result
        } = runner_parse_result;

        let mut strategy_order_manager = self.strategy_order_map.get_mut(&tp_type).unwrap();

        for order_result in order_result {
            // info!("strategy receive order result:\t{:?}", order_result);
            match order_result {
                ERunnerParseOrderResult::OrderExecuted(order) => {
                    // 删除已开仓的open订单
                    self.opening_orders.remove(&order.get_id());
                    // 将调整策略订单状态（挂单状态改为已完成状态）
                    let order_id = &order.get_id();
                    match strategy_order_manager.peek_mut_by_order_id(order_id) {
                        Err(e) => {
                            error!("{:?}", e);
                        }
                        Ok(strategy_order) => {
                            match strategy_order.get_state() {
                                EStrategyOrderState::Opening => {
                                    match strategy_order_manager.opened_by_order_id(order_id) {
                                        Err(e) => { error!("{:?}", e); }
                                        Ok(Err(e)) => { error!("{:?}", e); }
                                        Ok(Ok(_)) => {}
                                    }
                                }
                                EStrategyOrderState::Closing => {
                                    match strategy_order_manager.closed_by_order_id(order_id) {
                                        Err(e) => { error!("{:?}", e); }
                                        Ok(Err(e)) => { error!("{:?}", e); }
                                        Ok(Ok(_)) => {}
                                    }
                                }
                                unexpected_state => {
                                    error!("unexpected_state:{:?}\t expected Opening or Closing", unexpected_state);
                                }
                            }
                        }
                    }

                    let strategy_order = SStrategyOrder::new(&order);
                    if let Some(order) = strategy_order_manager.add(strategy_order) {
                        error!("order can not insert into strategy_order:{:?}", order);
                    }
                }
            }
        }
        //  2. 撤回所有未成交的open订单
        for uuid in self.opening_orders.iter() {
            result.push(EStrategyAction::CancelOrder(uuid.clone()));
        }
        //  3. 根据当前盘口价计算挂单：
        // 3.1 计算当前价格&仓位
        let locked_assets = tp_order_map
            .calculate_total_assets();
        let total_assets = available_assets.clone() + locked_assets;
        let tmp_base_asset = SAsset { as_type: tp_type.get_base_currency(), balance: Decimal::from(0) };
        let tmp_quote_asset = SAsset { as_type: tp_type.get_quote_currency(), balance: Decimal::from(0) };
        let assets_base = total_assets
            .get(&tp_type.get_base_currency())
            .unwrap_or(&tmp_base_asset);
        let assets_quote = total_assets
            .get(&tp_type.get_quote_currency())
            .unwrap_or(&tmp_quote_asset);
        // todo 只做多
        let direction = EOrderDirection::Long;
        let strategy_order_manager = self.strategy_order_map.get(&tp_type).unwrap();

        // 3.2 循环计算卖单
        // 价格
        let mut price = new_kline.close_price;
        // 基础货币量
        let mut base_quantity = assets_base.balance;
        // 计价货币量
        let mut quote_quantity = assets_quote.balance;
        // 实际仓位占比
        let mut position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
        // 截止价格
        let mut cut_off_price = price * (Decimal::from(1) + self.cut_off_price_percentage);

        let mut position = EOrderPosition::Close;
        let mut action = EOrderAction::Sell;
        
        // todo 优化效率
        //  现象："3.2 循环计算卖单"的循环计算逻辑 对计算速度影响较大
        //  推测原因：strategy_order_manager.long_opened_orders仅保存uuid 为了得到SStrategyOrder 需要进行频繁回表操作
        //  建议：可以给SStrategyOrderManager添加一个函数 自动生成strategy_order_manager.long_opened_orders对应的SStrategyOrder数组
        let opened_orders_vec = match direction {
                EOrderDirection::Long => { 
                    let mut result = Vec::new();
                    for (_, uuid_vec) in  strategy_order_manager.long_opened_orders.iter().rev() {
                        for uuid in uuid_vec {
                            result.push(strategy_order_manager.peek_by_id(uuid).unwrap())
                        }
                    }
                    result
                }
                EOrderDirection::Short => { 
                    let mut result = Vec::new();
                    for (_, uuid_vec) in  strategy_order_manager.short_opened_orders.iter() {
                        for uuid in uuid_vec {
                            result.push(strategy_order_manager.peek_by_id(uuid).unwrap())
                        }
                    }
                    result
                }
        };
        
        for strategy_order in opened_orders_vec {
            if price >= cut_off_price {
                break
            }
            // info!("sell_cut_off_price:{:?}\tdirection:{:?}\tposition:{:?}\tprice:{:?}\tbase_quantity:{:?}\tquote_quantity:{:?}", cut_off_price, direction,position,price,base_quantity,quote_quantity);
            match self.get_next_order_with_static_position(
                direction,
                position,
                price,
                base_quantity,
                quote_quantity,
                Some(strategy_order),
            ) {
                None => {
                    error!("error with self.get_next_order_with_static_position");
                    break;
                }
                Some(SNextOrderFormat {
                         id: id,
                         price: order_price,
                         quantity: order_quantity,
                         base_quantity: new_base_quantity,
                         quote_quantity: new_quote_quantity,
                     }) => {
                    // 新建订单
                    result.push(EStrategyAction::NewOrder(SStrategyOrderAdd {
                        id,
                        tp_type,
                        action,
                        price: order_price,
                        quantity: order_quantity,
                    }));
                    // 更新数据
                    price = order_price;
                    base_quantity = new_base_quantity;
                    quote_quantity = new_quote_quantity;
                    position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
                }
            };
        }

        // 3.3 循环计算买单
        // 重置数据
        price = new_kline.close_price;
        base_quantity = assets_base.balance;
        quote_quantity = assets_quote.balance;
        position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
        cut_off_price = price * (Decimal::from(1) - self.cut_off_price_percentage);
        position = EOrderPosition::Open;
        action = EOrderAction::Buy;

        while price > cut_off_price {
            // info!("buy_cut_off_price:{:?}\tdirection:{:?}\tposition:{:?}\tprice:{:?}\tbase_quantity:{:?}\tquote_quantity:{:?}",cut_off_price, direction, position, price, base_quantity, quote_quantity);
            match self.get_next_order_with_static_position(
                direction,
                position,
                price,
                base_quantity,
                quote_quantity,
                None,
            ) {
                None => {
                    error!("error with self.get_next_order_with_static_position");
                    break;
                }
                Some(SNextOrderFormat {
                         id: id,
                         price: order_price,
                         quantity: order_quantity,
                         base_quantity: new_base_quantity,
                         quote_quantity: new_quote_quantity,
                     }) => {
                    // 新建订单
                    result.push(EStrategyAction::NewOrder(SStrategyOrderAdd {
                        id,
                        tp_type,
                        action,
                        price: order_price,
                        quantity: order_quantity,
                    }));
                    // 更新数据
                    price = order_price;
                    base_quantity = new_base_quantity;
                    quote_quantity = new_quote_quantity;
                    position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
                }
            };
        }
        //  4. 向runner发送撤单和订单请求
        // info!("result:{:?}", result);
        result
    }

    fn verify(&mut self, tp_type: &ETradingPairType, parse_action_results: Vec<ERunnerSyncActionResult>) {
        // 5. 根据runner反馈情况，将成功挂单的order进行记录。
        let mut strategy_order_manager = self.strategy_order_map.get_mut(&tp_type).unwrap();
        for result in parse_action_results {
            // info!("strategy verify:\t{:?}", result);
            match result {
                ERunnerSyncActionResult::OrderPlaced(order) => {
                    // 尝试从opening_orders中加入该订单
                    self.opening_orders.insert(order.get_id());
                    // 记录成功的订单
                    match strategy_order_manager.peek_mut_by_order_id(&order.get_id()) {
                        Err(_) => {
                            // 找不到对应的strategy_order
                            // 生成新的strategy_order
                            let strategy_order = SStrategyOrder::new(&order);
                            if let Some(order) = strategy_order_manager.add(strategy_order) {
                                error!("order can not insert into strategy_order:{:?}", order);
                            }
                        }
                        Ok(strategy_order) => {
                            // 更新strategy_order
                            if strategy_order.get_state() != EStrategyOrderState::Closing {
                                error!("strategy_order state error, expected state Closing. strategy_order:{:?}", strategy_order);
                            } else {
                                if let Err(e) = strategy_order.bind_close(&order) {
                                    error!("{:?}", e);
                                }
                            }
                        }
                    }
                }
                ERunnerSyncActionResult::OrderCanceled(order) => {
                    // 尝试从opening_orders中删除该订单
                    self.opening_orders.remove(&order.get_id());
                    // 更新strategy_order
                    match strategy_order_manager.peek_mut_by_order_id(&order.get_id()) {
                        Err(e) => {
                            // 找不到对应的strategy_order
                            error!("ERunnerSyncActionResult::OrderCanceled(order)-strategy_order_manager中在找不到对应的strategy_order:{:?}", e);
                        }
                        Ok(strategy_order) => {
                            // 更新strategy_order
                            if strategy_order.get_state() == EStrategyOrderState::Closing {
                                if let Err(e) = strategy_order.cancel_close() {
                                    error!("{:?}", e);
                                }
                            } else if strategy_order.get_state() == EStrategyOrderState::Opening {
                                if let Err(e) = strategy_order.cancel_open() {
                                    error!("{:?}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}