//! Runner和User(主要是Strategy)的交互协议

use std::collections::HashMap;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    data_runtime::order::order::{SAddOrder, SOrder},
    data_source::{
        kline::SKlineUnitData,
        trading_pair::ETradingPairType
    }
};
use crate::data_runtime::order::EOrderAction;

/// Runner处理K线的结果-订单部分
#[derive(Debug)]
pub enum ERunnerParseOrderResult {
    /// 订单已完成
    OrderExecuted(SOrder),
}

/// Runner 处理K线的结果
#[derive(Debug)]
pub struct SRunnerParseKlineResult {
    pub tp_type:ETradingPairType,
    pub new_kline: SKlineUnitData,
    pub new_funding_rate: Decimal,
    pub order_result: Vec<ERunnerParseOrderResult>,
}

/// 添加策略订单
#[derive(Debug)]
pub struct SStrategyOrderAdd {
    /// 用于映射StrategyOrder的id
    pub id: Uuid,
    pub tp_type: ETradingPairType,
    pub action: EOrderAction,
    pub price: Decimal,
    pub quantity: Decimal,
}

/// 策略行为
#[derive(Debug)]
pub enum EStrategyAction {
    NewOrder(SStrategyOrderAdd),
    CancelOrder(Uuid),
}

/// Runner同步策略行为的结果
#[derive(Debug)]
pub enum ERunnerSyncActionResult {
    /// 已完成挂单
    OrderPlaced(SOrder),
    ///  已完成撤单
    OrderCanceled(SOrder),
}