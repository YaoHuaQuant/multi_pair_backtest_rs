//! 数据管理器
use std::collections::HashMap;

use chrono::{DateTime, Local};
use rust_decimal::Decimal;

use crate::data_source::db::api::data_api_db::SDataApiDb;
use crate::data_source::db::api::TDataApi;
use crate::data_source::db::SDbClickhouse;
use crate::data_source::funding_rate::SFundingRateData;
use crate::data_source::trading_pair::ETradingPairType;
use crate::data_source::trading_pair::trading_pair::STradingPair;
use crate::data_source::trading_pair::trading_pair_map::{RTradingPairManagerResult, STradingPairMap};

/// 数据管理器
#[derive(Debug)]
pub struct SDataManager<A: TDataApi> {
    /// 数据接口
    pub data_api: A,
    /// 交易对管理器
    pub trading_pair_map: STradingPairMap,
}

impl SDataManager<SDataApiDb> {
    pub async fn build(date_from: &DateTime<Local>, date_to: &DateTime<Local>) -> Self {
        let db = SDbClickhouse::new();
        let data_api = SDataApiDb::new(db);

        let mut trading_pair_manager = STradingPairMap::new();
        // 插入trading_pair配置 todo 插入更多配置
        let kline = data_api.get_kline(date_from, date_to).await.unwrap();
        let funding_rate: Option<SFundingRateData> = None; // todo 插入资金费率
        trading_pair_manager.add_trading_pair(ETradingPairType::BtcUsdt, kline, funding_rate);

        Self { data_api, trading_pair_map: trading_pair_manager }
    }
}

impl<A: TDataApi> SDataManager<A> {
    /// 获取所有交易对
    pub fn get_trading_pairs(&self) -> &HashMap<ETradingPairType, STradingPair> {
        &self.trading_pair_map.inner
    }

    /// 获取特定交易对
    pub fn get_mut_trading_pairs(&mut self) -> &mut HashMap<ETradingPairType, STradingPair> {
        &mut self.trading_pair_map.inner
    }

    /// 获取所有交易对
    pub fn get_trading_pair(&self, tp_type: ETradingPairType) -> RTradingPairManagerResult<&STradingPair> {
        self.trading_pair_map.get(tp_type)
    }

    /// 获取特定交易对
    pub fn get_mut_trading_pair(&mut self, tp_type: ETradingPairType) -> RTradingPairManagerResult<&mut STradingPair> {
        self.trading_pair_map.get_mut(tp_type)
    }

    /// 获取特定K线的收盘时间
    pub fn get_close_price(&self, tp_type: ETradingPairType, date_time: &DateTime<Local>) -> RTradingPairManagerResult<Option<Decimal>> {
        match self.get_trading_pair(tp_type)?.get_kline(date_time) {
            None => {Ok(None)}
            Some(kline) => {Ok(Some(kline.close_price))}
        }
    }
}