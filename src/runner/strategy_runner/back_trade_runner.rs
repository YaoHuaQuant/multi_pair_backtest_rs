use std::cmp::Reverse;
use std::thread;
use chrono::{DateTime, Duration, Local};
use log::{error, info, log};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;
use crate::asset::asset_manager_v2::SAssetV2Manager;
use crate::asset::asset_v2::SAssetV2;
use crate::asset::EAssetType;
use crate::asset::trading_pair::ETradingPairType;
use crate::data::db::api::data_api_db::SDataApiDb;
use crate::data::db::api::TDataApi;
use crate::data::db::SDbClickhouse;
use crate::data::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data::kline::{SKlineData, SKlineUnitData};
use crate::order::EOrderAction;
use crate::protocol::{ERunnerParseActionResult, ERunnerParseOrderResult, EStrategyAction, SRunnerParseResult};
use crate::runner::strategy_runner::back_trade_config::{config_date_from, config_date_to, INIT_BALANCE_BTC, INIT_BALANCE_USDT, MAKER_ORDER_FEE, TAKER_ORDER_FEE};
use crate::runner::strategy_runner::order::order::{EOrderUpdate, ROrderResult, SAddOrder, SOrder};
use crate::runner::strategy_runner::order::order_manager::{ROrderManagerResult, SOrderManager};
use crate::runner::strategy_runner::trading_pair::trading_pair::STradingPair;
use crate::runner::strategy_runner::trading_pair::trading_pair_manager::{RTradingPairManagerResult, STradingPairManager};
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
    /// 数据接口
    pub data_api: A,
    /// 用户资产管理器
    pub user_asset_manager: SAssetV2Manager,
    /// 手续费管理器
    pub fee_asset_manager: SAssetV2Manager,
    /// 交易对管理器
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
        // 初始化资产asset配置
        let mut user_asset_manager = SAssetV2Manager::new();
        let mut fee_asset_manager = SAssetV2Manager::new();
        user_asset_manager.merges(vec![
            SAssetV2 { as_type: EAssetType::Btc, balance: init_balance_btc },
            SAssetV2 { as_type: EAssetType::Usdt, balance: init_balance_usdt },
        ]);
        fee_asset_manager.merges(vec![
            SAssetV2 { as_type: EAssetType::Btc, balance: Decimal::from(0) },
            SAssetV2 { as_type: EAssetType::Usdt, balance: Decimal::from(0) },
        ]);

        let mut trading_pair_manager = STradingPairManager::new();
        // 插入trading_pair配置 todo 插入更多配置
        let kline = data_api.get_kline(date_from, date_to).await.unwrap();
        let funding_rate: Option<SFundingRateData> = None; // todo 插入资金费率
        trading_pair_manager.add_trading_pair(ETradingPairType::BtcUsdt, kline, funding_rate);

        let strategy = SStrategyMkTest {};

        Self {
            data_api,
            user_asset_manager,
            fee_asset_manager,
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
            info!("当前k线时间:\t{}", current_date);
            // 输出用户当前资产状况
            info!("用户可用资产:\t{:?}", self.user_asset_manager.asset_map);
            info!("用户锁定资产:\t{:?}", self.trading_pair_manager.calculate_total_assets());
            // 输出所有挂单
            info!("所有挂单:");
            for (_, trading_pair) in self.trading_pair_manager.trading_pair_map.iter() {
                for (_, order) in trading_pair.order_manager.orders.iter() {
                    info!("订单 {} {:?}", order.get_id(), order);
                }
            }
            // 在单分钟k线内遍历所有交易对
            for (tp_type, mut trading_pair) in &mut self.trading_pair_manager.trading_pair_map {
                match Self::parse_trading_pair_kline(
                    &current_date,
                    &mut trading_pair,
                    &mut self.user_asset_manager,
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
                            &mut self.user_asset_manager,
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
        user_asset_manager: &mut SAssetV2Manager,
        tp_type: &ETradingPairType,
    ) -> RBackTradeRunnerResult<SRunnerParseResult>
    {
        // 根据K线 结算订单数据 结算资产数据 todo 收取手续费-手续费累计给fee_manager
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
                Some(order) => { Some(order.get_price()) }
            };
            let lowest_sell_price = match selling_order_heap.peek() {
                None => { None }
                Some(Reverse(order)) => { Some(order.get_price()) }
            };
            log::debug!("盘口信息 - 交易对: {:?}\t买一价格:{:?}\t卖一价格:{:?}", tp_type, highest_buy_price, lowest_sell_price);
        }

        // 买单结算 用quote_currency换base_current
        while let Some(order) = buying_order_heap.peek() {
            // 操作方向校验
            if (order.get_action() != EOrderAction::Buy) {
                log::error!("EOrderAction Error: Expected Buy - Actually {:?}", order.get_action());
                // return Err(EBackTradeRunnerError::OrderActionError(order.action));
            }
            // 挂单价格大于等于当前k线最低价格，则买单成交。
            if order.get_price() < kline_unit_data.low_price {
                break;
            }
            info!("结算买单: {:?}", order);
            let mut order = buying_order_heap.pop().unwrap();
            // 校验资产
            let base_quantity = order.get_quantity();
            let mut base_asset = user_asset_manager.get_mut(base_asset_type).unwrap();
            // 结算资产
            // 提取订单锁定的计价资产 生成基础资产
            match order.execute() {
                // _consumed_quote_asset会被自动析构 代表订单的锁定资产被消耗
                Ok(_consumed_quote_asset) => {
                    // 用户获得基础资产
                    let obtain_base_asset = SAssetV2 {
                        as_type: base_asset_type,
                        balance: base_quantity,
                    };
                    let _ = base_asset.merge(obtain_base_asset);
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }

        // 卖单结算 用base_current换quote_currency
        while let Some(Reverse(order)) = selling_order_heap.peek() {
            // 操作方向校验
            if (order.get_action() != EOrderAction::Sell) {
                log::error!("EOrderAction Error: Expected Sell - Actually {:?}", order.get_action());
                // return Err(EBackTradeRunnerError::OrderActionError(order.action));
            }
            // 挂单价格大于等于当前k线最低价格，则买单成交。
            if order.get_price() > kline_unit_data.high_price {
                break;
            }
            info!("结算卖单: {:?}", order);
            let Reverse(mut order) = selling_order_heap.pop().unwrap();
            // 校验资产
            let quote_quantity = order.get_amount();
            let mut quote_asset = user_asset_manager.get_mut(quote_asset_type).unwrap();
            // 结算资产
            // 提取订单锁定的基础资产 生成计价资产
            match order.execute() {
                // _consumed_base_asset会被自动析构 代表订单的锁定资产被消耗
                Ok(_consumed_base_asset) => {
                    // 用户获得基础资产
                    let obtain_quote_asset = SAssetV2 {
                        as_type: quote_asset_type,
                        balance: quote_quantity,
                    };
                    let _ = quote_asset.merge(obtain_quote_asset);
                    order_results.push(ERunnerParseOrderResult::OrderExecuted(order));
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
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
        user_asset_manager: &mut SAssetV2Manager,
    ) -> Vec<ERunnerParseActionResult>
    {
        // 根据策略行为，校验、调整订单数据。
        let mut parse_action_result: Vec<ERunnerParseActionResult> = Vec::new();
        // 根据action类型进行分类 之后进行分批批处理
        let mut add_orders: Vec<SAddOrder> = Vec::new();
        let mut cancel_orders: Vec<Uuid> = Vec::new();

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
                        cancel_orders.push(uuid)
                    }
                }
            }
        }

        // 优先处理取消的订单（需要做堆重构）
        match order_manager.remove_orders(cancel_orders) {
            Err(e) => { error!("Error: {:?}", e); }
            Ok(removed_order_vec) => {
                // 订单执行成功 进行资产结算
                for mut order in removed_order_vec {
                    info!("取消订单: {:?}", order);
                    // 订单成功取消 释放锁定资产
                    if let Some(asset) = order.cancel() {
                        let mut user_asset = match order.get_action() {
                            EOrderAction::Buy => { user_asset_manager.get_mut(quote_asset_type).unwrap() }
                            EOrderAction::Sell => { user_asset_manager.get_mut(base_asset_type).unwrap() }
                        };
                        if let Err(e) = user_asset.merge(asset) {
                            error!("Error: {:?}", e);
                        }
                        parse_action_result.push(ERunnerParseActionResult::OrderCanceled(order));
                    }
                }
            }
        }

        // 处理新增订单 资产结算
        for add_order in add_orders {
            // info!("Start: add_order");
            // info!("add_order:{:?}", add_order);
            let mut new_order = SOrder::new(add_order.price, add_order.quantity, add_order.action);
            let (necessary_asset_quantity, user_asset) = match add_order.action {
                EOrderAction::Buy => {
                    (add_order.price * add_order.quantity, user_asset_manager.get_mut(quote_asset_type).unwrap())
                }
                EOrderAction::Sell => {
                    (add_order.quantity, user_asset_manager.get_mut(base_asset_type).unwrap())
                }
            };
            // info!("necessary_asset_quantity:{:?}", necessary_asset_quantity);
            if let Ok(locked_quote_asset) = user_asset.split(necessary_asset_quantity) {
                // info!("locked_quote_asset:{:?}", locked_quote_asset);
                if let Err(e) = new_order.submit(locked_quote_asset) {
                    error!("Error: {:?}", e);
                }
                order_manager.insert_order(new_order);
                parse_action_result.push(ERunnerParseActionResult::OrderPlaced(new_order));
                info!("新增订单: {:?}", new_order);
            }
            // info!("Finish: add_order");
        }
        parse_action_result
    }
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