pub mod asset;
pub mod asset_map;
pub mod asset_map_v3;
pub mod asset_leveraged;
pub mod asset_union;

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