use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;
use crate::data_runtime::asset::asset_map::{SAssetMap};
use crate::data_runtime::asset::EAssetType;
use crate::data_source::trading_pair::ETradingPairType;
use crate::runner::logger::kline_unit::SDataLogKlineUnit;
use crate::runner::logger::user_unit::SDataLogUserUnit;
use crate::utils::assets_map_denominate_usdt;

/// 数据日志
#[derive(Debug, Default)]
pub struct SDataLogger {
    /// 用户日志数据
    pub user_data: BTreeMap<(DateTime<Local>, Uuid), SDataLogUserUnit>,

    /// k线日志数据
    pub kline_data: BTreeMap<DateTime<Local>, SDataLogKlineUnit>,
}

impl SDataLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_user_data(&mut self, data: SDataLogUserUnit) {
        let _ = self.user_data.insert((data.time, data.user_id), data);
    }

    pub fn add_kline_data(&mut self, data: SDataLogKlineUnit) {
        let _ = self.kline_data.insert(data.time, data);
    }

    pub fn output_user(&self, path: String) {
        dbg!(&path);
        let file = File::create(path.clone()).unwrap();
        let mut wtr = csv::Writer::from_writer(file);

        for (_, user_log) in self.user_data.iter() {
            // dbg!(&user_log);
            // 获取单类交易对的价格
            let fn_get_trading_pair_price = |tp_type: &ETradingPairType| {
                match user_log.trading_pair_prices.get(tp_type) {
                    None => { None }
                    Some(x) => { Some(x.clone()) }
                }
            };

            // 获取单类资产的balance
            let fn_get_asset_balance = |map: &SAssetMap, as_type: &EAssetType| {
                match map.get(as_type) {
                    Ok(x) => { x.balance }
                    Err(_e) => { Decimal::from(0) }
                }
            };

            // 获取单类资产的USDT计价
            let fn_get_usdt_asset_slice_from_asset_map = |map: &SAssetMap,
                                                          as_type: &EAssetType,
                                                          trading_pair_prices: &HashMap<ETradingPairType, Decimal>| {
                let mut new_map = SAssetMap::new();
                match map.get(as_type) {
                    Ok(map_slice) => {
                        new_map.merge_asset(map_slice.clone());
                        assets_map_denominate_usdt(
                            &new_map,
                            trading_pair_prices,
                        )
                    }
                    Err(_e) => { Decimal::from(0) }
                }
            };

            let assets_total = &user_log.total_assets;
            let assets_available = &user_log.available_assets;
            let assets_locked = &user_log.locked_assets;
            let trading_pair_prices = &user_log.trading_pair_prices;
            let total_usdt = assets_map_denominate_usdt(
                &assets_total,
                trading_pair_prices,
            );
            let usdt_total = fn_get_asset_balance(&assets_total, &EAssetType::Usdt);
            let merged_output = SMergeOutput {
                time: user_log.time,
                price_btc_usdt: fn_get_trading_pair_price(&ETradingPairType::BtcUsdt),
                price_btc_usdt_future: fn_get_trading_pair_price(&ETradingPairType::BtcUsdtFuture),
                price_btc_usd_cm_future: fn_get_trading_pair_price(&ETradingPairType::BtcUsdCmFuture),
                user_id: user_log.user_id,
                user_name: user_log.user_name.clone(),
                total_usdt,
                total_available_usdt: assets_map_denominate_usdt(
                    &assets_available,
                    trading_pair_prices,
                ),
                assets_total_usdt: total_usdt - usdt_total,
                total_locked_usdt: assets_map_denominate_usdt(
                    &assets_locked,
                    trading_pair_prices,
                ),
                total_fee_usdt: user_log.total_fee_usdt,
                usdt_total,
                usdt_available: fn_get_asset_balance(&assets_available, &EAssetType::Usdt),
                usdt_locked: fn_get_asset_balance(&assets_locked, &EAssetType::Usdt),
                btc_total: fn_get_asset_balance(&assets_total, &EAssetType::Btc),
                btc_available: fn_get_asset_balance(&assets_available, &EAssetType::Btc),
                btc_locked: fn_get_asset_balance(&assets_locked, &EAssetType::Btc),
                btc_total_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_total, &EAssetType::Btc, trading_pair_prices),
                btc_available_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_available, &EAssetType::Btc, trading_pair_prices),
                btc_locked_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_locked, &EAssetType::Btc, trading_pair_prices),
                btc_usdt_future_total: fn_get_asset_balance(&assets_total, &EAssetType::BtcUsdtFuture),
                btc_usdt_future_available: fn_get_asset_balance(&assets_available, &EAssetType::BtcUsdtFuture),
                btc_usdt_future_locked: fn_get_asset_balance(&assets_locked, &EAssetType::BtcUsdtFuture),
                btc_usdt_future_total_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_total, &EAssetType::BtcUsdtFuture, trading_pair_prices),
                btc_usdt_future_available_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_available, &EAssetType::BtcUsdtFuture, trading_pair_prices),
                btc_usdt_future_locked_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_locked, &EAssetType::BtcUsdtFuture, trading_pair_prices),
                btc_usd_cm_future_total: fn_get_asset_balance(&assets_total, &EAssetType::BtcUsdCmFuture),
                btc_usd_cm_future_available: fn_get_asset_balance(&assets_available, &EAssetType::BtcUsdCmFuture),
                btc_usd_cm_future_locked: fn_get_asset_balance(&assets_locked, &EAssetType::BtcUsdCmFuture),
                btc_usd_cm_future_total_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_total, &EAssetType::BtcUsdCmFuture, trading_pair_prices),
                btc_usd_cm_future_available_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_available, &EAssetType::BtcUsdCmFuture, trading_pair_prices),
                btc_usd_cm_future_locked_usdt: fn_get_usdt_asset_slice_from_asset_map(assets_locked, &EAssetType::BtcUsdCmFuture, trading_pair_prices),

                target_position_ratio: user_log.target_position_ratio,
                actual_position_ratio: user_log.get_actual_position_ratio(),

                unfulfilled_buy_order_cnt: user_log.transfer_info.unfulfilled_buy_order_cnt,
                unfulfilled_sell_order_cnt: user_log.transfer_info.unfulfilled_sell_order_cnt,
                executed_buy_order_cnt: user_log.transfer_info.executed_buy_order_cnt,
                executed_sell_order_cnt: user_log.transfer_info.executed_sell_order_cnt,
                unfulfilled_buy_usdt_cnt: user_log.transfer_info.unfulfilled_buy_usdt_cnt,
                unfulfilled_sell_usdt_cnt: user_log.transfer_info.unfulfilled_sell_usdt_cnt,
                executed_buy_usdt_cnt: user_log.transfer_info.executed_buy_usdt_cnt,
                executed_sell_usdt_cnt: user_log.transfer_info.executed_sell_usdt_cnt,
            };
            wtr.serialize(merged_output).unwrap();
        }

        wtr.flush().unwrap();
        println!("用户日志写入完成：{}", path);
    }
}

#[derive(Debug, Serialize)]
struct SMergeOutput {
    pub time: DateTime<Local>,

    // -----价格信息-----
    /// Btc现货价格
    pub price_btc_usdt: Option<Decimal>,
    /// U本位合约/Usdt 价格
    pub price_btc_usdt_future: Option<Decimal>,
    /// 币本位合约/Btc 价格
    pub price_btc_usd_cm_future: Option<Decimal>,

    // -----用户信息-----
    pub user_id: Uuid,
    pub user_name: String,

    // -----仓位信息-----
    /// 目标仓位
    pub target_position_ratio: Option<Decimal>,
    /// 实际仓位
    pub actual_position_ratio: Decimal,

    // -----交易信息-----
    /// 挂单的买入交易数
    pub unfulfilled_buy_order_cnt: i64,
    /// 挂单的卖出交易数
    pub unfulfilled_sell_order_cnt: i64,
    /// 已成交的买入交易数
    pub executed_buy_order_cnt: i64,
    /// 已成交的卖出交易数
    pub executed_sell_order_cnt: i64,
    /// 挂单的买入资产量（USDT计价）
    pub unfulfilled_buy_usdt_cnt: Decimal,
    /// 挂单的卖出资产量（USDT计价）
    pub unfulfilled_sell_usdt_cnt: Decimal,
    /// 已成交的买入资产量（USDT计价）
    pub executed_buy_usdt_cnt: Decimal,
    /// 已成交的卖出资产量（USDT计价）
    pub executed_sell_usdt_cnt: Decimal,

    // -----资产信息-----
    /// 资产总量（USDT计价）
    pub total_usdt: Decimal,
    /// USDT总持有量（现金资产总量）
    pub usdt_total: Decimal,
    /// 非现金资产总量
    pub assets_total_usdt: Decimal,
    /// 可用资产总量（USDT计价）
    pub total_available_usdt: Decimal,
    /// 锁定资产总量（USDT计价）
    pub total_locked_usdt: Decimal,

    /// 累计手续费（Usdt计价）
    pub total_fee_usdt: Decimal,

    /// USDT可用量
    pub usdt_available: Decimal,
    /// USDT锁定量
    pub usdt_locked: Decimal,

    /// BTC总持有量
    pub btc_total: Decimal,
    /// BTC可用量
    pub btc_available: Decimal,
    /// BTC锁定量
    pub btc_locked: Decimal,
    /// BTC总持有量（USDT计价）
    pub btc_total_usdt: Decimal,
    /// BTC可用量（USDT计价）
    pub btc_available_usdt: Decimal,
    /// BTC锁定量（USDT计价）
    pub btc_locked_usdt: Decimal,

    /// BTC U本位合约 总持有量
    pub btc_usdt_future_total: Decimal,
    /// BTC U本位合约 可用量
    pub btc_usdt_future_available: Decimal,
    /// BTC U本位合约 锁定量
    pub btc_usdt_future_locked: Decimal,
    /// BTC U本位合约 总持有量（USDT计价）
    pub btc_usdt_future_total_usdt: Decimal,
    /// BTC U本位合约 可用量（USDT计价）
    pub btc_usdt_future_available_usdt: Decimal,
    /// BTC U本位合约 锁定量（USDT计价）
    pub btc_usdt_future_locked_usdt: Decimal,

    /// BTC 币本位合约 总持有量
    pub btc_usd_cm_future_total: Decimal,
    /// BTC 币本位合约 可用量
    pub btc_usd_cm_future_available: Decimal,
    /// BTC 币本位合约 锁定量
    pub btc_usd_cm_future_locked: Decimal,
    /// BTC 币本位合约 总持有量（USDT计价）
    pub btc_usd_cm_future_total_usdt: Decimal,
    /// BTC 币本位合约 可用量（USDT计价）
    pub btc_usd_cm_future_available_usdt: Decimal,
    /// BTC 币本位合约 锁定量（USDT计价）
    pub btc_usd_cm_future_locked_usdt: Decimal,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::thread;
    use chrono::Local;
    use rust_decimal::Decimal;
    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::EAssetType;
    use crate::data_runtime::user::{SUser, SUserConfig};
    use crate::data_source::trading_pair::ETradingPairType;
    use crate::runner::logger::data_logger::SDataLogger;
    use crate::runner::logger::transfer_unit::SDataLogTransferUnit;
    use crate::runner::logger::user_unit::SDataLogUserUnit;
    use crate::strategy::mk_test::SStrategyMkTest;

    #[test]
    pub fn test1() {
        let mut data = SDataLogger::new();

        // 插入数据1
        let time = Local::now();
        let mut user = SUser::new(
            SUserConfig {
                user_name: "test user".to_string(),
                init_balance_usdt: Decimal::from(10_000),
                init_balance_btc: Decimal::from(1),
            },
            SStrategyMkTest::new(),
        );
        let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();
        trading_pair_prices.insert(ETradingPairType::BtcUsdt, Decimal::from(10_000));

        let user_data = SDataLogUserUnit::new(
            time,
            &user,
            None,
            &trading_pair_prices,
            &SDataLogTransferUnit::default(),
        );

        data.add_user_data(user_data);

        // 插入数据2
        thread::sleep(std::time::Duration::from_secs(1));
        let time = Local::now();
        user.merge_available_asset(SAsset { as_type: EAssetType::Usdt, balance: Decimal::from(20_000) });
        user.merge_available_asset(SAsset { as_type: EAssetType::Btc, balance: Decimal::from(9) });
        let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();
        trading_pair_prices.insert(ETradingPairType::BtcUsdt, Decimal::from(11_000));

        let user_data = SDataLogUserUnit::new(
            time,
            &user,
            None,
            &trading_pair_prices,
            &SDataLogTransferUnit::default(),
        );

        data.add_user_data(user_data);

        // 插入数据3
        thread::sleep(std::time::Duration::from_secs(1));
        let time = Local::now();
        let _ = user.split_available_asset(EAssetType::Usdt, Decimal::from(10_000));
        let _ = user.split_available_asset(EAssetType::Btc, Decimal::from(5));
        let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();
        trading_pair_prices.insert(ETradingPairType::BtcUsdt, Decimal::from(9_000));

        let user_data = SDataLogUserUnit::new(
            time,
            &user,
            None,
            &trading_pair_prices,
            &SDataLogTransferUnit::default(),
        );

        data.add_user_data(user_data);

        // 输出数据
        data.output_user(String::from("data/test_user_log.csv"));
    }
}