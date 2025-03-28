pub mod date_time {
    use chrono::{DateTime, Local, Timelike};

    /// 归一化时间戳到分钟（秒归零）
    pub fn normalize_to_minute(dt: &DateTime<Local>) -> DateTime<Local> {
        dt.with_second(0).unwrap().with_nanosecond(0).unwrap()
    }
}
