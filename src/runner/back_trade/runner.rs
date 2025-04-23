use std::collections::HashMap;
use chrono::{DateTime, Duration, Local};
use log::{debug, error, info};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::{
    data_runtime::{
        asset::{
            asset::SAsset,
            EAssetType,
        },
        order::EOrderAction,
        order::order::{SOrder},
    },
    data_source::{
        data_manager::SDataManager,
        db::api::TDataApi,
        kline::SKlineUnitData,
        trading_pair::ETradingPairType,
    },
    protocol::{ERunnerSyncActionResult, ERunnerParseOrderResult, EStrategyAction, SRunnerParseKlineResult},
    runner::{
        back_trade::config::SBackTradeRunnerConfig,
        TRunner,
    },
    strategy::TStrategy,
};
use crate::data_runtime::user::SUser;
use crate::protocol::SStrategyOrderAdd;
use crate::runner::logger::data_logger::SDataLogger;
use crate::runner::logger::kline_unit::SDataLogKlineUnit;
use crate::runner::logger::transfer_unit::{SDataLogTransferExecutedUnit, SDataLogTransferUnfulfilledUnit, SDataLogTransferUnit};
use crate::runner::logger::user_unit::SDataLogUserUnit;

pub type RBackTradeRunnerResult<T> = Result<T, EBackTradeRunnerError>;

#[derive(Debug)]
pub enum EBackTradeRunnerError {
    KlineNotFoundError(ETradingPairType, DateTime<Local>),
    OrderActionError(EOrderAction),
    AssetLockedNotEnoughError(EAssetType),
    DenominateSupportOnlyBtcAndUsdtError(EAssetType),
}

/// 回测执行器
#[derive(Debug)]
pub struct SBackTradeRunner<D: TDataApi> {
    /// 配置
    pub config: SBackTradeRunnerConfig,
    /// 数据管理器
    pub data_manager: SDataManager<D>,
    /// 记录最新的交易对报价
    // pub trading_pair_prices: HashMap<ETradingPairType, Decimal>,
    /// 数据日志
    pub data_logger: SDataLogger,
}

impl<S: TStrategy, D: TDataApi> TRunner<S> for SBackTradeRunner<D> {
    fn run(&mut self, users: &mut Vec<SUser<S>>) {
        // 循环遍历k线 根据时间间隔1分钟
        let mut current_date = self.config.date_from;
        while current_date < self.config.date_to {
            info!("当前k线时间:\t{}", current_date);

            // 用于记录报价
            let mut trading_pair_klines: HashMap<ETradingPairType, SKlineUnitData> = HashMap::new();
            let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();

            // 用于记录交易量 key-user_id value-transfer_info
            let mut transfer_info_map: HashMap<Uuid, SDataLogTransferUnit> = HashMap::new();

            // 在单分钟k线内遍历所有交易对
            for (tp_type, trading_pair) in self.data_manager.trading_pair_map.inner.iter() {
                // 获取k线数据
                let option_kline = trading_pair.get_kline(&current_date);
                if let None = option_kline {
                    let err = EBackTradeRunnerError::KlineNotFoundError(tp_type.clone(), current_date.clone());
                    error!("{:?}", err);
                    return ();
                }
                let kline_unit_data = option_kline.unwrap().clone();
                // 查询当前k线对应的资金费率
                let funding_rate = trading_pair.get_funding_rate(&current_date).unwrap_or(&Decimal::from(0)).clone();

                debug!("K线信息 - 交易对: {:?}\t开盘价:{}\t收盘价:{}\t最高价:{}\t最低价:{}\t资金费率:{}", tp_type, kline_unit_data.open_price, kline_unit_data.close_price, kline_unit_data.high_price, kline_unit_data.low_price, funding_rate);

                // 记录日志
                trading_pair_klines.insert(tp_type.clone(), kline_unit_data);
                trading_pair_prices.entry(tp_type.clone())
                    .and_modify(|e| *e = kline_unit_data.close_price)
                    .or_insert(kline_unit_data.close_price);

                for user in users.iter_mut() {
                    match self.parse_new_kline(
                        tp_type,
                        &kline_unit_data,
                        funding_rate,
                        user,
                    ) {
                        Err(e) => {
                            error!("{:?}", e);
                        }
                        Ok(runner_parse_result) => {
                            // 将增量数据传输给策略模块，获取策略行为。
                            // 将策略行为进行排序 cancel order在前 new order在后
                            // 记录transfer info
                            let transfer_info_executed = Self::get_parse_new_kline_transfer_info(&runner_parse_result);
                            // info!("runner_parse_result.order_result: {:?}", runner_parse_result.order_result);
                            let strategy_actions = user.get_strategy_result(runner_parse_result);
                            // info!("strategy_actions:");
                            // for action in strategy_actions.iter() {
                            //     info!("action: {:?}", action);
                            // }

                            // 根据策略行为，调整订单数据。
                            let parse_action_results = self.sync_strategy_action(
                                strategy_actions,
                                tp_type,
                                user,
                            );
                            // 记录transfer info
                            let transfer_info_unfulfilled = Self::get_sync_strategy_action_transfer_info(&parse_action_results);
                            let transfer_info = SDataLogTransferUnit::from(transfer_info_unfulfilled, transfer_info_executed);
                            transfer_info_map.entry(user.id).and_modify(|v| {
                                v.executed_buy_order_cnt += transfer_info.executed_buy_order_cnt;
                                v.unfulfilled_buy_order_cnt += transfer_info.unfulfilled_buy_order_cnt;
                                v.unfulfilled_sell_order_cnt += transfer_info.unfulfilled_sell_order_cnt;
                                v.executed_buy_order_cnt += transfer_info.executed_buy_order_cnt;
                                v.executed_sell_order_cnt += transfer_info.executed_sell_order_cnt;
                                v.unfulfilled_buy_usdt_cnt += transfer_info.unfulfilled_buy_usdt_cnt;
                                v.unfulfilled_sell_usdt_cnt += transfer_info.unfulfilled_sell_usdt_cnt;
                                v.executed_buy_usdt_cnt += transfer_info.executed_buy_usdt_cnt;
                                v.executed_sell_usdt_cnt += transfer_info.executed_sell_usdt_cnt;
                            }).or_insert(transfer_info);
                            // 向策略模块反馈校验、调整结果
                            user.strategy.verify(tp_type, parse_action_results);
                        }
                    }
                }
            }

            // 记录日志
            self.data_logger.add_kline_data(SDataLogKlineUnit::new(current_date, trading_pair_klines));
            for user in users.iter() {
                let transfer_info = transfer_info_map.get(&user.id).unwrap();
                let user_data = SDataLogUserUnit::new(current_date, user, None, &trading_pair_prices, transfer_info);
                debug!("用户信息:{:?}\t资产 {:?}\t现金 {:?}\t累计手续费 {:?}",
                    user_data.user_name, user_data.total_assets_usdt, user_data.total_usdt, user_data.total_fee_usdt);
                self.data_logger.add_user_data(user_data);
            }

            // 时间递增 1 分钟
            current_date += Duration::minutes(1);

            // thread::sleep(std::time::Duration::from_secs(2));
        }
        // 回测结束 输出结果
        self.data_logger.output_user(String::from(format!("data/back_trade/{}.csv", Local::now().format("%Y%m%d_%H%M%S"))));
        ()
    }
}

impl<D: TDataApi> SBackTradeRunner<D> {
    pub async fn new(config: SBackTradeRunnerConfig, data_manager: SDataManager<D>) -> Self {
        Self {
            config,
            data_manager,
            data_logger: Default::default(),
        }
    }


    /// 处理新的k线和资金费率，更新订单和资产，记录增量处理结果。
    fn parse_new_kline<S: TStrategy>(
        &self,
        tp_type: &ETradingPairType,
        kline_unit_data: &SKlineUnitData,
        funding_rate: Decimal,
        user: &mut SUser<S>,
    ) -> RBackTradeRunnerResult<SRunnerParseKlineResult>
    {
        // 根据K线 结算订单数据 结算资产数据
        let mut order_results: Vec<ERunnerParseOrderResult> = Vec::new(); // 订单已成交列表
        let base_asset_type = tp_type.get_base_currency();
        let quote_asset_type = tp_type.get_quote_currency();
        let maker_order_fee = self.config.maker_order_fee;
        let user_asset_manager = &mut user.available_assets;
        let order_manager = user.tp_order_map.get_mut(tp_type).unwrap();

        { // debug
            let highest_buy_price = match order_manager.peek_highest_buy_order().unwrap() {
                None => { None }
                Some(order) => { Some(order.get_price()) }
            };
            let lowest_sell_price = match order_manager.peek_lowest_sell_order().unwrap() {
                None => { None }
                Some(order) => { Some(order.get_price()) }
            };
            debug!("盘口信息 - 交易对: {:?}\t买一价格:{:?}\t卖一价格:{:?}", tp_type, highest_buy_price, lowest_sell_price);
        }

        // 买单结算 用quote_currency换base_current
        while let Some(order) = order_manager.peek_highest_buy_order().unwrap() {
            // 操作方向校验
            if order.get_action() != EOrderAction::Buy {
                log::error!("EOrderAction Error: Expected Buy - Actually {:?}", order.get_action());
                // return Err(EBackTradeRunnerError::OrderActionError(order.action));
            }
            // 挂单价格大于等于当前k线最低价格，则买单成交。
            if order.get_price() < kline_unit_data.low_price {
                break;
            }
            let mut order = order_manager.pop_highest_buy_order().unwrap().unwrap();
            let base_quantity = order.get_quantity();
            let quote_quantity = order.get_amount();
            // 计算手续费(计价资产)
            let fee_quote_asset = SAsset {
                as_type: quote_asset_type,
                balance: quote_quantity * maker_order_fee,
            };
            // 结算资产
            // 提取订单锁定的计价资产 生成基础资产
            match order.execute(Some(fee_quote_asset.clone())) {
                // consumed_quote_asset会被自动析构 代表订单的锁定资产被消耗
                Ok(_consumed_quote_asset) => {
                    // 用户获得基础资产
                    let obtain_base_asset = SAsset {
                        as_type: base_asset_type,
                        balance: base_quantity - base_quantity * maker_order_fee,
                    };

                    // info!("结算买单: {:?}\t手续费:{:?}\t用户获得资产:{:?}\t用户消耗资产:{:?}", order.get_id(), &fee_quote_asset, &obtain_base_asset, &_consumed_quote_asset);

                    user_asset_manager.merge_asset(obtain_base_asset);
                    order_results.push(ERunnerParseOrderResult::OrderExecuted(order.clone()));
                    if let Err(e) = order_manager.add_finished_order(order) {
                        error!("{:?}", e);
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }

        // 卖单结算 用base_current换quote_currency
        while let Some(order) = order_manager.peek_lowest_sell_order().unwrap() {
            // 操作方向校验
            if order.get_action() != EOrderAction::Sell {
                error!("EOrderAction Error: Expected Sell - Actually {:?}", order.get_action());
                // return Err(EBackTradeRunnerError::OrderActionError(order.action));
            }
            // 挂单价格大于等于当前k线最低价格，则买单成交。
            if order.get_price() > kline_unit_data.high_price {
                break;
            }
            let mut order = order_manager.pop_lowest_sell_order().unwrap().unwrap();
            let base_quantity = order.get_quantity();
            // 计算手续费(基础资产)
            let fee_base_asset = SAsset {
                as_type: base_asset_type,
                balance: base_quantity * maker_order_fee,
            };
            let quote_quantity = order.get_amount();
            // 结算资产
            // 提取订单锁定的基础资产 生成计价资产
            match order.execute(Some(fee_base_asset.clone())) {
                // consumed_base_asset会被自动析构 代表订单的锁定资产被消耗
                Ok(_consumed_base_asset) => {
                    // 用户获得计价资产
                    let obtain_quote_asset = SAsset {
                        as_type: quote_asset_type,
                        balance: quote_quantity - quote_quantity * maker_order_fee,
                    };

                    // info!("结算卖单: {:?}\t手续费:{:?}\t用户获得资产:{:?}\t用户消耗资产:{:?}", order.get_id(), &fee_base_asset, &obtain_quote_asset, &_consumed_base_asset);

                    user_asset_manager.merge_asset(obtain_quote_asset);
                    order_results.push(ERunnerParseOrderResult::OrderExecuted(order.clone()));
                    if let Err(e) = order_manager.add_finished_order(order) {
                        error!("{:?}", e);
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }

        // 将k线和订单结算结果 用于反馈给strategy
        let runner_parse_result = SRunnerParseKlineResult {
            tp_type: tp_type.clone(),
            new_kline: kline_unit_data.clone(),
            new_funding_rate: funding_rate,
            order_result: order_results,
        };

        Ok(runner_parse_result)
    }

    /// 根据策略行为，同步订单数据。
    fn sync_strategy_action<S: TStrategy>(
        &self,
        strategy_actions: Vec<EStrategyAction>,
        tp_type: &ETradingPairType,
        user: &mut SUser<S>,
    ) -> Vec<ERunnerSyncActionResult>
    {
        let user_asset_manager = &mut user.available_assets;
        let order_manager = user.tp_order_map.get_mut(tp_type).unwrap();
        let base_asset_type = tp_type.get_base_currency();
        let quote_asset_type = tp_type.get_quote_currency();


        // 根据策略行为，校验、调整订单数据。
        let mut parse_action_result: Vec<ERunnerSyncActionResult> = Vec::new();
        // 根据action类型进行分类 之后进行分批批处理
        let mut add_orders: Vec<SStrategyOrderAdd> = Vec::new();
        let mut cancel_orders: Vec<Uuid> = Vec::new();

        for action in strategy_actions {
            match action {
                EStrategyAction::NewOrder(order) => {
                    add_orders.push(order)
                }
                EStrategyAction::CancelOrder(uuid) => {
                    // 判断uuid是否有效
                    if let Some(_) = order_manager.peek_order(&uuid) {
                        cancel_orders.push(uuid)
                    } else {
                        info!("Cancel Fail! : {:?}", uuid);
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
                    // info!("取消订单: {:?}", order);
                    // 订单成功取消 释放锁定资产
                    if let Some(asset) = order.cancel() {
                        let user_asset = match order.get_action() {
                            EOrderAction::Buy => { user_asset_manager.get_mut(quote_asset_type).unwrap() }
                            EOrderAction::Sell => { user_asset_manager.get_mut(base_asset_type).unwrap() }
                        };
                        if let Err(e) = user_asset.merge(asset) {
                            error!("Error: {:?}", e);
                        }
                        parse_action_result.push(ERunnerSyncActionResult::OrderCanceled(order));
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
                // info!("新增订单: {:?}", &new_order);
                if let Err(e) = order_manager.insert_order(new_order.clone()) {
                    error!("Error: {:?}", e);
                }
                parse_action_result.push(ERunnerSyncActionResult::OrderPlaced(new_order));
            }
            // info!("Finish: add_order");
        }
        parse_action_result
    }

    /// 根据parse_new_kline结果 计算
    fn get_parse_new_kline_transfer_info(runner_parse_result: &SRunnerParseKlineResult) -> SDataLogTransferExecutedUnit {
        let mut result = SDataLogTransferExecutedUnit::default();
        for order_result in &runner_parse_result.order_result {
            if let ERunnerParseOrderResult::OrderExecuted(order) = order_result {
                match order.get_action() {
                    EOrderAction::Buy => {
                        result.executed_buy_order_cnt += 1;
                        result.executed_buy_usdt_cnt += order.get_price() * order.get_quantity();
                    }
                    EOrderAction::Sell => {
                        result.executed_sell_order_cnt += 1;
                        result.executed_sell_usdt_cnt += order.get_price() * order.get_quantity();
                    }
                }
            }
        }
        result
    }

    /// 根据sync_strategy_action结果 计算
    fn get_sync_strategy_action_transfer_info(parse_action_results: &Vec<ERunnerSyncActionResult>) -> SDataLogTransferUnfulfilledUnit {
        let mut result = SDataLogTransferUnfulfilledUnit::default();
        for action_result in parse_action_results {
            if let ERunnerSyncActionResult::OrderPlaced(order) = action_result {
                match order.get_action() {
                    EOrderAction::Buy => {
                        result.unfulfilled_buy_order_cnt += 1;
                        result.unfulfilled_buy_usdt_cnt += order.get_price() * order.get_quantity();
                    }
                    EOrderAction::Sell => {
                        result.unfulfilled_sell_order_cnt += 1;
                        result.unfulfilled_sell_usdt_cnt += order.get_price() * order.get_quantity();
                    }
                }
            }
        }
        result
    }
}