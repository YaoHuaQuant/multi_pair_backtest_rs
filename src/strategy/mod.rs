pub mod strategy_mk_test;

use crate::protocol::{EStrategyAction, SRunnerParseResult};

pub trait TStrategy {
    fn run(runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction>;
}