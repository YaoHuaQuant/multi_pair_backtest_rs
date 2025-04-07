//! 执行脚本 程序主要入口

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use tokio::runtime::Runtime;

use crate::{
    data_source::{
        db::api::data_api_db::SDataApiDb,
        data_manager::SDataManager
    },
    config::*,
    runner::{
        back_trade::{
            config::SBackTradeRunnerConfig,
            runner::SBackTradeRunner
        },
        TRunner
    },
    strategy::{
        strategy_mk_test::SStrategyMkTest,
        TStrategy
    },
    data_runtime::user::{SUser, SUserConfig}
};

pub struct SScript<R, S>
where
    R: TRunner<S>,
    S: TStrategy,
{
    pub users: Vec<SUser<S>>,
    pub runner: R,
}

impl <R, S> SScript<R, S>
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

    pub fn run(&mut self) {
        self.runner.run(&mut self.users);
    }
}

impl Default for SScript<SBackTradeRunner<SDataApiDb>, SStrategyMkTest>
{
    fn default() -> Self {
        let user_config = SUserConfig {
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        let strategy = SStrategyMkTest::new();
        let users = vec![SUser::<SStrategyMkTest>::new(user_config, strategy)];

        let runner_config = SBackTradeRunnerConfig {
            taker_order_fee: Decimal::from_f64(TAKER_ORDER_FEE).unwrap(),
            maker_order_fee: Decimal::from_f64(MAKER_ORDER_FEE).unwrap(),
            date_from: config_date_from(),
            date_to: config_date_to(),
        };

        let rt = Runtime::new().unwrap();

        let data_manager = rt.block_on(SDataManager::build(&runner_config.date_from, &runner_config.date_to));
        let runner = rt.block_on(SBackTradeRunner::new(runner_config, data_manager));
        SScript {
            users,
            runner,
        }
    }
}