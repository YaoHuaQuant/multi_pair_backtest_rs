use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use crate::config::{fee::{MAKER_ORDER_FEE, TAKER_ORDER_FEE}};
use crate::config::back_trade_period::{config_date_from, config_date_to};

#[derive(Debug, Clone)]
pub struct SBackTradeRunnerConfig {
    ///  吃单手续费
    pub taker_order_fee: Decimal,
    ///  挂单手续费
    pub maker_order_fee: Decimal,
    ///  回测起始日期
    pub date_from: DateTime<Local>,
    ///  回测结束日期
    pub date_to: DateTime<Local>,
}

impl Default for SBackTradeRunnerConfig {
    fn default() -> Self {
        Self {
            taker_order_fee: Decimal::from_f64(TAKER_ORDER_FEE).unwrap(),
            maker_order_fee: Decimal::from_f64(MAKER_ORDER_FEE).unwrap(),
            date_from: config_date_from(),
            date_to: config_date_to(),
        }
    }
}