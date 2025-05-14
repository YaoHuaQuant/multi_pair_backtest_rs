//! 测试用价格模型-阶跃信号

use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use crate::strategy::model::TPriceModel;

/// 测试模型 使用阶跃信号
pub struct SPriceModelStep {
    /// 周期，单位：秒
    period: i64,
    /// 高点
    top: Decimal,
    /// 低点
    button: Decimal,
    /// 原点时间
    origin_date: DateTime<Local>,
}
impl SPriceModelStep {
    pub fn new(period: i64, top: Decimal, button: Decimal, origin_date: DateTime<Local>) -> Self {
        Self {
            period,
            top,
            button,
            origin_date,
        }
    }
}

impl TPriceModel for SPriceModelStep {
    /// todo 实际返回值是仓位 而不是价格
    fn get_price(&self, time: DateTime<Local>) -> Option<Decimal> {
        let duration = time.signed_duration_since(self.origin_date);
        let seconds = duration.num_seconds();

        let mo = ((seconds % self.period) + self.period) % self.period;
        if mo >= self.period / 2 {
            Some(self.top)
        } else {
            Some(self.button)
        }
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
    use crate::strategy::model::price_model_step_test::SPriceModelStep;
    use crate::strategy::model::TPriceModel;

    #[test]
    pub fn test1() {
        // 周期2小时
        let period = 60 * 60 * 2;
        // 高点
        let top = Decimal::from_f64(100.0).unwrap();
        // 低点
        let button = Decimal::from_f64(50.0).unwrap();
        // 原点
        let origin = Local.ymd(2025, 1, 1).and_hms(0, 0, 0);

        // 构建模型
        let model = SPriceModelStep::new(period, top, button, origin);

        // 返回类型测试
        let now = Local::now();
        let value = model.get_price(now);
        assert!(matches!(value, Some(_)));

        // 测试数据
        let time = origin + Duration::seconds((period as f64 * 0.0) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, button);
        
        let time = origin + Duration::seconds((period as f64 * 0.1) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, button);

        let time = origin + Duration::seconds((period as f64 * 0.5) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, top);

        let time = origin + Duration::seconds((period as f64 * 0.9) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, top);

        let time = origin + Duration::seconds((period as f64 * 1.0) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, button);

        let time = origin + Duration::seconds((period as f64 * 1.5) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, top);

        let time = origin + Duration::seconds((period as f64 * (-0.5)) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, top);

        let time = origin + Duration::seconds((period as f64 * (-0.9)) as i64);
        let value = model.get_price(time).unwrap();
        assert_eq!(value, button);
    }
}