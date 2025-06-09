/// 手续费配置
pub mod fee {
    /// 吃单手续费0.05%
    pub static TAKER_ORDER_FEE: f64 = 0.0005;

    /// 挂单手续费0.02%
    pub static MAKER_ORDER_FEE: f64 = 0.0002;
}

/// 交易对配置
pub mod trading_pair {
    pub mod btc_usdt {
        /// USDT现货最低交易量(以基础资产BTC为单位)
        pub static TRADDING_PAIR_BTC_USDT_MIN_QUANTITY: f64 = 0.00001;

        /// USDT现货最低交易量(以计价资产USDT为单位)
        pub static TRADDING_PAIR_USDT_MIN_QUANTITY: f64 = 10.0;

        // /// USDT现货最低收益率百分比(不考虑手续费)
        // pub static TRADDING_PAIR_USDT_MIN_PROFIT_PERCENTAGE:f64 = 0.0016;
        //
        // /// USDT现货最高收益率百分比(不考虑手续费)
        // pub static TRADDING_PAIR_USDT_MAX_PROFIT_PERCENTAGE:f64 = 0.05;
        //
        // /// USDT现货平仓价最小间距（相比于开仓价的百分比）
        // pub static TRADDING_PAIR_USDT_CLOSE_PRICE_STEP_PERCENTAGE:f64 = 0.001;
    }
    pub mod btc_usd_cm {
        /// BTC币本位合约最低交易量(以基础资产BTC币本位合约为单位)
        pub static TRADDING_PAIR_BTC_USD_CM_MIN_QUANTITY: f64 = 1.0;
    }
    pub mod btc_usdt_future {
        /// BTC U本位合约最低交易量(以基础资产U币本位合约为单位)
        pub static TRADDING_PAIR_BTC_USDT_FUTURE_FUTURE_MIN_QUANTITY: f64 = 1.0;
    }
}


/// 用户相关配置
pub mod user {
    /// 账户名称
    pub static USER_NAME: &str = "Satoshi Nakamoto";

    /// 初始资金量USDT
    pub static INIT_BALANCE_USDT: f64 = 100_000_.0;

    // /// 初始资金量BTC
    // pub static INIT_BALANCE_BTC: f64 = 0.0;
}

pub mod back_trade_period {
    use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, TimeZone};
    pub const SAMPLE_PERIOD: TimeDelta = Duration::minutes(1);
    // pub const SAMPLE_PERIOD: TimeDelta = Duration::hours(8);

    // 回测配置1（震荡走势 周期15小时）
    // 回测周期(东八区)'2025-01-27 13:40:00' and '2025-01-28 04:22:00'
    // 起始价格 100066.44 结束价格100009.02
    // 最高价格 102160.43 最低价格 97808.1
    
    /// 回测起始日期
    pub fn config_date_from() -> DateTime<Local> {
        Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 27).expect("无效的日期"), NaiveTime::from_hms_opt(13, 40, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    }
    
    /// 回测结束日期
    pub fn config_date_to() -> DateTime<Local> {
        Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 28).expect("无效的日期"), NaiveTime::from_hms_opt(4, 22, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    }

    // 回测配置2（震荡走势 周期2个月）
    // 回测周期(东八区)'2024-12-06 03:43:00' and '2025-02-04 13:01:00'
    // 起始价格 100034.98 结束价格 99985.83
    // 最高价格 109194.17(+9.19%) 最低价格 89417.88(-10.06%)

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 12, 6).expect("无效的日期"), NaiveTime::from_hms_opt(03, 43, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    // 
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 2, 4).expect("无效的日期"), NaiveTime::from_hms_opt(13, 1, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }

    // 回测配置3（Debug用）

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 1).expect("无效的日期"), NaiveTime::from_hms_opt(00, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    //
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 5).expect("无效的日期"), NaiveTime::from_hms_opt(00, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }

    // 回测配置4（单边上涨 周期6天）
    // 回测周期(东八区)'2025-01-13 14:30:00' and '2025-01-20 05:18:00'
    // 起始价格 90803.64 结束价格 102640
    // 最高价格 106357.57 最低价格 89417.88

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 13).expect("无效的日期"), NaiveTime::from_hms_opt(14, 30, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    // 
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 20).expect("无效的日期"), NaiveTime::from_hms_opt(5, 18, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }

    // 回测配置5（单边下跌 周期4天）
    // 回测周期(东八区)'2024-12-16 12:00:00' and '2024-12-20 12:00:00'
    // 起始价格 103710 结束价格 92731.97
    // 最高价格 108258.39 最低价格 92720.02

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 12, 16).expect("无效的日期"), NaiveTime::from_hms_opt(12, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    //
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 12, 20).expect("无效的日期"), NaiveTime::from_hms_opt(12, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }

    // 回测配置6（4年比特币周期）
    // 回测周期(东八区)'2018-08-28 00:00:00' and '2022-05-07 00:00:00'
    // 起始价格 6904 结束价格 36028.18
    // 最高价格 69000 最低价格 3157.67

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2018, 08, 28).expect("无效的日期"), NaiveTime::from_hms_opt(00, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    // 
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2022, 5, 7).expect("无效的日期"), NaiveTime::from_hms_opt(00, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }

    // 回测配置7（2019~2025）
    // 回测周期(东八区)'2019-01-01 00:00:00' and '2025-01-01 00:00:00'
    // 起始价格 xxx 结束价格 xxx
    // 最高价格 xxx 最低价格 xxx

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2019, 1, 1).expect("无效的日期"), NaiveTime::from_hms_opt(00, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    // 
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 1).expect("无效的日期"), NaiveTime::from_hms_opt(00, 00, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }

    // 回测配置8（目标仓位为负数的情况）
    // 回测周期(东八区)'2021-05-30 00:00:00' and '2021-07-21 00:00:00'
    // 起始价格 xxx 结束价格 xxx
    // 最高价格 xxx 最低价格 xxx

    // /// 回测起始日期
    // pub fn config_date_from() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2021, 7, 1).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
    // 
    // /// 回测结束日期
    // pub fn config_date_to() -> DateTime<Local> {
    //     Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2021, 7, 5).expect("无效的日期"), NaiveTime::from_hms_opt(8, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间")
    // }
}


pub struct SDebugConfig {
    /// 是否输出debug信息
    pub is_debug: bool,
    /// 是否输出info信息
    pub is_info: bool,
}

impl Default for SDebugConfig {
    fn default() -> Self {
        Self {
            is_debug: false,
            is_info: true,
        }
    }
}