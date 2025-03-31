use std::collections::HashMap;
use rust_decimal::Decimal;

pub mod asset;
pub mod trading_pair;
pub mod asset_manager;
pub mod asset_v2;
pub mod asset_manager_v2;

/// 资产类型
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum EAssetType {
    Usdt,
    /// U本位合约
    BtcUsdtFuture,
    /// Btc现货
    Btc,
    /// 币本位合约
    BtcUsdCmFuture,
}