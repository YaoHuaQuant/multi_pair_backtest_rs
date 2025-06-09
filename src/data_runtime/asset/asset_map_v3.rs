use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::ops::{Add, AddAssign};
use rust_decimal::Decimal;
use crate::data_runtime::asset::asset::{EAssetError, RemainBalance, RequireBalance};
use crate::data_runtime::asset::asset_union::{EAssetUnion, EAssetUnionError};
use crate::data_runtime::asset::EAssetType;


pub type RAssetMapV3Result<T> = Result<T, EAssetMapV3Error>;

pub type RequireType = EAssetType;
pub type ActualType = EAssetType;

#[derive(Debug)]
pub enum EAssetMapV3Error {
    AssetNotFoundError(EAssetType),
    /// 可用额度不足(可用额度，所需额度)
    BalanceNotEnough(RemainBalance, RequireBalance),
    /// 资产类型不匹配
    AssetTypeInconsistentError(RequireType, ActualType),
}

impl From<EAssetError> for EAssetMapV3Error {
    fn from(value: EAssetError) -> Self {
        match value {
            EAssetError::BalanceNotEnough(a, b) => { Self::BalanceNotEnough(a, b) }
            EAssetError::AssetTypeInconsistentError(a, b, _c) => { Self::AssetTypeInconsistentError(a, b) }
        }
    }
}

impl From<EAssetUnionError> for EAssetMapV3Error {
    fn from(value: EAssetUnionError) -> Self {
        match value {
            EAssetUnionError::AssetTypeInconsistentError(a, b) => {
                Self::AssetTypeInconsistentError(a.get_asset_type(), b.get_asset_type())
            }
        }
    }
}

/// AssetType 和 Asset 的Map映射
#[derive(Debug, Default, Clone)]
pub struct SAssetMapV3 {
    pub inner: HashMap<EAssetType, EAssetUnion>,
}

impl SAssetMapV3 {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn get(&self, as_type: &EAssetType) -> RAssetMapV3Result<&EAssetUnion> {
        match self.inner.get(as_type) {
            None => { Err(EAssetMapV3Error::AssetNotFoundError(as_type.clone())) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn get_mut(&mut self, as_type: EAssetType) -> RAssetMapV3Result<&mut EAssetUnion> {
        match self.inner.get_mut(&as_type) {
            None => { Err(EAssetMapV3Error::AssetNotFoundError(as_type)) }
            Some(item) => { Ok(item) }
        }
    }

    /// 插入一个EAsset
    pub fn merge_asset(&mut self, other: EAssetUnion) {
        let as_type = other.get_asset_type();
        // 如果当前type不存在 则新增
        match self.get_mut(as_type) {
            Ok(asset) => {
                asset.merge(other).unwrap();
            }
            Err(e) => {
                if let EAssetMapV3Error::AssetNotFoundError(as_type) = e {
                    self.inner.insert(as_type, other);
                }
            }
        }
    }

    pub fn merge_assets(&mut self, other_vec: Vec<EAssetUnion>) {
        for other in other_vec {
            self.merge_asset(other)
        }
    }

    /// 拆分出一部分新资产
    /// deprecated
    pub fn split(&mut self, as_type: EAssetType, balance: Decimal) -> RAssetMapV3Result<EAssetUnion> {
        let asset = self.get_mut(as_type)?;
        match asset.split(balance) {
            None => { Err(EAssetMapV3Error::BalanceNotEnough(asset.get_balance(), balance)) }
            Some(asset) => { Ok(asset) }
        }
    }

    /// 拆分出一部分新资产
    pub fn split_allow_negative(&mut self, as_type: EAssetType, balance: Decimal) -> RAssetMapV3Result<EAssetUnion> {
        let asset = self.get_mut(as_type)?;
        Ok(asset.split_allow_negative(balance))
    }

    pub fn iter(&self) -> Iter<'_, EAssetType, EAssetUnion> {
        self.inner.iter()
    }
}

impl Add for SAssetMapV3 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let mut map = self.inner.clone();
        for (key, value) in other.inner {
            map.entry(key)
                .and_modify(|e| *e += value.clone())
                .or_insert(value);
        }
        SAssetMapV3{ inner: map }
    }
}

impl AddAssign for SAssetMapV3 {
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

    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::asset_map_v3::{EAssetMapV3Error, SAssetMapV3};
    use crate::data_runtime::asset::asset_union::EAssetUnion;
    use crate::data_runtime::asset::EAssetType;

    pub fn get_test_data() -> SAssetMapV3 {
        let mut manager = SAssetMapV3::new();
        manager.merge_asset(EAssetUnion::Btc(SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10),
        }));
        manager.merge_asset(EAssetUnion::Usdt(SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(100_000),
        }));
        manager
    }

    #[test]
    pub fn test1() {
        let mut manager = get_test_data();
        let asset1 = manager.get(&EAssetType::BtcUsdtFuture);
        assert!(matches!(asset1, Err(EAssetMapV3Error::AssetNotFoundError(EAssetType::BtcUsdtFuture))));
        let asset2 = manager.get_mut(EAssetType::Btc);
        assert!(asset2.is_ok());
        let asset2 = asset2.unwrap();
        assert_eq!(asset2.get_asset_type(), EAssetType::Btc);
        assert_eq!(asset2.get_balance(), Decimal::from(10));
    }

    #[test]
    pub fn test_split_success() {
        let mut manager = get_test_data();

        // test split success
        let asset3 = manager.split(EAssetType::Btc, Decimal::from(1)).unwrap();
        assert_eq!(asset3.get_asset_type(), EAssetType::Btc);
        assert_eq!(asset3.get_balance(), Decimal::from(1));

        let asset1 = manager.get(&EAssetType::Btc).unwrap();
        assert_eq!(asset1.get_balance(), Decimal::from(9));
    }

    #[test]
    pub fn test_split_fail() {
        let mut manager = get_test_data();

        // test split fail
        let asset = manager.split(EAssetType::Btc, Decimal::from(11));
        assert!(asset.is_err());
        assert!(matches!(asset, Err(EAssetMapV3Error::BalanceNotEnough(_, _))));
        if let Err(EAssetMapV3Error::BalanceNotEnough(remain, require)) = asset {
            assert_eq!(remain, Decimal::from(10));
            assert_eq!(require, Decimal::from(11));
        }
        
        let asset2 = manager.get_mut(EAssetType::Btc);
        assert!(asset2.is_ok());
        let asset2 = asset2.unwrap();
        assert_eq!(asset2.get_asset_type(), EAssetType::Btc);
        assert_eq!(asset2.get_balance(), Decimal::from(10));
    }
}