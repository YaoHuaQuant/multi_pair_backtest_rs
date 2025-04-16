//! 等仓位占比策略
//! 策略明细：
//! 1. 从当前盘口向下挂买单，向上挂卖单，根据当前资产和仓位，调整挂单价格。
//! 2. 每当新一轮k线开始，取消所有旧挂单，重新计算新挂单的价格。
//! 3. 挂单遵循少量多次，挂单量以config中的基础资产的quantity为准
//! 4. 挂单价格策略：
//!     只在盘口价格±5%进行挂单
//!     只考虑短时单边上涨和单边下跌的场景：
//!         单边上涨时，每一笔订单卖出后，仓位均回到目标值。
//!         单边下跌时，每一笔订单买入后，仓位均回到目标值。
//!
//!
//! 策略逻辑顺序：
//! 1. 从runner获取order的执行情况，将成功执行的order进行记录。
//!
//! 2. 撤回所有剩余的订单
//!
//! 3. 根据当前盘口价计算挂单：
//! 3.1. 计算买单，根据当前价格，向上计算下一个挂单价位，生成新的订单。
//! 3.2 如果最新挂单的价格不高于盘口+5%，则循环计算2.1~2.2。
//! 3.3. 计算卖单，根据当前价格，向下计算下一个挂单价位，生成新的订单。
//! 3.4 如果最新挂单的价格不低于盘口-5%，则循环计算2.3~2.4。
//!
//! 4. 向runner发送撤单和订单请求
//!
//! 5. 根据runner反馈情况，将成功挂单的order进行记录。
//!

use std::collections::HashSet;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::order::trading_pair_order_manager_map::STradingPairOrderManagerMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::protocol::{ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult};
use crate::strategy::TStrategy;

pub struct SStrategyMk2 {
    pub target_position_ratio: Decimal,
    pub order_list: HashSet<Uuid>,
}

impl SStrategyMk2 {
    pub fn new(target_position_ratio: Decimal) -> Self {
        Self {
            target_position_ratio,
            order_list: Default::default(),
        }
    }

    pub fn default() -> Self {
        // 默认仓位50%
        Self::new(Decimal::from(0.5))
    }
}

impl TStrategy for SStrategyMk2 {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
    ) -> Vec<EStrategyAction> {
        // 1. 从runner获取order的执行情况，将成功执行的order进行记录。
        
        //  2. 撤回所有剩余的订单

        //  3. 根据当前盘口价计算挂单：

        //  3.1. 计算买单，根据当前价格，向上计算下一个挂单价位，生成新的订单。
        //  3.2 如果最新挂单的价格不高于盘口+5%，则循环计算2.1~2.2。
        //  3.3. 计算卖单，根据当前价格，向下计算下一个挂单价位，生成新的订单。
        //  3.4 如果最新挂单的价格不低于盘口-5%，则循环计算2.3~2.4。

        //  4. 向runner发送撤单和订单请求
        todo!()
    }

    fn verify(&mut self, tp_type: &ETradingPairType, parse_action_results: Vec<ERunnerSyncActionResult>) {
        // 5. 根据runner反馈情况，将成功挂单的order进行记录。
        todo!()
    }
}