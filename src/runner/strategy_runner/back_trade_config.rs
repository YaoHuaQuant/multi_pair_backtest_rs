use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

/// 吃单手续费
pub static TAKER_ORDER_FEE: f64 = 0.0005;

/// 挂单手续费
pub static MAKER_ORDER_FEE: f64 = 0.0002;

/// 初始资金量USDT
pub static INIT_BALANCE_USDT: f64 = 1_000_000_.0;

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