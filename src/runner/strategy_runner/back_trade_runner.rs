use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use crate::assert::asset::EAssetType;
use crate::assert::asset_manager::SAssetManager;
use crate::assert::trading_pair::ETradingPairType;
use crate::data::db::api::data_api_db::SDataApiDb;
use crate::data::db::api::TDataApi;
use crate::data::db::SDbClickhouse;
use crate::data::funding_rate::{SFundingRateData, SFundingRateUnitData};
use crate::data::kline::SKlineUnitData;
use crate::protocol::{EStrategyAction, SRunnerParseResult};
use crate::runner::strategy_runner::back_trade_config::{config_date_from, config_date_to, INIT_BALANCE_BTC, INIT_BALANCE_USDT, MAKER_ORDER_FEE, TAKER_ORDER_FEE};
use crate::runner::strategy_runner::order::runner_trading_pair_manager::STradingPairManager;
use crate::strategy::strategy_mk_test::SStrategyMkTest;
use crate::strategy::TStrategy;

/// 回测执行器
#[derive(Debug)]
pub struct SBackTradeRunner<A: TDataApi, S: TStrategy> {
    pub data_api: A,
    pub asset_manager: SAssetManager,
    pub trading_pair_manager: STradingPairManager,

    pub taker_order_fee: Decimal,
    pub maker_order_fee: Decimal,
    pub init_balance_usdt: Decimal,
    pub init_balance_btc: Decimal,
    pub date_from: DateTime<Local>,
    pub date_to: DateTime<Local>,

    pub strategy: S,
}

impl SBackTradeRunner<SDataApiDb, SStrategyMkTest> {
    pub async fn new() -> Self {
        let taker_order_fee = Decimal::from_f64(TAKER_ORDER_FEE).unwrap();
        let maker_order_fee = Decimal::from_f64(MAKER_ORDER_FEE).unwrap();
        let init_balance_usdt = Decimal::from_f64(INIT_BALANCE_USDT).unwrap();
        let init_balance_btc = Decimal::from_f64(INIT_BALANCE_BTC).unwrap();
        let date_from = config_date_from();
        let date_to = config_date_to();

        let db = SDbClickhouse::new();
        let data_api = SDataApiDb::new(db);
        let mut asset_manager = SAssetManager::new();
        // 插入asset配置
        asset_manager.add_assets(vec![EAssetType::Btc, EAssetType::Usdt, EAssetType::BtcUsdCmFuture, EAssetType::BtcUsdtFuture]);

        let mut trading_pair_manager = STradingPairManager::new();
        // 插入trading_pair配置 todo 插入更多配置
        let kline = data_api.get_kline(date_from, date_to).await.unwrap();
        let funding_rate: Option<SFundingRateData> = None; // todo 插入资金费率
        trading_pair_manager.add_trading_pair(ETradingPairType::BtcUsdt, kline, funding_rate);

        let strategy = SStrategyMkTest {};

        Self {
            data_api,
            asset_manager,
            trading_pair_manager,
            taker_order_fee,
            maker_order_fee,
            init_balance_usdt,
            init_balance_btc,
            date_from,
            date_to,
            strategy,
        }
    }

    pub fn run(&self) {
        // todo 循环遍历交易对
        for (tp_type, trading_pair) in self.trading_pair_manager.trading_pair_map {
            // todo 循环遍历k线
        }
        // todo 查询当前k线对应的资金费率
        // todo 根据K线 结算订单数据 结算资产数据
        // todo 将k线和订单结算结果反馈给strategy
        // todo 获取strategy的最新action 并更新订单
        todo!()
    }

    /// 处理新的k线和资金费率，更新订单和资产，记录增量处理结果。
    fn parse_kline(&self, kline_unit_data: SKlineUnitData, funding_rate_unit_data: Option<SFundingRateUnitData>) -> SRunnerParseResult {
        todo!()
    }

    /// 将增量数据传输给策略模块，获取策略行为。
    fn get_strategy_result(&self, runner_parse_result: SRunnerParseResult) -> Vec<EStrategyAction> {
        todo!()
    }

    /// 根据策略行为，调整订单数据。
    fn update_order_data(actions: Vec<EStrategyAction>) {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    use crate::data::db::api::data_api_db::SDataApiDb;
    use crate::runner::strategy_runner::back_trade_runner::SBackTradeRunner;
    use crate::strategy::strategy_mk_test::SStrategyMkTest;

    #[tokio::test]
    pub async fn test() {
        let runner = SBackTradeRunner::<SDataApiDb, SStrategyMkTest>::new().await;
        println!("{:?}", runner);
    }
}