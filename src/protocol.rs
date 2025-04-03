//! Runner和User(主要是Strategy)的交互协议
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_source::kline::SKlineUnitData;
use crate::data_runtime::order::order::{SAddOrder, SOrder};

/// Runner处理结果-订单部分
#[derive(Debug)]
pub enum ERunnerParseOrderResult {
    // 订单已完成
    OrderExecuted(SOrder),
}

/// Runner Parse结果
/// 用于反馈给strategy
#[derive(Debug)]
pub struct SRunnerParseResult {
    pub date_time: DateTime<Local>,
    pub new_kline: SKlineUnitData,
    pub new_funding_rate: Decimal,
    pub order_result: Vec<ERunnerParseOrderResult>,
}

/// 策略行为
#[derive(Debug)]
pub enum EStrategyAction {
    NewOrder(SAddOrder),
    CancelOrder(Uuid),
}

/// Runner处理结果-策略行为校验结果
#[derive(Debug)]
pub enum ERunnerParseActionResult {
    /// 已完成挂单
    OrderPlaced(SOrder),
    ///  已完成撤单
    OrderCanceled(SOrder),
}