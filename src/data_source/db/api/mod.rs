pub mod data_api_db;
pub mod data_api_csv;

use std::future::Future;
use chrono::{DateTime, Local};
use crate::data_source::db::api::data_api_db::SDataApiDb;
use crate::data_source::db::{RDBResult, SDbClickhouse};
use crate::data_source::funding_rate::SFundingRateData;
use crate::data_source::kline::SKlineData;

pub trait TDataApi {
    fn get_kline(&self, table_name: &str, from: &DateTime<Local>, to: &DateTime<Local>) -> impl Future<Output=RDBResult<SKlineData>> + Send;
    fn get_funding_rate(&self, table_name: &str, from: &DateTime<Local>, to: &DateTime<Local>) -> impl Future<Output=RDBResult<SFundingRateData>> + Send;
    fn default() -> Box<impl TDataApi> {
        Box::new(SDataApiDb::new(SDbClickhouse::new()))
    }
}