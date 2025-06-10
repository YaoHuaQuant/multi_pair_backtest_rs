use std::collections::HashMap;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::asset::asset_map_v3::SAssetMapV3;
use crate::data_runtime::asset::asset_union::EAssetUnion;
use crate::data_runtime::asset::EAssetType;
use crate::data_runtime::user::SUser;
use crate::data_source::trading_pair::ETradingPairType;
use crate::runner::logger::order_unit::SDataLogOrderUnit;
use crate::runner::logger::transfer_unit::SDataLogTransferUnit;
use crate::strategy::TStrategy;
use crate::utils::{assets_map_denominate_usdt, assets_map_denominate_usdt_old};

#[derive(Debug, Clone)]
pub struct SDataLogUserUnit {
    pub time: DateTime<Local>,
    pub user_id: Uuid,
    pub user_name: String,

    /// 交易对报价
    pub trading_pair_prices: HashMap<ETradingPairType, Decimal>,

    // -----交易信息-----
    pub transfer_info: SDataLogTransferUnit,

    // -----挂单信息-----
    pub order_info: SDataLogOrderUnit,

    // -----资产信息-----
    /// 总资产
    pub total_assets: SAssetMapV3,
    /// 总资产（Usdt计价）
    pub total_assets_usdt: Decimal,

    /// USDT资产数量
    pub total_usdt: Decimal,

    /// 可用资产
    pub available_assets: SAssetMapV3,
    /// 可用产（Usdt计价）
    pub available_assets_usdt: Decimal,

    /// 锁定资产
    pub locked_assets: SAssetMapV3,
    /// 锁定资产（Usdt计价）
    pub locked_assets_usdt: Decimal,

    /// 累计手续费
    pub total_fee: SAssetMap,

    /// 累计手续费（Usdt计价 以当前时刻的价格计价）
    pub total_fee_usdt: Decimal,

    /// 目标仓位
    pub target_position_ratio: Option<Decimal>,
}

impl SDataLogUserUnit {
    pub fn new<S: TStrategy>(
        time: DateTime<Local>,
        user: &SUser<S>,
        target_position_ratio: Option<Decimal>,
        trading_pair_prices: &HashMap<ETradingPairType, Decimal>,
        transfer_info: &SDataLogTransferUnit,
    ) -> Self {
        let total_usdt = user.total_asset().get(&EAssetType::Usdt)
            .unwrap_or(&EAssetUnion::from(SAsset { as_type: EAssetType::Usdt, balance: Decimal::from(0) }))
            .get_balance();

        let order_manager = user.tp_order_map.get(&ETradingPairType::BtcUsdt).unwrap();

        let btc_usdt_highest_buy_price = match order_manager.peek_highest_buy_order().unwrap() {
            None => { None }
            Some(order) => { Some(order.get_price()) }
        };
        let btc_usdt_lowest_sell_price = match order_manager.peek_lowest_sell_order().unwrap() {
            None => { None }
            Some(order) => { Some(order.get_price()) }
        };

        let order_info = SDataLogOrderUnit {
            btc_usdt_highest_buy_price,
            btc_usdt_lowest_sell_price,
        };
        // ------
        Self {
            time,
            user_id: user.id,
            user_name: user.name.clone(),
            trading_pair_prices: trading_pair_prices.clone(),
            transfer_info: transfer_info.clone(),
            order_info,
            total_assets: user.total_asset(),
            total_assets_usdt: assets_map_denominate_usdt(&user.total_asset(), &trading_pair_prices),
            total_usdt,
            available_assets: user.available_assets.clone(),
            available_assets_usdt: assets_map_denominate_usdt(&user.available_assets, &trading_pair_prices),
            locked_assets: user.locked_assets(),
            locked_assets_usdt: assets_map_denominate_usdt(&user.locked_assets(), &trading_pair_prices),
            total_fee: user.total_fee(),
            total_fee_usdt: assets_map_denominate_usdt_old(&user.total_fee(), &trading_pair_prices), // todo 将old函数改为new
            target_position_ratio,
        }
    }

    /// 获取实际仓位
    pub fn get_actual_position_ratio(&self) -> Decimal {
        if self.total_assets_usdt != Decimal::from(0) {
            Decimal::from(1) - self.total_usdt / self.total_assets_usdt
        } else {
            Decimal::from(0)
        }
    }

    /// 获取目标仓位
    pub fn get_target_position_ratio(&self) -> Option<Decimal> {
        self.target_position_ratio
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use chrono::Local;
    use rust_decimal::Decimal;
    use crate::data_runtime::user::{SUser, SUserConfig};
    use crate::data_source::trading_pair::ETradingPairType;
    use crate::runner::logger::transfer_unit::SDataLogTransferUnit;
    use crate::runner::logger::user_unit::SDataLogUserUnit;
    use crate::strategy::mk_test::SStrategyMkTest;

    fn get_test_data() -> SDataLogUserUnit {
        let time = Local::now();
        let user = SUser::new(
            SUserConfig {
                user_name: "test user".to_string(),
                init_balance_usdt: Decimal::from(10_000),
                init_balance_btc: Decimal::from(1),
            },
            SStrategyMkTest::default(),
        );
        let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();
        trading_pair_prices.insert(ETradingPairType::BtcUsdt, Decimal::from(10_000));

        let data = SDataLogUserUnit::new(
            time,
            &user,
            None,
            &trading_pair_prices,
            &SDataLogTransferUnit::default(),
        );
        data
    }
    #[test]
    pub fn test1() {
        let data = get_test_data();
        dbg!(&data);
        dbg!(&data.get_actual_position_ratio());
    }
}