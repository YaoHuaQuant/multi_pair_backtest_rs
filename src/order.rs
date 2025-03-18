/// 持仓方向
pub enum EOrderDirection {
    Long,
    Short
}

/// 交易操作
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EOrderAction {
    Buy,
    Sell
}