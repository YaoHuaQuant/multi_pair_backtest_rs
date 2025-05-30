//! 执行器执行脚本 程序主要入口

use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use chrono::{DateTime, Duration, Local};
use log::info;
use threadpool::ThreadPool;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use tokio::runtime::Runtime;
use crate::{
    config::*,
    data_runtime::user::{SUser, SUserConfig},
    data_source::{
        data_manager::SDataManager,
        db::api::data_api_db::SDataApiDb,
    },
    runner::{
        back_trade::{
            config::SBackTradeRunnerConfig,
            runner::SBackTradeRunner,
        },
        TRunner,
    },
    strategy::TStrategy,
};
use crate::config::back_trade_period::{config_date_from, config_date_to};
use crate::config::fee::{MAKER_ORDER_FEE, TAKER_ORDER_FEE};
use crate::config::user::INIT_BALANCE_USDT;
use crate::data_source::trading_pair::ETradingPairType;
use crate::runner::logger::data_logger::SDataLogger;
use crate::runner::{SRunnerResult, TRunnerGetPrice};

pub struct SScript<R, S>
where
    R: TRunner<S>,
    S: TStrategy,
{
    pub users: Vec<SUser<S>>,
    pub runner: R,
}

impl<R, S> SScript<R, S>
where
    R: TRunner<S>,
    S: TStrategy,
{
    pub fn new(users: Vec<SUser<S>>, runner: R) -> Self {
        Self {
            users,
            runner,
        }
    }

    pub fn run(&mut self, debug_config: SDebugConfig) -> SRunnerResult {
        self.runner.run(&mut self.users, debug_config)
    }
}

impl<S> SScript<SBackTradeRunner<SDataApiDb>, S>
where
    S: TStrategy + Default,
{
    /// 回测 单线程计算
    pub fn back_trader_single_thread_computing() {
        println!("启动单线程回测");
        let script_start_time = Local::now().format("%Y%m%d_%H%M%S");

        // 配置runner
        let date_from = config_date_from();
        let date_to = config_date_to() + Duration::minutes(1);
        let runner_config = SBackTradeRunnerConfig {
            taker_order_fee: Decimal::from_f64(TAKER_ORDER_FEE).unwrap(),
            maker_order_fee: Decimal::from_f64(MAKER_ORDER_FEE).unwrap(),
            date_from,
            date_to,
        };
        let rt = Runtime::new().unwrap();
        let data_manager = rt.block_on(SDataManager::build(&runner_config.date_from, &runner_config.date_to));
        let runner = SBackTradeRunner::new(runner_config, data_manager);

        // 配置 user
        let strategy = S::default();
        let position = strategy.get_position(date_from).unwrap();
        let price = runner.get_price(date_from, ETradingPairType::BtcUsdt).unwrap().close_price;
        let init_asset_total_usdt = Decimal::from_f64(INIT_BALANCE_USDT).unwrap();
        let init_balance_btc = init_asset_total_usdt * position / price;
        let init_balance_usdt = init_asset_total_usdt * (Decimal::from(1) - position);
        let user_config = SUserConfig {
            user_name: user::USER_NAME.to_string(),
            init_balance_usdt,
            init_balance_btc,
        };
        let users = vec![
            SUser::<S>::new(user_config, strategy)
        ];

        let debug_config = SDebugConfig::default();
        // let debug_config = SDebugConfig { is_debug: true, is_info: true };
        let result = SScript {
            users,
            runner,
        }.run(debug_config);
        result.data_logger.output_user(String::from(format!("data/back_trade/{}.csv", script_start_time)));
    }

    /// 回测 多线程计算
    pub fn back_trader_multi_thread_computing() {
        info!("启动多线程回测");
        let script_start_time = Local::now().format("%Y%m%d_%H%M%S");
        // let strategy = S::default();

        // 日期拆分
        let date_from = config_date_from();
        let date_to = config_date_to() + Duration::minutes(1);
        // let date_split_step = Duration::days(20);   // 每个分组的间隔时间（20天）

        // 自适应日期分区大小
        info!("回测日期：{:?} ~ {:?}", date_from, date_to);
        let date_split_step = (date_to - date_from) / 100;
        let date_split_step = if date_split_step < Duration::days(1) { Duration::days(1) } else { date_split_step };    // 分区大小不小于1天
        let date_split_step = if date_split_step > Duration::days(20) { Duration::days(20) } else { date_split_step };   // 分区大小不大于20天
        info!("多线程日期分区大小：{:?}", date_split_step);

        let mut date_split_vec = Vec::new();    // 存储每组的from和to
        let mut tmp_date = date_from;
        loop {
            if tmp_date + date_split_step >= date_to {
                date_split_vec.push((tmp_date, date_to - Duration::minutes(1)));
                break;
            }
            date_split_vec.push((tmp_date, tmp_date + date_split_step - Duration::minutes(1)));
            tmp_date += date_split_step;
        }

        // 并发任务所需的数据和参数
        let num_threads = num_cpus::get();
        let running_threads = num_threads - 4;
        info!("检测到 {} 个 CPU 核心，启动 {} 个线程进行处理。", num_threads, num_threads);
        let pool = ThreadPool::new(running_threads); // 限制同时运行的线程数为20
        let date_split_vec = Arc::new(date_split_vec);
        let (tx, rx) = mpsc::channel();
        let progress = Arc::new(Mutex::new(0)); // 记录任务已完成的分钟数
        let total_tasks = date_split_vec.len();  // 任务的总数
        info!("任务总数：{:?}", total_tasks);

        // 收集键值对到一个中间容器中
        let tasks: Vec<(DateTime<Local>, DateTime<Local>)> = date_split_vec
            .iter()
            .map(|(date_from, date_to)| (*date_from, *date_to))
            .collect();

        // 构造并发任务（Map）
        let mut count = 0;
        for (date_from, date_to) in tasks {
            count += 1;
            let tx = tx.clone();
            let progress = Arc::clone(&progress);
            let tmp_count = count;
            pool.execute(move || {
                info!("提交任务：N0.{:?}\tfrom-{:?}\tto-{:?}", tmp_count, date_from.clone(), date_to.clone());

                // 配置runner
                let runner_config = SBackTradeRunnerConfig {
                    taker_order_fee: Decimal::from_f64(TAKER_ORDER_FEE).unwrap(),
                    maker_order_fee: Decimal::from_f64(MAKER_ORDER_FEE).unwrap(),
                    date_from: date_from.clone(),
                    date_to: date_to.clone(),
                };
                let rt = Runtime::new().unwrap();
                let data_manager = rt.block_on(SDataManager::build(&runner_config.date_from, &runner_config.date_to));
                let runner = SBackTradeRunner::new(runner_config, data_manager);

                // 配置 user
                let strategy = S::default();
                let position = strategy.get_position(date_from).unwrap();
                let price = runner.get_price(date_from, ETradingPairType::BtcUsdt).unwrap().close_price;
                let init_asset_total_usdt = Decimal::from_f64(INIT_BALANCE_USDT).unwrap();
                let init_balance_btc = init_asset_total_usdt * position / price;
                let init_balance_usdt = init_asset_total_usdt * (Decimal::from(1) - position);
                let user_config = SUserConfig {
                    user_name: user::USER_NAME.to_string(),
                    init_balance_usdt,
                    init_balance_btc,
                };

                // 执行回测
                let result = SScript {
                    users: vec![
                        SUser::<S>::new(user_config, S::default())
                    ],
                    runner,
                }.run(SDebugConfig { is_debug: false, is_info: false });
                // 处理回测结果
                tx.send(result).unwrap();
                let mut prog = progress.lock().unwrap();
                *prog += 1;
            });
        }

        // 记录并发任务进度
        let progress_clone = Arc::clone(&progress);
        thread::spawn(move || loop {
            thread::sleep(std::time::Duration::from_secs(2));
            let prog = progress_clone.lock().unwrap();
            let percent = (*prog as f64 / total_tasks as f64) * 100.0;
            info!("当前进度：{:.2}%", percent);
            if *prog >= total_tasks {
                break;
            }
        });

        // 收集并发处理结果（Reduce）
        // 将结果merge为一个SDataLogger
        let mut results = SDataLogger::new();
        for _ in 0..total_tasks {
            let mut result = rx.recv().unwrap();
            // println!("result:{:?}", result);
            results.append(&mut result.data_logger);
        }

        info!("已完成！");

        // 将结果存储到文件中
        results.output_user(String::from(format!("data/back_trade/{}.csv", script_start_time)));
    }
}