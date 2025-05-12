use dotenv::dotenv;
use log::debug;
use multi_pair_backtest_rs::data_source::db::api::data_api_db::SDataApiDb;
use multi_pair_backtest_rs::runner::back_trade::runner::SBackTradeRunner;
use multi_pair_backtest_rs::script::SScript;
use multi_pair_backtest_rs::strategy::mk1::SStrategyMk1;
use multi_pair_backtest_rs::strategy::mk2::SStrategyMk2;
use multi_pair_backtest_rs::strategy::mk3::SStrategyMk3;
use multi_pair_backtest_rs::strategy::mk3_2::SStrategyMk3_2;
use multi_pair_backtest_rs::strategy::model::price_model_sin_test::SPriceModelSin;
use multi_pair_backtest_rs::strategy::model::price_model_step_test::SPriceModelStep;

fn main() {
    dotenv().ok();
    env_logger::Builder::from_default_env().format_timestamp_micros().format_level(true).init();
    // info!("系统初始化完成");      // 绿色输出
    // debug!("详细调试信息: {}", 42); // 仅在DEBUG级别显示
    // warn!("这是一个警告");        // 黄色警告
    // error!("发生错误: {}", "数据异常"); // 红色错误

    debug!("Scrypt初始化");
    // let mut scrypt = SScript::<SBackTradeRunner<SDataApiDb>, SStrategyMk3<SPriceModelSin>>::default();
    let mut scrypt = SScript::<SBackTradeRunner<SDataApiDb>, SStrategyMk3_2<SPriceModelSin>>::default();

    debug!("Scrypt运行");
    scrypt.run();
}

