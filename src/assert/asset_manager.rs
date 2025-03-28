use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::assert::asset::{EAssetType, RAssetResult, SAsset};

pub type RAssetManagerResult<T> = Result<T, EAssetManagerError>;

#[derive(Debug)]
pub enum EAssetManagerError {
    AssetNotFoundError(EAssetType)
}

#[derive(Debug)]
pub struct SAssetManager {
    pub asset_map: HashMap<EAssetType, SAsset>,
}

impl SAssetManager {
    pub fn new() -> Self {
        Self {
            asset_map: Default::default(),
        }
    }

    pub fn add_asset(&mut self, as_type: EAssetType) {
        self.asset_map.entry(as_type).or_insert(SAsset::new(as_type));
    }

    pub fn add_assets(&mut self, as_types: Vec<EAssetType>) {
        for as_type in as_types {
            self.add_asset(as_type)
        }
    }

    pub fn get(&self, as_type: EAssetType) -> RAssetManagerResult<&SAsset> {
        match self.asset_map.get(&as_type) {
            None => { Err(EAssetManagerError::AssetNotFoundError(as_type)) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn get_mut(&mut self, as_type: EAssetType) -> RAssetManagerResult<&mut SAsset> {
        match self.asset_map.get_mut(&as_type) {
            None => { Err(EAssetManagerError::AssetNotFoundError(as_type)) }
            Some(item) => { Ok(item) }
        }
    }

    // region ----- 转发SAsset函数-----
    pub fn get_available(&self, as_type: EAssetType) -> RAssetManagerResult<Decimal> {
        Ok(self.get(as_type)?.get_available())
    }

    pub fn get_locked(&self, as_type: EAssetType) -> RAssetManagerResult<Decimal> {
        Ok(self.get(as_type)?.get_locked())
    }

    pub fn amount_total(&self, as_type: EAssetType) -> RAssetManagerResult<Decimal> {
        Ok(self.get(as_type)?.amount_total())
    }

    pub fn add(&mut self, as_type: EAssetType, amount: Decimal) -> RAssetManagerResult<()> {
        Ok(self.get_mut(as_type)?.add(amount))
    }

    pub fn withdraw(&mut self, as_type: EAssetType, amount: Decimal) -> RAssetManagerResult<RAssetResult<()>> {
        Ok(self.get_mut(as_type)?.withdraw(amount))
    }

    pub fn lock(&mut self, as_type: EAssetType, amount: Decimal) -> RAssetManagerResult<RAssetResult<()>> {
        Ok(self.get_mut(as_type)?.lock(amount))
    }

    pub fn unlock(&mut self, as_type: EAssetType, amount: Decimal) -> RAssetManagerResult<RAssetResult<()>> {
        Ok(self.get_mut(as_type)?.unlock(amount))
    }

    pub fn withdraw_locked(&mut self, as_type: EAssetType, amount: Decimal) -> RAssetManagerResult<RAssetResult<()>> {
        Ok(self.get_mut(as_type)?.withdraw_locked(amount))
    }
    // endregion ----- 转发SAsset函数-----
}

#[cfg(test)]
mod tests {
    use crate::assert::asset::EAssetType;
    use crate::assert::asset_manager::{EAssetManagerError, SAssetManager};

    #[test]
    pub fn test1() {
        let mut manager =  SAssetManager::new();
        manager.add_asset(EAssetType::Btc);
        let asset1 = manager.get(EAssetType::Usdt);
        let asset2 = manager.get(EAssetType::Btc);
        assert!(matches!(asset1, Err(EAssetManagerError::AssetNotFoundError(EAssetType::Usdt))));
        assert!(asset2.is_ok());
        assert_eq!(asset2.unwrap().as_type, EAssetType::Btc);
    }
}