use std::cmp::Reverse;
use std::collections::HashSet;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::assert::asset::EAssetType;
use crate::assert::trading_pair::ETradingPairType;
use crate::data::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data::kline::{SKlineData, SKlineUnitData};
use crate::order::EOrderAction;
use crate::runner::strategy_runner::runner_order::{EOrderUpdate, SOrder};
use crate::runner::strategy_runner::runner_order_manager::{EOrderManagerError, EOrderManagerUpdate, SOrderManager, SOrderUuidAndUpdate};

/// 交易对
#[derive(Debug)]
pub struct STradingPair {
    /// 交易对类型
    pub tp_type: ETradingPairType,
    /// 基础货币 如btc
    pub base_currency: EAssetType,
    /// 计价货币 如usdt
    pub quote_currency: EAssetType,
    pub kline_data: SKlineData,
    pub funding_rate: Option<SFundingRateData>,
    pub order_manager: SOrderManager,
}

impl STradingPair {
    pub fn new(tp_type: ETradingPairType, kline_data: SKlineData, funding_rate: Option<SFundingRateData>) -> Self {
        Self {
            tp_type,
            base_currency: tp_type.get_base_currency(),
            quote_currency: tp_type.get_quote_currency(),
            kline_data,
            funding_rate,
            order_manager: Default::default(),
        }
    }

    // region ----- 转发SOrderManager函数 -----
    pub fn add_order(&mut self, price: Decimal, quantity: Decimal, action: EOrderAction) -> Uuid {
        self.order_manager.add_order(price, quantity, action)
    }

    pub fn peek_order(&self, uuid: Uuid) -> Option<&SOrder> {
        self.order_manager.peek_order(uuid)
    }

    pub fn update_or_remove_orders(&mut self, uuid_update_list: Vec<SOrderUuidAndUpdate>) -> Result<(), EOrderManagerError> {
        self.order_manager.update_or_remove_orders(uuid_update_list)
    }

    pub fn peek_highest_buy_order(&self) -> Option<&SOrder> {
        self.order_manager.peek_highest_buy_order()
    }

    pub fn pop_highest_buy_order(&mut self) -> Option<SOrder> {
        self.order_manager.pop_highest_buy_order()
    }

    pub fn peek_lowest_sell_order(&self) -> Option<&SOrder> {
        self.order_manager.peek_lowest_sell_order()
    }

    pub fn pop_lowest_sell_order(&mut self) -> Option<SOrder> {
        self.order_manager.pop_lowest_sell_order()
    }
    // endregion ----- 转发SOrderManager函数-----

    // region ----- 转发SFundingRateData函数-----
    pub fn insert_funding_rate(&mut self, time: DateTime<Local>, funding_rate: Decimal) {
        match &mut self.funding_rate {
            None => {}
            Some(data) => { data.insert(time, funding_rate) }
        }
    }

    pub fn get_funding_rate(&self, time: DateTime<Local>) -> Option<&Decimal> {
        match &self.funding_rate {
            None => { None }
            Some(data) => { data.get(time) }
        }
    }

    pub fn range_funding_rate(&self, start: DateTime<Local>, end: DateTime<Local>) -> Option<impl Iterator<Item=(&DateTime<Local>, &SFundingRateUnitData)>> {
        match &self.funding_rate {
            None => { None }
            Some(data) => { Some(data.range(start, end)) }
        }
    }

    pub fn iter_funding_rate(&self) -> Option<impl Iterator<Item=(&DateTime<Local>, &SFundingRateUnitData)>> {
        match &self.funding_rate {
            None => { None }
            Some(data) => { Some(data.iter()) }
        }
    }
    // endregion ----- 转发SFundingRateData函数-----

    // region ----- 转发SKlineData函数-----
    pub fn insert_kline(
        &mut self,
        open_time: DateTime<Local>,
        close_time: DateTime<Local>,
        open_price: Decimal,
        close_price: Decimal,
        high_price: Decimal,
        low_price: Decimal,
        volume: Decimal,
    ) {
        self.kline_data.insert(open_time, close_time, open_price, close_price, high_price, low_price, volume)
    }

    pub fn get_kline(&self, time: DateTime<Local>) -> Option<&SKlineUnitData> {
        self.kline_data.get(time)
    }

    pub fn range_kline(&self, start: DateTime<Local>, end: DateTime<Local>) -> impl Iterator<Item=(&DateTime<Local>, &SKlineUnitData)> {
        self.kline_data.range(start, end)
    }

    pub fn iter_kline(&self) -> impl Iterator<Item=(&DateTime<Local>, &SKlineUnitData)> {
        self.kline_data.iter()
    }
    // endregion ----- 转发SKlineData函数-----
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn test1() {
        // todo
    }
}