use chrono::{Datelike, NaiveDateTime};
use num_cpus;
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};
use std::cmp::Ordering;
use rand::Rng;

#[derive(Debug, Clone)]
struct KLine {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

fn backtest(k_lines: &[KLine]) -> f64 {
    let sum: f64 = k_lines.iter().map(|k| k.close).sum();
    sum / k_lines.len() as f64
}

fn generate_mock_data() -> Vec<KLine> {
    let mut data = Vec::new();
    let start_time = NaiveDateTime::parse_from_str("2020-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
        .unwrap()
        .timestamp();
    let minutes = 4 * 365 * 24 * 60;
    for i in 0..minutes {
        let timestamp = start_time + i * 60;
        data.push(KLine {
            timestamp,
            open: 10000.0,
            high: 10500.0,
            low: 9500.0,
            close: 10000.0 + (i % 100) as f64,
        });
    }
    data
}

fn main() {
    let data = generate_mock_data();

    let mut monthly_data: HashMap<(i32, u32), Vec<KLine>> = HashMap::new();
    for k in data {
        let dt = NaiveDateTime::from_timestamp_opt(k.timestamp, 0).unwrap();
        let key = (dt.year(), dt.month());
        monthly_data.entry(key).or_default().push(k);
    }

    let num_threads = num_cpus::get();
    println!("检测到 {} 个 CPU 核心，启动 {} 个线程进行处理。", num_threads, num_threads);

    let monthly_data = Arc::new(monthly_data);
    let (tx, rx) = mpsc::channel();
    let progress = Arc::new(Mutex::new(0));
    let total_tasks = monthly_data.len();

    // 收集键值对到一个中间容器中
    let tasks: Vec<((i32, u32), Vec<KLine>)> = monthly_data
        .iter()
        .map(|(key, k_lines)| (*key, k_lines.clone()))
        .collect();

    for (key, k_lines) in tasks {
        let tx = tx.clone();
        let progress = Arc::clone(&progress);
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let delay_ms = rng.gen_range(1..=1000);
            thread::sleep(Duration::from_millis(delay_ms));
            let result = backtest(&k_lines);
            tx.send((key, result)).unwrap();
            let mut prog = progress.lock().unwrap();
            *prog += 1;
        });
    }

    let progress_clone = Arc::clone(&progress);
    thread::spawn(move || loop {
        // thread::sleep(Duration::from_secs(2));
        thread::sleep(Duration::from_millis(10));
        let prog = progress_clone.lock().unwrap();
        let percent = (*prog as f64 / total_tasks as f64) * 100.0;
        println!("当前进度：{:.2}%", percent);
        if *prog >= total_tasks {
            break;
        }
    });

    let mut results = Vec::new();
    for _ in 0..total_tasks {
        let (key, result) = rx.recv().unwrap();
        results.push((key, result));
    }

    results.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
    });
    for ((year, month), value) in results {
        println!("{}年{}月的回测结果：{:.2}", year, month, value);
    }

    println!("所有回测任务完成！");
}
