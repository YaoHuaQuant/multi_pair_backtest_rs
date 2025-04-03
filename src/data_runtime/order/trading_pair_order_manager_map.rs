//! 交易对订单管理器

use std::collections::HashMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::order_manager::SOrderManagerV2;

/// TradingPair 和 OrderManager 的Map映射
#[derive(Debug, Default)]
pub struct STradingPairOrderManagerMap {
    pub inner: HashMap<ETradingPairType, SOrderManagerV2>,
}

impl STradingPairOrderManagerMap {
    /// 统计每种资产的总锁定量
    pub fn calculate_total_assets(&self) -> SAssetMap {
        let mut result = SAssetMap::new();
        for (_, order_manager) in self.inner.iter() {
            let asset_manager = order_manager.calculate_total_assets();
            result += asset_manager;
        }
        result
    }

    pub fn get(&self, key: &ETradingPairType) -> Option<&SOrderManagerV2> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &ETradingPairType) -> Option<&mut SOrderManagerV2> {
        self.inner.get_mut(key)
    }
}