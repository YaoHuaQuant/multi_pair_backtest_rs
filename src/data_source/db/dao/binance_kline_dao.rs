use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use clickhouse::Row;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::Deserialize;
use crate::data_source::db::{RDBResult, SDbClickhouse};
use crate::data_source::kline::{SKlineData, SKlineUnitData};

/// 从Binance获取的K线数据结构类型
/// 包括现货K线和合约K线
#[derive(Debug, Row, Deserialize)] // 使结构体支持 ClickHouse 读取
pub struct SBinanceKlineDao {
    pub open_time: i32,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub volume: f64,
    pub close_time: i32,
    pub quote_asset_volume: f64,
    pub num_of_trades: f64,
    pub taker_buy_base_volume: f64,
    pub taker_buy_quote_volume: f64,
}

pub mod tables {
    pub static BTC_USDT_1M_TABLE_NAME:&str = "kline_btc_usdt_1m";
    pub static BTC_MARGINED_FUTURE_BTC_1M_TABLE_NAME:&str = "kline_btc_margined_future_btc_1m";
}

impl SBinanceKlineDao {
    pub async fn select_range(table_name:&str, db: &SDbClickhouse, from: &DateTime<Local>, to: &DateTime<Local>) -> RDBResult<Vec<SBinanceKlineDao>> {
        let client = db.get_client();
        // 2️⃣ 查询数据
        let query = format!("\
            SELECT \
                open_time, open_price, high_price, low_price, close_price, volume, close_time, quote_asset_volume, num_of_trades, taker_buy_base_volume, taker_buy_quote_volume \
            FROM '{}' \
            WHERE open_time BETWEEN '{}' AND '{}'\
        ", table_name, from.timestamp(), to.timestamp());

        // println!("query:{}", query);
        let data_vec: Vec<Self> = client.query(query.as_str()).fetch_all().await?;

        Ok(data_vec)
    }
}

impl Into<SKlineUnitData> for SBinanceKlineDao {
    fn into(self) -> SKlineUnitData {
        SKlineUnitData {
            open_time: Local.from_utc_datetime(&NaiveDateTime::from_timestamp(self.open_time as i64, 0)),
            close_time: Local.from_utc_datetime(&NaiveDateTime::from_timestamp(self.close_time as i64, 0)),
            open_price: Decimal::from_f64(self.open_price).unwrap(),
            close_price: Decimal::from_f64(self.close_price).unwrap(),
            high_price: Decimal::from_f64(self.high_price).unwrap(),
            low_price: Decimal::from_f64(self.low_price).unwrap(),
            volume: Decimal::from_f64(self.volume).unwrap(),
        }
    }
}

impl Into<SKlineData> for Vec<SBinanceKlineDao> {
    fn into(self) -> SKlineData {
        let mut result = SKlineData::new();
        for item in self {
            let unit_data: SKlineUnitData = item.into();
            result.insert_unit(unit_data);
        }
        result
    }
}


#[cfg(test)]
mod tests {
    use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    use crate::data_source::db::dao::binance_kline_dao::SBinanceKlineDao;
    use crate::data_source::db::{RDBResult, SDbClickhouse};
    use crate::data_source::db::dao::binance_kline_dao::tables::BTC_MARGINED_FUTURE_BTC_1M_TABLE_NAME;
    use crate::data_source::kline::SKlineData;

    #[tokio::test]
    pub async fn test_select() {
        let db = SDbClickhouse::new();
        let now = Local::now();
        let from = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2020, 1, 27).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间");
        let to = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2020, 2, 27).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间");
        let table_name = BTC_MARGINED_FUTURE_BTC_1M_TABLE_NAME;
        let data = SBinanceKlineDao::select_range(table_name, &db, &from, &to).await;

        match data {
            Ok(data) => {
                for item in data {
                    println!("{:?}", item)
                }
            }
            Err(e) => { println!("Error: {:?}", e) }
        }
    }

    #[tokio::test]
    pub async fn test_into() -> RDBResult<()> {
        let db = SDbClickhouse::new();
        let now = Local::now();
        let from = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2020, 1, 27).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间");
        let to = Local.from_local_datetime(&NaiveDateTime::new(NaiveDate::from_ymd_opt(2020, 2, 27).expect("无效的日期"), NaiveTime::from_hms_opt(0, 0, 0).expect("无效的时间"))).single().expect("无法转换为本地时间");
        let table_name = BTC_MARGINED_FUTURE_BTC_1M_TABLE_NAME;
        let data = SBinanceKlineDao::select_range(table_name, &db, &from, &to).await?;

        let kline: SKlineData = data.into();
        println!("{:?}", kline);
        Ok(())
    }
}