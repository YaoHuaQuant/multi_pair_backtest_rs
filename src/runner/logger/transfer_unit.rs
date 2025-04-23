use rust_decimal::Decimal;

/// 统计交易数据
#[derive(Debug, Default, Clone)]
pub struct SDataLogTransferUnit {
    /// 挂单的买入交易数
    pub unfulfilled_buy_order_cnt: i64,
    /// 挂单的卖出交易数
    pub unfulfilled_sell_order_cnt: i64,
    /// 已成交的买入交易数
    pub executed_buy_order_cnt: i64,
    /// 已成交的卖出交易数
    pub executed_sell_order_cnt: i64,
    /// 挂单的买入资产量（USDT计价）
    pub unfulfilled_buy_usdt_cnt: Decimal,
    /// 挂单的卖出资产量（USDT计价）
    pub unfulfilled_sell_usdt_cnt: Decimal,
    /// 已成交的买入资产量（USDT计价）
    pub executed_buy_usdt_cnt: Decimal,
    /// 已成交的卖出资产量（USDT计价）
    pub executed_sell_usdt_cnt: Decimal,
}

impl SDataLogTransferUnit {
    pub fn from(
        unfulfilled:SDataLogTransferUnfulfilledUnit,
        executed:SDataLogTransferExecutedUnit
    ) -> Self {
        Self {
            unfulfilled_buy_order_cnt: unfulfilled.unfulfilled_buy_order_cnt,
            unfulfilled_sell_order_cnt: unfulfilled.unfulfilled_sell_order_cnt,
            executed_buy_order_cnt: executed.executed_buy_order_cnt,
            executed_sell_order_cnt: executed.executed_sell_order_cnt,
            unfulfilled_buy_usdt_cnt: unfulfilled.unfulfilled_buy_usdt_cnt,
            unfulfilled_sell_usdt_cnt: unfulfilled.unfulfilled_sell_usdt_cnt,
            executed_buy_usdt_cnt: executed.executed_buy_usdt_cnt,
            executed_sell_usdt_cnt: executed.executed_sell_usdt_cnt,
        }
    }
}

/// 挂单数据部分
#[derive(Debug, Default, Clone)]
pub struct SDataLogTransferUnfulfilledUnit {
    /// 挂单的买入交易数
    pub unfulfilled_buy_order_cnt: i64,
    /// 挂单的卖出交易数
    pub unfulfilled_sell_order_cnt: i64,
    /// 挂单的买入资产量（USDT计价）
    pub unfulfilled_buy_usdt_cnt: Decimal,
    /// 挂单的卖出资产量（USDT计价）
    pub unfulfilled_sell_usdt_cnt: Decimal,
}

/// 已成交数据部分
#[derive(Debug, Default, Clone)]
pub struct SDataLogTransferExecutedUnit {
    /// 已成交的买入交易数
    pub executed_buy_order_cnt: i64,
    /// 已成交的卖出交易数
    pub executed_sell_order_cnt: i64,
    /// 已成交的买入资产量（USDT计价）
    pub executed_buy_usdt_cnt: Decimal,
    /// 已成交的卖出资产量（USDT计价）
    pub executed_sell_usdt_cnt: Decimal,
}

