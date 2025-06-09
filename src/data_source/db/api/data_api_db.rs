use std::fmt::{Debug, Formatter};
use chrono::{DateTime, Local};
use crate::data_source::db::api::TDataApi;
use crate::data_source::db::dao::kline_btc_usdt_1m_dao::SKlineBtcUSDT1mDao;
use crate::data_source::db::{RDBResult, SDbClickhouse};
use crate::data_source::funding_rate::SFundingRateData;
use crate::data_source::kline::SKlineData;

pub struct SDataApiDb {
    pub db: SDbClickhouse,
}

impl SDataApiDb {
    pub fn new(db: SDbClickhouse) -> Self {
        Self { db }
    }
}

impl Default for SDataApiDb {
    fn default() -> Self {
        Self::new(SDbClickhouse::new())
    }
}

impl TDataApi for SDataApiDb {
    async fn get_kline(&self, from: &DateTime<Local>, to: &DateTime<Local>) -> RDBResult<SKlineData> {
        let result: SKlineData = SKlineBtcUSDT1mDao::select_range(&self.db, from, to).await?.into();
        Ok(result)
    }

    async fn get_funding_rate(&self, _from: &DateTime<Local>, _to: &DateTime<Local>) -> RDBResult<SFundingRateData> {
        todo!()
    }
}

impl Debug for SDataApiDb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SDataApiDb")
    }
}


#[cfg(test)]
mod tests {
    use chrono::Local;
    use crate::data_source::db::api::data_api_db::SDataApiDb;
    use crate::data_source::db::api::TDataApi;
    use crate::data_source::db::SDbClickhouse;

    #[tokio::test]
    pub async fn test_kline() {
        let db = SDbClickhouse::new();
        let now = Local::now();
        let from = now - chrono::Duration::hours(24 * 200);
        let to = now - chrono::Duration::hours(24 * 200) + chrono::Duration::minutes(100);

        let api = SDataApiDb::new(db);

        let data = api.get_kline(&from, &to).await;

        match data {
            Ok(data) => {
                for (_, item) in data.iter() {
                    println!("{:?}", item)
                }
            }
            Err(e) => { println!("Error: {:?}", e) }
        }
    }
}