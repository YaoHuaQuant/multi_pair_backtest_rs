use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use clickhouse::Row;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::Deserialize;
use crate::data_source::db::{RDBResult, SDbClickhouse};
use crate::data_source::kline::{SKlineData, SKlineUnitData};

#[derive(Debug, Row, Deserialize)] // 使结构体支持 ClickHouse 读取
pub struct SKlineBtcUSDT1mDao {
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

impl SKlineBtcUSDT1mDao {
    pub async fn select_range(db: &SDbClickhouse, from: &DateTime<Local>, to: &DateTime<Local>) -> RDBResult<Vec<SKlineBtcUSDT1mDao>> {
        let client = db.get_client();
        // 2️⃣ 查询数据
        let query = format!("\
            SELECT \
                open_time, open_price, high_price, low_price, close_price, volume, close_time, quote_asset_volume, num_of_trades, taker_buy_base_volume, taker_buy_quote_volume \
            FROM kline_btc_usdt_1m \
            WHERE open_time BETWEEN '{}' AND '{}'\
        ", from.timestamp(), to.timestamp());

        // println!("query:{}", query);
        let data_vec: Vec<Self> = client.query(query.as_str()).fetch_all().await?;

        Ok(data_vec)
    }
}

impl Into<SKlineUnitData> for SKlineBtcUSDT1mDao {
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

impl Into<SKlineData> for Vec<SKlineBtcUSDT1mDao> {
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
    use chrono::Local;
    use crate::data_source::db::dao::kline_btc_usdt_1m_dao::SKlineBtcUSDT1mDao;
    use crate::data_source::db::{RDBResult, SDbClickhouse};
    use crate::data_source::kline::SKlineData;

    #[tokio::test]
    pub async fn test_select() {
        let db = SDbClickhouse::new();
        let now = Local::now();
        let from = now - chrono::Duration::hours(24 * 200);
        let to = now - chrono::Duration::hours(24 * 200) + chrono::Duration::minutes(100);
        let data = SKlineBtcUSDT1mDao::select_range(&db, &from, &to).await;

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
        let from = now - chrono::Duration::hours(24 * 200);
        let to = now - chrono::Duration::hours(24 * 200) + chrono::Duration::minutes(3);
        let data = SKlineBtcUSDT1mDao::select_range(&db, &from, &to).await?;

        let kline: SKlineData = data.into();
        println!("{:?}", kline);
        Ok(())
    }
}