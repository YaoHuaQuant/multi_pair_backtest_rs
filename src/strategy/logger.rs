use rust_decimal::Decimal;

/// 用于记录Strategy运行中的中间过程数据
#[derive(Clone)]
pub struct SStrategyLogger {
    /// 目标仓位占比
    pub target_position_ratio:Decimal,
}

impl SStrategyLogger {
    /// 空数据
    /// 用于适配一些不支持特定数据的Strategy
    pub fn none() -> Self {
        Self{ target_position_ratio: Decimal::from(-1) }
    }
}