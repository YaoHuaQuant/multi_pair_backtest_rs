use std::collections::VecDeque;
use std::str::FromStr;
use log::info;
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::order::EOrderAction;
use crate::protocol::{ERunnerParseActionResult, EStrategyAction, SRunnerParseResult};
use crate::runner::strategy_runner::order::order::{SAddOrder, SOrder};
use crate::strategy::TStrategy;

/// 测试用策略
#[derive(Debug)]
pub struct SStrategyMkTest {
    pub remove_list: VecDeque<Uuid>,
}

impl SStrategyMkTest{
    pub fn new() -> Self {
        Self { remove_list: Default::default() }
    }
}

impl TStrategy for SStrategyMkTest {
    fn run(&mut self, runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction> {
        // 输出执行器结果
        for order_result in runner_parse_result.order_result {
            info!("strategy receive order result:\t{:?}", order_result)
        }

        let mut result = Vec::new();

        // 从remove_list中删除一个订单
        if let Some(uuid) = self.remove_list.pop_front() {
            result.push(EStrategyAction::CancelOrder(uuid));
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
        result.push(EStrategyAction::NewOrder(action_new_order1));
        result.push(EStrategyAction::NewOrder(action_new_order2));
        result
    }

    fn verify(&mut self, parse_action_results: Vec<ERunnerParseActionResult>) {
        for result in parse_action_results {
            info!("strategy verify:\t{:?}", result);
            match result {
                ERunnerParseActionResult::OrderPlaced(order) => {
                    match order.get_action() {
                        EOrderAction::Buy => {self.remove_list.push_back(order.get_id())}
                        EOrderAction::Sell => {}
                    }
                }
                ERunnerParseActionResult::OrderCanceled(_) => {}
            }
        }
    }
}