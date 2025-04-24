use std::collections::VecDeque;
use std::str::FromStr;
use log::info;
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::EOrderAction;
use crate::protocol::{ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult, SStrategyOrderAdd};
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::strategy::TStrategy;

/// 测试用策略
#[derive(Debug, Default)]
pub struct SStrategyMkTest {
    pub remove_list: VecDeque<Uuid>,
}

impl SStrategyMkTest {
    pub fn new() -> Self {
        Self { remove_list: Default::default() }
    }
}

impl TStrategy for SStrategyMkTest {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
    ) -> Vec<EStrategyAction> {
        let SRunnerParseKlineResult {
            tp_type,
            new_kline: kline_unit,
            new_funding_rate: _,
            order_result
        } = runner_parse_result;
        // 输出执行器结果
        for order_result in order_result {
            info!("strategy receive order result:\t{:?}", order_result)
        }

        let mut result = Vec::new();

        // 从remove_list中删除一个订单
        if let Some(uuid) = self.remove_list.pop_front() {
            result.push(EStrategyAction::CancelOrder(uuid));
        }

        let action_new_order1 = SStrategyOrderAdd {
            id: None,
            tp_type,
            action: EOrderAction::Buy,
            price: kline_unit.low_price,
            quantity: Decimal::from_str("0.1").unwrap(),
        };
        let action_new_order2 = SStrategyOrderAdd {
            id: None,
            tp_type,
            action: EOrderAction::Sell,
            price: kline_unit.high_price,
            quantity: Decimal::from_str("0.1").unwrap(),
        };
        result.push(EStrategyAction::NewOrder(action_new_order1));
        result.push(EStrategyAction::NewOrder(action_new_order2));
        result
    }

    fn verify(&mut self, tp_type: &ETradingPairType, parse_action_results: Vec<ERunnerSyncActionResult>) {
        for result in parse_action_results {
            info!("strategy verify:\t{:?}", result);
            match result {
                ERunnerSyncActionResult::OrderPlaced(order, _) => {
                    match order.get_action() {
                        EOrderAction::Buy => { self.remove_list.push_back(order.get_id()) }
                        EOrderAction::Sell => {}
                    }
                }
                ERunnerSyncActionResult::OrderCanceled(_) => {}
            }
        }
    }
}