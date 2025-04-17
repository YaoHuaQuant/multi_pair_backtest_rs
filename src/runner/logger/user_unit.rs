use std::collections::HashMap;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::data_runtime::asset::asset::SAsset;
use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::asset::EAssetType;
use crate::data_runtime::user::SUser;
use crate::data_source::trading_pair::ETradingPairType;
use crate::strategy::TStrategy;
use crate::utils::assets_denominate_usdt;

#[derive(Debug)]
pub struct SDataLogUserUnit {
    pub time: DateTime<Local>,
    pub user_id: Uuid,
    pub user_name: String,

    /// 交易对报价
    pub trading_pair_prices: HashMap<ETradingPairType, Decimal>,

    /// 总资产
    pub total_assets: SAssetMap,
    /// 总资产（Usdt计价）
    pub total_assets_usdt: Decimal,

    /// USDT资产数量
    pub total_usdt: Decimal,

    /// 可用资产
    pub available_assets: SAssetMap,
    /// 可用产（Usdt计价）
    pub available_assets_usdt: Decimal,

    /// 锁定资产
    pub locked_assets: SAssetMap,
    /// 锁定资产（Usdt计价）
    pub locked_assets_usdt: Decimal,

    /// 累计手续费
    pub total_fee: SAssetMap,

    /// 累计手续费（Usdt计价）
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
    ) -> Self {
        let total_usdt = user.total_asset().get(&EAssetType::Usdt)
            .unwrap_or(&SAsset { as_type: EAssetType::Usdt, balance: Decimal::from(0) })
            .balance;
        // ------
        Self {
            time,
            user_id: user.id,
            user_name: user.name.clone(),
            trading_pair_prices: trading_pair_prices.clone(),
            total_assets: user.total_asset(),
            total_assets_usdt: assets_denominate_usdt(&user.total_asset(), &trading_pair_prices),
            total_usdt,
            available_assets: user.available_assets.clone(),
            available_assets_usdt: assets_denominate_usdt(&user.available_assets, &trading_pair_prices),
            locked_assets: user.locked_assets(),
            locked_assets_usdt: assets_denominate_usdt(&user.locked_assets(), &trading_pair_prices),
            total_fee: user.total_fee(),
            total_fee_usdt: assets_denominate_usdt(&user.total_fee(), &trading_pair_prices),
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
            SStrategyMkTest::new(),
            String::from("test user"),
        );
        let mut trading_pair_prices: HashMap<ETradingPairType, Decimal> = HashMap::new();
        trading_pair_prices.insert(ETradingPairType::BtcUsdt, Decimal::from(10_000));

        let data = SDataLogUserUnit::new(
            time,
            &user,
            None,
            &trading_pair_prices,
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