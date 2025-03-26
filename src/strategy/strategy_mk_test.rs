use std::str::FromStr;
use rust_decimal::Decimal;
use crate::order::EOrderAction;
use crate::protocol::{EStrategyAction, SRunnerParseResult};
use crate::runner::strategy_runner::order::runner_order::SOrder;
use crate::strategy::TStrategy;

/// 测试用策略
#[derive(Debug)]
pub struct SStrategyMkTest {}

impl TStrategy for SStrategyMkTest {
    fn run(runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction> {
        let kline_unit = runner_parse_result.new_kline;
        let action_new_order = SOrder {
            id: Default::default(),
            state: Default::default(),
            action: EOrderAction::Buy,
            price: kline_unit.high_price,
            quantity: Decimal::from_str("0.1").unwrap(),
        };
        let result = vec![EStrategyAction::NewOrder(action_new_order)];
        result
    }
}