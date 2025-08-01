use crate::data_runtime::asset::EAssetType;

pub mod trading_pair;
pub mod trading_pair_map;

/// 交易对类型
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ETradingPairType {
    /// Btc/Usdt
    BtcUsdt,
    /// U本位合约/Usdt
    BtcUsdtFuture,
    /// 币本位合约/Btc
    BtcUsdCmFuture,
}

impl ETradingPairType {
    pub fn get_base_currency_type(self) -> EAssetType {
        match self {
            ETradingPairType::BtcUsdt => { EAssetType::Btc }
            ETradingPairType::BtcUsdtFuture => { EAssetType::BtcUsdtFuture }
            ETradingPairType::BtcUsdCmFuture => { EAssetType::BtcUsdCmFuture }
        }
    }

    pub fn get_quote_currency_type(self) -> EAssetType {
        match self {
            ETradingPairType::BtcUsdt => { EAssetType::Usdt }
            ETradingPairType::BtcUsdtFuture => { EAssetType::Usdt }
            ETradingPairType::BtcUsdCmFuture => { EAssetType::Btc }
        }
    }
}