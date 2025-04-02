use rust_decimal::Decimal;
use uuid::Uuid;
use crate::asset::asset_v2::SAssetV2;
use crate::asset::EAssetType;
use crate::order::EOrderAction;

/// 订单状态
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum EOrderState {
    // 待提交（未绑定asset对象/未锁定资产）
    #[default]
    Pending,

    // 已提交 未成交
    Unfulfilled,

    // 已成交（并释放锁定资产 由交易对进行资产置换 ）
    Executed,

    // 已取消（释放锁定资产）
    Canceled,

    // 未知（todo 部分成交 由交易对进行部分资产置换）
    Unknown,
}

pub type ROrderResult<T> = Result<T, EOrderError>;

pub type PriceDecimal = Decimal;
pub type QuantityDecimal = Decimal;
pub type RequiredQuantityDecimal = Decimal;
pub type ExpectedState = EOrderState;
pub type ActualState = EOrderState;

/// 订单管理器异常
#[derive(Debug)]
pub enum EOrderError {
    /// 状态校验错误
    StateVerificationError(ExpectedState, ActualState),
    /// 提供的Asset的资产量小于所需的资产量 将资产返回
    AssetQuantityNotEnoughError(EAssetType, RequiredQuantityDecimal, QuantityDecimal, SAssetV2),
    /// 未锁定Asset
    LockedAssetNotExistError(SOrder),
    /// 成交了一个已存在fee asset的订单
    ExecuteOrderWithFeeAssetError(SOrder),
}


/// 订单更新
#[derive(Clone, Debug)]
pub enum EOrderUpdate {
    Price(Decimal),
    Quantity(Decimal),
    PriceAndQuantity(PriceDecimal, QuantityDecimal),
}

/// 策略执行器订单
#[derive(Debug, Clone)]
pub struct SOrder {
    id: Uuid,
    /// 订单状态
    state: EOrderState,
    /// 订单操作
    action: EOrderAction,
    /// 挂单价格（汇率=计价资产/基础资产）
    price: Decimal,
    /// 挂单量（基础资产）
    quantity: Decimal,
    /// 挂单金额（计价资产）
    amount: Decimal,
    /// 锁定的资产对象（买单锁定计价资产 卖单锁定基础资产）
    locked_asset: Option<SAssetV2>,
    /// 已支付的fee资产对象（只有Executed状态的Order能够持有此对象）
    paid_fee_asset: Option<SAssetV2>,
}

#[derive(Debug, Clone, Copy)]
pub struct SAddOrder {
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
            amount: price * quantity,
            locked_asset: None,
            paid_fee_asset: None,
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
                self.price = price;
                self.amount = self.price * self.quantity
            }
            EOrderUpdate::Quantity(quantity) => {
                self.quantity = quantity;
                self.amount = self.price * self.quantity
            }
            EOrderUpdate::PriceAndQuantity(price, quantity) => {
                self.price = price;
                self.quantity = quantity;
                self.amount = self.price * self.quantity
            }
        }
    }

    /// 状态校验
    fn state_check(&self, expected_state: ExpectedState) -> ROrderResult<()> {
        if self.state != expected_state {
            // 状态校验失败
            Err(EOrderError::StateVerificationError(expected_state, self.state))
        } else {
            Ok(())
        }
    }

    /// 提交订单 绑定资产
    /// 允许绑定高于订单所需的资产量
    pub fn submit(&mut self, asset: SAssetV2) -> ROrderResult<()> {
        // 状态校验
        self.state_check(EOrderState::Pending)?;
        // 校验资产量
        if match self.action {
            EOrderAction::Buy => { asset.balance < self.amount }
            EOrderAction::Sell => { asset.balance < self.quantity }
        } {
            // 资产不足
            Err(EOrderError::AssetQuantityNotEnoughError(asset.as_type, self.amount, asset.balance, asset))
        } else {
            // 资产充足
            self.locked_asset = Some(asset);
            self.state = EOrderState::Unfulfilled;
            Ok(())
        }
    }

    /// 订单成交
    /// 返回已锁定的资产
    /// 绑定手续费资产
    pub fn execute(&mut self, paid_fee_asset: Option<SAssetV2>) -> ROrderResult<SAssetV2> {
        // 状态校验
        self.state_check(EOrderState::Unfulfilled)?;
        match &self.paid_fee_asset {
            Some(_) => {
                Err(EOrderError::ExecuteOrderWithFeeAssetError(self.clone()))
            }
            None => {
                match self.locked_asset.take() {
                    None => { Err(EOrderError::LockedAssetNotExistError(self.clone())) }
                    Some(asset) => {
                        self.state = EOrderState::Executed;
                        self.paid_fee_asset = Some(match paid_fee_asset {
                            None => {
                                SAssetV2 {
                                    as_type: asset.as_type,
                                    balance: Decimal::from(0),
                                }
                            }
                            Some(asset) => { asset }
                        }.clone());
                        Ok(asset)
                    }
                }
            }
        }
    }

    /// 订单取消
    /// 释放已锁定的资产
    pub fn cancel(&mut self) -> Option<SAssetV2> {
        self.state = EOrderState::Canceled;
        self.locked_asset.take()
    }

    // region ----- get函数 -----
    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_state(&self) -> EOrderState {
        self.state
    }

    pub fn get_action(&self) -> EOrderAction {
        self.action
    }

    pub fn get_price(&self) -> Decimal {
        self.price
    }

    pub fn get_quantity(&self) -> Decimal {
        self.quantity
    }

    pub fn get_amount(&self) -> Decimal {
        self.amount
    }

    pub fn get_locked_asset(&self) -> &Option<SAssetV2> {
        &self.locked_asset
    }

    pub fn get_paid_fee_asset(&self) -> &Option<SAssetV2> {
        &self.paid_fee_asset
    }
    // endregion ----- get函数 -----
}

#[cfg(test)]
mod tests {
    use rust_decimal::prelude::*;
    use crate::asset::asset_v2::SAssetV2;
    use crate::asset::EAssetType;
    use crate::runner::strategy_runner::order::order::{EOrderError, EOrderState, EOrderUpdate, SOrder};

    fn get_pending_data() -> SOrder {
        let price = Decimal::from_str("3.1415926").unwrap();
        let quantity = Decimal::from_str("10.0").unwrap();
        SOrder::new_buy_order(price, quantity)
    }

    fn get_unfulfilled_data() -> SOrder {
        let mut order = get_pending_data();
        let asset = SAssetV2 {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("31.4159260").unwrap(),
        };
        let _ = order.submit(asset).unwrap();
        order
    }

    #[test]
    pub fn test_new() {
        let mut order = get_pending_data();

        dbg!(&order);
        assert_eq!(order.get_price(), Decimal::from_str("3.1415926").unwrap());
        assert_eq!(order.get_quantity().clone(), Decimal::from_str("10.0").unwrap());
        assert_eq!(order.get_state(), EOrderState::Pending);
    }

    #[test]
    pub fn test_update() {
        let mut order = get_pending_data();

        let new_price = Decimal::from_str("3.14").unwrap();
        order.update(EOrderUpdate::Price(new_price));
        assert_eq!(order.get_price(), Decimal::from_str("3.14").unwrap());
        assert_eq!(order.get_quantity().clone(), Decimal::from_str("10.0").unwrap());
        assert_eq!(order.get_amount(), Decimal::from_str("31.4").unwrap());
        assert_eq!(order.get_state(), EOrderState::Pending);


        let new_quantity = Decimal::from_str("0.002").unwrap();
        order.update(EOrderUpdate::Quantity(new_quantity));
        assert_eq!(order.get_price(), Decimal::from_str("3.14").unwrap());
        assert_eq!(order.get_quantity().clone(), Decimal::from_str("0.002").unwrap());
        assert_eq!(order.get_amount(), Decimal::from_str("0.00628").unwrap());
        assert_eq!(order.get_state(), EOrderState::Pending);
    }

    #[test]
    pub fn test_submit_fail_state_error() {
        let mut order = get_pending_data();
        order.state = EOrderState::Executed;
        let asset = SAssetV2 {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("10").unwrap(),
        };
        let r = order.submit(asset);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderError::StateVerificationError(_,_))));
        if let Err(EOrderError::StateVerificationError(a, b)) = r {
            assert_eq!(a, EOrderState::Pending);
            assert_eq!(b, EOrderState::Executed);
        }
    }

    #[test]
    pub fn test_submit_fail_quantity_not_enough() {
        let mut order = get_pending_data();
        let asset = SAssetV2 {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("10").unwrap(),
        };
        println!("before:{:?}", order);
        let r = order.submit(asset);
        println!("after:{:?}", order);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderError::AssetQuantityNotEnoughError(_,_, _, _))));
        if let Err(EOrderError::AssetQuantityNotEnoughError(o, a, b, c)) = r {
            assert_eq!(a, Decimal::from_str("31.415926").unwrap());
            assert_eq!(b, Decimal::from_str("10").unwrap());
            let SAssetV2 { as_type, balance } = c;
            assert_eq!(as_type, EAssetType::Usdt);
            assert_eq!(balance, Decimal::from_str("10").unwrap());
        }
    }

    #[test]
    pub fn test_submit_success() {
        let mut order = get_pending_data();
        let asset = SAssetV2 {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("50").unwrap(),
        };
        println!("before:{:?}", order);
        let r = order.submit(asset);
        println!("after:{:?}", order);
        assert!(r.is_ok());
    }

    #[test]
    pub fn test_execute_fail_state_error() {
        let mut order = get_pending_data();
        let r = order.execute(None);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderError::StateVerificationError(_,_))));
        if let Err(EOrderError::StateVerificationError(a, b)) = r {
            assert_eq!(a, EOrderState::Unfulfilled);
            assert_eq!(b, EOrderState::Pending);
        }
    }

    #[test]
    pub fn test_execute_fail_no_asset_error() {
        let mut order = get_unfulfilled_data();
        order.locked_asset = None;
        let r = order.execute(None);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderError::LockedAssetNotExistError(_))));
    }

    #[test]
    pub fn test_execute_fail_with_fee_error() {
        let mut order = get_unfulfilled_data();
        order.paid_fee_asset = Some(SAssetV2 { as_type: EAssetType::Usdt, balance: Decimal::from(1) });
        let r = order.execute(None);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderError::ExecuteOrderWithFeeAssetError(_))));
    }

    #[test]
    pub fn test_execute_success() {
        let mut order = get_unfulfilled_data();
        let r = order.execute(None);
        assert!(r.is_ok());
        let SAssetV2 { as_type, balance } = r.unwrap();
        assert_eq!(as_type, EAssetType::Usdt);
        assert_eq!(balance, Decimal::from_str("31.4159260").unwrap());
    }

    #[test]
    pub fn test_cancel_pending() {
        let mut order = get_unfulfilled_data();
        let r = order.cancel();
        assert!(r.is_some());
        let SAssetV2 { as_type, balance } = r.unwrap();
        assert_eq!(as_type, EAssetType::Usdt);
        assert_eq!(balance, Decimal::from_str("31.4159260").unwrap());
    }

    #[test]
    pub fn test_cancel_unfulfilled() {
        let mut order = get_pending_data();
        let r = order.cancel();
        assert!(r.is_none());
    }
}