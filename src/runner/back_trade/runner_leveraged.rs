//! 在SBackTradeRunner的基础上 支持杠杆资产
use std::collections::HashMap;
use chrono::{DateTime, Local};
use log::{debug, error, info};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::{data_runtime::{
    asset::{
        asset::SAsset,
        EAssetType,
    },
    order::EOrderAction,
}, data_source::{
    data_manager::SDataManager,
    db::api::TDataApi,
    kline::SKlineUnitData,
    trading_pair::ETradingPairType,
}, protocol::{ERunnerSyncActionResult, ERunnerParseOrderResult, EStrategyAction, SRunnerParseKlineResult}, runner::{
    back_trade::config::SBackTradeRunnerConfig,
    TRunner,
}, strategy::TStrategy};
use crate::config::back_trade_period::SAMPLE_PERIOD;
use crate::data_runtime::asset::asset_leveraged::SAssetLeveraged;
use crate::data_runtime::asset::asset_union::EAssetUnion;
use crate::data_runtime::order::order_v3::SOrderV3;
use crate::data_runtime::user::SUser;
use crate::protocol::strategy_order::SStrategyOrderAdd;
use crate::runner::logger::data_logger::SDataLogger;
use crate::runner::logger::kline_unit::SDataLogKlineUnit;
use crate::runner::logger::transfer_unit::{SDataLogTransferExecutedUnit, SDataLogTransferUnfulfilledUnit, SDataLogTransferUnit};
use crate::runner::logger::user_unit::SDataLogUserUnit;
use crate::runner::{SDebugConfig, SRunnerResult, TRunnerGetPrice};
use crate::utils::{assets_denominate_usdt};

pub type RLeveragedBackTradeRunnerResult<T> = Result<T, ELeveragedBackTradeRunnerError>;

#[derive(Debug)]
pub enum ELeveragedBackTradeRunnerError {
    KlineNotFoundError(ETradingPairType, DateTime<Local>),
    OrderActionError(EOrderAction),
    AssetLockedNotEnoughError(EAssetType),
    DenominateSupportOnlyBtcAndUsdtError(EAssetType),
    MarginMustBeBtcOrUsdtError(EAssetUnion),
}

/// 回测执行器
#[derive(Debug)]
pub struct SLeveragedBackTradeRunner<D: TDataApi> {
    /// 配置
    pub config: SBackTradeRunnerConfig,
    /// 数据管理器
    pub data_manager: SDataManager<D>,
    /// 记录最新的交易对报价
    pub trading_pair_prices: HashMap<ETradingPairType, Decimal>,
    /// 数据日志
    pub data_logger: SDataLogger,
}

impl<S: TStrategy, D: TDataApi> TRunner<S> for SLeveragedBackTradeRunner<D> {
    fn run(&mut self, users: &mut Vec<SUser<S>>, debug_config: SDebugConfig) -> SRunnerResult {
        // 初始化报价
        self.trading_pair_prices.insert(ETradingPairType::BtcUsdt, Decimal::from(1));
        // self.trading_pair_prices.insert(ETradingPairType::BtcUsdtFuture, Decimal::from(1));
        self.trading_pair_prices.insert(ETradingPairType::BtcUsdCmFuture, Decimal::from(1));
        // 循环遍历k线 根据时间间隔1分钟
        let mut current_date = self.config.date_from;
        while current_date < self.config.date_to {
            if debug_config.is_info { info!("当前k线时间:\t{}", current_date) };

            // 用于记录报价
            let mut trading_pair_klines: HashMap<ETradingPairType, SKlineUnitData> = HashMap::new();
            // let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();
            // self.trading_pair_prices.clear(); // 当价格不完整时 使用旧的报价

            // 用于记录交易量 key-user_id value-transfer_info
            let mut transfer_info_map: HashMap<Uuid, SDataLogTransferUnit> = HashMap::new();

            // 用于记录盘口价格 key-(user_id, tp_type)
            let mut highest_buy_order_map: HashMap<(Uuid, ETradingPairType), Option<Decimal>> = HashMap::new();
            let mut lowest_sell_order_map: HashMap<(Uuid, ETradingPairType), Option<Decimal>> = HashMap::new();

            let mut continue_flag = false;
            // 在单分钟k线内遍历所有交易对
            for (tp_type, trading_pair) in self.data_manager.trading_pair_map.inner.iter() {
                // 获取k线数据
                let option_kline = trading_pair.get_kline(&current_date);
                if let None = option_kline {
                    let err = ELeveragedBackTradeRunnerError::KlineNotFoundError(tp_type.clone(), current_date.clone());
                    error!("{:?}", err);
                    continue_flag = true;
                    continue;
                }
                let kline_unit_data = option_kline.unwrap().clone();
                // 查询当前k线对应的资金费率
                let funding_rate = trading_pair.get_funding_rate(&current_date).unwrap_or(&Decimal::from(0)).clone();

                if debug_config.is_info {
                    info!("K线信息 - 交易对: {:?}\t开盘价:{}\t收盘价:{}\t最高价:{}\t最低价:{}\t资金费率:{:.4?}%", tp_type, kline_unit_data.open_price, kline_unit_data.close_price, kline_unit_data.high_price, kline_unit_data.low_price, funding_rate*Decimal::from(100));
                }

                // 记录日志
                trading_pair_klines.insert(tp_type.clone(), kline_unit_data);
                self.trading_pair_prices.entry(tp_type.clone())
                    .and_modify(|e| *e = kline_unit_data.close_price)
                    .or_insert(kline_unit_data.close_price);

                // dbg!(&self.trading_pair_prices);

                for user in users.iter_mut() {
                    let (
                        runner_parse_result,
                        highest_buy_price,
                        lowest_sell_price
                    ) = self.parse_new_kline(
                        tp_type,
                        &kline_unit_data,
                        funding_rate,
                        user,
                        &debug_config,
                    );
                    // 记录盘口价格
                    highest_buy_order_map.insert((user.id, tp_type.clone()), highest_buy_price);
                    lowest_sell_order_map.insert((user.id, tp_type.clone()), lowest_sell_price);

                    match runner_parse_result {
                        Err(e) => {
                            error!("{:?}", e);
                        }
                        Ok(runner_parse_result) => {
                            // 将增量数据传输给策略模块，获取策略行为。
                            // 将策略行为进行排序 cancel order在前 new order在后
                            // 记录transfer info
                            let transfer_info_executed = Self::get_parse_new_kline_transfer_info(&runner_parse_result);
                            // info!("runner_parse_result.order_result: {:?}", runner_parse_result.order_result);
                            let strategy_actions = user.get_strategy_result(runner_parse_result, &debug_config);
                            // info!("strategy_actions:");
                            // for action in strategy_actions.iter() {
                            //     info!("action: {:?}", action);
                            // }

                            // 根据策略行为，调整订单数据。
                            // dbg!(&strategy_actions);
                            let parse_action_results = self.sync_strategy_action(
                                strategy_actions,
                                tp_type,
                                user,
                                &debug_config,
                            );
                            // info!("parse_action_results:");
                            // for action in parse_action_results.iter() {
                            //     info!("action: {:?}", action);
                            // }
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
                            user.strategy.verify(tp_type, parse_action_results, &debug_config);
                        }
                    }
                    // info!("before:");
                    // dbg!(&user.available_assets);
                    // 根据最新价格 更新杠杆资产的数据
                    user.available_assets.update_leveraged(&self.trading_pair_prices);
                    // info!("after:");
                    dbg!(&user.available_assets);
                }
            }

            if continue_flag {
                // 时间递增
                current_date += SAMPLE_PERIOD;
                continue;
            }
            // 记录日志
            self.data_logger.add_kline_data(SDataLogKlineUnit::new(current_date, trading_pair_klines));
            for user in users.iter() {
                let mut buy_order_num = 0;
                let mut sell_order_num = 0;
                for (_, order_manager) in user.tp_order_map.inner.iter() {
                    buy_order_num += order_manager.buy_orders.len();
                    sell_order_num += order_manager.sell_orders.len();
                }
                // debug!("transfer_info_map: {:?}", transfer_info_map);
                // debug!("user.id: {:?}", user.id);
                // dbg!(&user.tp_order_map);

                let transfer_info = transfer_info_map.get(&user.id).unwrap();
                let target_position_ratio = Some(Decimal::from(user.strategy.get_log_info().target_position_ratio));
                let user_data = SDataLogUserUnit::new(current_date, user, target_position_ratio, &self.trading_pair_prices, transfer_info);
                let position_ratio = (user_data.total_assets_usdt - user_data.total_usdt) / user_data.total_assets_usdt * Decimal::from(100);

                if debug_config.is_info {
                    info!("用户信息:{:?}\t仓位:{:.2?}%\t总资产 {:.4?}\t资产 {:.4?}\t现金 {:.4?}\t累计手续费 {:.4?}\t买单数量:{:?}\t卖单数量:{:?}",
                        user_data.user_name, 
                        position_ratio, 
                        user_data.total_assets_usdt, 
                        user_data.total_assets_usdt - user_data.total_usdt,
                        user_data.total_usdt, 
                        user_data.total_fee_usdt, 
                        buy_order_num, 
                        sell_order_num
                    );
                }
                self.data_logger.add_user_data(user_data);
            }

            // 时间递增
            current_date += SAMPLE_PERIOD;
            // current_date += Duration::minutes(1);

            // thread::sleep(std::time::Duration::from_secs(2));
        }
        // 回测结束 输出结果
        // self.data_logger.output_user(String::from(format!("data/back_trade/{}.csv", Local::now().format("%Y%m%d_%H%M%S"))));
        SRunnerResult {
            date_from: self.config.date_from,
            date_to: self.config.date_to,
            data_logger: self.data_logger.clone(),
        }
    }
}

impl<D: TDataApi> TRunnerGetPrice for SLeveragedBackTradeRunner<D> {
    fn get_price(&self, date: DateTime<Local>, tp_type: ETradingPairType) -> Option<&SKlineUnitData> {
        match self.data_manager.trading_pair_map.get_kline(tp_type, &date) {
            Err(e) => {
                error!("{:?}", e);
                None
            }
            Ok(option_kline) => { option_kline }
        }
    }
}

impl<D: TDataApi> SLeveragedBackTradeRunner<D> {
    pub fn new(config: SBackTradeRunnerConfig, data_manager: SDataManager<D>) -> Self {
        Self {
            config,
            data_manager,
            trading_pair_prices: Default::default(),
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
        debug_config: &SDebugConfig,
    ) -> (RLeveragedBackTradeRunnerResult<SRunnerParseKlineResult>, Option<Decimal>, Option<Decimal>)
    {
        // 根据K线 结算订单数据 结算资产数据
        let mut order_results: Vec<ERunnerParseOrderResult> = Vec::new(); // 订单已成交列表
        let base_asset_type = tp_type.get_base_currency_type();
        let quote_asset_type = tp_type.get_quote_currency_type();
        let maker_order_fee = self.config.maker_order_fee;
        let user_asset_manager = &mut user.available_assets;
        let order_manager = user.tp_order_map.get_mut(tp_type).unwrap();

        let highest_buy_price = match order_manager.peek_highest_buy_order().unwrap() {
            None => { None }
            Some(order) => { Some(order.get_price()) }
        };
        let lowest_sell_price = match order_manager.peek_lowest_sell_order().unwrap() {
            None => { None }
            Some(order) => { Some(order.get_price()) }
        };
        if debug_config.is_info { info!("盘口信息 - 交易对: {:?}\t买一价格:{:?}\t卖一价格:{:?}", tp_type, highest_buy_price, lowest_sell_price); }

        // 买单结算 用quote_currency换base_current
        while let Some(order) = order_manager.peek_highest_buy_order().unwrap() {
            // 操作方向校验
            if order.get_action() != EOrderAction::Buy {
                log::error!("EOrderAction Error: Expected Buy - Actually {:?}", order.get_action());
            }
            // 挂单价格必须大于等于当前k线最低价格，则买单成交。否则不进行计算
            if order.get_price() < kline_unit_data.low_price {
                break;
            }
            let mut order = order_manager.pop_highest_buy_order().unwrap().unwrap();
            let tp_type = order.get_tp_type();
            let price = order.get_price();
            let base_quantity = order.get_quantity();
            let quote_quantity = order.get_amount();
            // 计算手续费(计价资产)
            let fee_quote_asset = EAssetUnion::from(SAsset {
                as_type: quote_asset_type,
                // 手续费必须为正数（避免做空时支付空的手续费）
                balance: quote_quantity.abs() * maker_order_fee,
            });
            // 计算手续费(USDT计价)
            let fee_usdt = SAsset {
                as_type: EAssetType::Usdt,
                balance: assets_denominate_usdt(&fee_quote_asset, &self.trading_pair_prices),
            };
            // 结算资产
            // 提取订单锁定的计价资产 生成基础资产
            match order.execute(Some(fee_usdt)) {
                // consumed_margin_asset会被自动析构 代表订单的锁定资产被消耗
                Ok(consumed_margin_asset) => {
                    let debug_consumed_margin_asset = consumed_margin_asset.clone();
                    // 用户获得资产
                    let balance = base_quantity - base_quantity.abs() * maker_order_fee;
                    let obtain_base_asset = match tp_type {
                        // 现货交易时 获得基础资产
                        ETradingPairType::BtcUsdt => {
                            EAssetUnion::from(SAsset {
                                as_type: base_asset_type,
                                balance,
                            })
                        }
                        ETradingPairType::BtcUsdtFuture => {
                            // 合约交易时 获得杠杆资产
                            EAssetUnion::BtcUsdtFuture(SAssetLeveraged::new(
                                tp_type,
                                balance,
                                consumed_margin_asset,
                                price,
                            ).unwrap())
                        }
                        ETradingPairType::BtcUsdCmFuture => {
                            // 合约交易时 获得杠杆资产
                            EAssetUnion::BtcUsdCmFuture(SAssetLeveraged::new(
                                tp_type,
                                balance,
                                consumed_margin_asset,
                                price,
                            ).unwrap())
                        }
                    };

                    if debug_config.is_debug {
                        debug!("\n结算买单: {:?}\n挂单价:{:?}\n挂单量:{:?}\n手续费:{:?}\n用户获得资产:{:?}\n用户消耗资产:{:?}", 
                            order.get_id(),
                            order.get_price(),
                            order.get_quantity(),
                            &fee_quote_asset, 
                            &obtain_base_asset, 
                            &debug_consumed_margin_asset);
                    }

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
            }
            // 挂单价格大于等于当前k线最低价格，则买单成交。
            if order.get_price() > kline_unit_data.high_price {
                break;
            }
            let mut order = order_manager.pop_lowest_sell_order().unwrap().unwrap();
            let tp_type = order.get_tp_type();
            let price = order.get_price();
            let base_quantity = order.get_quantity();
            // let quote_quantity = order.get_amount();
            // 计算手续费
            let fee_base_asset = match base_asset_type {
                EAssetType::Usdt | EAssetType::Btc => {
                    // 现货手续费为基础资产
                    EAssetUnion::from(SAsset {
                        as_type: base_asset_type,
                        balance: base_quantity.abs() * maker_order_fee,
                    })
                }
                EAssetType::BtcUsdtFuture => {
                    // 合约手续费为保证金资产
                    EAssetUnion::Usdt(SAsset {
                        as_type: EAssetType::Usdt,
                        balance: base_quantity.abs() * price * maker_order_fee,
                    })
                }
                EAssetType::BtcUsdCmFuture => {
                    // 合约手续费为保证金资产
                    EAssetUnion::Btc(SAsset {
                        as_type: EAssetType::Btc,
                        balance: base_quantity.abs() * maker_order_fee,
                    })
                }
            };

            // 计算手续费(USDT计价)
            let fee_usdt = SAsset {
                as_type: EAssetType::Usdt,
                balance: assets_denominate_usdt(&fee_base_asset, &self.trading_pair_prices),
            };
            // 结算资产
            // 提取订单锁定的基础资产 生成计价资产
            match order.execute(Some(fee_usdt)) {
                // consumed_margin_asset 代表订单的锁定资产被消耗
                Ok(consumed_margin_asset) => {
                    let debug_consumed_margin_asset = consumed_margin_asset.clone();

                    // 用户获得资产
                    let balance = base_quantity - base_quantity.abs() * maker_order_fee;
                    let obtain_quote_asset = match tp_type {
                        // 现货交易时 获得计价资产
                        ETradingPairType::BtcUsdt => {
                            EAssetUnion::from(SAsset {
                                as_type: quote_asset_type,
                                balance,
                            })
                        }
                        ETradingPairType::BtcUsdtFuture => {
                            // 合约交易时 获得杠杆资产
                            EAssetUnion::BtcUsdtFuture(SAssetLeveraged::new(
                                tp_type,
                                balance,
                                consumed_margin_asset,
                                price,
                            ).unwrap())
                        }
                        ETradingPairType::BtcUsdCmFuture => {
                            // 合约交易时 获得杠杆资产
                            EAssetUnion::BtcUsdCmFuture(SAssetLeveraged::new(
                                tp_type,
                                balance,
                                consumed_margin_asset,
                                price,
                            ).unwrap())
                        }
                    };

                    if debug_config.is_debug {
                        debug!("\n结算卖单: {:?}\n挂单价:{:?}\n挂单量:{:?}\n手续费:{:?}\n用户获得资产:{:?}\n用户消耗资产:{:?}", 
                            order.get_id(),
                            order.get_price(), 
                            order.get_quantity(),
                            &fee_base_asset, 
                            &obtain_quote_asset,
                            &debug_consumed_margin_asset
                        );
                    }
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

        (Ok(runner_parse_result), highest_buy_price, lowest_sell_price)
    }

    /// 根据策略行为，同步订单数据。
    fn sync_strategy_action<S: TStrategy>(
        &self,
        strategy_actions: Vec<EStrategyAction>,
        tp_type: &ETradingPairType,
        user: &mut SUser<S>,
        debug_config: &SDebugConfig,
    ) -> Vec<ERunnerSyncActionResult>
    {
        let user_asset_manager = &mut user.available_assets;
        let order_manager = user.tp_order_map.get_mut(tp_type).unwrap();
        let base_asset_type = tp_type.get_base_currency_type();
        let quote_asset_type = tp_type.get_quote_currency_type();


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
                        if debug_config.is_debug { debug!("Cancel Fail! : {:?}", uuid); }
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
                    if debug_config.is_debug { debug!("取消订单: {:?}", order); }
                    // 订单成功取消 释放锁定资产
                    if let Some(asset) = order.cancel() {
                        let user_asset = match order.get_action() {
                            EOrderAction::Buy => { user_asset_manager.get_mut(quote_asset_type).unwrap() }
                            EOrderAction::Sell => { user_asset_manager.get_mut(base_asset_type).unwrap() }
                        };
                        if let Err(e) = user_asset.merge(EAssetUnion::from(asset)) {
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
            // if debug_config.is_info { info!("add_order:\t{:?}", add_order); }

            let SStrategyOrderAdd {
                id: _,
                tp_type,
                action,
                price,
                base_quantity,
                margin_quantity
            } = add_order;
            let mut new_order = SOrderV3::new(tp_type, price, base_quantity, action);
            let margin_asset_type = match tp_type {
                ETradingPairType::BtcUsdt => {
                    match action {
                        EOrderAction::Buy => { quote_asset_type }
                        EOrderAction::Sell => { base_asset_type }
                    }
                }
                ETradingPairType::BtcUsdtFuture | ETradingPairType::BtcUsdCmFuture => {quote_asset_type}
            };
            let user_asset = user_asset_manager.get_mut(margin_asset_type).unwrap();
            // info!("\nmargin_quantity:\t{:?}", margin_quantity);
            let split_user_asset = user_asset.split_allow_negative(margin_quantity);
            // info!("\nsplit_user_asset:\t{:?}", split_user_asset);
            let locked_margin_asset = match split_user_asset {
                EAssetUnion::Usdt(_) | EAssetUnion::Btc(_) => { split_user_asset }
                EAssetUnion::BtcUsdtFuture(leveraged_asset) | EAssetUnion::BtcUsdCmFuture(leveraged_asset) => {
                    EAssetUnion::from(leveraged_asset.get_margin().clone())
                }
            };
            // info!("locked_margin_asset:{:?}", locked_margin_asset);
            match locked_margin_asset {
                EAssetUnion::Usdt(asset) | EAssetUnion::Btc(asset) => {
                    if let Err(e) = new_order.submit(asset.clone()) {
                        error!("Error: {:?}", e);
                    }

                    // if debug_config.is_info { debug!("新增订单: {:?}", &new_order); }

                    if let Err(e) = order_manager.insert_order(new_order.clone()) {
                        error!("Error: {:?}", e);
                    }
                    parse_action_result.push(ERunnerSyncActionResult::OrderPlaced(new_order, add_order.id));
                    // info!("Finish: add_order");
                }
                EAssetUnion::BtcUsdtFuture(_) | EAssetUnion::BtcUsdCmFuture(_) => {
                    error!("{:?}", ELeveragedBackTradeRunnerError::MarginMustBeBtcOrUsdtError(locked_margin_asset))
                    // info!("Fail: add_order");
                }
            }
        }
        parse_action_result
    }

    /// 根据parse_new_kline结果 计算交易量
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

    /// 根据sync_strategy_action结果 计算交易量
    fn get_sync_strategy_action_transfer_info(parse_action_results: &Vec<ERunnerSyncActionResult>) -> SDataLogTransferUnfulfilledUnit {
        let mut result = SDataLogTransferUnfulfilledUnit::default();
        for action_result in parse_action_results {
            if let ERunnerSyncActionResult::OrderPlaced(order, _) = action_result {
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