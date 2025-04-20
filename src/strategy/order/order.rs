use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::order::{EOrderAction, EOrderDirection};
use crate::data_runtime::order::order::SOrder;

#[derive(Debug)]
pub enum EStrategyOrderState {
    // /// 待提交
    // Pending,
    /// 已提交 待开仓
    Opening,
    /// 已开仓 待提交平单
    Opened,
    /// 已提交平单
    Closing,
    /// 已平仓
    Closed,
    /// 已取消
    Canceled,
}

#[derive(Debug)]
pub struct SStrategyOrder {
    pub id: Uuid,
    /// 开单id
    pub open_order_id: Uuid,
    /// 平单id
    pub close_order_id: Option<Uuid>,
    /// 订单状态
    pub state: EStrategyOrderState,
    /// 订单方向（多空）
    pub direction: EOrderDirection,
    /// 挂单量（基础资产）
    pub quantity: Decimal,
    /// 开仓价格
    pub open_price: Decimal,
    /// 平仓价格
    pub close_price: Option<Decimal>,
}

impl SStrategyOrder {
    /// 基于已提交的open订单进行构建
    pub fn new(opening_order: &SOrder) -> Self {
        let SOrder {
            id: open_order_id,
            action,
            price: open_price,
            quantity,
            ..
        } = opening_order.clone();

        let direction = match action {
            EOrderAction::Buy => { EOrderDirection::Long }
            EOrderAction::Sell => { EOrderDirection::Short }
        };
        Self {
            id: Uuid::new_v4(),
            open_order_id,
            close_order_id: None,
            state: EStrategyOrderState::Opening,
            direction,
            quantity,
            open_price,
            close_price: None,
        }
    }

    /// 取消开单 删除结构体自身
    pub fn cancel_open(&mut self) {
        todo!()
    }

    /// 开单结算
    pub fn  opened(&mut self) {
        todo!()
    }

    /// 绑定平单
    pub fn bind_close(&mut self, closing_order:&SOrder) {
        todo!()
    }

    /// 平单结算
    pub fn closed(&mut self) {
        todo!()
    }

    /// 取消平单 等待重新绑定平单
    pub fn cancel_close(&mut self) {
        todo!()
    }
}


