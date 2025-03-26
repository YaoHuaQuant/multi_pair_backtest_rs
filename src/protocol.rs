use rust_decimal::Decimal;
use uuid::Uuid;
/** runner与strategy的交互逻辑*/
use crate::data::funding_rate::SFundingRateUnitData;
use crate::data::kline::SKlineUnitData;
use crate::runner::strategy_runner::order::runner_order::SOrder;

/// Runner处理结果-订单部分
pub enum ERunnerParseOrderResult {
    OrderExecuted(SOrder) // 订单已完成
}

/// Runner Parse结果
/// 用于反馈给strategy
pub struct SRunnerParseResult {
    pub new_kline: SKlineUnitData,
    pub new_funding_rate: Option<SFundingRateUnitData>,
    pub order_result: Vec<ERunnerParseOrderResult>,
}

pub type PriceChange = Option<Decimal>;
pub type QuantityChange = Option<Decimal>;

/// 策略行为
pub enum EStrategyAction {
    NewOrder(SOrder),
    CancelOrder(Uuid),
    ModifyOrder(Uuid, PriceChange, QuantityChange),
}