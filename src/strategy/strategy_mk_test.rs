use std::str::FromStr;
use log::info;
use rust_decimal::Decimal;
use crate::order::EOrderAction;
use crate::protocol::{ERunnerParseActionResult, EStrategyAction, SRunnerParseResult};
use crate::runner::strategy_runner::order::order::{SAddOrder, SOrder};
use crate::strategy::TStrategy;

/// 测试用策略
#[derive(Debug)]
pub struct SStrategyMkTest {}

impl TStrategy for SStrategyMkTest {
    fn run(&mut self, runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction> {
        // 输出执行器结果
        for order_result in runner_parse_result.order_result {
            info!("strategy receive order result:\t{:?}", order_result)
        }

        let kline_unit = runner_parse_result.new_kline;
        let action_new_order1 = SAddOrder {
            action: EOrderAction::Buy,
            price: kline_unit.low_price,
            quantity: Decimal::from_str("0.1").unwrap(),
        };
        let action_new_order2 = SAddOrder {
            action: EOrderAction::Sell,
            price: kline_unit.high_price,
            quantity: Decimal::from_str("0.1").unwrap(),
        };
        let result = vec![
            EStrategyAction::NewOrder(action_new_order1),
            EStrategyAction::NewOrder(action_new_order2),
        ];
        result
    }

    fn verify(&mut self, parse_action_results: Vec<ERunnerParseActionResult>) {
        for result in parse_action_results {
            info!("strategy verify:\t{:?}", result)
        }
    }
}