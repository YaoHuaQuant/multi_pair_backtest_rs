use std::ops::{AddAssign};
use rust_decimal::Decimal;
use crate::data_runtime::asset::EAssetType;

/// 资产对象
#[derive(Debug, Clone)]
pub struct SAsset {
    /// 资产类型
    pub as_type: EAssetType,
    /// 资产余额
    pub balance: Decimal,
}

pub type RAssetResult<T> = Result<T, EAssetError>;

pub type RemainBalance = Decimal;
pub type RequireBalance = Decimal;

/// Asset异常
#[derive(Debug)]
pub enum EAssetError {
    /// 可用额度不足(可用额度，所需额度)
    BalanceNotEnough(RemainBalance, RequireBalance),
    /// 资产类型不匹配
    AssetTypeInconsistentError(EAssetType, EAssetType, SAsset),
}

impl SAsset {
    pub fn new(as_type: EAssetType) -> Self {
        Self {
            as_type,
            balance: Decimal::from(0),
        }
    }

    /// 合并另一个相同类型的资产到当前资产
    /// 如果执行出错，则可以在Err中获取到输入参数，避免asset资产被消耗
    pub fn merge(&mut self, other: SAsset) -> RAssetResult<()> {
        if self.as_type == other.as_type {
            self.balance += other.balance;
            Ok(())
        } else {
            Err(EAssetError::AssetTypeInconsistentError(self.as_type, other.as_type, other))
        }
    }

    /// 拆分出一部分新资产
    pub fn split(&mut self, balance: Decimal) -> RAssetResult<Self> {
        if self.balance >= balance {
            self.balance -= balance;
            Ok(Self {
                as_type: self.as_type,
                balance,
            })
        } else {
            Err(EAssetError::BalanceNotEnough(self.balance, balance))
        }
    }

    /// 拆分出一部分新资产
    /// 允许被拆分之后的资产为负值
    pub fn split_allow_negative(&mut self, balance: Decimal) -> Self {
        self.balance -= balance;
        Self {
            as_type: self.as_type,
            balance,
        }
    }
    
    pub fn get_balance(&self) -> Decimal {
        self.balance
    }
    
    pub fn get_type(&self) -> EAssetType {
        self.as_type
    }
}


impl AddAssign for SAsset {
    fn add_assign(&mut self, rhs: Self) {
        // debug 没有考虑到type不相同的情况
        self.balance += rhs.balance
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use crate::data_runtime::asset::asset::{EAssetError, SAsset};
    use crate::data_runtime::asset::EAssetType;

    #[test]
    pub fn test() {
        // test new
        let mut asset1 = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(100),
        };
        assert_eq!(asset1.as_type, EAssetType::Usdt);
        assert_eq!(asset1.balance, Decimal::from(100));

        // test split success
        let asset2 = asset1.split(Decimal::from(20));
        assert!(asset2.is_ok());
        let asset2 = asset2.unwrap();
        assert_eq!(asset1.as_type, EAssetType::Usdt);
        assert_eq!(asset1.balance, Decimal::from(80));
        assert_eq!(asset2.as_type, EAssetType::Usdt);
        assert_eq!(asset2.balance, Decimal::from(20));

        // test split fail
        let asset3 = asset1.split(Decimal::from(100));
        assert!(asset3.is_err());

        // test merge success
        let r = asset1.merge(asset2);
        assert!(r.is_ok());
        assert_eq!(asset1.as_type, EAssetType::Usdt);
        assert_eq!(asset1.balance, Decimal::from(100));

        // tes merge fail
        let asset4 = SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(1),
        };
        let r = asset1.merge(asset4);
        assert!(r.is_err());
        let _tmp = Decimal::from(1);
        assert!(matches!(r, Err(EAssetError::AssetTypeInconsistentError(EAssetType::Usdt, EAssetType::Btc, _tmp))));
    }
}