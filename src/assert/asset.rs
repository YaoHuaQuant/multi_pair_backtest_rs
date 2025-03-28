use rust_decimal::Decimal;

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

/// 资产对象
#[derive(Debug)]
pub struct SAsset {
    /// 资产类型
    pub as_type: EAssetType,
    /// 可用额度
    amount_available: Decimal,
    /// 锁定额度
    amount_locked: Decimal,
}

pub type RAssetResult<T> = Result<T, EAssetError>;

/// Asset异常
#[derive(Debug)]
pub enum EAssetError {
    /// 可用额度不足(可用额度，所需额度)
    AmountAvailableNotEnough(Decimal, Decimal),
    /// 锁定额度不足(锁定额度，所需额度)
    AmountLockedNotEnough(Decimal, Decimal),
}

impl SAsset {
    pub fn new(as_type: EAssetType) -> Self {
        Self {
            as_type,
            amount_available: Decimal::from(0),
            amount_locked: Decimal::from(0),
        }
    }

    pub fn get_available(&self) -> Decimal {
        self.amount_available
    }

    pub fn get_locked(&self) -> Decimal {
        self.amount_locked
    }

    /// 计算资金总量
    pub fn amount_total(&self) -> Decimal {
        self.amount_available + self.amount_locked
    }

    /// 增加可用额度（初始化、转账、入金场景）
    pub fn add(&mut self, amount: Decimal) {
        self.amount_available += amount;
    }

    /// 提取可用额度（转账、出金场景）
    pub fn withdraw(&mut self, amount: Decimal) -> RAssetResult<()> {
        if amount <= self.amount_available {
            self.amount_available -= amount;
            Ok(())
        } else {
            Err(EAssetError::AmountAvailableNotEnough(self.amount_available, amount))
        }
    }

    /// 锁定额度（挂单、修改订单场景）
    pub fn lock(&mut self, amount: Decimal) -> RAssetResult<()> {
        if amount <= self.amount_available {
            self.amount_available -= amount;
            self.amount_locked += amount;
            Ok(())
        } else {
            Err(EAssetError::AmountAvailableNotEnough(self.amount_available, amount))
        }
    }

    /// 解锁额度（撤单、修改订单场景）
    pub fn unlock(&mut self, amount: Decimal) -> RAssetResult<()> {
        if amount <= self.amount_locked {
            self.amount_locked -= amount;
            self.amount_available += amount;
            Ok(())
        } else {
            Err(EAssetError::AmountLockedNotEnough(self.amount_locked, amount))
        }
    }

    /// 提取锁定额度（订单完成场景）
    pub fn withdraw_locked(&mut self, amount: Decimal) -> RAssetResult<()> {
        if amount <= self.amount_locked {
            self.amount_locked -= amount;
            Ok(())
        } else {
            Err(EAssetError::AmountLockedNotEnough(self.amount_locked, amount))
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use crate::assert::asset::{EAssetError, EAssetType, SAsset};

    const INIT_AMOUNT_AVAILABLE: i64 = 100;
    const INIT_AMOUNT_LOCKED: i64 = 200;

    fn get_test_data() -> SAsset {
        SAsset {
            as_type: EAssetType::Usdt,
            amount_available: Decimal::from(INIT_AMOUNT_AVAILABLE),
            amount_locked: Decimal::from(INIT_AMOUNT_LOCKED),
        }
    }

    #[test]
    pub fn test_total() {
        let asset = get_test_data();
        assert_eq!(asset.amount_total(), Decimal::from(300))
    }

    #[test]
    pub fn test_add() {
        let mut asset = get_test_data();
        asset.add(Decimal::from(50));
        assert_eq!(asset.get_available(), Decimal::from(150))
    }

    #[test]
    pub fn test_withdraw() {
        let mut asset = get_test_data();
        let result = asset.withdraw(Decimal::from(50));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_available(), Decimal::from(50));
        let result = asset.withdraw(Decimal::from(50));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_available(), Decimal::from(0));
        let result = asset.withdraw(Decimal::from(50));
        let v1 = Decimal::from(0);
        let v2 = Decimal::from(50);
        dbg!(&result);
        assert!(matches!(result, Err(EAssetError::AmountAvailableNotEnough(_v1, _v2))));
        assert_eq!(asset.get_available(), Decimal::from(0))
    }

    #[test]
    pub fn test_withdraw_locked() {
        let mut asset = get_test_data();
        let result = asset.withdraw_locked(Decimal::from(100));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_locked(), Decimal::from(100));
        let result = asset.withdraw_locked(Decimal::from(100));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_locked(), Decimal::from(0));
        let result = asset.withdraw_locked(Decimal::from(100));
        let v1 = Decimal::from(0);
        let v2 = Decimal::from(100);
        dbg!(&result);
        assert!(matches!(result, Err(EAssetError::AmountLockedNotEnough(_v1, _v2))));
        assert_eq!(asset.get_locked(), Decimal::from(0))
    }

    #[test]
    pub fn test_lock() {
        let mut asset = get_test_data();
        let result = asset.lock(Decimal::from(50));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_available(), Decimal::from(50));
        assert_eq!(asset.get_locked(), Decimal::from(250));
        let result = asset.lock(Decimal::from(50));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_available(), Decimal::from(0));
        assert_eq!(asset.get_locked(), Decimal::from(300));
        let result = asset.lock(Decimal::from(50));
        let v1 = Decimal::from(0);
        let v2 = Decimal::from(50);
        dbg!(&result);
        assert!(matches!(result, Err(EAssetError::AmountAvailableNotEnough(_v1, _v2))));
        assert_eq!(asset.get_available(), Decimal::from(0));
        assert_eq!(asset.get_locked(), Decimal::from(300));
    }

    #[test]
    pub fn test_unlock() {
        let mut asset = get_test_data();
        let result = asset.unlock(Decimal::from(100));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_locked(), Decimal::from(100));
        assert_eq!(asset.get_available(), Decimal::from(200));
        let result = asset.unlock(Decimal::from(100));
        assert!(matches!(result, Ok(())));
        assert_eq!(asset.get_locked(), Decimal::from(0));
        assert_eq!(asset.get_available(), Decimal::from(300));
        let result = asset.unlock(Decimal::from(100));
        let v1 = Decimal::from(0);
        let v2 = Decimal::from(100);
        dbg!(&result);
        assert!(matches!(result, Err(EAssetError::AmountLockedNotEnough(_v1, _v2))));
        assert_eq!(asset.get_locked(), Decimal::from(0));
        assert_eq!(asset.get_available(), Decimal::from(300));
    }
}