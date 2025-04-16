use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

use crate::strategy::strategy_mk_test::SStrategyMkTest;

/// 吃单手续费
pub static TAKER_ORDER_FEE: f64 = 0.0005;

/// 挂单手续费
pub static MAKER_ORDER_FEE: f64 = 0.0002;

/// USDT现货最低交易量(以基础资产BTC为单位)
pub static TRADDING_PAIR_BTC_USDT_MIN_QUANTITY: f64 = 0.00001;

/// BTC币本位合约最低交易量(以基础资产BTC币本位合约为单位)
pub static TRADDING_PAIR_BTC_USD_CM_MIN_QUANTITY: f64 = 1.0;

/// BTC U本位合约最低交易量(以基础资产U币本位合约为单位)
pub static TRADDING_PAIR_BTC_USDT_FUTURE_FUTURE_MIN_QUANTITY: f64 = 1.0;

/// 账户名称
pub static USER_NAME: &str = "Satoshi Nakamoto";

/// 初始资金量USDT
pub static INIT_BALANCE_USDT: f64 = 100_000_.0;

/// 初始资金量BTC
pub static INIT_BALANCE_BTC: f64 = 1.0;

/// 回测起始日期
pub fn config_date_from() -> DateTime<Local> {
    Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 15).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
}

/// 回测结束日期
pub fn config_date_to() -> DateTime<Local> {
    Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 15).expect("无效的日期"), NaiveTime::from_hms_opt(0, 30, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
}

/// 默认策略
pub type DefaultStrategy = SStrategyMkTest;
