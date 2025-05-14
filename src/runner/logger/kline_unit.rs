use std::collections::HashMap;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use crate::data_source::kline::SKlineUnitData;
use crate::data_source::trading_pair::ETradingPairType;

#[derive(Debug, Clone)]
pub struct SDataLogKlineUnit {
    pub time: DateTime<Local>,
    /// Btc现货价格
    pub price_btc_usdt: Option<Decimal>,
    /// U本位合约/Usdt 价格
    pub price_btc_usdt_future: Option<Decimal>,
    /// 币本位合约/Btc 价格
    pub price_btc_usd_cm_future: Option<Decimal>,
}

impl SDataLogKlineUnit {
    pub fn new(time: DateTime<Local>, data: HashMap<ETradingPairType, SKlineUnitData>) -> Self {
        let f = |tp_type: &ETradingPairType| {
            match data.get(tp_type) {
                None => { None }
                Some(x) => { Some(x.close_price) }
            }
        };
        Self {
            time,
            price_btc_usdt: f(&ETradingPairType::BtcUsdt),
            price_btc_usdt_future: f(&ETradingPairType::BtcUsdtFuture),
            price_btc_usd_cm_future: f(&ETradingPairType::BtcUsdCmFuture),
        }
    }
}