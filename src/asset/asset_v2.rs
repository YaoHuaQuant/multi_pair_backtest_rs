use std::ops::{Add, AddAssign};
use rust_decimal::Decimal;
use crate::asset::EAssetType;

/// 资产对象
#[derive(Debug, Clone, Copy)]
pub struct SAssetV2 {
    /// 资产类型
    pub as_type: EAssetType,
    /// 资产余额
    pub balance: Decimal,
}

pub type RAssetV2Result<T> = Result<T, EAssetV2Error>;

pub type RemainBalance = Decimal;
pub type RequireBalance = Decimal;

/// Asset异常
#[derive(Debug)]
pub enum EAssetV2Error {
    /// 可用额度不足(可用额度，所需额度)
    BalanceNotEnough(RemainBalance, RequireBalance),
    /// 资产类型不匹配
    AssetTypeInconsistentError(EAssetType, EAssetType, SAssetV2),
}

impl SAssetV2 {
    pub fn new(as_type: EAssetType) -> Self {
        Self {
            as_type,
            balance: Decimal::from(0),
        }
    }

    /// 合并另一个相同类型的资产到当前资产
    /// 如果执行出错，则可以在Err中获取到输入参数，避免asset资产被消耗
    pub fn merge(&mut self, other: SAssetV2) -> RAssetV2Result<()> {
        if self.as_type == other.as_type {
            self.balance += other.balance;
            Ok(())
        } else {
            Err(EAssetV2Error::AssetTypeInconsistentError(self.as_type, other.as_type, other))
        }
    }

    /// 拆分出一部分新资产
    pub fn split(&mut self, balance: Decimal) -> RAssetV2Result<Self> {
        if self.balance >= balance {
            self.balance -= balance;
            Ok(Self {
                as_type: self.as_type,
                balance,
            })
        } else {
            Err(EAssetV2Error::BalanceNotEnough(self.balance, balance))
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use crate::asset::asset_v2::{EAssetV2Error, SAssetV2};
    use crate::asset::EAssetType;

    #[test]
    pub fn test() {
        // test new
        let mut asset1 = SAssetV2{
            as_type: EAssetType::Usdt,
            balance: Decimal::from(100),
        };
        assert_eq!(asset1.as_type, EAssetType::Usdt);
        assert_eq!(asset1.balance, Decimal::from(100));

        // test split success
        let mut asset2 = asset1.split(Decimal::from(20));
        assert!(asset2.is_ok());
        let mut asset2 = asset2.unwrap();
        assert_eq!(asset1.as_type, EAssetType::Usdt);
        assert_eq!(asset1.balance, Decimal::from(80));
        assert_eq!(asset2.as_type, EAssetType::Usdt);
        assert_eq!(asset2.balance, Decimal::from(20));

        // test split fail
        let mut asset3 = asset1.split(Decimal::from(100));
        assert!(asset3.is_err());

        // test merge success
        let r = asset1.merge(asset2);
        assert!(r.is_ok());
        assert_eq!(asset1.as_type, EAssetType::Usdt);
        assert_eq!(asset1.balance, Decimal::from(100));

        // tes merge fail
        let asset4 = SAssetV2{
            as_type: EAssetType::Btc,
            balance: Decimal::from(1),
        };
        let r = asset1.merge(asset4);
        assert!(r.is_err());
        let _tmp = Decimal::from(1);
        assert!(matches!(r, Err(EAssetV2Error::AssetTypeInconsistentError(EAssetType::Usdt, EAssetType::Btc, _tmp))));
    }
}