use chrono::{DateTime, Local, TimeZone};
use rust_decimal::Decimal;
use std::f64::consts::PI;
use rust_decimal::prelude::FromPrimitive;

struct SineModel {
    period: f64,              // 周期，单位：秒
    amplitude: Decimal,       // 振幅
    origin: DateTime<Local>,  // 原点时间
}

impl SineModel {
    fn new(period: f64, amplitude: Decimal, origin: DateTime<Local>) -> Self {
        Self {
            period,
            amplitude,
            origin,
        }
    }

    fn evaluate(&self, t: DateTime<Local>) -> Decimal {
        let duration = t.signed_duration_since(self.origin);
        let seconds = duration.num_seconds() as f64;

        let angle = 2.0 * PI * seconds / self.period;
        let sine_value = angle.sin();

        // 将 f64 转换为 Decimal
        let sine_decimal = Decimal::from_f64(sine_value).unwrap_or(Decimal::from(0));

        self.amplitude * sine_decimal
    }
}

fn main() {
    let origin = Local.ymd(2025, 4, 29).and_hms(0, 0, 0);
    let model = SineModel::new(86400.0, Decimal::from(1), origin); // 周期为一天，振幅为1

    let now = Local::now();
    let value = model.evaluate(now);

    println!("Sine value at {} is {}", now, value);
}

