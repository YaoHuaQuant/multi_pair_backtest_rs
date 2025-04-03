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
    use crate::data_source::trading_pair::trading_pair_map::STradingPairMap;
    use crate::data_source::trading_pair::trading_pair_enum::ETradingPairType;

    #[test]
    pub fn test_add_pair() {
        let mut data = STradingPairMap::new();
        dbg!(&data);

        println!("add BtcUsdt Pair:");

        data.add_trading_pair(ETradingPairType::BtcUsdt, SKlineData::new(), None);
        dbg!(&data);
    }

    #[test]
    pub fn test_calculate_total_assets() {
        // todo

        // let mut manager = SOrderManager::new();
        //
        // let price_vec_buy = vec![
        //     Decimal::from_str("1").unwrap(),
        //     Decimal::from_str("1").unwrap(),
        //     Decimal::from_str("2").unwrap(),
        //     Decimal::from_str("3").unwrap(),
        //     Decimal::from_str("5").unwrap(),
        // ];
        //
        // for price in price_vec_buy {
        //     let id = manager.add_order(SAddOrder {
        //         action: EOrderAction::Buy,
        //         price,
        //         quantity: Decimal::from_str("1").unwrap(),
        //     });
        //     let mut order = manager.orders.get_mut(&id).unwrap();
        //     let asset = SAssetV2 {
        //         as_type: EAssetType::Usdt,
        //         balance: Decimal::from(price),
        //     };
        //     let r = order.submit(asset);
        // }
        //
        // let price_vec_sell = vec![
        //     Decimal::from_str("1").unwrap(),
        //     Decimal::from_str("1").unwrap(),
        //     Decimal::from_str("1").unwrap(),
        //     Decimal::from_str("1").unwrap(),
        //     Decimal::from_str("1").unwrap(),
        // ];
        //
        // for price in price_vec_sell {
        //     let id = manager.add_order(SAddOrder {
        //         action: EOrderAction::Sell,
        //         price,
        //         quantity: Decimal::from_str("1").unwrap(),
        //     });
        //     let mut order = manager.orders.get_mut(&id).unwrap();
        //     let asset = SAssetV2 {
        //         as_type: EAssetType::Btc,
        //         balance: Decimal::from(price),
        //     };
        //     let r = order.submit(asset);
        // }
        //
        // let r = manager.calculate_total_assets();
        // println!("result:{:?}", r);
        // let usdt = r.get(&EAssetType::Usdt);
        // assert!(usdt.is_some());
        // assert_eq!(*usdt.unwrap(), Decimal::from(12));
        //
        // let btc = r.get(&EAssetType::Btc);
        // assert!(btc.is_some());
        // assert_eq!(*btc.unwrap(), Decimal::from(5));
    }
}