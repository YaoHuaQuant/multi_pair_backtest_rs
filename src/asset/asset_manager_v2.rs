use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::asset::asset_v2::{EAssetV2Error, RemainBalance, RequireBalance, SAssetV2};
use crate::asset::EAssetType;

pub type RAssetManagerV2Result<T> = Result<T, EAssetManagerV2Error>;

#[derive(Debug)]
pub enum EAssetManagerV2Error {
    AssetNotFoundError(EAssetType),
    /// 可用额度不足(可用额度，所需额度)
    BalanceNotEnough(RemainBalance, RequireBalance),
    /// 资产类型不匹配
    AssetTypeInconsistentError(EAssetType, EAssetType, SAssetV2),
}

impl From<EAssetV2Error> for EAssetManagerV2Error {
    fn from(value: EAssetV2Error) -> Self {
        match value {
            EAssetV2Error::BalanceNotEnough(a, b) => { Self::BalanceNotEnough(a, b) }
            EAssetV2Error::AssetTypeInconsistentError(a, b, c) => { Self::AssetTypeInconsistentError(a, b, c) }
        }
    }
}

#[derive(Debug)]
pub struct SAssetV2Manager {
    pub asset_map: HashMap<EAssetType, SAssetV2>,
}

impl SAssetV2Manager {
    pub fn new() -> Self {
        Self {
            asset_map: Default::default(),
        }
    }

    pub fn add_asset(&mut self, as_type: EAssetType) {
        self.asset_map
            .entry(as_type)
            .or_insert(SAssetV2 {
                as_type,
                balance: Decimal::from(0),
            });
    }

    pub fn add_assets(&mut self, as_type_list: Vec<EAssetType>) {
        for asset in as_type_list {
            self.add_asset(asset)
        }
    }

    pub fn get(&self, as_type: EAssetType) -> RAssetManagerV2Result<&SAssetV2> {
        match self.asset_map.get(&as_type) {
            None => { Err(EAssetManagerV2Error::AssetNotFoundError(as_type)) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn get_mut(&mut self, as_type: EAssetType) -> RAssetManagerV2Result<&mut SAssetV2> {
        match self.asset_map.get_mut(&as_type) {
            None => { Err(EAssetManagerV2Error::AssetNotFoundError(as_type)) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn merge(&mut self, other: SAssetV2) {
        let as_type = other.as_type;
        // 如果当前type不存在 则新增
        if let Err(EAssetManagerV2Error::AssetNotFoundError(as_type)) = self.get_mut(as_type) {
            self.add_asset(as_type)
        }
        let asset = self.get_mut(as_type).unwrap();
        asset.merge(other).unwrap();
    }

    pub fn merges(&mut self, other_vec: Vec<SAssetV2>) {
        for other in other_vec {
            self.merge(other)
        }
    }

    /// 拆分出一部分新资产
    pub fn split(&mut self, as_type: EAssetType, balance: Decimal) -> RAssetManagerV2Result<SAssetV2> {
        let asset = self.get_mut(as_type)?;
        Ok(asset.split(balance)?)
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use crate::asset::asset_manager_v2::{EAssetManagerV2Error, SAssetV2Manager};
    use crate::asset::asset_v2::SAssetV2;
    use crate::asset::EAssetType;

    #[test]
    pub fn test1() {
        let mut manager = SAssetV2Manager::new();
        manager.add_asset(EAssetType::Btc);
        let mut asset1 = manager.get(EAssetType::Usdt);
        assert!(matches!(asset1, Err(EAssetManagerV2Error::AssetNotFoundError(EAssetType::Usdt))));
        let mut asset2 = manager.get_mut(EAssetType::Btc);
        assert!(asset2.is_ok());
        let mut asset2 = asset2.unwrap();
        assert_eq!(asset2.as_type, EAssetType::Btc);
        assert_eq!(asset2.balance, Decimal::from(0));

        // test merge
        let r = asset2.merge(SAssetV2 {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        });
        assert!(r.is_ok());
        assert_eq!(asset2.balance, Decimal::from(10));
    }

    #[test]
    pub fn test_merge() {
        let mut manager = SAssetV2Manager::new();
        manager.add_asset(EAssetType::Btc);
        let mut asset1 = manager.get(EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(0));

        // test merge success
        let asset2 = SAssetV2 {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        };
        manager.merge(asset2);
        let mut asset1 = manager.get(EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(10));


        // test merge fail
        let asset3 = SAssetV2 {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(100),
        };
        manager.merge(asset3);
        let asset4 = manager.get(EAssetType::Usdt).unwrap();
        assert_eq!(asset4.balance, Decimal::from(100));
        assert_eq!(asset4.as_type, EAssetType::Usdt);
    }

    #[test]
    pub fn test_split() {
        let mut manager = SAssetV2Manager::new();
        manager.add_asset(EAssetType::Btc);
        let mut asset1 = manager.get(EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(0));

        // test merge success
        let asset2 = SAssetV2 {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        };
        manager.merge(asset2);
        let mut asset1 = manager.get(EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(10));


        // test split success
        let asset3 = manager.split(EAssetType::Btc, Decimal::from(1)).unwrap();
        assert_eq!(asset3.as_type, EAssetType::Btc);
        assert_eq!(asset3.balance, Decimal::from(1));

        let mut asset1 = manager.get(EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(9));

        // test split fail
        let asset4 = manager.split(EAssetType::Usdt, Decimal::from(1));
        assert!(asset4.is_err());
        let as_type = EAssetType::Usdt;
        assert!(matches!(asset4, Err(EAssetManagerV2Error::AssetNotFoundError(as_type))));
        if let Err(EAssetManagerV2Error::AssetNotFoundError(a)) = asset4 {
            assert_eq!(a, as_type);
        }

        // test split fail
        let asset5 = manager.split(EAssetType::Btc, Decimal::from(10));
        assert!(asset4.is_err());
        let d1 = Decimal::from(9);
        let d10 = Decimal::from(10);
        assert!(matches!(asset5, Err(EAssetManagerV2Error::BalanceNotEnough(_d1, _d10))));
        if let Err(EAssetManagerV2Error::BalanceNotEnough(a, b)) = asset5 {
            assert_eq!(a, d1);
            assert_eq!(b, d10);
        }
    }
}