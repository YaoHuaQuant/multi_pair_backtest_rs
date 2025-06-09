pub mod order;
pub mod order_manager;
pub mod trading_pair_order_manager_map;
pub mod order_v3;
pub mod order_manager_v3;
pub mod trading_pair_order_manager_map_v3;

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

impl EOrderDirection {
    /// 获取相反的方向
    pub fn rev(&self) -> Self {
        match self {
            EOrderDirection::Long => {EOrderDirection::Short}
            EOrderDirection::Short => {EOrderDirection::Long}
        }
    }
}