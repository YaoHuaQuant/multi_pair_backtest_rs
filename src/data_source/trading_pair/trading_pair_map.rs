use std::collections::HashMap;

use chrono::{DateTime, Local};
use rust_decimal::Decimal;

use crate::data_source::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data_source::kline::{SKlineData, SKlineUnitData};
use crate::data_source::trading_pair::ETradingPairType;
use crate::data_source::trading_pair::trading_pair::STradingPair;

pub type RTradingPairManagerResult<T> = Result<T, ETradingPairManagerError>;

#[derive(Debug)]
pub enum ETradingPairManagerError {
    TradingPairNotFoundError(ETradingPairType)
}

/// 交易对管理器
/// 交易对类型-交易对 映射
#[derive(Default, Debug)]
pub struct STradingPairMap {
    /// 交易对
    pub inner: HashMap<ETradingPairType, STradingPair>,
}

impl STradingPairMap {
    pub fn new() -> Self {
        Self { inner: Default::default() }
    }

    pub fn add_trading_pair(&mut self, ty_type: ETradingPairType, kline_data: SKlineData, funding_rate: Option<SFundingRateData>) {
        self.inner.entry(ty_type).or_insert(STradingPair::new(ty_type, kline_data, funding_rate));
    }

    pub fn add_trading_pairs(&mut self, ty_type: ETradingPairType, kline_data: SKlineData, funding_rate: Option<SFundingRateData>) {
        self.inner.entry(ty_type).or_insert(STradingPair::new(ty_type, kline_data, funding_rate));
    }

    pub fn get(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<&STradingPair> {
        match self.inner.get(&tp_type) {
            None => { Err(ETradingPairManagerError::TradingPairNotFoundError(tp_type)) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn get_mut(&mut self, tp_type: ETradingPairType) -> RTradingPairManagerResult<&mut STradingPair> {
        match self.inner.get_mut(&tp_type) {
            None => { Err(ETradingPairManagerError::TradingPairNotFoundError(tp_type)) }
            Some(item) => { Ok(item) }
        }
    }

    // region ----- 转发STradingPair函数-----
    pub fn insert_funding_rate(&mut self, tp_type: ETradingPairType, time: &DateTime<Local>, funding_rate: Decimal) -> RTradingPairManagerResult<()> {
        Ok(self.get_mut(tp_type)?.insert_funding_rate(time, funding_rate))
    }

    pub fn get_funding_rate(&self, tp_type: ETradingPairType, time: &DateTime<Local>) -> RTradingPairManagerResult<Option<&Decimal>> {
        Ok(self.get(tp_type)?.get_funding_rate(time))
    }

    pub fn range_funding_rate(&self, tp_type: ETradingPairType, start: DateTime<Local>, end: DateTime<Local>)
                              -> RTradingPairManagerResult<Option<impl Iterator<Item=(&DateTime<Local>, &SFundingRateUnitData)>>>
    {
        Ok(self.get(tp_type)?.range_funding_rate(start, end))
    }

    pub fn iter_funding_rate(&self, tp_type: ETradingPairType)
                             -> RTradingPairManagerResult<Option<impl Iterator<Item=(&DateTime<Local>, &SFundingRateUnitData)>>>
    {
        Ok(self.get(tp_type)?.iter_funding_rate())
    }

    pub fn insert_kline(
        &mut self,
        tp_type: ETradingPairType,
        open_time: DateTime<Local>,
        close_time: DateTime<Local>,
        open_price: Decimal,
        close_price: Decimal,
        high_price: Decimal,
        low_price: Decimal,
        volume: Decimal,
    ) -> RTradingPairManagerResult<()> {
        Ok(self.get_mut(tp_type)?.insert_kline(open_time, close_time, open_price, close_price, high_price, low_price, volume))
    }

    pub fn get_kline(&self, tp_type: ETradingPairType, time: &DateTime<Local>) -> RTradingPairManagerResult<Option<&SKlineUnitData>> {
        Ok(self.get(tp_type)?.get_kline(time))
    }

    pub fn range_kline(&self, tp_type: ETradingPairType, start: DateTime<Local>, end: DateTime<Local>) -> RTradingPairManagerResult<impl Iterator<Item=(&DateTime<Local>, &SKlineUnitData)>> {
        Ok(self.get(tp_type)?.range_kline(start, end))
    }

    pub fn iter_kline(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<impl Iterator<Item=(&DateTime<Local>, &SKlineUnitData)>> {
        Ok(self.get(tp_type)?.iter_kline())
    }
    // endregion ----- 转发STradingPair函数-----
}

#[cfg(test)]
mod tests {
    use crate::data_source::kline::SKlineData;
    use crate::data_source::trading_pair::ETradingPairType;
    use crate::data_source::trading_pair::trading_pair_map::STradingPairMap;

    fn get_test_data() -> STradingPairMap {
        let mut data = STradingPairMap::new();
        data.add_trading_pair(ETradingPairType::BtcUsdt, SKlineData::new(), None);
        data.add_trading_pair(ETradingPairType::BtcUsdtFuture, SKlineData::new(), None);
        data.add_trading_pair(ETradingPairType::BtcUsdCmFuture, SKlineData::new(), None);

        data
    }

    #[test]
    pub fn test_add_pair() {
        let mut data = STradingPairMap::new();
        dbg!(&data);

        println!("add BtcUsdt Pair:");

        data.add_trading_pair(ETradingPairType::BtcUsdt, SKlineData::new(), None);
        dbg!(&data);
    }
}