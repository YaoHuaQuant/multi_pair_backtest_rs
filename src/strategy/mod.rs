use crate::config::SDebugConfig;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::protocol::{ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult};
use crate::strategy::logger::SStrategyLogger;

pub mod mk_test;
pub mod mk1;
pub mod mk2;
pub mod mk3;
pub mod order;
pub mod model;
pub mod logger;
pub mod mk3_2;
pub mod mk4;

/// 策略接口
pub trait TStrategy {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
        debug_config: &SDebugConfig,
    ) -> Vec<EStrategyAction>;
    
    fn verify(
        &mut self,
        tp_type: &ETradingPairType, 
        parse_action_results: Vec<ERunnerSyncActionResult>,
        debug_config: &SDebugConfig,
    );
    
    fn get_log_info(&self) -> SStrategyLogger;
}

