use chrono::{DateTime, Duration, Local};

fn price(date: DateTime<Local>) -> f64 {
    // 示例函数：价格随时间的正弦变化
    let seconds = date.timestamp() as f64;
    (seconds / 10000.0).sin() * 10000.0 + 50000.0
}

fn price_d1(date: DateTime<Local>) -> f64 {
    let h = Duration::minutes(1); // 时间间隔：1秒
    let date_plus = date + h;
    let date_minus = date - h;
    (price(date_plus) - price(date_minus)) / (2.0 * h.num_seconds() as f64)
}

fn price_d2(date: DateTime<Local>) -> f64 {
    let h = Duration::minutes(1); // 时间间隔：1秒
    let date_plus = date + h;
    let date_minus = date - h;
    (price(date_plus) - 2.0 * price(date) + price(date_minus)) / (h.num_seconds().pow(2) as f64)
}

fn main() {
    let now = Local::now();
    println!("当前时间: {}", now);
    println!("价格: {:.2}", price(now));
    println!("一阶导数: {:.2}", price_d1(now));
    println!("二阶导数: {:.2}", price_d2(now));
}