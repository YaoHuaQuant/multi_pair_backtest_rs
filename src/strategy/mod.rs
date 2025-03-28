pub mod strategy_mk_test;

use crate::protocol::{ERunnerParseActionResult, EStrategyAction, SRunnerParseResult};

pub trait TStrategy {
    fn run(&mut self, runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction>;
    fn verify(&mut self, parse_action_results: Vec<ERunnerParseActionResult>);
}