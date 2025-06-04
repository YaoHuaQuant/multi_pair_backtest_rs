use chrono::{DateTime, Duration, Local, TimeDelta};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use crate::strategy::model::TPriceModel;

/// 通用仓位模型
pub struct SPositionModel<P: TPriceModel> {
    price_model: P,
    /// 差分求导时的自变量delta
    delta_time: TimeDelta,
    position_max: Decimal,
    position_min: Decimal,
    first_derivative_mean: Decimal,
    first_derivative_std: Decimal,
    second_derivative_mean: Decimal,
    second_derivative_std: Decimal,
}

impl<P: TPriceModel> SPositionModel<P> {
    pub fn from(price_model: P, position_max: f64, position_min: f64) -> Self {
        Self {
            price_model,
            delta_time: Duration::hours(12),
            position_max: Decimal::from_f64(position_max).unwrap(),
            position_min: Decimal::from_f64(position_min).unwrap(),
            first_derivative_mean: Decimal::from_f64(30.108271100239573).unwrap(),
            first_derivative_std: Decimal::from_f64(31.619871679773368).unwrap(),
            second_derivative_mean: Decimal::from_f64(-0.03086514940302392).unwrap(),
            second_derivative_std: Decimal::from_f64(0.17549696907579349).unwrap(),
        }
    }

    /// 获取一阶导数
    pub fn get_first_derivative(&self, time: DateTime<Local>) -> Option<Decimal> {
        let delta = self.delta_time;
        let date_plus = time + delta;
        let date_minus = time - delta;
        match self.price_model.get_price(date_plus) {
            None => { None }
            Some(price_plus) => {
                match self.price_model.get_price(date_minus) {
                    None => { None }
                    // Some(price_minus) => { Some(price_plus - price_minus) }
                    Some(price_minus) => { Some((price_plus - price_minus) / Decimal::from_f64(2.0 * delta.num_minutes().to_f64().unwrap()).unwrap() * Decimal::from(24 * 60)) }
                }
            }
        }
    }

    /// 获取一阶导数标准信号
    pub fn get_first_derivative_standard(&self, time: DateTime<Local>) -> Option<Decimal> {
        match self.get_first_derivative(time) {
            None => { None }
            Some(first_derivative) => {
                Some((first_derivative - self.first_derivative_mean) / self.first_derivative_std)
            }
        }
    }

    /// 获取二阶导数
    pub fn get_second_derivative(&self, time: DateTime<Local>) -> Option<Decimal> {
        let delta = self.delta_time;
        let date_plus = time + delta;
        let date_minus = time - delta;
        match self.get_first_derivative(date_plus) {
            None => { None }
            Some(price_plus) => {
                match self.get_first_derivative(date_minus) {
                    None => { None }
                    Some(price_minus) => { Some((price_plus - price_minus) / Decimal::from_f64(2.0 * delta.num_minutes().to_f64().unwrap()).unwrap() * Decimal::from(24 * 60)) }
                }
            }
        }
    }

    /// 获取二阶导数标准信号
    pub fn get_second_derivative_standard(&self, time: DateTime<Local>) -> Option<Decimal> {
        match self.get_second_derivative(time) {
            None => { None }
            Some(second_derivative) => {
                Some((second_derivative - self.second_derivative_mean) / self.second_derivative_std)
            }
        }
    }

    /// 获取预期仓位
    pub fn get_position(&self, time: DateTime<Local>) -> Option<Decimal> {
        let position_max = self.position_max;
        let position_min = self.position_min;
        let position_delta = position_max - position_min;
        match self.get_first_derivative_standard(time) {
            None => { None }
            Some(first_derivative) => {
                match self.get_second_derivative_standard(time) {
                    None => { None }
                    Some(second_derivative) => {
                        let angle_rad = second_derivative.to_f64().unwrap().atan2(first_derivative.to_f64().unwrap()); // 结果范围：[-π, π]
                        let angle_deg = (angle_rad.to_degrees() + 360.0) % 360.0; // 转换为度数
                        if angle_deg >= 0.0 && angle_deg <= 90.0 {
                            Some(position_max)
                        } else if angle_deg > 90.0 && angle_deg <= 180.0 {
                            Some(position_max - position_delta * Decimal::from_f64((angle_deg - 90.0) / 90.0).unwrap())
                        } else if angle_deg > 180.0 && angle_deg <= 270.0 {
                            Some(position_min)
                        } else {
                            Some(position_min + position_delta * Decimal::from_f64((angle_deg - 270.0) / 90.0).unwrap())
                        }
                    }
                }
            }
        }
    }

    /// 获取价格
    pub fn get_price(&self, time: DateTime<Local>) -> Option<Decimal> {
        self.price_model.get_price(time)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use rust_decimal::Decimal;
    use serde::Serialize;
    use crate::strategy::model::position_model::SPositionModel;
    use crate::strategy::model::price_model_long_term_trend::SPriceModelLongTermTrend;
    #[derive(Debug, Serialize)]
    struct STestOutputFormat {
        pub time: DateTime<Local>,
        pub price: Decimal,
        pub position: Decimal,
        pub first_standard: Decimal,
        pub first_standard_std: Decimal,
        pub second_standard: Decimal,
        pub second_standard_std: Decimal,
    }
    #[test]
    pub fn test1() -> Result<(), Box<dyn std::error::Error>> {
        // 准备文件
        let path = String::from("data/test/position_test.csv");
        // dbg!(&path);
        let file = File::create(path.clone()).unwrap();
        let mut wtr = csv::Writer::from_writer(file);

        // 准备模型
        let price_model = SPriceModelLongTermTrend::default();
        let position_model = SPositionModel::from(price_model, 0.9, 0.1);

        // 准备数据
        let date_from = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2018, 1, 1).unwrap(), NaiveTime::from_hms_opt(00, 0, 0).unwrap())).single().unwrap();
        let date_to = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), NaiveTime::from_hms_opt(00, 0, 0).unwrap())).single().unwrap();
        let mut date_now = date_from;
        let mut count = 1;
        while date_now < date_to {
            let price = position_model.get_price(date_now).unwrap();
            let position = position_model.get_position(date_now).unwrap();
            let first_standard = position_model.get_first_derivative(date_now).unwrap();
            let second_standard = position_model.get_second_derivative(date_now).unwrap();
            let first_standard_std = position_model.get_first_derivative_standard(date_now).unwrap();
            let second_standard_std = position_model.get_second_derivative_standard(date_now).unwrap();

            // println!("count: {:?}\tdate: {:?}\tprice: {:.2?}\tposition:{:.4?}%\tfirst:{:?}\tsecond:{:?}\tfirst_std:{:?}\tsecond_std:{:?}",
            //          count, date_now, price, position * Decimal::from(100), first_standard, second_standard, first_standard_std, second_standard_std);

            // println!("count: {:?}\tdate: {:?}\tprice: {:.2?}\tposition:{:.4?}%\tfirst_std:{:.4?}\tsecond_std:{:.4?}",
            //          count, date_now, price, position * Decimal::from(100), first_standard_std, second_standard_std);

            let merged_output = STestOutputFormat {
                time: date_now,
                price,
                position,
                first_standard,
                first_standard_std,
                second_standard,
                second_standard_std,
            };
            wtr.serialize(merged_output).unwrap();

            date_now += Duration::days(1);
            count += 1;
        }

        // 输出到csv文件
        wtr.flush().unwrap();
        println!("仓位测试数据写入完成：{}", path);

        Ok(())
    }
}