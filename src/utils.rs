use std::collections::HashMap;
use rust_decimal::Decimal;
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::asset::asset_map_v3::SAssetMapV3;
use crate::data_runtime::asset::asset_union::EAssetUnion;
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
    asset: &EAssetUnion,
    trading_pair_prices: &HashMap<ETradingPairType, Decimal>,
) -> Decimal {
    // 先转换为usdt+btc
    let _new_asset = match asset.get_asset_type() {
        EAssetType::Usdt | EAssetType::Btc => { asset.clone() }
        EAssetType::BtcUsdtFuture => {
            let price = trading_pair_prices.get(&ETradingPairType::BtcUsdtFuture).unwrap();
            let btc_usdt_future_balance = asset.get_balance();
            let usdt_balance = btc_usdt_future_balance * price;
            EAssetUnion::from(SAsset {
                as_type: EAssetType::Usdt,
                balance: usdt_balance,
            })
        }
        EAssetType::BtcUsdCmFuture => {
            let price = trading_pair_prices.get(&ETradingPairType::BtcUsdCmFuture).unwrap();
            let btc_usd_cm_future_balance = asset.get_balance();
            let btc_balance = btc_usd_cm_future_balance * price;
            EAssetUnion::from(SAsset {
                as_type: EAssetType::Btc,
                balance: btc_balance,
            })
        }
    };

    // 再根据target_as_type进行转换
    let price = trading_pair_prices.get(&ETradingPairType::BtcUsdt).unwrap();
    match asset.get_asset_type() {
        EAssetType::Usdt => {
            asset.get_balance()
        }
        EAssetType::Btc => {
            asset.get_balance() * price
        }
        _ => Decimal::from(0)
    }
}

/// 将对应资产以USDT进行计价
pub fn assets_denominate_usdt_old(
    asset: &SAsset,
    trading_pair_prices: &HashMap<ETradingPairType, Decimal>,
) -> Decimal {
    // 先转换为usdt+btc
    let _new_asset = match asset.as_type {
        EAssetType::Usdt | EAssetType::Btc => { asset.clone() }
        EAssetType::BtcUsdtFuture => {
            let price = trading_pair_prices.get(&ETradingPairType::BtcUsdtFuture).unwrap();
            let btc_usdt_future_balance = asset.balance;
            let usdt_balance = btc_usdt_future_balance * price;
            SAsset {
                as_type: EAssetType::Usdt,
                balance: usdt_balance,
            }
        }
        EAssetType::BtcUsdCmFuture => {
            let price = trading_pair_prices.get(&ETradingPairType::BtcUsdCmFuture).unwrap();
            let btc_usd_cm_future_balance = asset.balance;
            let btc_balance = btc_usd_cm_future_balance * price;
            SAsset {
                as_type: EAssetType::Btc,
                balance: btc_balance,
            }
        }
    };

    // 再根据target_as_type进行转换
    let price = trading_pair_prices.get(&ETradingPairType::BtcUsdt).unwrap();
    match asset.as_type {
        EAssetType::Usdt => {
            asset.balance
        }
        EAssetType::Btc => {
            asset.balance * price
        }
        _ => Decimal::from(0)
    }
}

/// 将对应资产以USDT进行计价
pub fn assets_map_denominate_usdt(
    assets: &SAssetMapV3,
    trading_pair_prices: &HashMap<ETradingPairType, Decimal>,
) -> Decimal {
    // 先转换为usdt+btc
    let mut result = Decimal::from(0);
    for (_, asset) in assets.iter() {
        result += assets_denominate_usdt(asset, trading_pair_prices);
    }
    result
}

/// 将对应资产以USDT进行计价
pub fn assets_map_denominate_usdt_old(
    assets: &SAssetMap,
    trading_pair_prices: &HashMap<ETradingPairType, Decimal>,
) -> Decimal {
    // 先转换为usdt+btc
    let mut result = Decimal::from(0);
    for (_, asset) in assets.iter() {
        result += assets_denominate_usdt_old(asset, trading_pair_prices);
    }
    result
}