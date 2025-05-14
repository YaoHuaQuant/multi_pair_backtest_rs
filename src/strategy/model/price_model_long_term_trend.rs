//! 长周期趋势模型
//! 目标函数: Price(x) = M(x) * ( 1 + A(x) * S(x) )
//! M(x)为均值函数，A(x)为幅值函数，S(x)为周期函数。
//!
//! 1. M(x)=l*(x+x0)^k + c
//! 参数：
//! 1.1 L-(L>0)均值增长的速率
//! 1.2 x0-(x0>0)增长曲线的起始
//! 1.3 k-(0<k<1)控制增速衰减速度，值越小增速下降越快。
//! 1.4 c-(c>=0) y轴起点
//!
//! 2. A(x)=1/(1+log(1+a*(x+x1)))
//! 参数：
//! 2.1 a-(0<a)控制幅值衰减速度，a越大衰减越快。
//! 2.2 x1-(0<=x1)幅值函数的起点
//!
//! 3. S(x)=sin(x/T + x2)
//! 参数：
//! 3.1 T-(T>0)周期
//! 3.2 x2-(0<=x2<=2pi)初始相位

use std::f32::consts::PI;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use crate::strategy::model::TPriceModel;

/// 长周期趋势模型
/// 输入参数为时间（目标时间与原点时间的偏移值，以天为单位）
/// 输出参数为价格（美元）
pub struct SPriceModelLongTermTrend {
    /// 原点时间
    origin_date: DateTime<Local>,
    /// 幅值衰减速度
    a: f64,
    ///均值函数起点
    c: f64,
    /// 均值增长速率
    l: f64,
    /// 均质增速衰减速率
    k: f64,
    /// 均值增长曲线起点
    x0: f64,
    /// 幅值函数起点
    x1: f64,
    /// 周期函数初始相位
    x2: f64,
    /// 周期函数周期
    t: f64,
}

impl SPriceModelLongTermTrend {
    pub fn new(origin_date: DateTime<Local>, a: f64, c: f64, l: f64, k: f64, x0: f64, x1: f64, x2: f64, t: f64) -> Self {
        Self { origin_date, a, c, l, k, x0, x1, x2, t }
    }

    pub fn default() -> Self {
        Self {
            origin_date: Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2017, 8, 31).unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap())).single().unwrap(),
            a: 0.00124602,
            c: 295.231439,
            l: 40.4493398,
            k: 0.91441314,
            x0: 71.5570562,
            x1: 0.0000036314,
            x2: 1.97740554,
            t: 210000.0 * 10.0 / 60.0 / 24.0 / PI as f64 / 2.0,
        }
    }

    fn f_m(&self, input: f64) -> f64 {
        self.l * (input + self.x0).powf(self.k) + self.c
    }

    fn f_a(&self, input: f64) -> f64 {
        let mut tmp = 1.0 + self.a * (input + self.x1);
        // tmp 至少为1
        tmp = if tmp > 1.0 { tmp } else { 1.0 };

        1.0 / (1.0 + tmp.ln())
    }

    fn f_s(&self, input: f64) -> f64 {
        (input / self.t + self.x2).sin()
    }

    /// 输入f64格式的参数（时间）
    /// 输出f64格式的参数（价格）
    fn f_price(&self, input: f64) -> f64 {
        self.f_m(input) * (1.0 + self.f_a(input) * self.f_s(input))
    }
}


impl TPriceModel for SPriceModelLongTermTrend {
    fn get_price(&self, time: DateTime<Local>) -> Option<Decimal> {
        // 入参格式转换：计算输入时间相对与原点的偏移量 并换算成天(f64类型)
        let duration = time.signed_duration_since(self.origin_date);
        let days = duration.num_minutes() as f64 / 1444.0;
        // println!("duration: {:?}", duration);
        // println!("days: {:?}", days);
        // 计算价格 f64格式
        let price_f64 = self.f_price(days);
        // println!("price_f64: {:?}", price_f64);
        // 出参格式转换（Decimal类型）
        match Decimal::from_f64(price_f64) {
            None => { None }
            Some(price) => { Some(price) }
        }
    }

    fn update_model(&mut self, time: DateTime<Local>, price: Decimal) {
        // 不需要update
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use plotters::prelude::*;
    use rust_decimal::prelude::ToPrimitive;
    use crate::strategy::model::price_model_long_term_trend::SPriceModelLongTermTrend;
    use crate::strategy::model::TPriceModel;

    #[test]
    pub fn test1() -> Result<(), Box<dyn std::error::Error>> {
        // 创建一个绘图区域，输出为 PNG 文件，尺寸为 600x400 像素
        let root_area = BitMapBackend::new("/data/chart/test_line_chart.png", (600, 400)).into_drawing_area();
        root_area.fill(&WHITE)?;

        // 准备模型
        let model = SPriceModelLongTermTrend::default();
        // 准备数据

        let mut data = Vec::new();
        let date_from = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2018, 1, 1).unwrap(), NaiveTime::from_hms_opt(00, 0, 0).unwrap())).single().unwrap();
        let date_to = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), NaiveTime::from_hms_opt(00, 0, 0).unwrap())).single().unwrap();
        let mut date_now = date_from;
        let mut count = 1;
        while date_now < date_to {
            let x = model.get_price(date_now);
            let y = x.unwrap().to_i32();
            let price = y.unwrap();
            data.push((count, price));
            
            println!("count: {:?}\tdate: {:?}\tprice: {:?}", count, date_now, price);
            
            date_now += Duration::days(1);
            count += 1;
        }

        // 构建图表上下文，设置标题和坐标轴范围
        let mut chart = ChartBuilder::on(&root_area)
            .caption("折线图示例", ("sans-serif", 40))
            .margin(20)
            .x_label_area_size(2000)
            .y_label_area_size(100000)
            .build_cartesian_2d(0..100, 0..1000)?;

        // 绘制坐标网格
        chart.configure_mesh().draw()?;


        // 绘制折线图
        chart.draw_series(LineSeries::new(data, &BLUE))?;

        Ok(())
    }
}