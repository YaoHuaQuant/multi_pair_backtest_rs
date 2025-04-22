pub mod order;
pub mod order_manager;
pub mod trading_pair_order_manager_map;

/// 持仓方向
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

/// 仓位状态
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EOrderPosition {
    Open,
    Close
}