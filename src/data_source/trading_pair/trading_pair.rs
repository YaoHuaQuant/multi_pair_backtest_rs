use chrono::{DateTime, Local};
use rust_decimal::Decimal;

use crate::data_source::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data_source::kline::{SKlineData, SKlineUnitData};
use crate::data_source::trading_pair::ETradingPairType;
use crate::data_runtime::asset::EAssetType;

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
}

impl STradingPair {
    pub fn new(tp_type: ETradingPairType, kline_data: SKlineData, funding_rate: Option<SFundingRateData>) -> Self {
        Self {
            tp_type,
            base_currency: tp_type.get_base_currency_type(),
            quote_currency: tp_type.get_quote_currency_type(),
            kline_data,
            funding_rate,
        }
    }

    // region ----- 转发SFundingRateData函数-----
    pub fn insert_funding_rate(&mut self, time: &DateTime<Local>, funding_rate: Decimal) {
        match &mut self.funding_rate {
            None => {}
            Some(data) => { data.insert(time, funding_rate) }
        }
    }

    pub fn get_funding_rate(&self, time: &DateTime<Local>) -> Option<&Decimal> {
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

    pub fn get_kline(&self, time: &DateTime<Local>) -> Option<&SKlineUnitData> {
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