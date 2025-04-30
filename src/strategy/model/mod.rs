//! 策略模型相关配置

use chrono::{DateTime, Local};
use rust_decimal::Decimal;

pub mod feedback_control;
pub mod model_sin_test;
pub mod model_step_test;

/// 价格模型接口
pub trait TPriceModel {
    /// 根据给定时间 返回预测价格
    fn get_price(&self, time: DateTime<Local>) -> Option<Decimal>;
    
    /// 提供新数据 更新模型
    fn update_model(&mut self, time: DateTime<Local>, price:Decimal);
}