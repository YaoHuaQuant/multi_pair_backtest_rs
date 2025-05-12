use std::cmp::min;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Bound::Excluded;

use log::error;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use uuid::Uuid;
use crate::data_runtime::order::EOrderDirection;
use crate::data_runtime::order::order::SOrder;
use crate::strategy::order::order::{EStrategyOrderState, RStrategyOrderResult, SStrategyOrder};

pub type RStrategyOrderManagerV2Result<T> = Result<T, EStrategyOrderManagerV2Error>;

/// 订单管理器异常
#[derive(Debug)]
pub enum EStrategyOrderManagerV2Error {
    /// 找不到Uuid对应的StrategyOrder
    StrategyOrderIdNotFound(Uuid),
    /// 找不到Uuid对应的Order
    OrderIdNotFound(Uuid),
    // /// 插入订单失败
    // InsertOrderFail(SOrder),
    /// LongOrders/ShortOrders订单索引 在某个价格的UuidVec为空
    OrdersUuidVecEmptyError(EOrderDirection, Decimal),
    /// LongOrders/ShortOrders订单索引中的Uuid无法在Orders上找到
    OrdersUuidCannotFindInOrdersError(EOrderDirection, Decimal, Uuid),
    // /// 插入已完成的订单时 订单状态错误
    // InsertFinishedOrderStateError(EOrderState),
}

#[derive(Debug, Default, Clone)]
pub struct SStrategyOrderManagerV2 {
    /// 策略订单池
    pub strategy_orders: HashMap<Uuid, SStrategyOrder>,
    /// 普通订单id-策略订单id 索引
    pub order_strategy_order_index: HashMap<Uuid, Uuid>,
    /// 做多已开仓未平仓订单列表 K-平仓价格 V-策略订单uuid列表
    pub long_opened_orders: BTreeMap<Decimal, Vec<Uuid>>,
    /// 做空已开仓未平仓订单列表 K-平仓价格 V-策略订单uuid列表
    pub short_opened_orders: BTreeMap<Decimal, Vec<Uuid>>,
    /// 最低盈利百分比（不包括手续费）
    pub min_profit_percentage: Decimal,
    /// 最高盈利百分比（不包括手续费）
    pub max_profit_percentage: Decimal,
    /// 平仓价最小间距（相比于开仓价的百分比）
    pub close_price_step_percentage: Decimal,
}

impl SStrategyOrderManagerV2 {

    pub fn from(
        min_profit_percentage: Decimal,
        max_profit_percentage: Decimal,
        close_price_step_percentage: Decimal,
    ) -> Self
    {
        Self {
            strategy_orders: Default::default(),
            order_strategy_order_index: Default::default(),
            long_opened_orders: Default::default(),
            short_opened_orders: Default::default(),
            min_profit_percentage,
            max_profit_percentage,
            close_price_step_percentage,
        }
    }

    /// 插入一个SStrategyOrder
    /// 初始状态为Opening
    /// 返回值None表示插入成功
    /// 返回值Some(SStrategyOrder)表示插入失败 返回原始参数
    pub fn add(&mut self, order: SStrategyOrder) -> Option<SStrategyOrder> {
        let result = self.strategy_orders.insert(order.get_id(), order.clone());
        if let None = result {
            // 插入成功的场景
            // 设置open order的id索引
            let _ = self.order_strategy_order_index.insert(order.get_open_order_id(), order.get_id());
            // 设置close order的id索引
            if let Some(close_order_id) = order.get_close_order_id() {
                let _ = self.order_strategy_order_index.insert(close_order_id, order.get_id());
            }
        } else {
            error!("SStrategyOrderManagerV2.strategy_orders.insert(...) fail: duplicated id-{:?}", order.get_id());
        }
        result
    }

    /// 基于SOrder生成SStrategyOrder并插入
    pub fn add_with_order(&mut self, order: &SOrder) -> Option<SStrategyOrder> {
        self.add(SStrategyOrder::new(order))
    }

    /// 获取一个StrategyOrder的引用
    pub fn peek_by_id(&self, uuid: &Uuid) -> RStrategyOrderManagerV2Result<&SStrategyOrder> {
        match self.strategy_orders.get(uuid) {
            None => { Err(EStrategyOrderManagerV2Error::StrategyOrderIdNotFound(*uuid)) }
            Some(strategy_order) => { Ok(strategy_order) }
        }
    }

    /// 获取一个StrategyOrder的可变引用
    pub fn peek_mut_by_id(&mut self, uuid: &Uuid) -> RStrategyOrderManagerV2Result<&mut SStrategyOrder> {
        match self.strategy_orders.get_mut(uuid) {
            None => { Err(EStrategyOrderManagerV2Error::StrategyOrderIdNotFound(*uuid)) }
            Some(strategy_order) => { Ok(strategy_order) }
        }
    }

    /// 删除订单一个SStrategyOrder
    /// 返回值None表示删除失败
    /// 返回值Some(SStrategyOrder)表示删除成功 返回被删除的SStrategyOrder
    pub fn pop_by_id(&mut self, uuid: &Uuid) -> Option<SStrategyOrder> {
        match self.strategy_orders.remove(&uuid) {
            None => { None }
            Some(order) => {
                // 删除索引

                // 删除open order的id索引
                let _ = self.order_strategy_order_index.remove(&order.get_open_order_id());

                // 删除close order的id索引
                if let Some(close_order_id) = order.get_close_order_id() {
                    let _ = self.order_strategy_order_index.remove(&close_order_id);
                }

                // // 从 LongOrders/ShortOrders中删除对应元素
                // if order.get_state() == EStrategyOrderState::Opened {
                //     // 只有已开仓未平仓的订单会出现在LongOrders/ShortOrders中
                //     let order_set = match order.get_direction() {
                //         EOrderDirection::Long => { &mut self.long_opened_orders }
                //         EOrderDirection::Short => { &mut self.short_opened_orders }
                //     };
                //     let price_level = order_set.get_mut(&order.get_open_price());
                //     if let Some(level) = price_level {
                //         level.retain(|&id| id != *uuid);
                //         if level.is_empty() { order_set.remove(&order.get_open_price()); }
                //     }
                // }
                Some(order)
            }
        }
    }

    pub fn pop_by_order_id(&mut self, order_id: &Uuid) -> Option<SStrategyOrder> {
        match self.order_strategy_order_index.get(&order_id).cloned() {
            None => { None }
            Some(id) => {
                self.pop_by_id(&id)
            }
        }
    }

    /// 获取一个StrategyOrder的引用
    pub fn peek_by_order_id(&self, order_id: &Uuid) -> RStrategyOrderManagerV2Result<&SStrategyOrder> {
        match self.order_strategy_order_index.get(order_id) {
            None => { Err(EStrategyOrderManagerV2Error::OrderIdNotFound(*order_id)) }
            Some(id) => { self.peek_by_id(id) }
        }
    }

    /// 获取一个StrategyOrder的可变引用
    pub fn peek_mut_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerV2Result<&mut SStrategyOrder> {
        match self.order_strategy_order_index.get_mut(order_id).cloned() {
            None => { Err(EStrategyOrderManagerV2Error::OrderIdNotFound(*order_id)) }
            Some(id) => { self.peek_mut_by_id(&id) }
        }
    }

    /// 将订单写入opened_orders
    /// 返回订单的最低close价格
    fn push_into_opened_orders(&mut self, strategy_order_id: &Uuid) -> RStrategyOrderManagerV2Result<Decimal> {
        let mut strategy_order = self.peek_by_id(strategy_order_id)?.clone();
        let mut opened_orders = match strategy_order.get_direction() {
            EOrderDirection::Long => { &mut self.long_opened_orders }
            EOrderDirection::Short => { &mut self.short_opened_orders }
        };
        // 计算订单的最低close价格
        // 最低平仓价
        let mut price = strategy_order.get_open_price() * (Decimal::from(1) + self.min_profit_percentage);
        // 最高平仓价
        let price_max = strategy_order.get_open_price() * (Decimal::from(1) + self.max_profit_percentage);
        // 价格差异
        let price_step = strategy_order.get_open_price() * self.close_price_step_percentage;
        // 遍历寻找合适的close price
        while price < price_max {
            let price_low = price - price_step;
            let price_high = price + price_step;
            // 当[price_low,price_high]之间不存在元素时 结束遍历
            let hit_vec: Vec<(&Decimal, &Vec<Uuid>)> = opened_orders.range((Excluded(&price_low), Excluded(&price_high))).collect();
            if hit_vec.is_empty() {
                break;
            } else {
                // 当下一个价格大于等于price_max时 强制置为price_max 自动结束遍历
                price += price_step;
                price = min(price_max, price);
            }
        }
        // let price_level = opened_orders.entry(strategy_order.get_open_price()).or_insert_with(Vec::new);
        let price_level = opened_orders.entry(price).or_insert_with(Vec::new);

        price_level.push(strategy_order.get_id());

        // 设置期望平仓价格
        self.set_expected_close_price_by_id(&strategy_order.get_id(), Some(price))?;
        Ok(price)
    }

    /// 将订单移出opened_orders
    fn remove_from_opened_orders(&mut self, strategy_order_id: &Uuid) -> RStrategyOrderManagerV2Result<()> {
        let strategy_order = self.peek_by_id(strategy_order_id)?.clone();
        let mut opened_orders = match strategy_order.get_direction() {
            EOrderDirection::Long => { &mut self.long_opened_orders }
            EOrderDirection::Short => { &mut self.short_opened_orders }
        };
        let price_level = opened_orders.get_mut(&strategy_order.get_expected_close_price().unwrap());
        if let Some(level) = price_level {
            level.retain(|&id| id != *strategy_order_id);
            if level.is_empty() {
                opened_orders.remove(&strategy_order.get_open_price());
            }
        }
        Ok(())
    }

    /// 清理long_opened_orders中的无效指针
    pub fn clean_long_opened_orders(&mut self) {
        let data_keys: HashSet<_> = self.strategy_orders.keys().cloned().collect();
        self.long_opened_orders.retain(|price, ids| {
            ids.retain(|id| data_keys.contains(id));
            !ids.is_empty()
        });
    }
    
    /// 清理short_opened_orders中的无效指针
    pub fn clean_short_opened_orders(&mut self) {
        let data_keys: HashSet<_> = self.strategy_orders.keys().cloned().collect();
        self.short_opened_orders.retain(|price, ids| {
            ids.retain(|id| data_keys.contains(id));
            !ids.is_empty()
        });
    }
    
    pub fn clean_index(&mut self) {
        self.clean_long_opened_orders();
        self.clean_short_opened_orders();
    }

    // region --- 转发SStrategyOrder函数
    /// 取消开单 删除结构体自身
    /// 需要获取StrategyOrder的所有权并返回
    pub fn cancel_open_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerV2Result<RStrategyOrderResult<SStrategyOrder>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let strategy_order_id = strategy_order.get_id();
        match strategy_order.cancel_open() {
            Err(e) => { Ok(Err(e)) }
            Ok(_) => {
                Ok(Ok(self.pop_by_id(&strategy_order_id).unwrap()))
            }
        }
    }

    /// 开单结算
    /// 需要将订单写入opened_orders
    pub fn opened_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerV2Result<RStrategyOrderResult<Decimal>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let result = strategy_order.opened();
        let strategy_order_id = strategy_order.get_id();
        
        match result {
            Ok(_) => {
                let price = self.push_into_opened_orders(&strategy_order_id)?;
                Ok(Ok(price))
            }
            Err(e) => {Ok(Err(e))}
        }
    }

    /// 绑定平单
    /// 需要将订单移出opened_orders
    /// 需要将平单加入order_strategy_order_index索引
    pub fn bind_close_by_order_id(&mut self, order_id: &Uuid, closing_order: &SOrder) -> RStrategyOrderManagerV2Result<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let result = strategy_order.bind_close(closing_order);
        let strategy_order_id = strategy_order.get_id();
        if let Ok(()) = result {
            if let Err(e) = self.remove_from_opened_orders(&strategy_order_id) {
                error!("{:?}", e);
            } else {
                self.order_strategy_order_index.insert(closing_order.get_id(), strategy_order_id);
            }
        }
        Ok(result)
    }

    /// 绑定平单
    /// 需要将订单移出opened_orders
    /// 需要将平单加入order_strategy_order_index索引
    pub fn bind_close_by_id(&mut self, strategy_order_id: &Uuid, closing_order: &SOrder) -> RStrategyOrderManagerV2Result<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_id(strategy_order_id)?;
        let result = strategy_order.bind_close(closing_order);
        let strategy_order_id = strategy_order.get_id();
        if let Ok(()) = result {
            if let Err(e) = self.remove_from_opened_orders(&strategy_order_id) {
                error!("{:?}", e);
            } else {
                self.order_strategy_order_index.insert(closing_order.get_id(), strategy_order_id);
            }
        }
        Ok(result)
    }

    /// 平单结算
    /// 返回 strategy order 并在外部析构
    /// todo debug
    pub fn closed_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerV2Result<RStrategyOrderResult<SStrategyOrder>> {
        match self.pop_by_order_id(order_id) {
            None => {Err(EStrategyOrderManagerV2Error::OrderIdNotFound(order_id.clone()))}
            Some(mut strategy_order) => {
                match strategy_order.closed() {
                    Ok(_) => {Ok(Ok(strategy_order))}
                    Err(e) => {Ok(Err(e))}
                }
            }
        }
    }

    /// 取消平单
    /// 需要将订单写入opened_orders
    pub fn cancel_close_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerV2Result<RStrategyOrderResult<Decimal>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let result = strategy_order.cancel_close();
        let strategy_order_id = strategy_order.get_id();
        match result {
            Ok(_) => {
                let price = self.push_into_opened_orders(&strategy_order_id)?;
                Ok(Ok(price))
            }
            Err(e) => {Ok(Err(e))}
        }
    }

    fn set_expected_close_price_by_id(&mut self, id: &Uuid, expected_close_price: Option<Decimal>) -> RStrategyOrderManagerV2Result<()> {
        self.peek_mut_by_id(id)?.set_expected_close_price(expected_close_price);
        Ok(())
    }
    // end region --- 转发SStrategyOrder函数

    ///  基于特定限定(Long/Short Lowest/Highest) 查看第一个已开仓待平仓订单 获取其引用
    fn peek_conditioned_first_opened_order(&self, is_long: bool, is_highest: bool) -> RStrategyOrderManagerV2Result<Option<&SStrategyOrder>> {
        let order_set = if is_long { &self.long_opened_orders } else { &self.short_opened_orders };
        let direction = if is_long { EOrderDirection::Long } else { EOrderDirection::Short };
        let order = if is_highest {
            order_set.last_key_value()
        } else {
            order_set.first_key_value()
        };
        match order {
            None => { Ok(None) }
            Some((price, uuid_list)) => {
                match uuid_list.get(0) {
                    None => { Err(EStrategyOrderManagerV2Error::OrdersUuidVecEmptyError(direction, price.clone())) }
                    Some(uuid) => {
                        match self.peek_by_id(uuid) {
                            Err(_) => { Err(EStrategyOrderManagerV2Error::OrdersUuidCannotFindInOrdersError(direction, price.clone(), uuid.clone())) }
                            Ok(order) => { Ok(Some(order)) }
                        }
                    }
                }
            }
        }
    }

    /// 基于特定限定(Long/Short Lowest/Highest) 获取第一个已开仓待平仓订单
    fn pop_conditioned_first_opened_order(&mut self, is_long: bool, is_highest: bool) -> RStrategyOrderManagerV2Result<Option<SStrategyOrder>> {
        let id = match self.peek_conditioned_first_opened_order(is_long, is_highest)? {
            None => { None }
            Some(order) => { Some(order.get_id()) }
        };
        match id {
            None => { Ok(None) }
            Some(uuid) => {
                let result = self.pop_by_id(&uuid);
                // 从opened_orders中清除无效索引
                match is_long {
                    true => {self.clean_long_opened_orders();}
                    false => {self.clean_short_opened_orders();}
                }
                Ok(result)
            }
        }
    }

    ///  查看价格最高的做多已开仓待平仓订单 获取其引用
    pub fn peek_highest_long_opened_order(&self) -> RStrategyOrderManagerV2Result<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(true, true)
    }

    ///  获取价格最高的做多已开仓待平仓订单
    pub fn pop_highest_long_opened_order(&mut self) -> RStrategyOrderManagerV2Result<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(true, true)
    }

    ///  查看价格最低的做多已开仓待平仓订单 获取其引用
    pub fn peek_lowest_long_opened_order(&self) -> RStrategyOrderManagerV2Result<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(true, false)
    }

    ///  获取价格最低的做多已开仓待平仓订单
    pub fn pop_lowest_long_opened_order(&mut self) -> RStrategyOrderManagerV2Result<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(true, false)
    }

    ///  查看价格最高的做空已开仓待平仓订单 获取其引用
    pub fn peek_highest_short_opened_order(&self) -> RStrategyOrderManagerV2Result<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(false, true)
    }

    ///  获取价格最高的做空已开仓待平仓订单
    pub fn pop_highest_short_opened_order(&mut self) -> RStrategyOrderManagerV2Result<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(false, true)
    }

    ///  查看价格最低的做空已开仓待平仓订单 获取其引用
    pub fn peek_lowest_short_opened_order(&self) -> RStrategyOrderManagerV2Result<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(false, false)
    }

    ///  获取价格最低的做空已开仓待平仓订单
    pub fn pop_lowest_short_opened_order(&mut self) -> RStrategyOrderManagerV2Result<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(false, false)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use uuid::Uuid;

    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::EAssetType;
    use crate::data_runtime::order::{EOrderAction, EOrderDirection};
    use crate::data_runtime::order::order::SAddOrder;
    use crate::data_runtime::order::order_manager::SOrderManager;
    use crate::strategy::order::order::{EStrategyOrderState, SStrategyOrder};
    use crate::strategy::order::order_manager_v2::SStrategyOrderManagerV2;

    pub fn get_test_data_origin(direction: EOrderDirection) -> (Vec<Uuid>, SOrderManager, SStrategyOrderManagerV2) {
        let min_profit_percentage = Decimal::from_f64(0.01).unwrap();
        let max_profit_percentage = Decimal::from_f64(0.05).unwrap();
        let close_price_step_percentage = Decimal::from_f64(0.001).unwrap();
        let mut strategy_manager = SStrategyOrderManagerV2::from(
            min_profit_percentage,
            max_profit_percentage,
            close_price_step_percentage,
        );
        let mut manager = SOrderManager::new();

        let price_vec = vec![
            (Decimal::from_str("400").unwrap(), Decimal::from_f64(0.4).unwrap()),
            (Decimal::from_str("200").unwrap(), Decimal::from_f64(0.2).unwrap()),
            (Decimal::from_str("100").unwrap(), Decimal::from_f64(0.1).unwrap()),
            (Decimal::from_str("300").unwrap(), Decimal::from_f64(0.3).unwrap()),
            (Decimal::from_str("300").unwrap(), Decimal::from_f64(0.33).unwrap()),
            (Decimal::from_str("300").unwrap(), Decimal::from_f64(0.36).unwrap()),
            (Decimal::from_str("500").unwrap(), Decimal::from_f64(0.5).unwrap()),
            (Decimal::from_str("600").unwrap(), Decimal::from_f64(0.6).unwrap()),
        ];

        let mut id_vec = vec![];

        let action = match direction {
            EOrderDirection::Long => { EOrderAction::Buy }
            EOrderDirection::Short => { EOrderAction::Sell }
        };
        for (price, quantity) in price_vec {
            let new_order = SAddOrder {
                action,
                price,
                quantity,
            };
            let id = manager.add_new_order(new_order).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let strategy_order = SStrategyOrder::new(order);
            strategy_manager.add(strategy_order);
            id_vec.push(id)
        }
        (id_vec, manager, strategy_manager)
    }

    pub fn get_test_data_opened(direction: EOrderDirection) -> (Vec<Uuid>, SOrderManager, SStrategyOrderManagerV2) {
        let (mut id_vec, mut manager, mut strategy_manager)
            = get_test_data_origin(direction);

        for id in id_vec.iter() {
            let mut order = manager.orders.get_mut(&id).unwrap();
            let _ = order.submit(SAsset { as_type: EAssetType::Usdt, balance: order.get_price() * order.get_quantity() });
            let _ = strategy_manager.opened_by_order_id(&id);
        }
        (id_vec, manager, strategy_manager)
    }

    #[test]
    pub fn test_origin_data() {
        let direction = EOrderDirection::Long;
        let (id_vec, manager, strategy_manager) = get_test_data_origin(direction);
        // dbg!(&id_vec);
        // dbg!(&manager);
        // dbg!(&strategy_manager);
        for (price_level, vec_uuid) in strategy_manager.long_opened_orders.iter() {
            for strategy_order_id in vec_uuid {
                let strategy_order = strategy_manager.peek_by_id(&strategy_order_id);
                assert!(matches!(strategy_order, Ok(_)));
                let strategy_order = strategy_order.unwrap();
                let order = manager.peek_order(&strategy_order.get_open_order_id());
                assert!(matches!(order, Some(_)));
                let order = order.unwrap();
                assert_eq!(strategy_order.get_open_order_id(), order.get_id());
                assert_eq!(strategy_order.get_open_price(), order.get_price());
                assert_eq!(strategy_order.get_quantity(), order.get_quantity());
                assert_eq!(strategy_order.get_state(), EStrategyOrderState::Opened);
            }
        }
    }

    #[test]
    pub fn test_opened_data() {
        let direction = EOrderDirection::Long;
        let (id_vec, manager, strategy_manager) = get_test_data_opened(direction);
        let mut iter = strategy_manager.long_opened_orders.iter();
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(101));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(202));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(303));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from_f64(303.3).unwrap());
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from_f64(303.6).unwrap());
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(404));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(505));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(606));
        assert_eq!(value.len(), 1);
    }

    #[test]
    pub fn test_index() {
        let direction = EOrderDirection::Long;
        let (id_vec, manager, strategy_manager) = get_test_data_opened(direction);
        // dbg!(&id_vec);
        // dbg!(&manager);
        // dbg!(&strategy_manager);
        for order_id in id_vec {
            let order = manager.orders.get(&order_id);
            assert!(matches!(order, Some(_)));
            let order = order.unwrap();
            let strategy_order = strategy_manager.peek_by_order_id(&order_id);
            assert!(matches!(strategy_order, Ok(_)));
            let strategy_order = strategy_order.unwrap();
            assert_eq!(strategy_order.get_open_order_id(), order.get_id());
            assert_eq!(strategy_order.get_open_price(), order.get_price());
            assert_eq!(strategy_order.get_quantity(), order.get_quantity());
            assert_eq!(strategy_order.get_state(), EStrategyOrderState::Opened);
        }
    }

    #[test]
    pub fn test_bind_close_and_cancel_close() {
        let direction = EOrderDirection::Long;
        let (id_vec, mut manager, mut strategy_manager) = get_test_data_opened(direction);
        let close_action = EOrderAction::Sell;

        let open_order_id = id_vec.get(1).unwrap();
        let open_order = manager.peek_order(open_order_id).unwrap().clone();
        let strategy_order = strategy_manager.peek_mut_by_order_id(open_order_id);
        assert!(matches!(strategy_order, Ok(_)));
        let mut strategy_order = strategy_order.unwrap();
        let strategy_order_id = strategy_order.get_id();
        let close_order_id = manager.add_new_order(SAddOrder {
            action: close_action,
            price: open_order.get_price() * Decimal::from_f64(1.1).unwrap(),
            quantity: open_order.get_quantity(),
        }).unwrap();
        let close_order = manager.peek_order(&close_order_id).unwrap();

        // bind close
        let r = strategy_manager.bind_close_by_order_id(&open_order_id, close_order);
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Ok(_)));

        let r = strategy_manager.long_opened_orders.get(&open_order.get_price());
        assert!(matches!(r, None));

        // cancel close
        let r = strategy_manager.cancel_close_by_order_id(&open_order_id);
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Ok(_)));

        let r = strategy_manager.long_opened_orders.get(&(open_order.get_price() * Decimal::from_f64(1.01).unwrap()));
        // dbg!(&r);
        assert!(matches!(r, Some(_)));
        let r = r.unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    pub fn test_opened() {
        let direction = EOrderDirection::Long;
        let (id_vec, mut manager, mut strategy_manager) = get_test_data_origin(direction);

        let open_order_id = id_vec.get(1).unwrap();
        let open_order = manager.peek_order(open_order_id).unwrap().clone();
        let strategy_order = strategy_manager.peek_mut_by_order_id(open_order_id);
        assert!(matches!(strategy_order, Ok(_)));
        let mut strategy_order = strategy_order.unwrap();
        let strategy_order_id = strategy_order.get_id();

        // opening
        let r = strategy_manager.long_opened_orders.get(&open_order.get_price());
        assert!(matches!(r, None));

        // opened
        let r = strategy_manager.opened_by_order_id(&open_order_id);
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Ok(_)));

        let r = strategy_manager.long_opened_orders.get(&(open_order.get_price() * Decimal::from_f64(1.01).unwrap()));
        // dbg!(&r);
        assert!(matches!(r, Some(_)));
        let r = r.unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    pub fn test_peek_pop_long_opened_order() {
        let direction = EOrderDirection::Long;
        let (id_vec, manager, mut strategy_manager) = get_test_data_opened(direction);

        // dbg!(&id_vec);
        // dbg!(&manager);
        // dbg!(&strategy_manager);

        // peek
        let r = strategy_manager.peek_highest_long_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(606.0).unwrap()));

        let strategy_order = strategy_manager.peek_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(101.0).unwrap()));

        // peek
        let r = strategy_manager.peek_highest_long_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(606.0).unwrap()));
        let r = strategy_manager.peek_lowest_long_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(101.0).unwrap()));

        // peek fail
        let r = strategy_manager.peek_highest_short_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.peek_lowest_short_opened_order().unwrap();
        assert!(matches!(r, None));

        // pop
        let r = strategy_manager.pop_highest_long_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(606.0).unwrap()));

        let r = strategy_manager.peek_highest_long_opened_order();

        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(505.0).unwrap()));

        let r = strategy_manager.pop_highest_long_opened_order();
        assert!(matches!(r, Ok(_)));
        let strategy_order = r.unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(505.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(101.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(200));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(202.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(303.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(303.3).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(303.6).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(400));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(404.0).unwrap()));

        // pop fail
        let r = strategy_manager.pop_lowest_long_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.pop_highest_long_opened_order().unwrap();
        assert!(matches!(r, None));

        // peek fail
        let r = strategy_manager.peek_highest_long_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.peek_lowest_long_opened_order().unwrap();
        assert!(matches!(r, None));
    }

    #[test]
    pub fn test_peek_pop_short_opened_order() {
        let direction = EOrderDirection::Short;
        let (id_vec, manager, mut strategy_manager) = get_test_data_opened(direction);

        // dbg!(&id_vec);
        // dbg!(&manager);
        // dbg!(&strategy_manager);

        // peek
        let r = strategy_manager.peek_highest_short_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(606.0).unwrap()));

        let strategy_order = strategy_manager.peek_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(101.0).unwrap()));

        // peek
        let r = strategy_manager.peek_highest_short_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(606.0).unwrap()));
        let r = strategy_manager.peek_lowest_short_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(101.0).unwrap()));

        // peek fail
        let r = strategy_manager.peek_highest_long_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.peek_lowest_long_opened_order().unwrap();
        assert!(matches!(r, None));

        // pop
        let r = strategy_manager.pop_highest_short_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(606.0).unwrap()));

        let r = strategy_manager.peek_highest_short_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(505.0).unwrap()));

        let r = strategy_manager.pop_highest_short_opened_order();
        assert!(matches!(r, Ok(_)));
        let strategy_order = r.unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(505.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(101.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(200));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(202.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(303.0).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(303.3).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(303.6).unwrap()));
        let strategy_order = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(400));
        assert_eq!(strategy_order.get_expected_close_price(), Some(Decimal::from_f64(404.0).unwrap()));

        // pop fail
        let r = strategy_manager.pop_lowest_short_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.pop_highest_short_opened_order().unwrap();
        assert!(matches!(r, None));

        // peek fail
        let r = strategy_manager.peek_highest_short_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.peek_lowest_short_opened_order().unwrap();
        assert!(matches!(r, None));
    }
}