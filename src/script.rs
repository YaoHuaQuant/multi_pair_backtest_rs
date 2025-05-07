//! 执行脚本 程序主要入口

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
    strategy::{
        mk_test::SStrategyMkTest,
        TStrategy,
    },
};
use crate::strategy::mk1::SStrategyMk1;
use crate::strategy::mk2::SStrategyMk2;
use crate::strategy::mk3::SStrategyMk3;
use crate::strategy::mk3_2::SStrategyMk3_2;
use crate::strategy::model::model_sin_test::SPriceModelSin;
use crate::strategy::model::model_step_test::SPriceModelStep;

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

    pub fn run(&mut self) {
        self.runner.run(&mut self.users);
    }
}

impl SScript<SBackTradeRunner<SDataApiDb>, SStrategyMkTest>
{
    pub fn default() -> Self {
        let user_config = SUserConfig {
            user_name: USER_NAME.to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        let strategy = SStrategyMkTest::new();
        let users = vec![
            SUser::<SStrategyMkTest>::new(user_config, strategy)
        ];

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

impl SScript<SBackTradeRunner<SDataApiDb>, SStrategyMk1>
{
    pub fn default() -> Self {
        let user_config = SUserConfig {
            user_name: USER_NAME.to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        // let strategy = SStrategyMk1::default();
        let strategy = SStrategyMk1::default();
        let users = vec![
            SUser::<SStrategyMk1>::new(user_config, strategy)
        ];

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

impl SScript<SBackTradeRunner<SDataApiDb>, SStrategyMk2>
{
    pub fn default() -> Self {
        let user_config = SUserConfig {
            user_name: USER_NAME.to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        // let strategy = SStrategyMk1::default();
        let strategy = SStrategyMk2::default();
        let users = vec![
            SUser::<SStrategyMk2>::new(user_config, strategy)
        ];

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

impl SScript<SBackTradeRunner<SDataApiDb>, SStrategyMk3<SPriceModelSin>>
{
    pub fn default() -> Self {
        let user_config = SUserConfig {
            user_name: USER_NAME.to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        // let strategy = SStrategyMk1::default();
        let strategy = SStrategyMk3::<SPriceModelSin>::default();
        let users = vec![
            SUser::<SStrategyMk3<SPriceModelSin>>::new(user_config, strategy)
        ];

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

impl SScript<SBackTradeRunner<SDataApiDb>, SStrategyMk3<SPriceModelStep>>
{
    pub fn default() -> Self {
        let user_config = SUserConfig {
            user_name: USER_NAME.to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        // let strategy = SStrategyMk1::default();
        let strategy = SStrategyMk3::<SPriceModelStep>::default();
        let users = vec![
            SUser::<SStrategyMk3<SPriceModelStep>>::new(user_config, strategy)
        ];

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

impl SScript<SBackTradeRunner<SDataApiDb>, SStrategyMk3_2<SPriceModelStep>>
{
    pub fn default() -> Self {
        let user_config = SUserConfig {
            user_name: USER_NAME.to_string(),
            init_balance_usdt: Decimal::from_f64(INIT_BALANCE_USDT).unwrap(),
            init_balance_btc: Decimal::from_f64(INIT_BALANCE_BTC).unwrap(),
        };
        // let strategy = SStrategyMk1::default();
        let strategy = SStrategyMk3_2::<SPriceModelStep>::default();
        let users = vec![
            SUser::<SStrategyMk3_2<SPriceModelStep>>::new(user_config, strategy)
        ];

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