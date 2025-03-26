use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

fn main() {
    println!("构造的本地时间是: {}", Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2021, 1, 1).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间"));
}
