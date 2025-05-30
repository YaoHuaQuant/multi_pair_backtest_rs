//! StrategyMk4
//! 在Mk4的基础上
//! 1）引入长周期趋势模型（价格模型+仓位模型）
//! 2）将“只做多”策略改为“多空”策略
//!
//! 多空策略：
//! 1）挂买单
//!     优先平空仓
//!     其次加多仓（需控制开仓资金占比）
//! 2）挂卖单
//!     优先平多仓
//!     其次加空仓（需控制开仓资金占比）


use std::cmp::{max, min};
use std::collections::HashSet;
use chrono::{DateTime, Local};
use log::{debug, error};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;

use crate::{
    data_runtime::{
        asset::{
            asset::SAsset,
            asset_map::SAssetMap,
        },
        order::{
            EOrderAction,
            trading_pair_order_manager_map::STradingPairOrderManagerMap,
        },
    },
    data_source::trading_pair::ETradingPairType,
    protocol::{ERunnerParseOrderResult, ERunnerSyncActionResult, EStrategyAction, SRunnerParseKlineResult, SStrategyOrderAdd},
    strategy::{
        TStrategy,
    },
};
use crate::config::{SDebugConfig};
use crate::config::fee::MAKER_ORDER_FEE;
use crate::config::trading_pair::btc_usdt::TRADDING_PAIR_USDT_MIN_QUANTITY;
use crate::config::user::INIT_BALANCE_USDT;
use crate::data_runtime::order::EOrderDirection;
use crate::strategy::logger::SStrategyLogger;
use crate::strategy::model::feedback_control::{SPidIntegral, SStrategyPidConfig};
use crate::strategy::model::position_model::SPositionModel;
use crate::strategy::model::price_model_long_term_trend::SPriceModelLongTermTrend;
use crate::strategy::model::TPriceModel;
use crate::strategy::order::order::{EStrategyOrderState, SStrategyOrder};
use crate::strategy::order::order_manager_v2::SStrategyOrderManagerV2;
use crate::strategy::order::trading_pair_order_map_v2::SStrategyTradingPairOrderMapV2;

/// 订单管理器异常
#[derive(Debug)]
pub enum EStrategyMk4Error {
    /// 缺少已开仓订单 无法生成平仓单
    LackOpenedOrderError,
}

pub struct SStrategyMk4<M: TPriceModel> {
    /// 数据日志
    pub logger: SStrategyLogger,
    /// 仓位模型
    pub model: SPositionModel<M>,
    /// 记录所有已挂单未成交的订单
    pub opening_and_closing_orders: HashSet<Uuid>,
    /// 策略订单管理器
    pub strategy_order_map: SStrategyTradingPairOrderMapV2,
    /// 当订单价格与盘口价格相差一定百分比时，需要停止挂单。
    pub cut_off_price_percentage: Decimal,
    /// 最低盈利百分比（不包括手续费）
    pub minimum_profit_percentage: Decimal,
    /// 最高盈利百分比（不包括手续费）
    pub max_profit_percentage: Decimal,
    /// 平仓价最小间距（相比于开仓价的百分比）
    pub close_price_step_percentage: Decimal,
    /// open订单固定下单量百分比（close订单的下单量与open订单一致）
    pub const_open_quantity_percentage: Decimal,
    /// 最小订单价格间隙百分比
    pub const_delta_price_min_percentage: Decimal,
    /// 挂单手续费
    pub maker_order_fee_percentage: Decimal,
    /// pid参数配置
    pub pid_config: SStrategyPidConfig,
    /// 死区大小 取值范围(0, 1)
    pub dead_zone_range_percentage: Decimal,
    /// 活区大小 取值范围(0, 1)
    pub live_zone_range_percentage: Decimal,
}

/// 打包get_next_order_with_static_position的返回值
struct SNextOrderFormat {
    pub id: Option<Uuid>,
    pub price: Decimal,
    pub quantity: Decimal,
    pub base_quantity: Decimal,
    pub quote_quantity: Decimal,
}

impl Default for SStrategyMk4<SPriceModelLongTermTrend> {
    fn default() -> Self {
        // 构建长周期趋势模型
        let position_max = 0.95;
        let position_min = 0.05;
        let price_model = SPriceModelLongTermTrend::default();

        // 只在盘口价的±2.0%挂单
        let cut_off_price_percentage = 0.02;
        // 每单的最小盈利0.06%
        let minimum_profit_percentage = 0.0006;
        // 每单的最大盈利5.0%
        let max_profit_percentage = 0.05;
        // 平仓价最小间距为开仓价格的0.1%
        let close_price_step_percentage = 0.001;
        let maker_order_fee_percentage = MAKER_ORDER_FEE;
        let open_quantity_percentage = TRADDING_PAIR_USDT_MIN_QUANTITY;
        // 订单最小价格间距
        let delta_price_min_percentage = TRADDING_PAIR_USDT_MIN_QUANTITY / INIT_BALANCE_USDT;
        // 死区大小（等于开单与平单的价差最小值）
        let dead_zone_range_percentage = maker_order_fee_percentage * 2.0 + minimum_profit_percentage;
        // 活区大小
        let live_zone_range_percentage = dead_zone_range_percentage;
        // pid参数
        // let pid_p_parameter = 0.005; // pid比例项参数
        let pid_p_parameter = 1.0; // pid比例项参数（1.0代表没有比例项）
        // let pid_i_parameter = 0.025;  // pid积分项参数
        let pid_i_parameter = 0.0;  // pid积分项参数(0.0代表没有积分项)
        let pid_i_max_cumulative = 1.8; // pid积分项累计值最大值
        Self::new(
            price_model,
            Decimal::from_f64(cut_off_price_percentage).unwrap(),
            Decimal::from_f64(minimum_profit_percentage).unwrap(),
            Decimal::from_f64(max_profit_percentage).unwrap(),
            Decimal::from_f64(close_price_step_percentage).unwrap(),
            Decimal::from_f64(open_quantity_percentage).unwrap(),
            Decimal::from_f64(delta_price_min_percentage).unwrap(),
            Decimal::from_f64(maker_order_fee_percentage).unwrap(),
            SStrategyPidConfig {
                proportional: Decimal::from_f64(pid_p_parameter).unwrap(),
                integral: Some(SPidIntegral::new(
                    Decimal::from_f64(pid_i_parameter).unwrap(), // 积分项参数
                    Decimal::from_f64(pid_i_max_cumulative).unwrap(),
                )),
                derivative: None,
            },
            Decimal::from_f64(dead_zone_range_percentage).unwrap(),
            Decimal::from_f64(live_zone_range_percentage).unwrap(),
            position_max,
            position_min,
        )
    }
}

// impl Default for SStrategyMk4<SPriceModelSin> {
//     fn default() -> Self {
//         // 构建正弦波周期性价格模型
//         // 周期2天
//         // let period = 60 * 60 * 24 * 2;
//         // 周期2小时
//         let period = 60 * 60 * 2;
//         // 振幅10%
//         let amplitude = Decimal::from_f64(0.1 / 2.0).unwrap();
//         // 原点
//         let origin = Local.ymd(2025, 1, 1).and_hms(0, 0, 0);
//         // 期望均值仓位10%
//         let mean = Decimal::from_f64(0.1).unwrap();
//         let price_model = SPriceModelSin::new(period, amplitude, origin, mean);
//
//         // 只在盘口价的±2.0%挂单
//         let cut_off_price_percentage = 0.02;
//         // 每单的最小盈利0.06%
//         let minimum_profit_percentage = 0.0006;
//         // 每单的最大盈利5.0%
//         let max_profit_percentage = 0.05;
//         // 平仓价最小间距为开仓价格的0.1%
//         let close_price_step_percentage = 0.001;
//         let maker_order_fee_percentage = MAKER_ORDER_FEE;
//         let open_quantity_percentage = TRADDING_PAIR_USDT_MIN_QUANTITY;
//         // 订单最小价格间距
//         let delta_price_min_percentage = TRADDING_PAIR_USDT_MIN_QUANTITY / INIT_BALANCE_USDT;
//         // 死区大小（等于开单与平单的价差最小值）
//         let dead_zone_range_percentage = maker_order_fee_percentage * 2.0 + minimum_profit_percentage;
//         // 活区大小
//         let live_zone_range_percentage = dead_zone_range_percentage;
//         // pid参数
//         // let pid_p_parameter = 0.005; // pid比例项参数
//         let pid_p_parameter = 1.0; // pid比例项参数（1.0代表没有比例项）
//         // let pid_i_parameter = 0.00025;  // pid积分项参数
//         let pid_i_parameter = 0.0;  // pid积分项参数(0.0代表没有积分项)
//         let pid_i_max_cumulative = 1.8; // pid积分项累计值最大值
//         Self::new(
//             price_model,
//             Decimal::from_f64(cut_off_price_percentage).unwrap(),
//             Decimal::from_f64(minimum_profit_percentage).unwrap(),
//             Decimal::from_f64(max_profit_percentage).unwrap(),
//             Decimal::from_f64(close_price_step_percentage).unwrap(),
//             Decimal::from_f64(open_quantity_percentage).unwrap(),
//             Decimal::from_f64(delta_price_min_percentage).unwrap(),
//             Decimal::from_f64(maker_order_fee_percentage).unwrap(),
//             SStrategyPidConfig {
//                 proportional: Decimal::from_f64(pid_p_parameter).unwrap(),
//                 integral: Some(SPidIntegral::new(
//                     Decimal::from_f64(pid_i_parameter).unwrap(), // 积分项参数
//                     Decimal::from_f64(pid_i_max_cumulative).unwrap(),
//                 )),
//                 derivative: None,
//             },
//             Decimal::from_f64(dead_zone_range_percentage).unwrap(),
//             Decimal::from_f64(live_zone_range_percentage).unwrap(),
//         )
//     }
// }

impl<M: TPriceModel> SStrategyMk4<M> {
    pub fn new(
        price_model: M,
        cut_off_price_percentage: Decimal,
        minimum_profit_percentage: Decimal,
        max_profit_percentage: Decimal,
        close_price_step_percentage: Decimal,
        const_open_quantity_percentage: Decimal,
        const_delta_price_min_percentage: Decimal,
        maker_order_fee_percentage: Decimal,
        pid_config: SStrategyPidConfig,
        dead_zone_range_percentage: Decimal,
        live_zone_range_percentage: Decimal,
        position_max: f64,
        position_min: f64,
    ) -> Self
    {
        let model = SPositionModel::from(price_model, position_max, position_min);
        let mut strategy_order_map = SStrategyTradingPairOrderMapV2::default();
        let manager = SStrategyOrderManagerV2::from(
            minimum_profit_percentage,
            max_profit_percentage,
            close_price_step_percentage,
        );
        strategy_order_map.inner.insert(ETradingPairType::BtcUsdt, manager.clone());
        strategy_order_map.inner.insert(ETradingPairType::BtcUsdtFuture, manager.clone());
        strategy_order_map.inner.insert(ETradingPairType::BtcUsdCmFuture, manager.clone());
        Self {
            logger: SStrategyLogger { target_position_ratio: Decimal::from(0) },
            model,
            opening_and_closing_orders: Default::default(),
            strategy_order_map,
            cut_off_price_percentage,
            minimum_profit_percentage,
            max_profit_percentage,
            close_price_step_percentage,
            const_open_quantity_percentage,
            const_delta_price_min_percentage,
            maker_order_fee_percentage,
            pid_config,
            dead_zone_range_percentage,
            live_zone_range_percentage,
        }
    }

    /// 以给定仓位为目标
    /// 计算下一个订单的价格
    /// 以及下一个订单成交后的状态
    fn get_next_order_with_given_position(
        &self,
        direction: EOrderDirection,
        action: EOrderAction,
        target_position_ratio: Decimal,
        price: Decimal, // 如果上一个挂单被执行时的价格
        base_quantity: Decimal, // 如果上一个挂单被执行时的基础资产量
        quote_quantity: Decimal, // 如果上一个挂单被执行时的计价资产量
        opened_strategy_order: Option<&SStrategyOrder>, // 平仓单对应的strategy order
    ) -> Option<SNextOrderFormat>
    {
        // 实际仓位占比
        let position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
        // open订单固定下单量（close订单的下单量与open订单一致）
        let const_open_quantity: Decimal = self.const_open_quantity_percentage / price;
        // 最小订单价格间隙
        let const_delta_price_min = self.const_delta_price_min_percentage * price;

        {
            // debug!("Mk4 get_next_order_with_given_position:{:?}\t仓位:{:.2?}%\t当前价格:{:?}\tbase:{:?}\tquote:{:?}\tquantity:{:?}",action, position_ratio*Decimal::from(100), price, base_quantity, quote_quantity,const_open_quantity);
            // debug!("Mk4::get_next_order_with_given_position: target_position_ratio:\t{:.4?}%", target_position_ratio * Decimal::from(100));
        }

        let order_quantity = match action {
            EOrderAction::Buy => {
                const_open_quantity
            }
            EOrderAction::Sell => {
                -const_open_quantity
            }
        };
        // 计算订单价格（不考虑策略订单对的情况）
        let tmp_order_price =
            if position_ratio == target_position_ratio {
                // 实际仓位等于目标仓位 均匀挂单
                base_quantity * quote_quantity * price / (
                    base_quantity * quote_quantity
                        + base_quantity * order_quantity * price
                        + order_quantity * quote_quantity
                )
            } else {
                match (action, position_ratio > target_position_ratio) {
                    (EOrderAction::Buy, true) | (EOrderAction::Sell, false) => {
                        // 等待策略
                        // 做多 买单 实际仓位大于目标仓位：等待下跌 降低仓位
                        // 做多 卖单 实际仓位小于目标仓位：等待上涨 提升仓位
                        // 做空 买单 实际仓位大于目标仓位：等待上涨 降低仓位
                        // 做空 卖单 实际仓位小于目标仓位：等待下跌 提升仓位 不进行挂单
                        { // debug
                            // debug!("get_next_order_with_given_position: action:{:?}\tposition_ratio:\t{:.4?}%\ttarget_position_ratio:\t{:.4?}%", action, position_ratio*Decimal::from(100), target_position_ratio*Decimal::from(100));
                            // debug!("get_next_order_with_given_position: tmp_order_price:\t{:.4?}", target_position_ratio * quote_quantity / (base_quantity + order_quantity - target_position_ratio * base_quantity));
                            // debug!("get_next_order_with_given_position: target_position_ratio:\t{:?}\tbase_quantity:{:?}\tquote_quantity:{:?}\torder_quantity:{:?}", 
                            //     target_position_ratio,
                            //     base_quantity,
                            //     quote_quantity,
                            //     order_quantity
                            // );
                        }
                        if target_position_ratio < Decimal::from(0) && action == EOrderAction::Sell && direction == EOrderDirection::Short {
                            // 做空 卖单 实际仓位小于目标仓位：等待下跌 不进行挂单(挂一个极高价位的单)
                            price * Decimal::from(2)
                        } else {
                            target_position_ratio * quote_quantity / (base_quantity + order_quantity - target_position_ratio * base_quantity)
                        }
                    }
                    (EOrderAction::Sell, true) | (EOrderAction::Buy, false) => {
                        // 主动买卖策略
                        // 做多 卖单 实际仓位大于目标仓位：上涨减仓 降低仓位
                        // 做多 买单 实际仓位小于目标仓位：下跌加仓 提升仓位
                        // 做空 卖单 实际仓位大于目标仓位：上涨加仓 降低仓位
                        // 做空 买单 实际仓位小于目标仓位：下跌减仓 提升仓位

                        // 选择可挂单价格范围内 距离盘口最近的价格
                        price + match action {
                            EOrderAction::Buy => { -const_delta_price_min }
                            EOrderAction::Sell => { const_delta_price_min }
                        }
                    }
                }
            };

        let order_price = match action {
            EOrderAction::Buy => { min(price - const_delta_price_min, tmp_order_price) }
            EOrderAction::Sell => { max(price + const_delta_price_min, tmp_order_price) }
        };

        { // debug
            // debug!("get_next_order_with_given_position: order_price:\t{:.4?}", order_price)
        }

        let order_quantity = match opened_strategy_order {
            None => { const_open_quantity }
            Some(strategy_order) => { strategy_order.get_quantity() }
        };

        let id = match opened_strategy_order {
            None => { None }
            Some(strategy_order) => { Some(strategy_order.get_id()) }
        };

        // 重新计算仓位、资产
        let new_base_quantity = base_quantity + match action {
            EOrderAction::Buy => { order_quantity }
            EOrderAction::Sell => { -order_quantity }
        };
        let new_quote_quantity = quote_quantity + match action {
            EOrderAction::Buy => { -order_quantity * price }
            EOrderAction::Sell => { order_quantity * price }
        };

        Some(SNextOrderFormat {
            id,
            price: order_price,
            quantity: order_quantity,
            base_quantity: new_base_quantity,
            quote_quantity: new_quote_quantity,
        })
    }

    /// 根据静态目标仓位和实际仓位
    /// 获取动态目标仓位（即静态仓位+PID控制）
    fn get_dynamic_position_with_static_position(
        &self,
        price: Decimal, // 收盘价
        base_quantity: Decimal,
        quote_quantity: Decimal,
        target_position_ratio: Decimal,
    ) -> Decimal
    {
        // 实际仓位占比
        let position_ratio = base_quantity * price / (base_quantity * price + quote_quantity);
        // 计算差异
        let diff = target_position_ratio - position_ratio;
        // 计算积分项
        let integral = match &self.pid_config.integral {
            None => { Decimal::from(0) }
            Some(integral) => { integral.get_cumulative() * integral.get_parameter() }
        };
        position_ratio + self.pid_config.proportional * diff + integral
    }

    /// 仓位死区控制
    /// 输入值：控制前的实际仓位比例
    /// 返回值：控制后的仓位比例
    /// 只允许在仓位过高的情况下卖出
    /// 只允许在仓位过低的情况下买入
    fn dead_zone_control_by_position_ratio(
        &self, action: EOrderAction,
        target_position_ratio: Decimal,
    ) -> Decimal
    {
        match action {
            EOrderAction::Sell => {
                // 卖出 目标仓位不得低于死区上界
                let dead_zone_top = target_position_ratio + self.dead_zone_range_percentage / Decimal::from(2);
                max(dead_zone_top, target_position_ratio)
            }
            EOrderAction::Buy => {
                // 买入 目标仓位不得高于死区下界
                let dead_zone_button = target_position_ratio - self.dead_zone_range_percentage / Decimal::from(2);
                min(dead_zone_button, target_position_ratio)
            }
        }
    }

    /// 仓位活区控制
    /// 输入值：控制前的实际仓位比例
    /// 返回值：控制后的仓位比例
    /// 在活区范围内可以无视仓位进行交易
    /// 不允许在仓位过高的情况下卖出
    /// 不允许在仓位过低的情况下买入
    fn live_zone_control_by_position_ratio(
        &self,
        action: EOrderAction,
        target_position_ratio: Decimal,
    ) -> Decimal
    {
        match action {
            EOrderAction::Sell => {
                // 卖出 目标仓位不得高于活区上界
                let dead_zone_top = target_position_ratio + (Decimal::from(1) - target_position_ratio) * self.live_zone_range_percentage;
                min(dead_zone_top, target_position_ratio)
            }
            EOrderAction::Buy => {
                // 买入 目标仓位不得低于活区下界
                let dead_zone_button = target_position_ratio - target_position_ratio * self.live_zone_range_percentage;
                max(dead_zone_button, target_position_ratio)
            }
        }
    }

    /// open订单挂单逻辑
    /// 返回值(生成的订单集合, 结束状态的price, 结束状态的base_quantity, 结束状态的quote_quantity)
    fn generate_open_orders(
        &self,
        direction: EOrderDirection,
        tp_type: ETradingPairType,
        price: Decimal, // 盘口价
        base_quantity: Decimal,
        quote_quantity: Decimal,
        target_position_ratio: Decimal,
        debug_config: &SDebugConfig,
    ) -> (Vec<EStrategyAction>, Decimal, Decimal, Decimal)
    {
        let mut tmp_price = price;
        let mut tmp_base_quantity = base_quantity;
        let mut tmp_quote_quantity = quote_quantity;
        let mut orders = Vec::new();
        let action = match direction {
            EOrderDirection::Long => { EOrderAction::Buy }
            EOrderDirection::Short => { EOrderAction::Sell }
        };
        let cut_off_price = match action {
            EOrderAction::Buy => { tmp_price * (Decimal::from(1) - self.cut_off_price_percentage) }
            EOrderAction::Sell => { tmp_price * (Decimal::from(1) + self.cut_off_price_percentage) }
        };

        // 开仓 死区控制
        // debug!("Mk4:Buy\tsoft_target_position before 死区 控制:{:.4?}%", soft_target_position*Decimal::from(100));
        // debug!("Mk4:Buy\tposition_ratio:{:?}\ttarget_position_ratio:{:.4?}%", position_ratio, target_position_ratio*Decimal::from(100));
        let target_position_ratio = self.dead_zone_control_by_position_ratio(action, target_position_ratio); // 死区
        // debug!("Mk4:Buy\tsoft_target_position after 死区 控制:{:.4?}%", soft_target_position*Decimal::from(100));


        { // debug
            // let position = EOrderPosition::Open;
            // debug!("MK4::generate_open_orders: position:{:?}\taction:{:?}\ttarget_position_ratio:{:.4?}%\ttmp_price:{:?}\tcut_off_price:{:.4?}"
            //     ,position
            //     ,action
            //     ,target_position_ratio*Decimal::from(100)
            //     ,tmp_price
            //     ,cut_off_price
            // );
        }

        while match action {
            EOrderAction::Buy => { tmp_price > cut_off_price }
            EOrderAction::Sell => { tmp_price < cut_off_price }
        } {
            // let target_position_ratio = self.get_dynamic_position_with_static_position(tmp_price, tmp_base_quantity, tmp_quote_quantity, target_position_ratio);
            // {
            //     let position = EOrderPosition::Open;
            //     info!("Mk4:\taction:{:?}\tcut_off_price:{:?}\tdirection:{:?}\tposition:{:?}\ttarget_position_ratio:{:.4?}\tprice:{:?}\tbase_quantity:{:?}\tquote_quantity:{:?}",
            //         action,
            //         cut_off_price,
            //         direction,
            //         position,
            //         target_position_ratio * Decimal::from(100),
            //         price,
            //         base_quantity,
            //         quote_quantity
            //     );
            // }
            match self.get_next_order_with_given_position(
                direction,
                action,
                target_position_ratio,
                tmp_price,
                tmp_base_quantity,
                tmp_quote_quantity,
                None,
            ) {
                None => {
                    error!("Mk4:\t{:?}\terror with self.get_next_order_with_static_position", action);
                    break;
                }
                Some(SNextOrderFormat {
                         id,
                         price: order_price,
                         quantity: order_quantity,
                         base_quantity: new_base_quantity,
                         quote_quantity: new_quote_quantity,
                     }) => {
                    match action {
                        EOrderAction::Buy => {
                            if order_price <= cut_off_price {
                                break;
                            }
                        }
                        EOrderAction::Sell => {
                            if order_price >= cut_off_price {
                                break;
                            }
                        }
                    }

                    // 新建订单
                    orders.push(EStrategyAction::NewOrder(SStrategyOrderAdd {
                        id,
                        tp_type,
                        action,
                        price: order_price,
                        quantity: order_quantity,
                    }));
                    { // debug
                        // debug!("MK4::generate_open_orders: new order\ttp_type:{:?}\taction:{:?}\tprice:{:.2?}\tquantity:{:?}"
                        //     ,tp_type
                        //     ,action
                        //     ,order_price
                        //     ,order_quantity
                        // );
                    }
                    // 更新数据
                    tmp_price = order_price;
                    tmp_base_quantity = new_base_quantity;
                    tmp_quote_quantity = new_quote_quantity;
                }
            };
        }


        (orders, tmp_price, tmp_base_quantity, tmp_quote_quantity)
    }

    /// close订单挂单逻辑
    /// 返回值(生成的订单集合, 结束状态的price, 结束状态的base_quantity, 结束状态的quote_quantity)
    fn generate_close_orders(
        &self,
        direction: EOrderDirection,
        tp_type: ETradingPairType,
        price: Decimal, // 盘口价
        base_quantity: Decimal,
        quote_quantity: Decimal,
        target_position_ratio: Decimal,
        strategy_order_manager: &SStrategyOrderManagerV2,
        debug_config: &SDebugConfig,
    ) -> (Vec<EStrategyAction>, Decimal, Decimal, Decimal)
    {
        let mut tmp_price = price;
        let mut tmp_base_quantity = base_quantity;
        let mut tmp_quote_quantity = quote_quantity;
        let mut orders = Vec::new();

        // 计算方向
        let action = match direction {
            EOrderDirection::Long => { EOrderAction::Sell }
            EOrderDirection::Short => { EOrderAction::Buy }
        };

        let cut_off_price = match action {
            EOrderAction::Buy => { price * (Decimal::from(1) - self.cut_off_price_percentage) }
            EOrderAction::Sell => { price * (Decimal::from(1) + self.cut_off_price_percentage) }
        };

        {
            // let position = EOrderPosition::Close;
            // debug!("MK4::generate_close_orders: position:{:?}\taction:{:?}\ttarget_position_ratio:{:.4?}%\ttmp_price:{:?}\tcut_off_price:{:.4?}"
            //     ,position
            //     ,action
            //     ,target_position_ratio*Decimal::from(100)
            //     ,tmp_price
            //     ,cut_off_price
            // );
        }

        // 平仓 活区控制
        // debug!("Mk4:Buy\tsoft_target_position before 活区 控制:{:.4?}%", soft_target_position*Decimal::from(100));
        // debug!("Mk4:Buy\tposition_ratio:{:?}\ttarget_position_ratio:{:.4?}%", position_ratio, target_position_ratio*Decimal::from(100));
        let target_position_ratio = self.live_zone_control_by_position_ratio(action, target_position_ratio); // 活区
        // debug!("Mk4:Buy\tsoft_target_position after 活区 控制:{:.4?}%", soft_target_position*Decimal::from(100));

        let mut opened_orders_vec: Vec<(Decimal, &SStrategyOrder)> = Vec::new();
        let opened_orders = match direction {
            EOrderDirection::Long => { &strategy_order_manager.long_opened_orders }
            EOrderDirection::Short => { &strategy_order_manager.short_opened_orders }
        };

        let iter: Box<dyn Iterator<Item=(&Decimal, &Vec<Uuid>)>> = match direction {
            EOrderDirection::Long => { Box::new(opened_orders.iter()) }
            EOrderDirection::Short => { Box::new(opened_orders.iter().rev()) }
        };

        for (price, uuid_vec) in iter {
            for uuid in uuid_vec {
                opened_orders_vec.push((price.clone(), strategy_order_manager.peek_by_id(uuid).unwrap()))
            }
        }

        // debug
        // debug!("平仓订单价格:");
        // let mut count = 0;

        for (expected_close_price, strategy_order) in opened_orders_vec {
            // count += 1; // debug

            match action {
                EOrderAction::Buy => {
                    if tmp_price < cut_off_price {
                        break;
                    }
                }
                EOrderAction::Sell => {
                    if tmp_price > cut_off_price {
                        break;
                    }
                }
            }
            // let dynamic_target_position = self.get_dynamic_position_with_static_position(price, tmp_base_quantity, tmp_quote_quantity, target_position_ratio);
            { // debug
                // debug!("Mk4:\texpected_close_price:{:.2?}\tstrategy_order:{:?}", expected_close_price, strategy_order);
                // debug!("Mk4:Sell\tcut_off_price:{:.2?}\tdirection:{:?}\tprice:{:.2?}\tbase_quantity:{:?}\tquote_quantity:{:?}",
                //     cut_off_price,
                //     direction,
                //     price,base_quantity,
                //     quote_quantity
                // );
            }
            match self.get_next_order_with_given_position(
                direction,
                action,
                target_position_ratio,
                tmp_price,
                tmp_base_quantity,
                tmp_quote_quantity,
                Some(strategy_order),
            ) {
                None => {
                    error!("Mk4:Sell\terror with self.get_next_order_with_static_position");
                    break;
                }
                Some(SNextOrderFormat {
                         id,
                         price: order_price,
                         quantity: order_quantity,
                         base_quantity: new_base_quantity,
                         quote_quantity: new_quote_quantity,
                     }) => {
                    match action {
                        EOrderAction::Buy => {
                            tmp_price = min(expected_close_price, order_price);
                            if tmp_price < cut_off_price {
                                break;
                            }
                        }
                        EOrderAction::Sell => {
                            tmp_price = max(expected_close_price, order_price);
                            if tmp_price > cut_off_price {
                                break;
                            }
                        }
                    }
                    // 新建订单
                    orders.push(EStrategyAction::NewOrder(SStrategyOrderAdd {
                        id,
                        tp_type,
                        action,
                        price: tmp_price,
                        quantity: order_quantity,
                    }));
                    // 更新数据
                    tmp_base_quantity = new_base_quantity;
                    tmp_quote_quantity = new_quote_quantity;

                    { // debug
                        // debug!("Mk4-已开仓订单\tNo.{:?}\t方向:{:?}\t开仓价格:{:.2?}\t平仓价格:{:.2?}\t收益率{:.2?}%",
                        //     count, 
                        //     direction,
                        //     strategy_order.get_open_price(),
                        //     order_price,
                        //     match direction {
                        //         EOrderDirection::Long => {
                        //             order_price - strategy_order.get_open_price()
                        //         }EOrderDirection::Short => {
                        //             strategy_order.get_open_price() - order_price
                        //         }
                        //     } / strategy_order.get_open_price() * Decimal::from(100)
                        // );
                    }
                }
            };
        }
        (orders, tmp_price, tmp_base_quantity, tmp_quote_quantity)
    }

    /// 挂单逻辑
    /// 根据 当前价格+当前资产+预期仓位
    /// 计算所有挂单(只提供一定范围内的挂单)
    /// 如果预期仓位>0 则做多 先平空仓 再开多仓
    /// 如果预期仓位<0 则做空 先平多仓 再开空仓
    ///
    /// 返回值为订单集合
    fn generate_orders(
        &self,
        tp_type: ETradingPairType,
        price: Decimal,
        base_quantity: Decimal,
        quote_quantity: Decimal,
        target_position_ratio: Decimal,
        strategy_order_manager: &SStrategyOrderManagerV2,
        debug_config: &SDebugConfig,
    ) -> Vec<EStrategyAction>
    {
        // todo 添加“开仓资金占比”指标

        let mut orders = Vec::new();
        // 仓位PID控制
        let soft_target_position = self.get_dynamic_position_with_static_position(price, base_quantity, quote_quantity, target_position_ratio);
        if debug_config.is_info { debug!("Mk4::generate_orders: soft_target_position:{:.4?}%", soft_target_position*Decimal::from(100)) }

        // 计算多空
        let direction = if soft_target_position < Decimal::from(0) { EOrderDirection::Short } else { EOrderDirection::Long };
        // if debug_config.is_info { debug!("Mk4::generate_orders: direction:{:?}", direction) }


        // 优先 同向平仓(close direction)
        //  无法进行同向平仓时 进行逆向开仓(open direction.rev())
        //  其次 逆向平仓(close direction.rev())
        //  最后 同向开仓(open direction)

        // 同向平仓(close direction)
        let (mut close_orders, tmp_price, tmp_base_quantity, tmp_quote_quantity) = self.generate_close_orders(
            direction,
            tp_type,
            price,
            base_quantity,
            quote_quantity,
            soft_target_position,
            strategy_order_manager,
            debug_config,
        );
        let tp_num = close_orders.len();
        // { // debug
        //     debug!("Mk4::generate_orders: 同向平仓(close direction)订单");
        //     let mut count = 1;
        //     for order in &close_orders {
        //         debug!("{:?}\torder:{:?}", count, order);
        //         count += 1;
        //     }
        // }
        orders.append(&mut close_orders);

        // 逆向开仓(open direction.rev())
        let (mut close_orders, _, _, _) = self.generate_open_orders(
            direction.rev(),
            tp_type,
            tmp_price,
            tmp_base_quantity,
            tmp_quote_quantity,
            soft_target_position,
            debug_config,
        );
        let nk_num = close_orders.len();
        // { // debug
        //     debug!("Mk4::generate_orders: 逆向开仓(open direction.rev())订单");
        //     let mut count = 1;
        //     for order in &close_orders {
        //         debug!("{:?}\torder:{:?}", count, order);
        //         count += 1;
        //     }
        // }
        orders.append(&mut close_orders);

        // 逆向平仓(close direction.rev())
        let (mut rev_close_orders, tmp_price, tmp_base_quantity, tmp_quote_quantity) = self.generate_close_orders(
            direction.rev(),
            tp_type,
            price,
            base_quantity,
            quote_quantity,
            soft_target_position,
            strategy_order_manager,
            debug_config,
        );
        let np_num = rev_close_orders.len();
        // { // debug
        //     debug!("Mk4::generate_orders: 逆向平仓(close direction.rev())订单");
        //     let mut count = 1;
        //     for order in &close_orders {
        //         debug!("{:?}\torder:{:?}", count, order);
        //         count += 1;
        //     }
        // }
        orders.append(&mut rev_close_orders);

        // 同向开仓(open direction)
        let (mut close_orders, _, _, _) = self.generate_open_orders(
            direction,
            tp_type,
            tmp_price,
            tmp_base_quantity,
            tmp_quote_quantity,
            soft_target_position,
            debug_config,
        );
        let tk_num = close_orders.len();
        // { // debug
        //     debug!("Mk4::generate_orders: 同向开仓(open direction)订单");
        //     let mut count = 1;
        //     for order in &close_orders {
        //         debug!("{:?}\torder:{:?}", count, order);
        //         count += 1;
        //     }
        // }
        orders.append(&mut close_orders);

        if debug_config.is_info { debug!("Mk4:\t同向开仓:{:?}\t同向平仓:{:?}\t逆向开仓:{:?}\t逆向平仓:{:?}\t", tk_num, tp_num, nk_num, np_num) }

        // 返回结果
        orders
    }
}

impl<M: TPriceModel> TStrategy for SStrategyMk4<M> {
    fn run(
        &mut self,
        tp_order_map: &mut STradingPairOrderManagerMap,
        available_assets: &mut SAssetMap,
        runner_parse_result: SRunnerParseKlineResult,
        debug_config: &SDebugConfig,
    ) -> Vec<EStrategyAction>
    {
        let mut result = Vec::new();
        let SRunnerParseKlineResult {
            tp_type,
            new_kline,
            new_funding_rate: _,
            order_result
        } = runner_parse_result;
        let strategy_order_manager = self.strategy_order_map.get_mut(&tp_type).unwrap();
        // 1. 从runner获取order的执行情况，将成功执行的order进行记录。

        // debug!("【溢出测试】self.opening_orders lens before ParseOrderResult :{:?}", self.opening_and_closing_orders.len());
        for order_result in order_result {
            // info!("strategy receive order result:\t{:?}", order_result);
            match order_result {
                ERunnerParseOrderResult::OrderExecuted(order) => {
                    // 删除opening/closing订单
                    self.opening_and_closing_orders.remove(&order.get_id());
                    // 将调整策略订单状态（挂单状态改为已完成（已开仓/已平仓）状态）
                    let order_id = &order.get_id();
                    match strategy_order_manager.peek_mut_by_order_id(order_id) {
                        Err(e) => {
                            error!("SStrategyMk4::run():{:?}", e);
                        }
                        Ok(strategy_order) => {
                            match strategy_order.get_state() {
                                EStrategyOrderState::Opening => {
                                    match strategy_order_manager.opened_by_order_id(order_id) {
                                        Err(e) => { error!("SStrategyMk4::run():{:?}", e); }
                                        Ok(Err(e)) => { error!("SStrategyMk4::run():{:?}", e); }
                                        Ok(Ok(_)) => {}
                                    }
                                }
                                EStrategyOrderState::Closing => {
                                    match strategy_order_manager.closed_by_order_id(order_id) {
                                        Err(e) => { error!("SStrategyMk4::run():{:?}", e); }
                                        Ok(Err(e)) => { error!("SStrategyMk4::run():{:?}", e); }
                                        Ok(Ok(_)) => {}
                                    }
                                }
                                unexpected_state => {
                                    error!("unexpected_state:{:?}\t expected Opening or Closing", unexpected_state);
                                }
                            }
                        }
                    }

                    // if let Some(order) = strategy_order_manager.add_with_order(&order) {
                    //     error!("order can not insert into strategy_order:{:?}", order);
                    // }
                }
            }
        }

        // debug!("【溢出测试】self.opening_orders lens after ParseOrderResult:{:?}", self.opening_and_closing_orders.len());
        // { // debug only (溢出测试) 统计SStrategyOrderManage中的StrategyOrder数量
        //     let strategy_order_num = strategy_order_manager.strategy_orders.len();
        //     let opened_long_strategy_order_num = strategy_order_manager.long_opened_orders.len();
        //     let opened_short_strategy_order_num = strategy_order_manager.short_opened_orders.len();
        //     debug!("【溢出测试】统计StrategyOrderManager中的StrategyOrder数量 - \ttotal num:{:?}\tlong opened num:{:?}\tshort opened num:{:?}"
        //     ,strategy_order_num,opened_long_strategy_order_num, opened_short_strategy_order_num);
        // }

        //  2. 撤回所有opening/closing订单
        for uuid in self.opening_and_closing_orders.iter() {
            result.push(EStrategyAction::CancelOrder(uuid.clone()));
        }

        //  3. 根据当前盘口价计算挂单：
        // 3.1 计算当前价格&仓位
        let locked_assets = tp_order_map
            .calculate_total_assets();
        let total_assets = available_assets.clone() + locked_assets;
        let tmp_base_asset = SAsset { as_type: tp_type.get_base_currency(), balance: Decimal::from(0) };
        let tmp_quote_asset = SAsset { as_type: tp_type.get_quote_currency(), balance: Decimal::from(0) };
        let assets_base = total_assets
            .get(&tp_type.get_base_currency())
            .unwrap_or(&tmp_base_asset);
        let assets_quote = total_assets
            .get(&tp_type.get_quote_currency())
            .unwrap_or(&tmp_quote_asset);
        // 多空策略
        // let direction = EOrderDirection::Long;
        // 从strategy_order_manager中清除无效的索引数据
        let strategy_order_manager = self.strategy_order_map.get_mut(&tp_type).unwrap();
        strategy_order_manager.clean_index();
        let strategy_order_manager = self.strategy_order_map.get(&tp_type).unwrap();

        {
            // debug
            // debug!("Mk4 - strategy_order_manager:");
            // debug!("strategy_orders: {:?}", strategy_order_manager.strategy_orders);
            // debug!("order_strategy_order_index: {:?}", strategy_order_manager.order_strategy_order_index);
            // debug!("long_opened_orders: {:?}", strategy_order_manager.long_opened_orders);
        }

        // 长周期趋势模型
        // let target_position_ratio = self.model.get_price(new_kline.close_time).unwrap();
        let target_position_ratio = self.model.get_position(new_kline.close_time).unwrap();
        self.logger.target_position_ratio = target_position_ratio;
        // debug!("target_position_ratio: {:.4?}%", target_position_ratio*Decimal::from(100));

        // 计算买单和卖单
        // 价格
        let price = new_kline.close_price;
        // 基础货币量
        let base_quantity = assets_base.balance;
        // 计价货币量
        let quote_quantity = assets_quote.balance;
        let mut orders = self.generate_orders(
            tp_type,
            price,
            base_quantity,
            quote_quantity,
            target_position_ratio,
            strategy_order_manager,
            debug_config,
        );
        result.append(&mut orders);

        //  4. 向runner发送撤单和订单请求
        // info!("result:{:?}", result);
        result
    }

    fn verify(
        &mut self,
        tp_type: &ETradingPairType,
        parse_action_results: Vec<ERunnerSyncActionResult>,
        debug_config: &SDebugConfig,
    )
    {
        // 5. 根据runner反馈情况，将成功挂单的order进行记录。
        let strategy_order_manager = self.strategy_order_map.get_mut(&tp_type).unwrap();
        for result in parse_action_results {
            // info!("strategy verify:\t{:?}", result);
            match result {
                ERunnerSyncActionResult::OrderPlaced(order, strategy_order_id) => {
                    // 尝试向opening_orders中加入该订单
                    self.opening_and_closing_orders.insert(order.get_id());
                    // 判断该订单是开仓单还是平仓单
                    match strategy_order_id {
                        None => {
                            // 开仓单
                            // 新建StrategyOrder并插入StrategyOrderManager
                            if let Some(order) = strategy_order_manager.add_with_order(&order) {
                                error!("order can not insert into strategy_order:{:?}", order);
                            }
                        }
                        Some(strategy_order_id) => {
                            // 平仓单
                            // 从StrategyOrderManager中找到对应的StrategyOrderManager
                            match strategy_order_manager.peek_mut_by_id(&strategy_order_id) {
                                Err(e) => {
                                    error!("SStrategyMk4::verify():{:?}", e);
                                }
                                Ok(strategy_order) => {
                                    if strategy_order.get_state() != EStrategyOrderState::Opened {
                                        error!("SStrategyMk4::verify():strategy_order state error, expected state Opened, order_id:{:?}\tactual state {:?}\tstrategy_order:{:?}",
                                            order.get_id(), strategy_order.get_state(), strategy_order);
                                    } else {
                                        match strategy_order_manager.bind_close_by_id(&strategy_order_id, &order) {
                                            Err(e) => { error!("SStrategyMk4::verify():{:?}", e); }
                                            Ok(x) => {
                                                match x {
                                                    Err(e) => { error!("SStrategyMk4::verify():{:?}", e); }
                                                    Ok(_) => {
                                                        if debug_config.is_debug { debug!("SStrategyMk4::verify(): bind close success\tstrategy_order_id:{:?}\torder:{:?}", strategy_order_id, order); }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                ERunnerSyncActionResult::OrderCanceled(order) => {
                    // 尝试从opening_orders中删除该订单
                    if false == self.opening_and_closing_orders.remove(&order.get_id()) {
                        error!("SStrategyMk4::verify(): remove order from SStrategyMk4.opening_orders fail:{:?}", &order.get_id());
                    }
                    // 更新strategy_order
                    match strategy_order_manager.peek_mut_by_order_id(&order.get_id()) {
                        Err(e) => {
                            // 找不到对应的strategy_order
                            error!("SStrategyMk4::verify(): ERunnerSyncActionResult::OrderCanceled(order)-在strategy_order_manager中找不到对应的strategy_order:\norder info:{:?}\nerror info:{:?}", &order, e);
                        }
                        Ok(strategy_order) => {
                            // 更新strategy_order
                            if strategy_order.get_state() == EStrategyOrderState::Closing {
                                match strategy_order_manager.cancel_close_by_order_id(&order.get_id()) {
                                    Err(e) => { error!("SStrategyMk4::{:?}", e); }
                                    Ok(x) => {
                                        match x {
                                            Err(e) => { error!("SStrategyMk4::{:?}", e); }
                                            Ok(_) => {}
                                        }
                                    }
                                }
                            } else if strategy_order.get_state() == EStrategyOrderState::Opening {
                                match strategy_order_manager.cancel_open_by_order_id(&order.get_id()) {
                                    Err(e) => { error!("SStrategyMk4::{:?}", e); }
                                    Ok(x) => {
                                        match x {
                                            Err(e) => { error!("SStrategyMk4::{:?}", e); }
                                            Ok(_popped_strategy_order) => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // debug!("【溢出测试】 after verify:{:?}", self.opening_and_closing_orders.len());
    }

    fn get_log_info(&self) -> SStrategyLogger {
        self.logger.clone()
    }

    fn get_position(&self, time: DateTime<Local>) -> Option<Decimal> {
        self.model.get_position(time)
    }
}