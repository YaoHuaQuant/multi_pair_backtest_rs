pub mod asset;
pub mod asset_map;

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