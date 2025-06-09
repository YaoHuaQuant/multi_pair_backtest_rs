use rust_decimal::Decimal;
use crate::data_runtime::asset::asset::{EAssetError, SAsset};
use crate::data_runtime::asset::EAssetType;
use crate::data_runtime::order::EOrderDirection;
use crate::data_source::trading_pair::ETradingPairType;

/// 资产杠杆（仓位）对象
///
/// 保证金是用户需要实际支付的资产
///
/// 基础资产的量会影响手续费和资金费
///     手续费在开仓和平仓时，由用户支付。资金费从保证金中扣除或者加入保证金
///
/// 开仓时，计价资产价值 = -基础资产量*价格
///
/// 杠杆率=-计价资产/保证金
///
/// 保证金+计价资产=恒定值（除非添加或扣除资金费）
#[derive(Debug, Clone)]
pub struct SAssetLeveraged {
    /// 资产类型(交易对类型)
    tp_type: ETradingPairType,
    /// 基础资产
    base_asset: SAsset,
    /// 计价资产
    quote_asset: SAsset,
    /// 保证金(头寸，与计价资产类型相同)
    margin_asset: SAsset,
}

pub type RAssetLeveragedResult<T> = Result<T, EAssetLeveragedError>;

pub type RemainBalance = Decimal;
pub type RequireBalance = Decimal;
pub type RequireAssetType = EAssetType;
pub type ActualAssetType = EAssetType;
pub type RequireTradingPairType = ETradingPairType;
pub type ActualTradingPairType = ETradingPairType;


/// Asset异常
#[derive(Debug)]
pub enum EAssetLeveragedError {
    UnknownError,
    /// 交易对类型不匹配
    AssetTradingPairTypeInconsistentError(ActualTradingPairType, RequireTradingPairType, SAssetLeveraged),
    /// 基础资产类型不匹配
    AssetBaseTypeInconsistentError(ActualAssetType, RequireAssetType, SAssetLeveraged),
    /// 计价资产类型不匹配
    AssetQuoteTypeInconsistentError(ActualAssetType, RequireAssetType, SAssetLeveraged),
    /// 保证金类型不匹配
    AssetMarginTypeInconsistentError(ActualAssetType, RequireAssetType, SAsset),
    /// 保证金不足
    AssetMarginNotEnoughError(RemainBalance, RequireBalance),
}

impl SAssetLeveraged {
    pub fn new(
        tp_type: ETradingPairType,
        base_balance: Decimal,
        margin_asset: SAsset,
        price: Decimal,
    ) -> RAssetLeveragedResult<Self>
    {
        let base_type = tp_type.get_base_currency_type();
        let quote_type = tp_type.get_quote_currency_type();
        if margin_asset.as_type != quote_type {
            return Err(EAssetLeveragedError::AssetMarginTypeInconsistentError(
                margin_asset.as_type,
                quote_type,
                margin_asset,
            ));
        }
        let base_asset = SAsset {
            as_type: base_type,
            balance: base_balance,
        };
        let quote_asset = SAsset {
            as_type: quote_type,
            balance: -base_balance * price,
        };
        Ok(Self {
            tp_type,
            base_asset,
            quote_asset,
            margin_asset,
        })
    }

    pub fn update(&mut self, price: Decimal) {
        let diff_quote = self.base_asset.balance * price + self.quote_asset.balance;
        let diff_asset = self.quote_asset.split_allow_negative(diff_quote);
        self.margin_asset.merge(diff_asset).unwrap();
    }

    pub fn get_ty_type(&self) -> ETradingPairType {
        self.tp_type
    }

    pub fn get_base(&self) -> &SAsset {
        &self.base_asset
    }

    pub fn get_quote(&self) -> &SAsset {
        &self.quote_asset
    }

    pub fn get_margin(&self) -> &SAsset {
        &self.margin_asset
    }

    /// 获取仓位方向
    pub fn get_direction(&self) -> EOrderDirection {
        if self.quote_asset.balance < Decimal::from(0) {
            EOrderDirection::Long
        } else {
            EOrderDirection::Short
        }
    }

    /// 获取杠杆率(恒定为正)
    pub fn get_leverage(&self) -> Decimal {
        (self.quote_asset.balance / self.margin_asset.balance).abs()
    }

    /// 获取强平（清算）价格
    pub fn get_liquidation_price(&self) -> Decimal {
        -(self.quote_asset.balance + self.margin_asset.balance) / self.base_asset.balance
    }

    /// 补充保证金
    pub fn margin_top_up(&mut self, margin: SAsset) -> RAssetLeveragedResult<()> {
        if self.margin_asset.as_type != margin.as_type {
            Err(EAssetLeveragedError::AssetMarginTypeInconsistentError(margin.as_type, self.margin_asset.as_type, margin))
        } else {
            self.margin_asset.merge(margin).unwrap();
            Ok(())
        }
    }

    /// 提取保证金
    pub fn margin_withdraw(&mut self, amount: Decimal) -> RAssetLeveragedResult<SAsset> {
        match self.margin_asset.split(amount) {
            Ok(withdraw_asset) => {
                Ok(withdraw_asset)
            }
            Err(e) => {
                match e {
                    EAssetError::BalanceNotEnough(remain, expected) => {
                        Err(EAssetLeveragedError::AssetMarginNotEnoughError(remain, expected))
                    }
                    _ => { Err(EAssetLeveragedError::UnknownError) }
                }
            }
        }
    }


    /// 合并另一个相同类型的资产到当前资产
    /// 如果执行出错，则可以在Err中获取到输入参数，避免asset资产被消耗
    pub fn merge(&mut self, other: SAssetLeveraged) -> RAssetLeveragedResult<()> {
        if self.tp_type != other.tp_type {
            Err(EAssetLeveragedError::AssetTradingPairTypeInconsistentError(other.tp_type, self.tp_type, other))
        } else if self.base_asset.as_type != other.base_asset.as_type {
            Err(EAssetLeveragedError::AssetBaseTypeInconsistentError(other.base_asset.as_type, self.base_asset.as_type, other))
        } else if self.quote_asset.as_type != other.quote_asset.as_type {
            Err(EAssetLeveragedError::AssetQuoteTypeInconsistentError(other.quote_asset.as_type, self.quote_asset.as_type, other))
        } else if self.margin_asset.as_type != other.margin_asset.as_type {
            Err(EAssetLeveragedError::AssetMarginTypeInconsistentError(other.margin_asset.as_type, self.margin_asset.as_type, other.margin_asset.clone()))
        } else {
            let SAssetLeveraged {
                tp_type: _,
                base_asset,
                quote_asset,
                margin_asset
            } = other;
            self.base_asset.merge(base_asset).unwrap();
            self.quote_asset.merge(quote_asset).unwrap();
            self.margin_asset.merge(margin_asset).unwrap();
            Ok(())
        }
    }

    /// 拆分出一部分新资产
    pub fn split(&mut self, base_balance: Decimal) -> RAssetLeveragedResult<Self> {
        Ok(self.split_allow_negative(base_balance))
    }

    /// 拆分出一部分新资产
    /// 允许被拆分之后的资产为负值
    pub fn split_allow_negative(&mut self, base_balance: Decimal) -> Self {
        let remaining_base_balance = self.base_asset.balance;
        let remaining_quote_balance = self.quote_asset.balance;
        let remaining_margin_balance = self.margin_asset.balance;
        let quote_balance = remaining_quote_balance * base_balance / remaining_base_balance;
        let margin_balance = remaining_margin_balance * base_balance / remaining_base_balance;

        let new_base_asset = self.base_asset.split_allow_negative(base_balance);
        let new_quote_asset = self.quote_asset.split_allow_negative(quote_balance);
        let new_margin_asset = self.margin_asset.split_allow_negative(margin_balance);
        Self {
            tp_type: self.tp_type,
            base_asset: new_base_asset,
            quote_asset: new_quote_asset,
            margin_asset: new_margin_asset,
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::asset_leveraged::{EAssetLeveragedError, SAssetLeveraged};
    use crate::data_runtime::asset::EAssetType;
    use crate::data_runtime::order::EOrderDirection;
    use crate::data_source::trading_pair::ETradingPairType;

    pub fn get_test_data1() -> SAssetLeveraged {
        // test new
        let tp_type = ETradingPairType::BtcUsdtFuture;
        let base_balance = Decimal::from(1);
        let margin_asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(10_000),
        };
        let price = Decimal::from(100_000);
        SAssetLeveraged::new(
            tp_type,
            base_balance,
            margin_asset,
            price,
        ).unwrap()
    }

    pub fn get_test_data2() -> SAssetLeveraged {
        // test new
        let tp_type = ETradingPairType::BtcUsdtFuture;
        let base_balance = Decimal::from(2);
        let margin_asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(40_000),
        };
        let price = Decimal::from(100_000);
        SAssetLeveraged::new(
            tp_type,
            base_balance,
            margin_asset,
            price,
        ).unwrap()
    }

    pub fn get_test_data3() -> SAssetLeveraged {
        // test new
        let tp_type = ETradingPairType::BtcUsdCmFuture;
        let base_balance = Decimal::from(10);
        let margin_asset = SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(1),
        };
        let price = Decimal::from(10);
        SAssetLeveraged::new(
            tp_type,
            base_balance,
            margin_asset,
            price,
        ).unwrap()
    }
    #[test]
    pub fn test() {
        let asset1 = get_test_data1();

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(1));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-100_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(10_000));

        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(10));
    }

    #[test]
    pub fn test_update() {
        let mut asset1 = get_test_data1();
        let new_price = Decimal::from(180_000);
        asset1.update(new_price);

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(1));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-180_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(90_000));

        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(2));
    }

    #[test]
    pub fn test_get_liquidation_price() {
        let mut asset1 = get_test_data1();
        let liquidation_price = asset1.get_liquidation_price();
        assert_eq!(liquidation_price, Decimal::from(90_000));

        let new_price = Decimal::from(180_000);
        asset1.update(new_price);
        assert_eq!(liquidation_price, Decimal::from(90_000));

        let new_price = Decimal::from(10_000);
        asset1.update(new_price);
        assert_eq!(liquidation_price, Decimal::from(90_000));
    }

    #[test]
    pub fn test_get_liquidation_price2() {
        let tp_type = ETradingPairType::BtcUsdtFuture;
        let base_balance = Decimal::from(-1);
        let margin_asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(10_000),
        };
        let price = Decimal::from(100_000);
        let mut asset1 = SAssetLeveraged::new(
            tp_type,
            base_balance,
            margin_asset,
            price,
        ).unwrap();
        let liquidation_price = asset1.get_liquidation_price();
        assert_eq!(liquidation_price, Decimal::from(110_000));

        let new_price = Decimal::from(180_000);
        asset1.update(new_price);
        assert_eq!(liquidation_price, Decimal::from(110_000));

        let new_price = Decimal::from(10_000);
        asset1.update(new_price);
        assert_eq!(liquidation_price, Decimal::from(110_000));
    }

    #[test]
    pub fn test_get_liquidation_price3() {
        let tp_type = ETradingPairType::BtcUsdtFuture;
        let base_balance = Decimal::from(1);
        let margin_asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(200_000),
        };
        let price = Decimal::from(100_000);
        let mut asset1 = SAssetLeveraged::new(
            tp_type,
            base_balance,
            margin_asset,
            price,
        ).unwrap();
        let liquidation_price = asset1.get_liquidation_price();
        assert_eq!(liquidation_price, Decimal::from(-100_000));

        let new_price = Decimal::from(200_000);
        asset1.update(new_price);
        assert_eq!(liquidation_price, Decimal::from(-100_000));

        let new_price = Decimal::from(-200_000);
        asset1.update(new_price);
        assert_eq!(liquidation_price, Decimal::from(-100_000));
    }

    #[test]
    pub fn test_margin_top_up() {
        let mut asset1 = get_test_data1();
        let new_margin_asset = SAsset {
            as_type: EAssetType::Usdt,
            balance: Decimal::from(10_000),
        };
        let r = asset1.margin_top_up(new_margin_asset);
        assert!(r.is_ok());

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(1));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-100_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(20_000));

        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(5));
    }

    #[test]
    pub fn test_margin_top_up_fail() {
        let mut asset1 = get_test_data1();
        let new_margin_asset = SAsset {
            as_type: EAssetType::Btc,
            balance: Decimal::from(10_000),
        };
        let r = asset1.margin_top_up(new_margin_asset);
        assert!(r.is_err());

        assert!(matches!(r, Err(EAssetLeveragedError::AssetMarginTypeInconsistentError(_, _, _))));
        if let Err(EAssetLeveragedError::AssetMarginTypeInconsistentError(actual, expected, margin)) = r {
            assert_eq!(actual, EAssetType::Btc);
            assert_eq!(expected, EAssetType::Usdt);
            assert_eq!(margin.as_type, EAssetType::Btc);
            assert_eq!(margin.balance, Decimal::from(10_000));
        }

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(1));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-100_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(10_000));

        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(10));
    }
    
    #[test]
    pub fn test_margin_withdraw() {
        let mut asset1 = get_test_data1();
        let withdraw_amount  = Decimal::from(5_000);
        let r = asset1.margin_withdraw(withdraw_amount);
        assert!(r.is_ok());

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(1));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-100_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(5_000));

        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(20));
    }

    #[test]
    pub fn test_margin_withdraw_fail() {
        let mut asset1 = get_test_data1();
        let withdraw_amount  = Decimal::from(20_000);
        let r = asset1.margin_withdraw(withdraw_amount);
        assert!(r.is_err());

        assert!(matches!(r, Err(EAssetLeveragedError::AssetMarginNotEnoughError(_, _))));
        if let Err(EAssetLeveragedError::AssetMarginNotEnoughError(remain, required)) = r {
            assert_eq!(remain, Decimal::from(10_000));
            assert_eq!(required, Decimal::from(20_000));
        }

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(1));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-100_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(10_000));

        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(10));
        
    }
    
    #[test]
    pub fn test_split() {
        let mut asset1 = get_test_data1();

        // test split success
        let asset2 = asset1.split(Decimal::from_f64(0.1).unwrap());
        assert!(asset2.is_ok());
        let asset2 = asset2.unwrap();

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from_f64(0.9).unwrap());
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-90_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(9_000));
        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(10));

        assert_eq!(asset2.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset2.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset2.base_asset.balance, Decimal::from_f64(0.1).unwrap());
        assert_eq!(asset2.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset2.quote_asset.balance, Decimal::from(-10_000));
        assert_eq!(asset2.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset2.margin_asset.balance, Decimal::from(1_000));
        assert_eq!(asset2.get_direction(), EOrderDirection::Long);
        assert_eq!(asset2.get_leverage(), Decimal::from(10));
    }

    #[test]
    pub fn test_merge() {
        let mut asset1 = get_test_data1();
        let asset2 = get_test_data2();

        let r = asset1.merge(asset2);
        assert!(r.is_ok());

        assert_eq!(asset1.tp_type, ETradingPairType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.as_type, EAssetType::BtcUsdtFuture);
        assert_eq!(asset1.base_asset.balance, Decimal::from(3));
        assert_eq!(asset1.quote_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.quote_asset.balance, Decimal::from(-300_000));
        assert_eq!(asset1.margin_asset.as_type, EAssetType::Usdt);
        assert_eq!(asset1.margin_asset.balance, Decimal::from(50_000));
        assert_eq!(asset1.get_direction(), EOrderDirection::Long);
        assert_eq!(asset1.get_leverage(), Decimal::from(6));
    }

    #[test]
    pub fn test_merge_fail() {
        let mut asset1 = get_test_data1();
        let asset3 = get_test_data3();

        let r = asset1.merge(asset3);
        assert!(r.is_err());
        assert!(matches!(r, Err(EAssetLeveragedError::AssetTradingPairTypeInconsistentError(_, _, _))));
        if let Err(EAssetLeveragedError::AssetTradingPairTypeInconsistentError(actual, expected, _)) = r {
            assert_eq!(actual, ETradingPairType::BtcUsdCmFuture);
            assert_eq!(expected, ETradingPairType::BtcUsdtFuture);
        }
    }
}