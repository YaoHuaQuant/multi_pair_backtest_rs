use std::collections::BTreeMap;

use chrono::{DateTime, Local};
use rust_decimal::Decimal;

/// K线单根数据
#[derive(Debug)]
pub struct SKlineUnitData {
    /// 开盘时间-带本地时区的DataTime
    pub open_time: DateTime<Local>,
    /// 收盘时间
    pub close_time: DateTime<Local>,
    /// 开盘价
    pub open_price: Decimal,
    /// 收盘价
    pub close_price: Decimal,
    /// 最高价
    pub high_price: Decimal,
    /// 最低价
    pub low_price: Decimal,
    /// 交易量
    pub volume: Decimal,
}

#[derive(Debug)]
pub struct SKlineData {
    pub data: BTreeMap<DateTime<Local>, SKlineUnitData>,
}

impl SKlineData {
    pub fn new() -> Self {
        SKlineData {
            data: Default::default(),
        }
    }
    /// 插入新的数据点
    pub fn insert(
        &mut self,
        open_time: DateTime<Local>,
        close_time: DateTime<Local>,
        open_price: Decimal,
        close_price: Decimal,
        high_price: Decimal,
        low_price: Decimal,
        volume: Decimal,
    ) {
        self.data.insert(
            open_time,
            SKlineUnitData {
                open_time,
                close_time,
                open_price,
                close_price,
                high_price,
                low_price,
                volume,
            },
        );
    }

    /// 插入新的数据点
    pub fn insert_unit(
        &mut self,
        unit_data: SKlineUnitData,
    ) {
        self.data.insert(
            unit_data.open_time,
            unit_data,
        );
    }

    /// 获取特定时刻的k线数据
    pub fn get(&self, time: DateTime<Local>) -> Option<&SKlineUnitData> {
        self.data.get(&time)
    }

    /// 获取特定时间范围的k线数据
    pub fn range(&self, start: DateTime<Local>, end: DateTime<Local>) -> impl Iterator<Item=(&DateTime<Local>, &SKlineUnitData)> {
        self.data.range(start..=end)
    }

    /// 输出迭代器 用于遍历
    pub fn iter(&self) -> impl Iterator<Item=(&DateTime<Local>, &SKlineUnitData)> {
        self.data.iter()
    }
}


#[cfg(test)]
mod tests {
    use std::ptr;
    use chrono::Local;
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    use rust_decimal::Decimal;

    use crate::data::kline::{SKlineData, SKlineUnitData};

    const DATA_NUM: i64 = 9;

    fn get_test_data() -> SKlineData {
        let mut data = SKlineData::new();

        // 插入数据
        let now = Local::now();
        let mut inputs = Vec::new();
        for offset in 0..DATA_NUM {
            /// 开盘时间-带本地时区的DataTime
            let open_time = now - chrono::Duration::seconds(10 * DATA_NUM - offset * 10);
            /// 收盘时间
            let close_time = now - chrono::Duration::seconds(10 * DATA_NUM - offset * 10 + 5);
            let open_price = Decimal::from(100 + offset * 10);
            let close_price = Decimal::from(105 + offset * 10);
            let high_price = Decimal::from(110 + offset * 10);
            let low_price = Decimal::from(95 + offset * 10);
            let volume = Decimal::from(offset * 2);
            inputs.push(
                SKlineUnitData {
                    open_time,
                    close_time,
                    open_price,
                    close_price,
                    high_price,
                    low_price,
                    volume,
                }
            )
        }

        // 打乱输入数据并插入
        let mut rng = thread_rng();
        inputs.shuffle(&mut rng);
        for item in inputs {
            data.insert(
                item.open_time,
                item.close_time,
                item.open_price,
                item.close_price,
                item.high_price,
                item.low_price,
                item.volume,
            );
        };
        data
    }
    #[test]
    pub fn test_iter() {
        let data = get_test_data();

        let mut results = Vec::new();
        for item in data.iter() {
            results.push(item.1);
            println!("{:?}", item)
        }

        for offset in 0..DATA_NUM {
            let index: usize = offset as usize;
            assert_eq!(results.get(index).unwrap().open_price, Decimal::from(100 + offset * 10));
            assert_eq!(results.get(index).unwrap().close_price, Decimal::from(105 + offset * 10));
            assert_eq!(results.get(index).unwrap().high_price, Decimal::from(110 + offset * 10));
            assert_eq!(results.get(index).unwrap().low_price, Decimal::from(95 + offset * 10));
            assert_eq!(results.get(index).unwrap().volume, Decimal::from(offset * 2));
        }
    }

    #[test]
    pub fn test_get() {
        let data = get_test_data();

        for item in data.iter() {
            println!("item: {:?}", &item);
            let time = item.0.clone();
            let kline = item.1;
            let get_kline = data.get(time).unwrap();
            println!("get_get_kline: {:?}", &get_kline);
            // 判断两个引用是否指向同一个对象
            assert_eq!(ptr::eq(kline, get_kline), true);
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

        let time_from = data_vec.get(from as usize).unwrap().open_time;
        let time_to = data_vec.get(to as usize).unwrap().open_time;

        let range = data.range(time_from, time_to);
        println!("range data from {} to {}:", time_from, time_to);
        let mut count = 0;
        for item in range {
            println!("item: {:?}", &item);
            let kline = data_vec.get((from + count) as usize).unwrap().clone();
            let range_kline = item.1;
            // 判断两个引用是否指向同一个对象
            assert_eq!(ptr::eq(kline, range_kline), true);
            count += 1;
        }
    }
}