use std::collections::HashMap;
use rust_decimal::Decimal;
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::asset::EAssetType;
use crate::data_source::trading_pair::ETradingPairType;

pub mod date_time {
    use chrono::{DateTime, Local, Timelike};

    /// 归一化时间戳到分钟（秒归零）
    pub fn normalize_to_minute(dt: &DateTime<Local>) -> DateTime<Local> {
        dt.with_second(0).unwrap().with_nanosecond(0).unwrap()
    }
}

/// 将对应资产以USDT进行计价
pub fn assets_denominate_usdt(
    assets: &SAssetMap,
    trading_pair_prices: &HashMap<ETradingPairType, Decimal>,
) -> Decimal {
    // 先转换为usdt+btc
    let mut tmp = SAssetMap::new();
    for (_, asset) in assets.iter() {
        match asset.as_type {
            EAssetType::Usdt | EAssetType::Btc => { tmp.merge_asset(asset.clone()) }
            EAssetType::BtcUsdtFuture => {
                let price = trading_pair_prices.get(&ETradingPairType::BtcUsdtFuture).unwrap();
                let btc_usdt_future_balance = asset.balance;
                let usdt_balance = btc_usdt_future_balance * price;
                let new_asset = SAsset {
                    as_type: EAssetType::Usdt,
                    balance: usdt_balance,
                };
                tmp.merge_asset(new_asset)
            }
            EAssetType::BtcUsdCmFuture => {
                let price = trading_pair_prices.get(&ETradingPairType::BtcUsdCmFuture).unwrap();
                let btc_usd_cm_future_balance = asset.balance;
                let btc_balance = btc_usd_cm_future_balance * price;
                let new_asset = SAsset {
                    as_type: EAssetType::Btc,
                    balance: btc_balance,
                };
                tmp.merge_asset(new_asset)
            }
        }
    }
    // 再根据target_as_type进行转换
    let mut result = Decimal::from(0);
    let price = trading_pair_prices.get(&ETradingPairType::BtcUsdt).unwrap();
    for (_, asset) in tmp.inner.iter() {
        match asset.as_type {
            EAssetType::Usdt => {
                result += asset.balance;
            }
            EAssetType::Btc => {
                result += asset.balance * price
            }
            _ => {}
        }
    }
    result
}