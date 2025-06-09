use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_leveraged::SAssetLeveraged;
use crate::data_runtime::asset::asset_union::EAssetUnion;
use crate::data_runtime::asset::EAssetType;
use crate::data_runtime::order::EOrderAction;
use crate::data_source::trading_pair::ETradingPairType;

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

pub type ROrderV3Result<T> = Result<T, EOrderV3Error>;

pub type PriceDecimal = Decimal;
pub type QuantityDecimal = Decimal;
pub type RequiredQuantityDecimal = Decimal;
pub type ExpectedState = EOrderState;
pub type ActualState = EOrderState;

/// 订单管理器异常
#[derive(Debug)]
pub enum EOrderV3Error {
    /// 状态校验错误
    StateVerificationError(ExpectedState, ActualState),
    /// 提供的Asset的资产量小于所需的资产量 将资产返回
    AssetQuantityNotEnoughError(EAssetType, RequiredQuantityDecimal, QuantityDecimal, SAsset),
    /// 未锁定Asset
    LockedAssetNotExistError(SOrderV3),
    /// 成交了一个已存在fee asset的订单
    ExecuteOrderWithFeeAssetError(SOrderV3),
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
pub struct SOrderV3 {
    id: Uuid,
    /// 交易对类型
    tp_type: ETradingPairType,
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
    /// 当锁定资产量 < 所需量时 杠杆率>1
    /// 当锁定资产量 > 所需量时 杠杆率<1
    locked_asset: Option<EAssetUnion>,
    /// 已支付的fee资产对象（只有Executed状态的Order能够持有此对象）
    paid_fee_asset: Option<SAsset>,
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

impl SOrderV3 {
    pub fn new(tp_type: ETradingPairType, price: Decimal, quantity: Decimal, action: EOrderAction) -> Self {
        SOrderV3 {
            id: Uuid::new_v4(),
            tp_type,
            state: Default::default(),
            action,
            price,
            quantity,
            amount: price * quantity,
            locked_asset: None,
            paid_fee_asset: None,
        }
    }

    pub fn new_buy_order(tp_type: ETradingPairType, price: Decimal, quantity: Decimal) -> Self {
        Self::new(tp_type, price, quantity, EOrderAction::Buy)
    }

    pub fn new_sell_order(tp_type: ETradingPairType, price: Decimal, quantity: Decimal) -> Self {
        Self::new(tp_type, price, quantity, EOrderAction::Sell)
    }

    pub fn update(&mut self, update: EOrderUpdate) {
        match update {
            EOrderUpdate::Price(price) => {
                self.price = price;
            }
            EOrderUpdate::Quantity(quantity) => {
                self.quantity = quantity;
            }
            EOrderUpdate::PriceAndQuantity(price, quantity) => {
                self.price = price;
                self.quantity = quantity;
            }
        }
        self.amount = self.price * self.quantity
    }

    /// 状态校验
    fn state_check(&self, expected_state: ExpectedState) -> ROrderV3Result<()> {
        if self.state != expected_state {
            // 状态校验失败
            Err(EOrderV3Error::StateVerificationError(expected_state, self.state))
        } else {
            Ok(())
        }
    }

    /// 提交订单 绑定资产
    /// 卖单（平仓单）允许绑定高于订单所需的资产量
    /// 买单（开仓单）绑定资产量只需大于0即可 量的大小会影响杠杆
    /// asset: 保证金资产
    pub fn submit(&mut self, asset: SAsset) -> ROrderV3Result<()> {
        // 状态校验
        self.state_check(EOrderState::Pending)?;
        // 校验资产量
        let balance = asset.balance;
        
        if self.action == EOrderAction::Buy && balance <= Decimal::from(0) {
            // 资产不足
            Err(EOrderV3Error::AssetQuantityNotEnoughError(asset.as_type, Decimal::from(0), balance, asset))
        } else if self.action == EOrderAction::Sell && (balance <= Decimal::from(0) || balance > self.quantity) {
            Err(EOrderV3Error::AssetQuantityNotEnoughError(asset.as_type, self.quantity, balance, asset))
        } else {
            // 资产充足
            // 计算保证金
            self.locked_asset = match self.tp_type {
                ETradingPairType::BtcUsdt => {
                    // 现货保证金
                    match self.action {
                        EOrderAction::Buy => { Some(EAssetUnion::Usdt(asset)) }
                        EOrderAction::Sell => { Some(EAssetUnion::Btc(asset)) }
                    }
                }
                ETradingPairType::BtcUsdtFuture => {
                    // U本位合约 杠杆保证金
                    Some(EAssetUnion::BtcUsdtFuture(SAssetLeveraged::new(
                        self.tp_type,
                        self.quantity,
                        asset,
                        self.price,
                    ).unwrap()))
                }
                ETradingPairType::BtcUsdCmFuture => {
                    // 币本位合约 杠杆保证金
                    Some(EAssetUnion::BtcUsdCmFuture(SAssetLeveraged::new(
                        self.tp_type,
                        self.quantity,
                        asset,
                        self.price,
                    ).unwrap()))
                }
            };
            self.state = EOrderState::Unfulfilled;
            Ok(())
        }
    }

    /// 订单成交
    /// 绑定手续费资产
    /// 返回已锁定的资产
    pub fn execute(&mut self, paid_fee_asset: Option<SAsset>) -> ROrderV3Result<EAssetUnion> {
        // 状态校验
        self.state_check(EOrderState::Unfulfilled)?;
        match &self.paid_fee_asset {
            Some(_) => {
                Err(EOrderV3Error::ExecuteOrderWithFeeAssetError(self.clone()))
            }
            None => {
                match self.locked_asset.take() {
                    None => { Err(EOrderV3Error::LockedAssetNotExistError(self.clone())) }
                    Some(asset) => {
                        self.state = EOrderState::Executed;
                        self.paid_fee_asset = Some(match paid_fee_asset {
                            None => {
                                SAsset {
                                    as_type: asset.get_asset_type(),
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
    pub fn cancel(&mut self) -> Option<EAssetUnion> {
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

    pub fn get_locked_asset(&self) -> &Option<EAssetUnion> {
        &self.locked_asset
    }

    pub fn get_paid_fee_asset(&self) -> &Option<SAsset> {
        &self.paid_fee_asset
    }

    pub fn take_paid_fee_asset(&mut self) -> Option<SAsset> {
        self.paid_fee_asset.take()
    }
    // endregion ----- get函数 -----
}

#[cfg(test)]
mod tests {
    use rust_decimal::prelude::*;
    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::asset_union::EAssetUnion;
    use crate::data_runtime::asset::EAssetType;
    use crate::data_runtime::order::order_v3::{EOrderV3Error, EOrderState, EOrderUpdate, SOrderV3};
    use crate::data_source::trading_pair::ETradingPairType;

    fn get_pending_data() -> SOrderV3 {
        let tp_type = ETradingPairType::BtcUsdtFuture;
        let price = Decimal::from_str("3.1415926").unwrap();
        let quantity = Decimal::from_str("10.0").unwrap();
        SOrderV3::new_buy_order(tp_type, price, quantity)
    }

    /// 无杠杆测试数据
    fn get_unfulfilled_data() -> SOrderV3 {
        let mut order = get_pending_data();
        let asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("31.4159260").unwrap(),
        };
        let _ = order.submit(asset).unwrap();
        order
    }

    /// 高杠杆测试数据
    fn get_unfulfilled_high_leveraged_data() -> SOrderV3 {
        let mut order = get_pending_data();
        let asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("3.14159260").unwrap(),
        };
        let _ = order.submit(asset).unwrap();
        order
    }

    /// 低杠杆测试数据
    fn get_unfulfilled_low_leveraged_data() -> SOrderV3 {
        let mut order = get_pending_data();
        let asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("3.14159260").unwrap(),
        };
        let _ = order.submit(asset).unwrap();
        order
    }

    #[test]
    pub fn test_new() {
        let order = get_pending_data();

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
        let asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from_str("10").unwrap(),
        };
        let r = order.submit(asset);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderV3Error::StateVerificationError(_,_))));
        if let Err(EOrderV3Error::StateVerificationError(a, b)) = r {
            assert_eq!(a, EOrderState::Pending);
            assert_eq!(b, EOrderState::Executed);
        }
    }

    #[test]
    pub fn test_submit_success() {
        let mut order = get_pending_data();
        let asset = SAsset {
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
        assert!(matches!(r, Err(EOrderV3Error::StateVerificationError(_,_))));
        if let Err(EOrderV3Error::StateVerificationError(a, b)) = r {
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
        assert!(matches!(r, Err(EOrderV3Error::LockedAssetNotExistError(_))));
    }

    #[test]
    pub fn test_execute_fail_with_fee_error() {
        let mut order = get_unfulfilled_data();
        order.paid_fee_asset = Some(SAsset { as_type: EAssetType::Usdt, balance: Decimal::from(1) });
        let r = order.execute(None);
        assert!(r.is_err());
        assert!(matches!(r, Err(EOrderV3Error::ExecuteOrderWithFeeAssetError(_))));
    }

    #[test]
    pub fn test_execute_success() {
        let mut order = get_unfulfilled_data();
        let r = order.execute(None);
        assert!(r.is_ok());
        if let EAssetUnion::Usdt(asset) = r.unwrap() {
            let SAsset { as_type, balance } = asset;
            assert_eq!(as_type, EAssetType::Usdt);
            assert_eq!(balance, Decimal::from_str("31.4159260").unwrap());
        }
    }

    #[test]
    pub fn test_cancel_pending() {
        let mut order = get_unfulfilled_data();
        let r = order.cancel();
        assert!(r.is_some());
        if let EAssetUnion::Usdt(asset) = r.unwrap() {
            let SAsset { as_type, balance } = asset;
            assert_eq!(as_type, EAssetType::Usdt);
            assert_eq!(balance, Decimal::from_str("31.4159260").unwrap());
        }
    }

    #[test]
    pub fn test_cancel_unfulfilled() {
        let mut order = get_pending_data();
        let r = order.cancel();
        assert!(r.is_none());
    }
}