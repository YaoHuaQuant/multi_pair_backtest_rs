use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::ops::{Add, AddAssign};
use rust_decimal::Decimal;
use crate::data_runtime::asset::asset::{EAssetError, RemainBalance, RequireBalance, SAsset};
use crate::data_runtime::asset::EAssetType;

pub type RAssetMapResult<T> = Result<T, EAssetMapError>;

#[derive(Debug)]
pub enum EAssetMapError {
    AssetNotFoundError(EAssetType),
    /// 可用额度不足(可用额度，所需额度)
    BalanceNotEnough(RemainBalance, RequireBalance),
    /// 资产类型不匹配
    AssetTypeInconsistentError(EAssetType, EAssetType, SAsset),
}

impl From<EAssetError> for EAssetMapError {
    fn from(value: EAssetError) -> Self {
        match value {
            EAssetError::BalanceNotEnough(a, b) => { Self::BalanceNotEnough(a, b) }
            EAssetError::AssetTypeInconsistentError(a, b, c) => { Self::AssetTypeInconsistentError(a, b, c) }
        }
    }
}

/// AssetType 和 Asset 的Map映射
#[derive(Debug, Default, Clone)]
pub struct SAssetMap {
    pub inner: HashMap<EAssetType, SAsset>,
}

impl SAssetMap {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn add_asset_type(&mut self, as_type: EAssetType) {
        self.inner
            .entry(as_type)
            .or_insert(SAsset {
                as_type,
                balance: Decimal::from(0),
            });
    }

    pub fn add_asset_types(&mut self, as_type_list: Vec<EAssetType>) {
        for asset in as_type_list {
            self.add_asset_type(asset)
        }
    }

    pub fn get(&self, as_type: &EAssetType) -> RAssetMapResult<&SAsset> {
        match self.inner.get(as_type) {
            None => { Err(EAssetMapError::AssetNotFoundError(as_type.clone())) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn get_mut(&mut self, as_type: EAssetType) -> RAssetMapResult<&mut SAsset> {
        match self.inner.get_mut(&as_type) {
            None => { Err(EAssetMapError::AssetNotFoundError(as_type)) }
            Some(item) => { Ok(item) }
        }
    }

    /// 插入一个SAsset
    pub fn merge_asset(&mut self, other: SAsset) {
        let as_type = other.as_type;
        // 如果当前type不存在 则新增
        if let Err(EAssetMapError::AssetNotFoundError(as_type)) = self.get_mut(as_type) {
            self.add_asset_type(as_type)
        }
        let asset = self.get_mut(as_type).unwrap();
        asset.merge(other).unwrap();
    }

    pub fn merge_assets(&mut self, other_vec: Vec<SAsset>) {
        for other in other_vec {
            self.merge_asset(other)
        }
    }

    /// 拆分出一部分新资产
    pub fn split(&mut self, as_type: EAssetType, balance: Decimal) -> RAssetMapResult<SAsset> {
        let asset = self.get_mut(as_type)?;
        Ok(asset.split(balance)?)
    }

    pub fn iter(&self) -> Iter<'_, EAssetType, SAsset> {
        self.inner.iter()
    }
}

impl Add for SAssetMap {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let mut map = self.inner.clone();
        for (key, value) in other.inner {
            map.entry(key)
                .and_modify(|e| *e += value.clone())
                .or_insert(value);
        }
        SAssetMap{ inner: map }
    }
}

impl AddAssign for SAssetMap {
    fn add_assign(&mut self, rhs: Self) {
        for (key, value) in rhs.inner {
            self.inner.entry(key)
                .and_modify(|e| *e += value.clone())
                .or_insert(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use crate::data_runtime::asset::asset_map::{EAssetMapError, SAssetMap};
    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::EAssetType;

    #[test]
    pub fn test1() {
        let mut manager = SAssetMap::new();
        manager.add_asset_type(EAssetType::Btc);
        let asset1 = manager.get(&EAssetType::Usdt);
        assert!(matches!(asset1, Err(EAssetMapError::AssetNotFoundError(EAssetType::Usdt))));
        let asset2 = manager.get_mut(EAssetType::Btc);
        assert!(asset2.is_ok());
        let asset2 = asset2.unwrap();
        assert_eq!(asset2.as_type, EAssetType::Btc);
        assert_eq!(asset2.balance, Decimal::from(0));

        // test merge
        let r = asset2.merge(SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        });
        assert!(r.is_ok());
        assert_eq!(asset2.balance, Decimal::from(10));
    }

    #[test]
    pub fn test_merge() {
        let mut manager = SAssetMap::new();
        manager.add_asset_type(EAssetType::Btc);
        let asset1 = manager.get(&EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(0));

        // test merge success
        let asset2 = SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        };
        manager.merge_asset(asset2);
        let asset1 = manager.get(&EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(10));


        // test merge fail
        let asset3 = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(100),
        };
        manager.merge_asset(asset3);
        let asset4 = manager.get(&EAssetType::Usdt).unwrap();
        assert_eq!(asset4.balance, Decimal::from(100));
        assert_eq!(asset4.as_type, EAssetType::Usdt);
    }

    #[test]
    pub fn test_split() {
        let mut manager = SAssetMap::new();
        manager.add_asset_type(EAssetType::Btc);
        let asset1 = manager.get(&EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(0));

        // test merge success
        let asset2 = SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        };
        manager.merge_asset(asset2);
        let asset1 = manager.get(&EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(10));


        // test split success
        let asset3 = manager.split(EAssetType::Btc, Decimal::from(1)).unwrap();
        assert_eq!(asset3.as_type, EAssetType::Btc);
        assert_eq!(asset3.balance, Decimal::from(1));

        let asset1 = manager.get(&EAssetType::Btc).unwrap();
        assert_eq!(asset1.balance, Decimal::from(9));

        // test split fail
        let asset4 = manager.split(EAssetType::Usdt, Decimal::from(1));
        assert!(asset4.is_err());
        let as_type = EAssetType::Usdt;
        assert!(matches!(asset4, Err(EAssetMapError::AssetNotFoundError(_as_type))));
        if let Err(EAssetMapError::AssetNotFoundError(a)) = asset4 {
            assert_eq!(a, as_type);
        }

        // test split fail
        let asset5 = manager.split(EAssetType::Btc, Decimal::from(10));
        assert!(asset4.is_err());
        let d1 = Decimal::from(9);
        let d10 = Decimal::from(10);
        assert!(matches!(asset5, Err(EAssetMapError::BalanceNotEnough(_d1, _d10))));
        if let Err(EAssetMapError::BalanceNotEnough(a, b)) = asset5 {
            assert_eq!(a, d1);
            assert_eq!(b, d10);
        }
    }
}