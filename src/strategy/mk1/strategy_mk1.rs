use std::collections::VecDeque;
use std::str::FromStr;
use log::info;
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::EOrderAction;
use crate::protocol::{ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult};
use crate::data_runtime::order::order::SAddOrder;
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::data_runtime::user::SUser;
use crate::data_source::trading_pair::ETradingPairType;
use crate::strategy::TStrategy;

/// 测试用策略
#[derive(Debug, Default)]
pub struct SStrategyMk1 {
    pub remove_list: VecDeque<Uuid>,
}

impl SStrategyMk1{
    pub fn new() -> Self {
        Self { remove_list: Default::default() }
    }
}

impl TStrategy for SStrategyMk1 {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
    ) -> Vec<EStrategyAction> {
        todo!()
    }

    fn verify(&mut self, tp_type: &ETradingPairType, parse_action_results: Vec<ERunnerSyncActionResult>) {
        for result in parse_action_results {
            info!("strategy verify:\t{:?}", result);
            match result {
                ERunnerSyncActionResult::OrderPlaced(order) => {
                    match order.get_action() {
                        EOrderAction::Buy => {self.remove_list.push_back(order.get_id())}
                        EOrderAction::Sell => {}
                    }
                }
                ERunnerSyncActionResult::OrderCanceled(_) => {}
            }
        }
        todo!()
    }
}