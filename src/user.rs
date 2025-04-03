//! 用户实体
//! 主要用于打包策略、订单和资产
//! User的订单和资产数据只能被Runner修改，无法被User自身修改。

use std::fmt::Debug;

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;

use crate::config::{INIT_BALANCE_BTC, INIT_BALANCE_USDT};
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::protocol::{ERunnerParseActionResult, EStrategyAction, SRunnerParseResult};
use crate::strategy::TStrategy;

#[derive(Debug)]
pub struct SUserConfig {
    pub init_balance_usdt: Decimal,
    pub init_balance_btc: Decimal,
}

impl Default for SUserConfig {
    fn default() -> Self {
        Self {
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

    /// 订单管理器
    pub tp_order_map: STradingPairOrderManagerMap,

    /// 可用资产管理器
    pub available_assets: SAssetMap,

    /// 策略
    pub strategy: S,
}

impl<S: TStrategy> SUser<S> {
    pub fn new(config: SUserConfig, strategy: S) -> Self {
        Self {
            config,
            id: Uuid::new_v4(),
            tp_order_map: Default::default(),
            available_assets: Default::default(),
            strategy,
        }
    }

    /// 将增量数据传输给策略模块，获取策略行为。
    pub fn get_strategy_result(&mut self, runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction> {
        self.strategy.run(runner_parse_result)
    }

    /// 向策略模块反馈校验、调整结果
    pub fn response_verify_result(&mut self, runner_parse_action_results: Vec<ERunnerParseActionResult>) {
        self.strategy.verify(runner_parse_action_results)
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
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test1() {
        todo!()
    }
}