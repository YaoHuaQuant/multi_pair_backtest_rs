use std::collections::BTreeMap;
use chrono::{DateTime, Utc}; // 用于处理时间

#[derive(Debug, Clone)]
struct DataPoint {
    price: f64,
}

#[derive(Debug)]
struct TimeSeries {
    data: BTreeMap<DateTime<Utc>, DataPoint>, // 按时间有序存储数据
}

impl TimeSeries {
    /// 创建一个新的时间序列数据结构
    fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// 插入新的数据点
    fn insert(&mut self, time: DateTime<Utc>, price: f64) {
        self.data.insert(time, DataPoint { price });
    }

    /// 获取特定时间点的数据
    fn get(&self, time: DateTime<Utc>) -> Option<&DataPoint> {
        self.data.get(&time)
    }

    /// 按时间顺序遍历所有数据
    fn iter(&self) -> impl Iterator<Item = (&DateTime<Utc>, &DataPoint)> {
        self.data.iter()
    }

    /// 获取时间范围内的数据
    fn range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> impl Iterator<Item = (&DateTime<Utc>, &DataPoint)> {
        self.data.range(start..=end)
    }
}

fn main() {
    let mut ts = TimeSeries::new();

    // 生成一些时间戳
    let now = Utc::now();
    let earlier = now - chrono::Duration::seconds(10);
    let later = now + chrono::Duration::seconds(10);

    // 插入数据
    ts.insert(earlier, 100.0);
    ts.insert(now, 105.5);
    ts.insert(later, 110.2);

    // 查询特定时间的数据
    if let Some(dp) = ts.get(now) {
        println!("Data at {}: {:?}", now, dp);
    }

    // 遍历所有数据
    println!("All data in order:");
    for (time, dp) in ts.iter() {
        println!("Time: {}, Price: {}", time, dp.price);
    }

    // 获取某个时间范围内的数据
    println!("Data between {} and {}:", earlier, later);
    for (time, dp) in ts.range(earlier, later) {
        println!("Time: {}, Price: {}", time, dp.price);
    }
}
