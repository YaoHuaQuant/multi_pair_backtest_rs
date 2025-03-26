use rust_decimal::Decimal;
use uuid::Uuid;
use crate::order::EOrderAction;

/// 订单状态
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum EOrderState {
    // 未成交
    #[default]
    Unfulfilled,

    // 已成交
    Executed,

    // 已取消
    Canceled,

    // 未知（部分成交）
    Unknown,
}

/// 订单更新
#[derive(Clone, Copy)]
pub enum EOrderUpdate {
    Price(Decimal),
    Quantity(Decimal),
    State(EOrderState),
}

/// 策略执行器订单
#[derive(Debug, Clone, Copy)]
pub struct SOrder {
    pub id: Uuid,
    pub state: EOrderState,
    pub action: EOrderAction,
    pub price: Decimal,
    pub quantity: Decimal,
}

// impl Eq for SOrder {}
//
// impl PartialEq<Self> for SOrder {
//     fn eq(&self, other: &Self) -> bool {
//         self.id.eq(&other.id)
//     }
// }

impl SOrder {
    pub fn new(price: Decimal, quantity: Decimal, action: EOrderAction) -> Self {
        SOrder {
            id: Uuid::new_v4(),
            state: Default::default(),
            action,
            price,
            quantity,
        }
    }

    pub fn new_buy_order(price: Decimal, quantity: Decimal) -> Self {
        Self::new(price, quantity, EOrderAction::Buy)
    }

    pub fn new_sell_order(price: Decimal, quantity: Decimal) -> Self {
        Self::new(price, quantity, EOrderAction::Sell)
    }

    pub fn update(&mut self, update: EOrderUpdate) {
        match update {
            EOrderUpdate::Price(price) => {
                self.price = price
            }
            EOrderUpdate::Quantity(quantity) => {
                self.quantity = quantity
            }
            EOrderUpdate::State(state) => {
                self.state = state
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::prelude::*;
    use crate::runner::strategy_runner::order::runner_order::{EOrderState, EOrderUpdate, SOrder};

    #[test]
    pub fn test1() {
        let price = Decimal::from_str("3.1415926").unwrap();
        let quantity = Decimal::from_str("0.001").unwrap();
        let mut order = SOrder::new_buy_order(price, quantity);

        dbg!(&order);
        assert_eq!(order.price.clone(), Decimal::from_str("3.1415926").unwrap());
        assert_eq!(order.quantity.clone(), Decimal::from_str("0.001").unwrap());
        assert_eq!(order.state, EOrderState::Unfulfilled);

        let new_price = Decimal::from_str("3.14").unwrap();
        let new_quantity = Decimal::from_str("0.002").unwrap();
        order.update(EOrderUpdate::State(EOrderState::Canceled));
        order.update(EOrderUpdate::Price(new_price));
        order.update(EOrderUpdate::Quantity(new_quantity));

        dbg!(&order);
        assert_eq!(order.price.clone(), Decimal::from_str("3.14").unwrap());
        assert_eq!(order.quantity.clone(), Decimal::from_str("0.002").unwrap());
        assert_eq!(order.state, EOrderState::Canceled);
    }
}