//! Runner和User(主要是Strategy)的交互协议

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    data_source::{
        kline::SKlineUnitData,
        trading_pair::ETradingPairType,
    }
};
use crate::data_runtime::order::order_v3::SOrderV3;

/// Runner处理K线的结果-订单部分
#[derive(Debug)]
pub enum ERunnerParseOrderResult {
    /// 订单已完成
    OrderExecuted(SOrderV3),
}

/// Runner 处理K线的结果
#[derive(Debug)]
pub struct SRunnerParseKlineResult {
    pub tp_type: ETradingPairType,
    pub new_kline: SKlineUnitData,
    pub new_funding_rate: Decimal,
    pub order_result: Vec<ERunnerParseOrderResult>,
}

pub mod strategy_order {
    use rust_decimal::Decimal;
    use uuid::Uuid;
    use crate::data_runtime::order::EOrderAction;
    use crate::data_source::trading_pair::ETradingPairType;

    /// 添加策略订单
    /// 如何识别杠杆资产的多空：
    ///     1）tp_type必须是杠杆资产类型
    ///     2）action买入为开仓 卖出为平仓
    ///     3）买入时，base_quantity>0为做多，base_quantity<0时为做空。卖出时反之亦然。
    #[derive(Debug)]
    pub struct SStrategyOrderAdd {
        /// 用于映射StrategyOrder的id
        pub id: Option<Uuid>,
        pub tp_type: ETradingPairType,
        pub action: EOrderAction,
        pub price: Decimal,
        /// 基础货币量
        pub base_quantity: Decimal,
        
        /// 保证金量
        /// 购买现货时 保证金量=基础货币量*价格
        /// 购买杠杆资产时 只要保证 保证金量>0即可
        pub margin_quantity:Decimal,
    }

    impl SStrategyOrderAdd {
        pub fn new(
            id: Option<Uuid>,
            tp_type: ETradingPairType,
            action: EOrderAction,
            price: Decimal,
            base_quantity: Decimal,
            margin_quantity: Decimal,
        ) -> Self {
            Self {
                id,
                tp_type,
                action,
                price,
                base_quantity,
                margin_quantity,
            }
        }
    }
}


/// 策略行为
#[derive(Debug)]
pub enum EStrategyAction {
    NewOrder(strategy_order::SStrategyOrderAdd),
    CancelOrder(Uuid),
}

/// Runner同步策略行为的结果
#[derive(Debug)]
pub enum ERunnerSyncActionResult {
    /// 已完成挂单(关联StrategyOrder的id)
    OrderPlaced(SOrderV3, Option<Uuid>),
    ///  已完成撤单
    OrderCanceled(SOrderV3),
}