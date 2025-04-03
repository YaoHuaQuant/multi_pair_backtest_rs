use dotenv::dotenv;
use log::{info, error, warn, debug, trace};
use multi_pair_backtest_rs::data_source::db::api::data_api_db::SDataApiDb;
use multi_pair_backtest_rs::runner::back_trade::runner::SBackTradeRunner;
use multi_pair_backtest_rs::strategy::strategy_mk_test::SStrategyMkTest;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::Builder::from_default_env().format_timestamp_micros().format_level(true).init();
    // info!("系统初始化完成");      // 绿色输出
    // debug!("详细调试信息: {}", 42); // 仅在DEBUG级别显示
    // warn!("这是一个警告");        // 黄色警告
    // error!("发生错误: {}", "数据异常"); // 红色错误

    // debug!("BackTrader初始化");
    // let mut runner = SBackTradeRunner::<SDataApiDb, SStrategyMkTest>::new().await;
    //
    // debug!("BackTrader运行");
    // runner.run().unwrap();
}

