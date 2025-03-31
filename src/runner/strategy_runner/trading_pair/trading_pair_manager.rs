use std::collections::HashMap;

use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::asset::EAssetType;
use crate::asset::trading_pair::ETradingPairType;
use crate::data::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data::kline::{SKlineData, SKlineUnitData};
use crate::order::EOrderAction;
use crate::runner::strategy_runner::order::order::SOrder;
use crate::runner::strategy_runner::order::order_manager::ROrderManagerResult;
use crate::runner::strategy_runner::trading_pair::trading_pair::STradingPair;

pub type RTradingPairManagerResult<T> = Result<T, ETradingPairManagerError>;

#[derive(Debug)]
pub enum ETradingPairManagerError {
    TradingPairNotFoundError(ETradingPairType)
}

#[derive(Default, Debug)]
pub struct STradingPairManager {
    /// 交易对
    pub trading_pair_map: HashMap<ETradingPairType, STradingPair>,
}

impl STradingPairManager {
    pub fn new() -> Self {
        Self { trading_pair_map: Default::default() }
    }

    pub fn add_trading_pair(&mut self, ty_type: ETradingPairType, kline_data: SKlineData, funding_rate: Option<SFundingRateData>) {
        self.trading_pair_map.entry(ty_type).or_insert(STradingPair::new(ty_type, kline_data, funding_rate));
    }

    pub fn add_trading_pairs(&mut self, ty_type: ETradingPairType, kline_data: SKlineData, funding_rate: Option<SFundingRateData>) {
        self.trading_pair_map.entry(ty_type).or_insert(STradingPair::new(ty_type, kline_data, funding_rate));
    }

    pub fn get(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<&STradingPair> {
        match self.trading_pair_map.get(&tp_type) {
            None => { Err(ETradingPairManagerError::TradingPairNotFoundError(tp_type)) }
            Some(item) => { Ok(item) }
        }
    }

    pub fn get_mut(&mut self, tp_type: ETradingPairType) -> RTradingPairManagerResult<&mut STradingPair> {
        match self.trading_pair_map.get_mut(&tp_type) {
            None => { Err(ETradingPairManagerError::TradingPairNotFoundError(tp_type)) }
            Some(item) => { Ok(item) }
        }
    }

    /// 统计每种资产的总锁定量
    pub fn calculate_total_assets(&self) -> HashMap<EAssetType, Decimal> {
        let mut result = HashMap::new();
        for (_, trading_pair) in self.trading_pair_map.iter() {
            let map = trading_pair.calculate_total_assets();
            for (as_type, balance) in map.iter() {
                let ptr = result.entry(*as_type).or_insert(Decimal::from(0));
                *ptr += balance;
            }
        }
        result
    }


    /// 获取第一个日期
    pub fn get_first_date(&self) -> Option<&DateTime<Local>> {
        let mut max_value: Option<&DateTime<Local>> = None;
        for item in self.trading_pair_map.iter() {
            // 检查k线
            if let Some((date, _)) = item.1.kline_data.data.first_key_value() {
                match max_value {
                    None => { max_value = Some(date) }
                    Some(max) => {
                        if max > date {
                            max_value = Some(date)
                        }
                    }
                }
            }
            // 不检查资金费率
        }
        max_value
    }

    /// 获取最后一个日期
    pub fn get_last_date(&self) -> Option<&DateTime<Local>> {
        let mut min_value: Option<&DateTime<Local>> = None;
        for item in self.trading_pair_map.iter() {
            // 检查k线
            if let Some((date, _)) = item.1.kline_data.data.first_key_value() {
                match min_value {
                    None => { min_value = Some(date) }
                    Some(min) => {
                        if min < date {
                            min_value = Some(date)
                        }
                    }
                }
            }
            // 不检查资金费率
        }
        min_value
    }

    // region ----- 转发STradingPair函数-----
    pub fn add_order(&mut self, tp_type: ETradingPairType, price: Decimal, quantity: Decimal, action: EOrderAction) -> RTradingPairManagerResult<ROrderManagerResult<Uuid>> {
        Ok(self.get_mut(tp_type)?.add_order(price, quantity, action))
    }

    pub fn peek_order(&self, tp_type: ETradingPairType, uuid: Uuid) -> RTradingPairManagerResult<Option<&SOrder>> {
        Ok(self.get(tp_type)?.peek_order(uuid))
    }

    pub fn remove_orders(&mut self, tp_type: ETradingPairType, uuid_list: Vec<Uuid>) -> RTradingPairManagerResult<ROrderManagerResult<Vec<SOrder>>> {
        Ok(self.get_mut(tp_type)?.remove_orders(uuid_list))
    }

    pub fn peek_highest_buy_order(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<Option<&SOrder>> {
        Ok(self.get(tp_type)?.peek_highest_buy_order())
    }

    pub fn pop_highest_buy_order(&mut self, tp_type: ETradingPairType) -> RTradingPairManagerResult<Option<SOrder>> {
        Ok(self.get_mut(tp_type)?.pop_highest_buy_order())
    }

    pub fn peek_lowest_sell_order(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<Option<&SOrder>> {
        Ok(self.get(tp_type)?.peek_lowest_sell_order())
    }

    pub fn pop_lowest_sell_order(&mut self, tp_type: ETradingPairType) -> RTradingPairManagerResult<Option<SOrder>> {
        Ok(self.get_mut(tp_type)?.pop_lowest_sell_order())
    }

    pub fn calculate_trading_pair_assets(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<HashMap<EAssetType, Decimal>> {
        Ok(self.get(tp_type)?.calculate_total_assets())
    }

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
    use crate::asset::trading_pair::ETradingPairType;
    use crate::data::kline::SKlineData;
    use crate::runner::strategy_runner::trading_pair::trading_pair_manager::STradingPairManager;

    #[test]
    pub fn test_add_pair() {
        let mut data = STradingPairManager::new();
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