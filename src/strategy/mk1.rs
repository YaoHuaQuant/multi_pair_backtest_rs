//! StrategyMk1等仓位占比策略
//! 策略明细：
//! 1. 从当前盘口向下挂买单，向上挂卖单，根据当前资产和仓位，调整挂单价格。
//! 2. 每当新一轮k线开始，取消所有旧挂单，重新计算新挂单的价格。
//! 3. 挂单遵循少量多次，挂单量以config中的基础资产的quantity为准
//! 4. 挂单价格策略：
//!     只在盘口价格±5%进行挂单
//!     只考虑短时单边上涨和单边下跌的场景：
//!         单边上涨时，每一笔订单卖出后，仓位均回到目标值。
//!         单边下跌时，每一笔订单买入后，仓位均回到目标值。
//!
//!
//! 策略逻辑顺序：
//! 1. 从runner获取order的执行情况，将成功执行的order进行记录。
//!
//! 2. 撤回所有剩余的订单
//!
//! 3. 根据当前盘口价计算挂单：
//! 3.1. 计算买单，根据当前价格，向上计算下一个挂单价位，生成新的订单。
//! 3.2 如果最新挂单的价格不高于盘口+5%，则循环计算2.1~2.2。
//! 3.3. 计算卖单，根据当前价格，向下计算下一个挂单价位，生成新的订单。
//! 3.4 如果最新挂单的价格不低于盘口-5%，则循环计算2.3~2.4。
//!
//! 3.5 价格计算，设目标价位为Ot，实际价位O，基础资产量为b，计价资产量为q，买入基本资产量Db(其最小值为固定参数)，当前价格P0
//!     挂单价格与quantity计算：
//!     1)若Ot>O，即目标价位大于实际价位，
//!         则最低卖单挂单价格Ot\*q/(b-Db-Ot\*b)，挂单量为Db的最小值
//!         最高买单价格为当前盘口价，挂单量Ot\*q/R - (1-Ot)\*b
//!         如果挂单价格超过盘口的±5%，则不挂单
//!     2)若Ot<O，即目标价位小于实际价位，
//!         则最高买单挂单价格Ot\*q/(b+Db-Ot\*b)，挂单量为Db的最小值
//!         最低卖单价格为当前盘口价，挂单量Ot\*q/R - (1-Ot)\*b
//!         如果挂单价格超过盘口的±5%，则不挂单
//!     3)若Ot=O，即目标价位等于实际价位，
//!
//! 4. 向runner发送撤单和订单请求
//!
//! 5. 根据runner反馈情况，将成功挂单的order进行记录。
//!

use std::collections::HashSet;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;
use crate::config::{SDebugConfig, trading_pair::btc_usdt::TRADDING_PAIR_BTC_USDT_MIN_QUANTITY};
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::EOrderAction;
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::protocol::{ERunnerParseOrderResult, ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult, SStrategyOrderAdd};
use crate::strategy::logger::SStrategyLogger;
use crate::strategy::TStrategy;

pub struct SStrategyMk1 {
    /// 目标仓位占比
    pub target_position_ratio: Decimal,
    /// 仅记录存在于订单簿的订单
    pub order_list: HashSet<Uuid>,
    /// 截止挂单价差幅度
    /// 当订单价格与盘口价格相差一定百分比时，需要停止挂单。
    pub cut_off_price_percentage: Decimal,
}

impl SStrategyMk1 {
    pub fn new(target_position_ratio: Decimal, cut_off_price_percentage: Decimal) -> Self {
        Self {
            target_position_ratio,
            order_list: Default::default(),
            cut_off_price_percentage,
        }
    }
}

impl Default for SStrategyMk1 {
    fn default() -> Self {
        // 默认仓位50%
        Self::new(
            Decimal::from_f64(0.5).unwrap(),
            Decimal::from_f64(0.0001).unwrap(),
        )
    }
}

impl TStrategy for SStrategyMk1 {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
        debug_config: &SDebugConfig,
    ) -> Vec<EStrategyAction> {
        let mut result = Vec::new();
        // 1. 从runner获取order的执行情况，将成功执行的order进行记录。
        let SRunnerParseKlineResult {
            tp_type,
            new_kline,
            new_funding_rate: _,
            order_result
        } = runner_parse_result;
        for order_result in order_result {
            // info!("strategy receive order result:\t{:?}", order_result);
            match order_result {
                ERunnerParseOrderResult::OrderExecuted(order) => {
                    // 删除已执行的订单
                    self.order_list.remove(&order.get_id());
                }
            }
        }
        //  2. 撤回所有剩余的订单
        for uuid in self.order_list.iter() {
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
        // 价格
        let price = new_kline.close_price;
        // 基础货币量
        let base_quantity = assets_base.balance;
        // 计价货币量
        let quote_quantity = assets_quote.balance;
        // 实际仓位占比
        let actual_position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
        // 目标仓位占比
        let target_position_ratio = self.target_position_ratio;
        // 最小下单量
        let const_quantity_min: Decimal = Decimal::from_f64(TRADDING_PAIR_BTC_USDT_MIN_QUANTITY).unwrap();
        // 最小订单价格间隙
        let const_delta_price_min = Decimal::from_f64(0.00001).unwrap() * price;

        // 3.2 循环计算卖单
        let mut tmp_position_ratio = actual_position_ratio;
        let mut tmp_price = price;
        let mut tmp_base_quantity = base_quantity;
        let mut tmp_quote_quantity = quote_quantity;
        let mut tmp_quantity = const_quantity_min;
        let cut_off_sell_price = price * (Decimal::from(1) + self.cut_off_price_percentage);
        let cut_off_buy_price = price * (Decimal::from(1) - self.cut_off_price_percentage);
        // debug!("target_position_ratio:{:?}", target_position_ratio);
        // debug!("cut_off_price:\tbuy-{:?}\tsell-:{:?}", cut_off_buy_price, cut_off_sell_price);
        while tmp_price < cut_off_sell_price {
            // debug!("仓位:{:.2?}%\t订单价格:{:?}\tbase:{:?}\tquote:{:?}\tquantity:{:?}",tmp_position_ratio*Decimal::from(100), tmp_price, tmp_base_quantity, tmp_quote_quantity, tmp_quantity);
            if tmp_position_ratio > target_position_ratio {
                // 实际仓位大于目标仓位 上涨减仓 降低仓位
                tmp_price += const_delta_price_min;
                tmp_quantity = (Decimal::from(1) - target_position_ratio) * tmp_base_quantity - target_position_ratio * tmp_quote_quantity / tmp_price;
            } else if tmp_position_ratio < target_position_ratio {
                // 实际仓位小于目标仓位 等待上涨 提升仓位
                tmp_quantity = const_quantity_min;
                tmp_price = target_position_ratio * tmp_quote_quantity / (tmp_base_quantity - tmp_quantity - target_position_ratio * tmp_base_quantity);
            } else {
                // 实际仓位等于目标仓位 均匀挂单
                tmp_quantity = const_quantity_min;
                tmp_price = tmp_base_quantity * tmp_quote_quantity * tmp_price / (
                    tmp_base_quantity * tmp_quote_quantity
                        - tmp_base_quantity * tmp_quantity * tmp_price
                        - tmp_quantity * tmp_quote_quantity
                );
            }
            // 挂单
            // info!("Strategy Mk2 挂单\t-\tAction:{:?}\tprice:{:?}\tquantity:{:?}", EOrderAction::Sell, tmp_price, tmp_quantity);
            result.push(EStrategyAction::NewOrder(SStrategyOrderAdd {
                id: None,
                tp_type,
                action: EOrderAction::Sell,
                price: tmp_price,
                quantity: tmp_quantity,
            }));
            // 重新计算仓位、资产
            tmp_position_ratio = tmp_base_quantity * tmp_price / (tmp_base_quantity * tmp_price + tmp_quote_quantity);
            tmp_base_quantity -= tmp_quantity;
            tmp_quote_quantity += tmp_quantity * tmp_price;
        }

        // 3.3 循环计算买单
        tmp_position_ratio = actual_position_ratio;
        tmp_price = price;
        tmp_base_quantity = base_quantity;
        tmp_quote_quantity = quote_quantity;
        tmp_quantity = const_quantity_min;
        while tmp_price > cut_off_buy_price {
            // info!("仓位:{:.2?}%\t订单价格:{:?}\tbase:{:?}\tquote:{:?}\tquantity:{:?}",tmp_position_ratio*Decimal::from(100), tmp_price, tmp_base_quantity, tmp_quote_quantity, tmp_quantity);
            if tmp_position_ratio > target_position_ratio {
                // 实际仓位大于目标仓位 等待下跌 降低仓位
                tmp_quantity = const_quantity_min;
                tmp_price = target_position_ratio * tmp_quote_quantity / (tmp_base_quantity + tmp_quantity - target_position_ratio * tmp_base_quantity);
            } else if tmp_position_ratio < target_position_ratio {
                // 实际仓位小于目标仓位 下跌加仓 提升仓位
                tmp_price -= const_delta_price_min;
                tmp_quantity = -(Decimal::from(1) - target_position_ratio) * tmp_base_quantity + target_position_ratio * tmp_quote_quantity / tmp_price;
            } else {
                // 实际仓位等于目标仓位 均匀挂单
                tmp_quantity = const_quantity_min;
                tmp_price = tmp_base_quantity * tmp_quote_quantity * tmp_price / (
                    tmp_base_quantity * tmp_quote_quantity
                        + tmp_base_quantity * tmp_quantity * tmp_price
                        + tmp_quantity * tmp_quote_quantity
                );
            }
            // 挂单
            // info!("Strategy Mk2 挂单\t-\tAction:{:?}\tprice:{:?}\tquantity:{:?}", EOrderAction::Buy, tmp_price, tmp_quantity);
            result.push(EStrategyAction::NewOrder(SStrategyOrderAdd {
                id: None,
                tp_type,
                action: EOrderAction::Buy,
                price: tmp_price,
                quantity: tmp_quantity,
            }));
            // 重新计算仓位、资产
            tmp_position_ratio = tmp_base_quantity * tmp_price / (tmp_base_quantity * tmp_price + tmp_quote_quantity);
            tmp_base_quantity += tmp_quantity;
            tmp_quote_quantity -= tmp_quantity * tmp_price;
        }

        //  4. 向runner发送撤单和订单请求
        result
    }

    fn verify(
        &mut self,
        tp_type: &ETradingPairType,
        parse_action_results: Vec<ERunnerSyncActionResult>,
        debug_config: &SDebugConfig,
    ) {
        // 5. 根据runner反馈情况，将成功挂单的order进行记录。
        for result in parse_action_results {
            // info!("strategy verify:\t{:?}", result);
            match result {
                ERunnerSyncActionResult::OrderPlaced(order, _) => {
                    // 记录成功的订单
                    self.order_list.insert(order.get_id());
                }
                ERunnerSyncActionResult::OrderCanceled(order) => {
                    // 删除已撤销的订单
                    self.order_list.remove(&order.get_id());
                }
            }
        }
    }

    fn get_log_info(&self) -> SStrategyLogger {
        SStrategyLogger::none()
    }

    fn get_position(&self, _time: DateTime<Local>) -> Option<Decimal> {
        Some(Decimal::from(0))
    }
}