//! 交易对订单管理器

use std::collections::HashMap;

use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::asset::asset_map_v3::SAssetMapV3;
use crate::data_runtime::order::order_manager_v3::SOrderManagerV3;
use crate::data_source::trading_pair::ETradingPairType;

/// TradingPair 和 OrderManager 的Map映射
#[derive(Debug, Default)]
pub struct STradingPairOrderManagerMapV3 {
    pub inner: HashMap<ETradingPairType, SOrderManagerV3>,
}

impl STradingPairOrderManagerMapV3 {
    /// 统计每种资产的总锁定量
    pub fn calculate_total_assets(&self) -> SAssetMapV3 {
        let mut result = SAssetMapV3::new();
        for (_, order_manager) in self.inner.iter() {
            let asset_manager = order_manager.calculate_total_assets();
            result += asset_manager;
        }
        result
    }

    /// 统计累计手续费
    pub fn calculate_total_fees(&self) -> SAssetMap {
        let mut result = SAssetMap::new();
        for (_, order_manager) in self.inner.iter() {
            let asset_manager = order_manager.calculate_total_fee();
            result += asset_manager;
        }
        result
    }

    pub fn insert(&mut self, tp_type: ETradingPairType, order_manager: SOrderManagerV3) -> Option<SOrderManagerV3> {
        self.inner.insert(tp_type, order_manager)
    }

    pub fn get(&self, key: &ETradingPairType) -> Option<&SOrderManagerV3> {
        self.inner.get(key)
    }

    pub fn get_mut(&mut self, key: &ETradingPairType) -> Option<&mut SOrderManagerV3> {
        self.inner.get_mut(key)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use rust_decimal::Decimal;
    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::EAssetType;
    use crate::data_runtime::order::EOrderAction;
    use crate::data_runtime::order::order::SAddOrder;
    use crate::data_runtime::order::order_manager::SOrderManager;
    use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
    use crate::data_source::trading_pair::ETradingPairType;

    fn get_test_data() -> STradingPairOrderManagerMap {
        let mut data = STradingPairOrderManagerMap::default();
        let mut order_manager = SOrderManager::new();

        let price_vec1 = vec![
            Decimal::from_str("100").unwrap(),
            Decimal::from_str("200").unwrap(),
            Decimal::from_str("300").unwrap(),
            Decimal::from_str("400").unwrap(),
            Decimal::from_str("500").unwrap(),
            Decimal::from_str("600").unwrap(),
        ];

        for price in price_vec1 {
            let _id = order_manager.add_new_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("0.01").unwrap(),
            });
        }
        for (_uuid, order) in order_manager.orders.iter_mut() {
            let asset = SAsset { as_type: EAssetType::Usdt, balance: order.get_amount() };
            let _r = order.submit(asset);
        }

        let price_vec2 = vec![
            Decimal::from_str("100").unwrap(),
            Decimal::from_str("200").unwrap(),
            Decimal::from_str("300").unwrap(),
            Decimal::from_str("400").unwrap(),
            Decimal::from_str("500").unwrap(),
            Decimal::from_str("600").unwrap(),
        ];

        for price in price_vec2 {
            let _id = order_manager.add_new_order(SAddOrder {
                action: EOrderAction::Sell,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            });
        }
        for (_uuid, order) in order_manager.orders.iter_mut() {
            let asset = SAsset { as_type: EAssetType::Btc, balance: order.get_quantity() };
            let _r = order.submit(asset);
        }

        let _r = data.insert(ETradingPairType::BtcUsdt, order_manager);
        data
    }

    #[test]
    pub fn test_calculate_total_assets() {
        let data = get_test_data();
        let asset_map = data.calculate_total_assets();
        let pair_btc_usdt = asset_map.get(&EAssetType::Usdt);
        let pair_btc_btc = asset_map.get(&EAssetType::Btc);
        let pair_btc_btc_usdt_future = asset_map.get(&EAssetType::BtcUsdtFuture);
        assert!(pair_btc_usdt.is_ok());
        assert_eq!(pair_btc_usdt.unwrap().balance, Decimal::from(21));
        assert!(pair_btc_btc.is_ok());
        assert_eq!(pair_btc_btc.unwrap().balance, Decimal::from(6));
        assert!(pair_btc_btc_usdt_future.is_err());

    }
}
