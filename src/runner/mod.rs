//! 执行器
//! 用于执行strategy中的量化算法，并且应用在回测平台或者交易所。

use chrono::{DateTime, Local};
use crate::config::SDebugConfig;
use crate::strategy::TStrategy;
use crate::data_runtime::user::SUser;
use crate::data_source::kline::SKlineUnitData;
use crate::data_source::trading_pair::ETradingPairType;
use crate::runner::logger::data_logger::SDataLogger;

pub mod back_trade;
pub mod logger;

pub trait TRunner<S: TStrategy> {
    fn run(&mut self, users: &mut Vec<SUser<S>>, debug_config: SDebugConfig) -> SRunnerResult;
}

pub trait TRunnerGetPrice {
    /// 获取特定时刻的交易对价格
    fn get_price(&self, date:DateTime<Local>, tp_type:ETradingPairType) -> Option<&SKlineUnitData>;
}
#[derive(Debug)]
pub struct SRunnerResult {
    pub date_from:DateTime<Local>,
    pub date_to:DateTime<Local>,
    pub data_logger:SDataLogger,
}