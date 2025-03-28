use std::cmp::Reverse;
use std::thread;
use chrono::{DateTime, Duration, Local};
use log::{error, log};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;
use crate::asset::asset::{SAsset};
use crate::asset::asset_manager::SAssetManager;
use crate::asset::EAssetType;
use crate::asset::trading_pair::ETradingPairType;
use crate::data::db::api::data_api_db::SDataApiDb;
use crate::data::db::api::TDataApi;
use crate::data::db::SDbClickhouse;
use crate::data::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data::kline::{SKlineData, SKlineUnitData};
use crate::order::EOrderAction;
use crate::protocol::{ERunnerParseActionResult, ERunnerParseOrderResult, EStrategyAction, PriceChange, SRunnerParseResult};
use crate::runner::strategy_runner::back_trade_config::{config_date_from, config_date_to, INIT_BALANCE_BTC, INIT_BALANCE_USDT, MAKER_ORDER_FEE, TAKER_ORDER_FEE};
use crate::runner::strategy_runner::order::runner_order::{EOrderUpdate, SAddOrder, SOrder};
use crate::runner::strategy_runner::order::runner_order_manager::{EOrderManagerUpdate, ROrderManagerResult, SOrderManager, SOrderUuidAndUpdate};
use crate::runner::strategy_runner::trading_pair::runner_trading_pair::STradingPair;
use crate::runner::strategy_runner::trading_pair::runner_trading_pair_manager::{RTradingPairManagerResult, STradingPairManager};
use crate::strategy::strategy_mk_test::SStrategyMkTest;
use crate::strategy::TStrategy;

pub type RBackTradeRunnerResult<T> = Result<T, EBackTradeRunnerError>;

#[derive(Debug)]
pub enum EBackTradeRunnerError {
    KlineNotFoundError(ETradingPairType, DateTime<Local>),
    OrderActionError(EOrderAction),
    AssetLockedNotEnoughError(EAssetType),
}

/// 回测执行器
#[derive(Debug)]
pub struct SBackTradeRunner<A: TDataApi, S: TStrategy> {
    pub data_api: A,
    pub asset_manager: SAssetManager,
    pub trading_pair_manager: STradingPairManager,

    pub taker_order_fee: Decimal,
    pub maker_order_fee: Decimal,
    pub init_balance_usdt: Decimal,
    pub init_balance_btc: Decimal,
    pub date_from: DateTime<Local>,
    pub date_to: DateTime<Local>,

    pub strategy: S,
}

impl SBackTradeRunner<SDataApiDb, SStrategyMkTest> {
    pub async fn new() -> Self {
        let taker_order_fee = Decimal::from_f64(TAKER_ORDER_FEE).unwrap();
        let maker_order_fee = Decimal::from_f64(MAKER_ORDER_FEE).unwrap();
        let init_balance_usdt = Decimal::from_f64(INIT_BALANCE_USDT).unwrap();
        let init_balance_btc = Decimal::from_f64(INIT_BALANCE_BTC).unwrap();
        let date_from = config_date_from();
        let date_to = config_date_to();

        let db = SDbClickhouse::new();
        let data_api = SDataApiDb::new(db);
        let mut asset_manager = SAssetManager::new();
        // 插入资产asset配置
        asset_manager.add_assets(vec![EAssetType::Btc, EAssetType::Usdt, EAssetType::BtcUsdCmFuture, EAssetType::BtcUsdtFuture]);
        // 初始化资产
        asset_manager.add(EAssetType::Btc, init_balance_btc).unwrap();
        asset_manager.add(EAssetType::Usdt, init_balance_usdt).unwrap();

        let mut trading_pair_manager = STradingPairManager::new();
        // 插入trading_pair配置 todo 插入更多配置
        let kline = data_api.get_kline(date_from, date_to).await.unwrap();
        let funding_rate: Option<SFundingRateData> = None; // todo 插入资金费率
        trading_pair_manager.add_trading_pair(ETradingPairType::BtcUsdt, kline, funding_rate);

        let strategy = SStrategyMkTest {};

        Self {
            data_api,
            asset_manager,
            trading_pair_manager,
            taker_order_fee,
            maker_order_fee,
            init_balance_usdt,
            init_balance_btc,
            date_from,
            date_to,
            strategy,
        }
    }

    pub fn run(&mut self) -> RBackTradeRunnerResult<()> {
        // 循环遍历k线 根据时间间隔1分钟
        let mut current_date = self.date_from;
        while current_date < self.date_to {
            log::debug!("当前k线时间:\t{}", current_date);
            // 在单分钟k线内遍历所有交易对
            for (tp_type, mut trading_pair) in &mut self.trading_pair_manager.trading_pair_map {
                match Self::parse_trading_pair_kline(
                    &current_date,
                    &mut trading_pair,
                    &mut self.asset_manager,
                    &tp_type,
                ) {
                    Err(e) => {
                        error!("{:?}", e);
                    }
                    Ok(runner_parse_result) => {
                        // 将增量数据传输给策略模块，获取策略行为。
                        // 将策略行为进行排序 cancel order在前 new order在后
                        let strategy_actions = self.strategy.run(runner_parse_result);

                        // 根据策略行为，调整订单数据。
                        let parse_action_results = Self::action_verify_update(
                            strategy_actions,
                            &mut trading_pair,
                            &mut self.asset_manager,
                        );
                        // 向策略模块反馈校验、调整结果
                        self.strategy.verify(parse_action_results);
                    }
                }
            }

            // 时间递增 1 分钟
            current_date += Duration::minutes(1);

            // thread::sleep(std::time::Duration::from_secs(2));
        }
        // todo 回测结束 输出结果
        Ok(())
    }

    /// 处理新的k线和资金费率，更新订单和资产，记录增量处理结果。
    fn parse_trading_pair_kline(
        current_date: &DateTime<Local>,
        trading_pair: &mut STradingPair,
        asset_manager: &mut SAssetManager,
        tp_type: &ETradingPairType,
    ) -> RBackTradeRunnerResult<SRunnerParseResult>
    {
        // 根据K线 结算订单数据 结算资产数据 todo 收取手续费
        let mut order_results: Vec<ERunnerParseOrderResult> = Vec::new(); // 订单已成交列表
        let base_asset_type = trading_pair.base_currency;
        let quote_asset_type = trading_pair.quote_currency;

        // 获取k线数据
        let option_kline = trading_pair.get_kline(current_date);
        if let None = option_kline {
            return Err(EBackTradeRunnerError::KlineNotFoundError(tp_type.clone(), current_date.clone()));
        }
        let kline_unit_data = option_kline.unwrap().clone();

        log::debug!("K线信息 - 交易对: {:?}\t开盘价:{}\t收盘价:{}\t最高价:{}\t最低价:{}", tp_type, kline_unit_data.open_price, kline_unit_data.close_price, kline_unit_data.high_price, kline_unit_data.low_price);

        // 查询当前k线对应的资金费率
        let funding_rate = trading_pair.get_funding_rate(current_date).unwrap_or(&Decimal::from(0)).clone();

        let mut order_manager = &mut trading_pair.order_manager;

        let mut buying_order_heap = &mut order_manager.buying_order_heap;
        let mut selling_order_heap = &mut order_manager.selling_order_heap;
        { // debug
            let highest_buy_price = match buying_order_heap.peek() {
                None => { None }
                Some(order) => { Some(order.price) }
            };

            let lowest_sell_price = match selling_order_heap.peek() {
                None => { None }
                Some(Reverse(order)) => { Some(order.price) }
            };

            log::debug!("盘口信息 - 交易对: {:?}\t买一价格:{:?}\t卖一价格:{:?}", tp_type, highest_buy_price, lowest_sell_price);
        }

        // 买单结算 用quote_currency换base_current
        while let Some(order) = buying_order_heap.peek() {
            // 操作方向校验
            if (order.action != EOrderAction::Buy) {
                log::error!("EOrderAction Error: Expected Buy - Actually {:?}", order.action);
                // return Err(EBackTradeRunnerError::OrderActionError(order.action));
            }
            // 挂单价格大于等于当前k线最低价格，则买单成交。
            if order.price >= kline_unit_data.low_price {
                // 校验资产
                let quote_currency_quantity = order.price * order.quantity;
                let base_current_quantity = order.quantity;
                let mut quote_asset = asset_manager.get_mut(quote_asset_type).unwrap();
                // 结算资产
                if quote_asset.withdraw_locked(quote_currency_quantity).is_ok() {
                    let mut base_asset = asset_manager.get_mut(base_asset_type).unwrap();
                    base_asset.add(base_current_quantity);
                    // 订单转移至已成交列表
                    let order = buying_order_heap.pop().unwrap();
                    log::info!("Buy Order Executed {:?} ", order);
                    order_results.push(ERunnerParseOrderResult::OrderExecuted(order));
                    continue;
                } else {
                    // 结算失败 抛出异常
                    log::debug!("Locked Asset {:?} not enough: {} less than {}", quote_asset.as_type, quote_asset.get_locked(), quote_currency_quantity);
                    // return Err(EBackTradeRunnerError::AssetLockedNotEnoughError(quote_asset.as_type));
                }
            }
            break;
        }

        // 卖单结算 用base_current换quote_currency
        while let Some(Reverse(order)) = selling_order_heap.peek() {
            // 操作方向校验
            if (order.action != EOrderAction::Sell) {
                log::error!("EOrderAction Error: Expected Sell - Actually {:?}", order.action);
                // return Err(EBackTradeRunnerError::OrderActionError(order.action));
            }
            // 挂单价格小等于当前k线最高价格，则买单成交。
            if order.price <= kline_unit_data.high_price {
                // 校验资产

                let quote_currency_quantity = order.price * order.quantity;
                let base_current_quantity = order.quantity;

                let mut base_asset = asset_manager.get_mut(base_asset_type).unwrap();
                // 结算资产
                if base_asset.withdraw_locked(base_current_quantity).is_ok() {
                    let mut quote_asset = asset_manager.get_mut(quote_asset_type).unwrap();
                    quote_asset.add(quote_currency_quantity);
                    // 订单转移至已成交列表
                    let order = selling_order_heap.pop().unwrap().0;
                    log::info!("Sell Order Executed {:?} ", order);
                    order_results.push(ERunnerParseOrderResult::OrderExecuted(order));
                    continue;
                } else {
                    // 结算失败 抛出异常
                    log::debug!("Locked Asset {:?} not enough: {} less than {}", base_asset.as_type, base_asset.get_locked(), quote_currency_quantity);
                    // return Err(EBackTradeRunnerError::AssetLockedNotEnoughError(quote_asset.as_type));
                }
            }
            break;
        }


        // 将k线和订单结算结果 用于反馈给strategy
        let runner_parse_result = SRunnerParseResult {
            date_time: current_date.clone(),
            new_kline: kline_unit_data,
            new_funding_rate: funding_rate,
            order_result: order_results,
        };

        Ok(runner_parse_result)
    }

    /// 根据策略行为，调整订单数据。
    fn action_verify_update(
        strategy_actions: Vec<EStrategyAction>,
        trading_pair: &mut STradingPair,
        asset_manager: &mut SAssetManager,
    ) -> Vec<ERunnerParseActionResult>
    {
        // 根据策略行为，校验、调整订单数据。
        let mut parse_action_result: Vec<ERunnerParseActionResult> = Vec::new();
        // 根据action类型进行分类 之后进行分批批处理
        let mut add_orders: Vec<SAddOrder> = Vec::new();
        let mut cancel_or_modify_orders: Vec<SOrderUuidAndUpdate> = Vec::new();

        let mut order_manager = &mut trading_pair.order_manager;
        let base_asset_type = trading_pair.base_currency;
        let quote_asset_type = trading_pair.quote_currency;

        for action in strategy_actions {
            match action {
                EStrategyAction::NewOrder(order) => {
                    add_orders.push(order)
                }
                EStrategyAction::CancelOrder(uuid) => {
                    // 判断uuid是否有效
                    if let Some(_) = order_manager.peek_order(uuid) {
                        cancel_or_modify_orders.push(SOrderUuidAndUpdate {
                            uuid,
                            update: EOrderManagerUpdate::Remove,
                        })
                    }
                }
                EStrategyAction::ModifyOrder(uuid, price_change, quantity_change) => {
                    if price_change.is_some() && quantity_change.is_some() {
                        let price = price_change.unwrap();
                        let quantity = quantity_change.unwrap();
                        cancel_or_modify_orders.push(SOrderUuidAndUpdate {
                            uuid,
                            update: EOrderManagerUpdate::Update(EOrderUpdate::PriceAndQuantity(price, quantity)),
                        })
                    } else if let Some(price) = price_change {
                        cancel_or_modify_orders.push(SOrderUuidAndUpdate {
                            uuid,
                            update: EOrderManagerUpdate::Update(EOrderUpdate::Price(price)),
                        })
                    } else if let Some(quantity) = quantity_change {
                        cancel_or_modify_orders.push(SOrderUuidAndUpdate {
                            uuid,
                            update: EOrderManagerUpdate::Update(EOrderUpdate::Quantity(quantity)),
                        })
                    }
                }
            }
        }

        // 优先处理取消和调整的订单（需要做堆重构）
        match order_manager.update_or_remove_orders(cancel_or_modify_orders) {
            Err(e) => { log::error!("Error: {:?}", e); }
            Ok(success_list) => {
                // 执行成功 todo 进行资产结算
                for order_uuid_and_update in success_list {
                    let SOrderUuidAndUpdate { uuid, update } = order_uuid_and_update;
                    if let Some(order) = order_manager.peek_order(uuid) {
                        match update {
                            EOrderManagerUpdate::Update(order_update) => {
                                // 修改订单 重新锁定&解锁资产
                                match order_update {
                                    EOrderUpdate::State(_) => {}
                                    EOrderUpdate::Price(price) => {}
                                    EOrderUpdate::Quantity(quantity) => {}
                                    EOrderUpdate::PriceAndQuantity(price, quantity) => {}
                                }

                                parse_action_result.push(ERunnerParseActionResult::OrderModified(order.clone()));
                            }
                            EOrderManagerUpdate::Remove => {
                                // 取消订单 释放锁定资产
                                match order.action {
                                    EOrderAction::Buy => {
                                        // 释放定价资产
                                        let mut quote_asset = asset_manager.get_mut(quote_asset_type).unwrap();
                                        let necessary_quote_asset = order.price * order.quantity;
                                        if quote_asset.unlock(necessary_quote_asset).is_ok() {
                                            parse_action_result.push(ERunnerParseActionResult::OrderCanceled(uuid));
                                        }
                                    }
                                    EOrderAction::Sell => {
                                        // 释放基本资产
                                        let mut base_asset = asset_manager.get_mut(base_asset_type).unwrap();
                                        let necessary_base_asset = order.quantity;
                                        if base_asset.unlock(necessary_base_asset).is_ok() {
                                            parse_action_result.push(ERunnerParseActionResult::OrderCanceled(uuid));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 处理新增订单 资产结算
        for add_order in add_orders {
            match add_order.action {
                EOrderAction::Buy => {
                    let necessary_quote_asset = add_order.price * add_order.quantity;
                    let mut quote_asset = asset_manager.get_mut(quote_asset_type).unwrap();
                    if quote_asset.lock(necessary_quote_asset).is_ok() {
                        order_manager.add_order(add_order);
                    }
                }
                EOrderAction::Sell => {
                    let necessary_base_asset = add_order.quantity;
                    let mut base_asset = asset_manager.get_mut(base_asset_type).unwrap();
                    if base_asset.lock(necessary_base_asset).is_ok() {
                        order_manager.add_order(add_order);
                    }
                }
            }
        }

        parse_action_result
    }

    /// 向策略模块反馈校验、调整结果
    fn response_verify_result(&mut self, action_result: Vec<ERunnerParseActionResult>) {}
}


#[cfg(test)]
mod tests {
    use crate::data::db::api::data_api_db::SDataApiDb;
    use crate::runner::strategy_runner::back_trade_runner::SBackTradeRunner;
    use crate::strategy::strategy_mk_test::SStrategyMkTest;

    #[tokio::test]
    pub async fn test() {
        let mut runner = SBackTradeRunner::<SDataApiDb, SStrategyMkTest>::new().await;
        println!("{:?}", runner);

        runner.run().unwrap();
        println!("{:?}", runner);
    }
}