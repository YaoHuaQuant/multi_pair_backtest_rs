use std::collections::BTreeMap;

use chrono::{DateTime, Local, Timelike};
use rust_decimal::Decimal;
use crate::utils;

/// 资金费率数据
#[derive(Debug)]
pub struct SFundingRateUnitData {
    /// 开盘时间-带本地时区的DataTime
    pub time: DateTime<Local>,
    /// 资金费率-正值表示多头支付给空头，负值表示空头支付给多头。
    pub funding_rate: Decimal,
}

#[derive(Debug)]
pub struct SFundingRateData {
    pub data: BTreeMap<DateTime<Local>, SFundingRateUnitData>,
}

impl SFundingRateData {
    pub fn new() -> Self {
        SFundingRateData {
            data: Default::default(),
        }
    }

    /// 插入新的数据点
    pub fn insert(&mut self, time: &DateTime<Local>, funding_rate: Decimal) {
        let time = utils::date_time::normalize_to_minute(time);
        self.data.insert(
            time,
            SFundingRateUnitData {
                time,
                funding_rate,
            },
        );
    }

    /// 插入新的数据点
    pub fn insert_unit(&mut self, unit_data: SFundingRateUnitData) {
        self.data.insert(
            unit_data.time,
            unit_data,
        );
    }

    /// 获取特定时刻的资金费率
    pub fn get(&self, time: &DateTime<Local>) -> Option<&Decimal> {
        match self.data.get(&utils::date_time::normalize_to_minute(time)) {
            None => { None }
            Some(item) => { Some(&item.funding_rate) }
        }
    }

    /// 获取特定时间范围的资金费率
    pub fn range(&self, start: DateTime<Local>, end: DateTime<Local>) -> impl Iterator<Item=(&DateTime<Local>, &SFundingRateUnitData)> {
        self.data.range(start..=end)
    }

    /// 输出迭代器 用于遍历
    pub fn iter(&self) -> impl Iterator<Item=(&DateTime<Local>, &SFundingRateUnitData)> {
        self.data.iter()
    }
}


#[cfg(test)]
mod tests {
    use chrono::Local;
    use rand::prelude::SliceRandom;
    use rand::thread_rng;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use crate::data::funding_rate::{SFundingRateData, SFundingRateUnitData};
    use crate::utils;

    const DATA_NUM: i64 = 9;

    fn get_test_data() -> SFundingRateData {
        let mut data = SFundingRateData::new();


        // 插入数据
        let now = Local::now();
        let mut inputs = Vec::new();
        for offset in 0..DATA_NUM {
            let time = utils::date_time::normalize_to_minute(&(now - chrono::Duration::minutes(10 * DATA_NUM - offset * 10)));
            let funding_rate = Decimal::from_f64(0.1 + offset as f64 * 0.01).unwrap();
            inputs.push(
                SFundingRateUnitData {
                    time,
                    funding_rate,
                }
            )
        }

        // 打乱输入数据并插入
        let mut rng = thread_rng();
        inputs.shuffle(&mut rng);
        for item in inputs {
            data.insert(
                &item.time,
                item.funding_rate,
            );
        };

        data
    }
    #[test]
    pub fn test_iter() {
        let data = get_test_data();

        let mut results = Vec::new();
        for item in data.iter() {
            results.push(item.1.funding_rate);
            println!("{:?}", item)
        }

        for offset in 0..DATA_NUM {
            let index: usize = offset as usize;
            assert_eq!(results.get(index).unwrap().clone(), Decimal::from_f64(0.1 + offset as f64 * 0.01).unwrap());
        }
    }

    #[test]
    pub fn test_get() {
        let data = get_test_data();

        for item in data.iter() {
            println!("item: {:?}", &item);
            let time = item.0.clone();
            let funding_rate = item.1.funding_rate.clone();
            let get_funding_rate = data.get(&time).unwrap().clone();
            println!("get_funding_rate: {:?}", &get_funding_rate);
            assert_eq!(funding_rate, get_funding_rate)
        }
    }

    #[test]
    pub fn test_range() {
        let data = get_test_data();

        let from = DATA_NUM / 3;
        let to = DATA_NUM * 2 / 3;

        let mut data_vec = Vec::new();
        println!("total data:");
        for item in data.iter() {
            println!("item: {:?}", &item);
            data_vec.push(item.1);
        }

        let time_from = data_vec.get(from as usize).unwrap().time;
        let time_to = data_vec.get(to as usize).unwrap().time;

        let range = data.range(time_from, time_to);
        println!("range data from {} to {}:", time_from, time_to);
        let mut count = 0;
        for item in range {
            println!("item: {:?}", &item);
            assert_eq!(item.1.funding_rate, data_vec.get((from + count) as usize).unwrap().funding_rate);
            count += 1;
        }
    }
}