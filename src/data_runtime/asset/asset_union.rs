use std::ops::AddAssign;
use log::error;
use rust_decimal::Decimal;
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_leveraged::SAssetLeveraged;
use crate::data_runtime::asset::EAssetType;
use crate::data_source::trading_pair::ETradingPairType;

pub type RAssetUnionResult<T> = Result<T, EAssetUnionError>;


pub type RequireType = EAssetUnion;
pub type ActualType = EAssetUnion;

#[derive(Debug)]
pub enum EAssetUnionError {
    /// 资产类型不匹配
    AssetTypeInconsistentError(RequireType, ActualType),
}

/// 资产联合体
/// 包含现货资产和杠杆资产
#[derive(Debug, Clone)]
pub enum EAssetUnion {
    Usdt(SAsset),
    Btc(SAsset),
    /// U本位合约
    BtcUsdtFuture(SAssetLeveraged),
    /// 币本位合约
    BtcUsdCmFuture(SAssetLeveraged),
}

impl EAssetUnion {
    pub fn merge(&mut self, other: Self) -> RAssetUnionResult<()> {
        match (self, other) {
            (EAssetUnion::Usdt(a), EAssetUnion::Usdt(b)) => {
                a.merge(b).unwrap();
                Ok(())
            }
            (EAssetUnion::Btc(a), EAssetUnion::Btc(b)) => {
                a.merge(b).unwrap();
                Ok(())
            }
            (EAssetUnion::BtcUsdtFuture(a), EAssetUnion::BtcUsdtFuture(b)) => {
                a.merge(b).unwrap();
                Ok(())
            }
            (EAssetUnion::BtcUsdCmFuture(a), EAssetUnion::BtcUsdCmFuture(b)) => {
                a.merge(b).unwrap();
                Ok(())
            }
            (required, actual) => Err(EAssetUnionError::AssetTypeInconsistentError(required.clone(), actual))
        }
    }

    pub fn get_asset_type(&self) -> EAssetType {
        match self {
            EAssetUnion::Usdt(_) => { EAssetType::Usdt }
            EAssetUnion::Btc(_) => { EAssetType::Btc }
            EAssetUnion::BtcUsdtFuture(_) => { EAssetType::BtcUsdtFuture }
            EAssetUnion::BtcUsdCmFuture(_) => { EAssetType::BtcUsdCmFuture }
        }
    }

    /// deprecated
    pub fn split(&mut self, balance: Decimal) -> Option<Self> {
        match self {
            EAssetUnion::Usdt(asset) => {
                match asset.split(balance) {
                    Ok(asset) => {
                        Some(EAssetUnion::Usdt(asset))
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        None
                    }
                }
            }
            EAssetUnion::Btc(asset) => {
                match asset.split(balance) {
                    Ok(asset) => {
                        Some(EAssetUnion::Btc(asset))
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        None
                    }
                }
            }
            EAssetUnion::BtcUsdtFuture(asset) => {
                match asset.split(balance) {
                    Ok(asset) => {
                        Some(EAssetUnion::BtcUsdtFuture(asset))
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        None
                    }
                }
            }
            EAssetUnion::BtcUsdCmFuture(asset) => {
                match asset.split(balance) {
                    Ok(asset) => {
                        Some(EAssetUnion::BtcUsdCmFuture(asset))
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        None
                    }
                }
            }
        }
    }

    pub fn split_allow_negative(&mut self, balance: Decimal) -> Self{
        match self {
            EAssetUnion::Usdt(asset) => {
                EAssetUnion::Usdt(asset.split_allow_negative(balance))
            }
            EAssetUnion::Btc(asset) => {
                EAssetUnion::Btc(asset.split_allow_negative(balance))
            }
            EAssetUnion::BtcUsdtFuture(asset) => {
                EAssetUnion::BtcUsdtFuture(asset.split_allow_negative(balance))
            }
            EAssetUnion::BtcUsdCmFuture(asset) => {
                EAssetUnion::BtcUsdCmFuture(asset.split_allow_negative(balance))
            }
        }
    }
    
    pub fn get_balance(&self) -> Decimal {
        match self {
            EAssetUnion::Usdt(asset) | EAssetUnion::Btc(asset) => {
                asset.balance
            }
            EAssetUnion::BtcUsdtFuture(asset) | EAssetUnion::BtcUsdCmFuture(asset) => {
                asset.get_base().balance
            }
        }
    }
}

impl AddAssign for EAssetUnion {
    fn add_assign(&mut self, rhs: Self) {
        // debug 没有考虑到type不相同的情况
        if let Err(e) = self.merge(rhs) {
            error!("{:?}", e)
        }
    }
}

impl From<SAsset> for EAssetUnion {
    fn from(value: SAsset) -> Self {
        match value.as_type.clone() {
            EAssetType::Usdt => {Self::Usdt(value)}
            EAssetType::Btc => {Self::Btc(value)}
            _ => {
                // debug 该分支为异常情况
                error!("impl From<SAsset> for EAssetUnion");
                Self::Usdt(SAsset{ as_type: EAssetType::Usdt, balance: Default::default() })
            }
        }
    }
}

impl From<SAssetLeveraged> for EAssetUnion {
    fn from(value: SAssetLeveraged) -> Self {
        match value.get_base().as_type.clone() {
            EAssetType::BtcUsdtFuture => {Self::BtcUsdCmFuture(value)}
            EAssetType::BtcUsdCmFuture => {Self::BtcUsdCmFuture(value)}
            _ => {
                // debug 该分支为异常情况
                error!("impl From<SAsset> for EAssetUnion");
                Self::BtcUsdtFuture(SAssetLeveraged::new(
                    ETradingPairType::BtcUsdt,
                    Decimal::default(),
                    SAsset{ as_type: EAssetType::Usdt, balance: Default::default() },
                    Decimal::default(),
                ).unwrap())
            }
        }
    }
}