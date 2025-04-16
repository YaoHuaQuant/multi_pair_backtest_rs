use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::protocol::{ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult};

pub mod strategy_mk_test;
pub mod mk1;
pub mod mk2;

/// 策略接口
pub trait TStrategy {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
    ) -> Vec<EStrategyAction>;
    fn verify(&mut self, tp_type: &ETradingPairType, parse_action_results: Vec<ERunnerSyncActionResult>);
}