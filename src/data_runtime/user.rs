//! 用户实体
//! 主要用于打包策略、订单和资产
//! User的订单和资产数据只能被Runner修改，无法被User自身修改。

use std::fmt::Debug;

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;

use crate::{
    config::{INIT_BALANCE_BTC, INIT_BALANCE_USDT},
    data_runtime::{
        asset::{
            asset::SAsset,
            asset_map::SAssetMap,
            EAssetType,
        },
        order::trading_pair_order_manager_map::STradingPairOrderManagerMap,
    },
    data_source::trading_pair::ETradingPairType,
    protocol::{ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult},
    strategy::TStrategy,
};
use crate::data_runtime::asset::asset_map::RAssetMapResult;

#[derive(Debug, Clone)]
pub struct SUserConfig {
    pub user_name: String,
    pub init_balance_usdt: Decimal,
    pub init_balance_btc: Decimal,
}

impl Default for SUserConfig {
    fn default() -> Self {
        Self {
            user_name: "Satoshi Nakamoto".to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct SUser<S: TStrategy> {
    /// 用户配置
    pub config: SUserConfig,

    pub id: Uuid,

    pub name: String,

    /// 订单管理器
    pub tp_order_map: STradingPairOrderManagerMap,

    /// 可用资产管理器
    pub available_assets: SAssetMap,

    /// 策略
    pub strategy: S,
}

impl<S: TStrategy> SUser<S> {
    pub fn new(config: SUserConfig, strategy: S) -> Self {
        let mut available_assets = SAssetMap::new();
        available_assets.merge_asset(SAsset { as_type: EAssetType::Usdt, balance: config.init_balance_usdt });
        available_assets.merge_asset(SAsset { as_type: EAssetType::Btc, balance: config.init_balance_btc });
        let mut tp_order_map = STradingPairOrderManagerMap { inner: Default::default() };
        tp_order_map.inner.insert(ETradingPairType::BtcUsdt, Default::default());
        tp_order_map.inner.insert(ETradingPairType::BtcUsdCmFuture, Default::default());
        tp_order_map.inner.insert(ETradingPairType::BtcUsdtFuture, Default::default());
        Self {
            config:config.clone(),
            id: Uuid::new_v4(),
            name: config.user_name,
            tp_order_map,
            available_assets,
            strategy,
        }
    }

    /// 将执行器的处理结果反馈给策略模块 获取策略Action结果
    pub fn get_strategy_result(&mut self, runner_parse_result: SRunnerParseKlineResult) -> Vec<EStrategyAction> {
        self.strategy.run(&mut self.tp_order_map, &mut self.available_assets, runner_parse_result)
    }

    /// 向策略模块反馈同步结果
    pub fn verify_sync_result(&mut self, tp_type: &ETradingPairType, runner_parse_action_results: Vec<ERunnerSyncActionResult>) {
        self.strategy.verify(tp_type, runner_parse_action_results)
    }

    /// 累计用户的总资产
    pub fn total_asset(&self) -> SAssetMap {
        self.locked_assets() + self.available_assets()
    }

    /// 累计用户的总资产
    pub fn locked_assets(&self) -> SAssetMap {
        self.tp_order_map.calculate_total_assets()
    }

    /// 累计用户的总资产
    pub fn available_assets(&self) -> SAssetMap {
        self.available_assets.clone()
    }

    /// 累计用户的总手续费
    pub fn total_fee(&self) -> SAssetMap {
        self.tp_order_map.calculate_total_fees()
    }

    /// 向可用资产插入SAsset
    pub fn merge_available_asset(&mut self, other: SAsset) {
        self.available_assets.merge_asset(other)
    }

    /// 从可用资产中拆分出一部分
    pub fn split_available_asset(&mut self, as_type: EAssetType, balance: Decimal) -> RAssetMapResult<SAsset> {
        self.available_assets.split(as_type, balance)
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
    use crate::data_runtime::user::{SUser, SUserConfig};
    use crate::data_source::trading_pair::ETradingPairType;
    use crate::strategy::mk_test::SStrategyMkTest;

    fn get_test_data() -> SUser<SStrategyMkTest> {
        let user_config = SUserConfig {
            user_name: "Satoshi Nakamoto".to_string(),
            init_balance_usdt: Decimal::from(10000),
            init_balance_btc: Decimal::from(0),
        };
        let mut user = SUser::new(user_config, SStrategyMkTest::new());
        let mut tp_order_map = STradingPairOrderManagerMap::default();
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
            let id = order_manager.add_new_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("0.01").unwrap(),
            });
        }
        for (uuid, order) in order_manager.orders.iter_mut() {
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
            let id = order_manager.add_new_order(SAddOrder {
                action: EOrderAction::Sell,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            });
        }
        for (uuid, order) in order_manager.orders.iter_mut() {
            let asset = SAsset { as_type: EAssetType::Btc, balance: order.get_quantity() };
            let _r = order.submit(asset);
        }

        let _r = tp_order_map.insert(ETradingPairType::BtcUsdt, order_manager);

        user.tp_order_map = tp_order_map;
        user
    }
    #[test]
    pub fn test_total_asset() {
        let mut user = get_test_data();
        let total_asset = user.total_asset();
        let usdt = total_asset.get(&EAssetType::Usdt);
        assert!(usdt.is_ok());
        assert_eq!(usdt.unwrap().balance, Decimal::from(10021));
        let btc = total_asset.get(&EAssetType::Btc);
        assert!(btc.is_ok());
        assert_eq!(btc.unwrap().balance, Decimal::from(6));
    }

    #[test]
    pub fn test_locked_asset() {
        let mut user = get_test_data();
        let locked_assets = user.locked_assets();
        let usdt = locked_assets.get(&EAssetType::Usdt);
        assert!(usdt.is_ok());
        assert_eq!(usdt.unwrap().balance, Decimal::from(21));
        let btc = locked_assets.get(&EAssetType::Btc);
        assert!(btc.is_ok());
        assert_eq!(btc.unwrap().balance, Decimal::from(6));
    }

    #[test]
    pub fn test_available_assets() {
        let mut user = get_test_data();
        let available_assets = user.available_assets();
        let usdt = available_assets.get(&EAssetType::Usdt);
        assert!(usdt.is_ok());
        assert_eq!(usdt.unwrap().balance, Decimal::from(10000));
        let btc = available_assets.get(&EAssetType::Btc);
        assert!(btc.is_ok());
        assert_eq!(btc.unwrap().balance, Decimal::from(0));
    }
}