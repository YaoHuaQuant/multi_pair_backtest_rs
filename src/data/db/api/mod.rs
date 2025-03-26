pub mod data_api_db;
pub mod data_api_csv;

use chrono::{DateTime, Local};
use crate::data::db::RDBResult;
use crate::data::kline::SKlineData;

pub trait TDataApi {
    async fn get_kline(&self, from:DateTime<Local>, to:DateTime<Local>)-> RDBResult<SKlineData>;
    async fn get_funding_rate(&self, from:DateTime<Local>, to:DateTime<Local>)-> RDBResult<SKlineData>;
}