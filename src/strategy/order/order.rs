use std::cmp::PartialEq;

use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::order::{EOrderAction, EOrderDirection};
use crate::data_runtime::order::order::SOrder;

pub type RStrategyOrderResult<T> = Result<T, EStrategyOrderError>;

/// 策略订单管理器异常
#[derive(Debug)]
pub enum EStrategyOrderError {
    /// 状态校验错误
    StateVerificationError(String),
    /// set close_order_id时 该值不是None 入参（已有Id， 新Id）
    CanNotSetCloseOrderIdWhileTheIdIsNotNoneError(Uuid, Uuid),
    /// 状态转换错误 入参（旧状态, 期望旧状态 ,新状态）
    StateTransferError(EStrategyOrderState, EStrategyOrderState, EStrategyOrderState),
    /// 开单和平单的quantity不一致（开单quantity 平单quantity）
    InconsistentQuantityBetweenOrderPair(Decimal, Decimal),
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
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

#[derive(Debug, Clone)]
pub struct SStrategyOrder {
    id: Uuid,
    /// 开单id
    open_order_id: Uuid,
    /// 平单id
    close_order_id: Option<Uuid>,
    /// 订单状态
    state: EStrategyOrderState,
    /// 订单方向（多空）
    direction: EOrderDirection,
    /// 挂单量（基础资产）
    quantity: Decimal,
    /// 开仓价格
    open_price: Decimal,
    /// 期望平仓价格
    expected_close_price: Option<Decimal>,
    /// 平仓价格
    close_price: Option<Decimal>,
}

impl SStrategyOrder {
    /// 基于已提交的open订单进行构建
    pub fn new(opening_order: &SOrder) -> Self {
        let open_order_id = opening_order.get_id();
        let action = opening_order.get_action();
        let open_price = opening_order.get_price();
        let quantity = opening_order.get_quantity();

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
            expected_close_price: None,
            close_price: None,
        }
    }

    /// 取消开单 删除结构体自身（通过外部方式自动析构）
    pub fn cancel_open(&mut self) -> RStrategyOrderResult<()> {
        self.set_state(EStrategyOrderState::Opening, EStrategyOrderState::Canceled)
    }

    /// 开单结算
    pub fn opened(&mut self) -> RStrategyOrderResult<()> {
        self.set_state(EStrategyOrderState::Opening, EStrategyOrderState::Opened)
    }

    /// 绑定平单
    pub fn bind_close(&mut self, closing_order: &SOrder) -> RStrategyOrderResult<()> {
        self.set_state(EStrategyOrderState::Opened, EStrategyOrderState::Closing)?;
        if self.quantity != closing_order.get_quantity() {
            Err(EStrategyOrderError::InconsistentQuantityBetweenOrderPair(self.quantity, closing_order.get_quantity()))
        } else {
            self.close_order_id = Some(closing_order.get_id());
            self.close_price = Some(closing_order.get_price());
            Ok(())
        }
    }

    /// 平单结算
    pub fn closed(&mut self) -> RStrategyOrderResult<()> {
        self.set_state(EStrategyOrderState::Closing, EStrategyOrderState::Closed)
    }

    /// 取消平单 等待重新绑定平单
    pub fn cancel_close(&mut self) -> RStrategyOrderResult<()> {
        self.set_state(EStrategyOrderState::Closing, EStrategyOrderState::Opened)?;
        self.close_order_id = None;
        self.close_price = None;
        Ok(())
    }

    // region --- getter and setter
    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_open_order_id(&self) -> Uuid {
        self.open_order_id
    }

    pub fn get_close_order_id(&self) -> Option<Uuid> {
        self.close_order_id
    }

    /// 设置close order id 避免旧id被覆盖
    pub fn set_close_order_id(&mut self, close_order_id: Uuid) -> RStrategyOrderResult<()> {
        if self.state != EStrategyOrderState::Closing {
            Err(EStrategyOrderError::StateVerificationError(format!("Can not set close_order_id in state:{:?}", self.state)))
        } else {
            if let Some(old_id) = self.close_order_id {
                Err(EStrategyOrderError::CanNotSetCloseOrderIdWhileTheIdIsNotNoneError(old_id, close_order_id))
            } else {
                self.close_order_id = Some(close_order_id);
                Ok(())
            }
        }
    }

    pub fn get_state(&self) -> EStrategyOrderState {
        self.state
    }

    /// 设置state 对state状态机转化加以限制
    pub fn set_state(&mut self, expected_old_state: EStrategyOrderState, new_state: EStrategyOrderState) -> RStrategyOrderResult<()> {
        if self.state != expected_old_state {
            Err(EStrategyOrderError::StateTransferError(self.state, expected_old_state, new_state))
        } else {
            self.state = new_state;
            Ok(())
        }
    }

    pub fn get_direction(&self) -> EOrderDirection {
        self.direction
    }

    pub fn get_quantity(&self) -> Decimal {
        self.quantity
    }

    pub fn get_open_price(&self) -> Decimal {
        self.open_price
    }

    pub fn get_close_price(&self) -> Option<Decimal> {
        self.close_price
    }
    
    pub fn get_expected_close_price(&self) -> Option<Decimal> {
        self.expected_close_price
    }
    
    pub fn set_expected_close_price(&mut self, expected_close_price: Option<Decimal>) {
        self.expected_close_price = expected_close_price
    }
    // endregion --- getter and setter
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use crate::data_runtime::order::EOrderDirection;
    use crate::data_runtime::order::order::SOrder;
    use crate::strategy::order::order::{EStrategyOrderError, EStrategyOrderState, SStrategyOrder};

    pub fn get_test_data_open_order() -> SOrder {
        let price = Decimal::from(100_000);
        let quantity = Decimal::from_f64(0.5).unwrap();
        SOrder::new_buy_order(
            price,
            quantity,
        )
    }

    pub fn get_test_data_close_order1() -> SOrder {
        let price = Decimal::from(110_000);
        let quantity = Decimal::from_f64(0.5).unwrap();
        SOrder::new_sell_order(
            price,
            quantity,
        )
    }

    pub fn get_test_data_close_order2() -> SOrder {
        let price = Decimal::from(110_000);
        let quantity = Decimal::from_f64(0.51).unwrap();
        SOrder::new_sell_order(
            price,
            quantity,
        )
    }

    #[test]
    pub fn test_new() {
        let open_order = get_test_data_open_order();
        // let close_order = get_test_data_close_order();
        let strategy_order = SStrategyOrder::new(&open_order);
        assert_eq!(strategy_order.get_open_order_id(), open_order.get_id());
        assert_eq!(strategy_order.get_close_order_id(), None);
        assert_eq!(strategy_order.get_direction(), EOrderDirection::Long);
        assert_eq!(strategy_order.get_state(), EStrategyOrderState::Opening);
        assert_eq!(strategy_order.get_quantity(), Decimal::from_f64(0.5).unwrap());
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100_000));
        assert_eq!(strategy_order.get_close_price(), None);
    }

    #[test]
    pub fn test_new_cancel_open() {
        let open_order = get_test_data_open_order();
        // let close_order = get_test_data_close_order();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.cancel_open();
        assert!(matches!(tmp, Ok(())));
        assert_eq!(strategy_order.get_open_order_id(), open_order.get_id());
        assert_eq!(strategy_order.get_close_order_id(), None);
        assert_eq!(strategy_order.get_direction(), EOrderDirection::Long);
        assert_eq!(strategy_order.get_state(), EStrategyOrderState::Canceled);
        assert_eq!(strategy_order.get_quantity(), Decimal::from_f64(0.5).unwrap());
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100_000));
        assert_eq!(strategy_order.get_close_price(), None);
    }

    #[test]
    pub fn test_new_cancel_close_fail() {
        let open_order = get_test_data_open_order();
        // let close_order = get_test_data_close_order();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.cancel_close();
        assert!(matches!(tmp, Err(EStrategyOrderError::StateTransferError(_,_, _))));
        if let Err(EStrategyOrderError::StateTransferError(old_state, exp_state, new_state)) = tmp {
            assert_eq!(old_state, EStrategyOrderState::Opening);
            assert_eq!(exp_state, EStrategyOrderState::Closing);
            assert_eq!(new_state, EStrategyOrderState::Opened);
        }
    }

    #[test]
    pub fn test_opened() {
        let open_order = get_test_data_open_order();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        assert_eq!(strategy_order.get_open_order_id(), open_order.get_id());
        assert_eq!(strategy_order.get_close_order_id(), None);
        assert_eq!(strategy_order.get_direction(), EOrderDirection::Long);
        assert_eq!(strategy_order.get_state(), EStrategyOrderState::Opened);
        assert_eq!(strategy_order.get_quantity(), Decimal::from_f64(0.5).unwrap());
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100_000));
        assert_eq!(strategy_order.get_close_price(), None);
    }

    #[test]
    pub fn test_opened_cancel_open_fail() {
        let open_order = get_test_data_open_order();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.cancel_open();
        assert!(matches!(tmp, Err(EStrategyOrderError::StateTransferError(_,_, _))));
        if let Err(EStrategyOrderError::StateTransferError(old_state, exp_state, new_state)) = tmp {
            assert_eq!(old_state, EStrategyOrderState::Opened);
            assert_eq!(exp_state, EStrategyOrderState::Opening);
            assert_eq!(new_state, EStrategyOrderState::Canceled);
        }
    }

    #[test]
    pub fn test_opened_cancel_close_fail() {
        let open_order = get_test_data_open_order();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.cancel_close();
        assert!(matches!(tmp, Err(EStrategyOrderError::StateTransferError(_,_, _))));
        if let Err(EStrategyOrderError::StateTransferError(old_state, exp_state, new_state)) = tmp {
            assert_eq!(old_state, EStrategyOrderState::Opened);
            assert_eq!(exp_state, EStrategyOrderState::Closing);
            assert_eq!(new_state, EStrategyOrderState::Opened);
        }
    }

    #[test]
    pub fn test_opened_state_fail() {
        let open_order = get_test_data_open_order();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Err(EStrategyOrderError::StateTransferError(_,_, _))));
        if let Err(EStrategyOrderError::StateTransferError(old_state, exp_state, new_state)) = tmp {
            assert_eq!(old_state, EStrategyOrderState::Opened);
            assert_eq!(exp_state, EStrategyOrderState::Opening);
            assert_eq!(new_state, EStrategyOrderState::Opened);
        }
    }

    #[test]
    pub fn test_closing() {
        let open_order = get_test_data_open_order();
        let close_order = get_test_data_close_order1();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Ok(())));
        assert_eq!(strategy_order.get_open_order_id(), open_order.get_id());
        assert_eq!(strategy_order.get_close_order_id(), Some(close_order.get_id()));
        assert_eq!(strategy_order.get_direction(), EOrderDirection::Long);
        assert_eq!(strategy_order.get_state(), EStrategyOrderState::Closing);
        assert_eq!(strategy_order.get_quantity(), Decimal::from_f64(0.5).unwrap());
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100_000));
        assert_eq!(strategy_order.get_close_price(), Some(Decimal::from(110_000)));
    }

    #[test]
    pub fn test_closing_cancel_open_fail() {
        let open_order = get_test_data_open_order();
        let close_order = get_test_data_close_order1();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.cancel_open();
        assert!(matches!(tmp, Err(EStrategyOrderError::StateTransferError(_,_, _))));
        if let Err(EStrategyOrderError::StateTransferError(old_state, exp_state, new_state)) = tmp {
            assert_eq!(old_state, EStrategyOrderState::Closing);
            assert_eq!(exp_state, EStrategyOrderState::Opening);
            assert_eq!(new_state, EStrategyOrderState::Canceled);
        }
    }

    #[test]
    pub fn test_bind_close_inconsistent_fail() {
        let open_order = get_test_data_open_order();
        let close_order = get_test_data_close_order2();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Err(EStrategyOrderError::InconsistentQuantityBetweenOrderPair(_, _))));
        if let Err(EStrategyOrderError::InconsistentQuantityBetweenOrderPair(old, new)) = tmp {
            assert_eq!(old, Decimal::from_f64(0.5).unwrap());
            assert_eq!(new, Decimal::from_f64(0.51).unwrap());
        }
    }

    #[test]
    pub fn test_bind_close_state_fail() {
        let open_order = get_test_data_open_order();
        let close_order = get_test_data_close_order1();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Err(EStrategyOrderError::StateTransferError(_, _,_))));
        if let Err(EStrategyOrderError::StateTransferError(old_state, exp_state, new_state)) = tmp {
            assert_eq!(old_state, EStrategyOrderState::Closing);
            assert_eq!(exp_state, EStrategyOrderState::Opened);
            assert_eq!(new_state, EStrategyOrderState::Closing);
        }
    }

    #[test]
    pub fn test_closed() {
        let open_order = get_test_data_open_order();
        let close_order = get_test_data_close_order1();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.closed();
        assert!(matches!(tmp, Ok(())));
        assert_eq!(strategy_order.get_open_order_id(), open_order.get_id());
        assert_eq!(strategy_order.get_close_order_id(), Some(close_order.get_id()));
        assert_eq!(strategy_order.get_direction(), EOrderDirection::Long);
        assert_eq!(strategy_order.get_state(), EStrategyOrderState::Closed);
        assert_eq!(strategy_order.get_quantity(), Decimal::from_f64(0.5).unwrap());
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100_000));
        assert_eq!(strategy_order.get_close_price(), Some(Decimal::from(110_000)));
    }

    #[test]
    pub fn test_cancel_closing() {
        let open_order = get_test_data_open_order();
        let close_order = get_test_data_close_order1();
        let mut strategy_order = SStrategyOrder::new(&open_order);
        let tmp = strategy_order.opened();
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.bind_close(&close_order);
        assert!(matches!(tmp, Ok(())));
        let tmp = strategy_order.cancel_close();
        assert!(matches!(tmp, Ok(())));
        assert_eq!(strategy_order.get_open_order_id(), open_order.get_id());
        assert_eq!(strategy_order.get_close_order_id(), None);
        assert_eq!(strategy_order.get_direction(), EOrderDirection::Long);
        assert_eq!(strategy_order.get_state(), EStrategyOrderState::Opened);
        assert_eq!(strategy_order.get_quantity(), Decimal::from_f64(0.5).unwrap());
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100_000));
        assert_eq!(strategy_order.get_close_price(), None);
    }
}
