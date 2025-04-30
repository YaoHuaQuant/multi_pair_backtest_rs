//! 测试用价格模型-正弦波

use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use crate::strategy::model::TPriceModel;
use std::f64::consts::PI;
use rust_decimal::prelude::FromPrimitive;

/// 测试模型 使用正弦波
pub struct SPriceModelSin {
    /// 周期，单位：秒
    period: i64,
    /// 振幅
    amplitude: Decimal,
    /// 原点时间
    origin: DateTime<Local>,
    /// 曲线均值
    mean: Decimal,

    // /// 缓存时间
    // buffered_time: DateTime<Local>,
    // /// 缓存数据
    // buffered_value: Decimal,
}
impl SPriceModelSin {
    pub fn new(period: i64, amplitude: Decimal, origin: DateTime<Local>, mean: Decimal) -> Self {
        Self {
            period,
            amplitude,
            origin,
            mean,
            // buffered_time: origin,
            // buffered_value: Decimal::from(0),
        }
    }
}

impl TPriceModel for SPriceModelSin {
    fn get_price(&self, time: DateTime<Local>) -> Option<Decimal> {
        // if time == self.buffered_time {
        //     Some(self.buffered_value)
        // } else {
        let duration = time.signed_duration_since(self.origin);
        let seconds = duration.num_seconds() as f64;

        let angle = 2.0 * PI * seconds / self.period as f64;
        let sine_value = angle.sin();

        // 将 f64 转换为 Decimal
        let sine_decimal = Decimal::from_f64(sine_value).unwrap_or(Decimal::from(0));
        let new_value = self.amplitude * sine_decimal + self.mean;
        // self.buffered_time = time;
        // self.buffered_value = new_value;
        Some(new_value)
        // }
    }

    fn update_model(&mut self, _time: DateTime<Local>, _price: Decimal) {
        // 不需要update
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local, TimeZone};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use crate::strategy::model::model_sin_test::SPriceModelSin;
    use crate::strategy::model::TPriceModel;

    #[test]
    pub fn test1() {
        // 周期2小时
        let period = 60 * 60 * 2;
        // 振幅40%
        let amplitude = Decimal::from_f64(0.4).unwrap();
        // 原点
        let origin = Local.ymd(2025, 1, 1).and_hms(0, 0, 0);
        // 均值
        let mean = Decimal::from(1);

        // 构建模型
        let model = SPriceModelSin::new(period, amplitude, origin, mean);

        // 返回类型测试
        let now = Local::now();
        let value = model.get_price(now);
        assert!(matches!(value, Some(_)));

        // 测试数据
        // sin30’=0.5
        let time = origin + Duration::seconds((period as f64 * 30.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, amplitude * Decimal::from_f64(0.5).unwrap() + mean);
        // sin90’=1.0
        let time = origin + Duration::seconds((period as f64 * 90.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, amplitude * Decimal::from_f64(1.0).unwrap() + mean);
        // sin150’=0.5
        let time = origin + Duration::seconds((period as f64 * 150.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, amplitude * Decimal::from_f64(0.5).unwrap() + mean);
        // sin180’=0
        let time = origin + Duration::seconds((period as f64 * 180.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap().trunc_with_scale(4);
        assert_eq!(value, amplitude * Decimal::from_f64(0.0).unwrap() + mean);
        // sin210’=-0.5
        let time = origin + Duration::seconds((period as f64 * 210.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap().trunc_with_scale(4);
        assert_eq!(value, amplitude * Decimal::from_f64(-0.5).unwrap() + mean);
        // sin270’=-1.0
        let time = origin + Duration::seconds((period as f64 * 270.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap().trunc_with_scale(4);
        assert_eq!(value, amplitude * Decimal::from_f64(-1.0).unwrap() + mean);
        // sin330’=-0.5
        let time = origin + Duration::seconds((period as f64 * 330.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap().trunc_with_scale(4);
        assert_eq!(value, amplitude * Decimal::from_f64(-0.5).unwrap() + mean);
        // sin360’=0
        let time = origin + Duration::seconds((period as f64 * 360.0 / 360.0) as i64);
        let value = model.get_price(time).unwrap().trunc_with_scale(4);
        assert_eq!(value, amplitude * Decimal::from_f64(0.0).unwrap() + mean);
    }
}