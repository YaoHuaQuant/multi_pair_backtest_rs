use uuid::Uuid;

pub enum EStrategyOrderState {
    /// 待提交
    Pending,
    /// 已提交 待开仓
    Opening,
    /// 已开仓 待提交平单
    Opened,
    /// 已提交平单
    Closing,
    /// 已平仓
    Closed,
    /// 已取消
    Canceled,
}

pub struct SStrategyOrder {
    pub id:Uuid
}


