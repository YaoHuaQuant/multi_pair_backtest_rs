//! 反馈控制相关

use rust_decimal::Decimal;

pub struct SStrategyPidConfig {
    ///  比例项 弹性参数
    /// 必填
    /// 取值区间 (0, 1) 值越大表示弹性越差 刚性越强 等待区间越长
    pub proportional: Decimal,
    ///  积分项 时间参数
    ///  选填（不需要考虑稳态误差）
    /// 取值区间(0,1) 值越大表示仓位收敛越快-单边趋势下提高刚性 减少弹性损耗
    /// 适用于单边趋势或者空仓启动场景
    pub integral: Option<SPidIntegral>,
    ///  微分项
    /// 选填（不需要考虑快速收敛）
    pub derivative: Option<Decimal>,
}

/// Pid中的积分项
pub struct SPidIntegral {
    /// 参数值
    parameter: Decimal,
    /// 累计值
    cumulative: Decimal,
    /// 累计值最大值
    max_cumulative: Decimal,
}

impl SPidIntegral {
    pub fn new(parameter: Decimal, max_cumulative: Decimal) -> Self {
        assert!(max_cumulative > Decimal::from(0));
        Self {
            parameter,
            cumulative: Decimal::from(0),
            max_cumulative,
        }
    }

    /// 累加一个diff值 并限制其大小
    pub fn add_up(&mut self, new_diff_value: Decimal) {
        let new_sum = self.cumulative + new_diff_value;
        if new_sum > self.max_cumulative {
            self.cumulative = self.max_cumulative;
        } else if new_sum < -self.max_cumulative {
            self.cumulative = -self.max_cumulative;
        } else {
            self.cumulative = new_sum;
        }
    }

    pub fn get_parameter(&self) -> Decimal {self.parameter}
    pub fn get_cumulative(&self) -> Decimal {self.cumulative}
    pub fn get_max_cumulative(&self) -> Decimal {self.max_cumulative}
}