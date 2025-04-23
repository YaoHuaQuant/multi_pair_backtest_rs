use std::collections::{BTreeMap, HashMap};

use log::error;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::data_runtime::order::EOrderDirection;
use crate::data_runtime::order::order::SOrder;
use crate::strategy::order::order::{EStrategyOrderState, RStrategyOrderResult, SStrategyOrder};

pub type RStrategyOrderManagerResult<T> = Result<T, EStrategyOrderManagerError>;

/// 订单管理器异常
#[derive(Debug)]
pub enum EStrategyOrderManagerError {
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

#[derive(Debug, Default)]
pub struct SStrategyOrderManager {
    /// 策略订单池
    pub strategy_orders: HashMap<Uuid, SStrategyOrder>,
    /// 普通订单id-策略订单id 索引
    pub order_strategy_order_index: HashMap<Uuid, Uuid>,
    /// 做多已开仓未平仓订单列表 K-价格 V-策略订单uuid列表
    pub long_opened_orders: BTreeMap<Decimal, Vec<Uuid>>,
    /// 做空已开仓未平仓订单列表 K-价格 V-策略订单uuid列表
    pub short_opened_orders: BTreeMap<Decimal, Vec<Uuid>>,
}

impl SStrategyOrderManager {
    pub fn new() -> Self {
        Self {
            strategy_orders: Default::default(),
            order_strategy_order_index: Default::default(),
            long_opened_orders: Default::default(),
            short_opened_orders: Default::default(),
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
        }
        result
    }

    /// 获取一个StrategyOrder的引用
    pub fn peek_by_id(&self, uuid: &Uuid) -> RStrategyOrderManagerResult<&SStrategyOrder> {
        match self.strategy_orders.get(uuid) {
            None => { Err(EStrategyOrderManagerError::StrategyOrderIdNotFound(*uuid)) }
            Some(strategy_order) => { Ok(strategy_order) }
        }
    }

    /// 获取一个StrategyOrder的可变引用
    pub fn peek_mut_by_id(&mut self, uuid: &Uuid) -> RStrategyOrderManagerResult<&mut SStrategyOrder> {
        match self.strategy_orders.get_mut(uuid) {
            None => { Err(EStrategyOrderManagerError::StrategyOrderIdNotFound(*uuid)) }
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

                // 从 LongOrders/ShortOrders中删除对应元素
                if order.get_state() == EStrategyOrderState::Opened {
                    // 只有已开仓未平仓的订单会出现在LongOrders/ShortOrders中
                    let order_set = match order.get_direction() {
                        EOrderDirection::Long => { &mut self.long_opened_orders }
                        EOrderDirection::Short => { &mut self.short_opened_orders }
                    };
                    let price_level = order_set.get_mut(&order.get_open_price());
                    if let Some(level) = price_level {
                        level.retain(|&id| id != *uuid);
                        if level.is_empty() { order_set.remove(&order.get_open_price()); }
                    }
                }
                Some(order)
            }
        }
    }

    pub fn pop_by_order_id(&mut self, order_id: &Uuid) -> Option<SStrategyOrder> {
        let id = self.order_strategy_order_index.get(&order_id).cloned();
        match id {
            None => { None }
            Some(id) => {
                self.pop_by_id(&id)
            }
        }
    }

    /// 获取一个StrategyOrder的引用
    pub fn peek_by_order_id(&self, order_id: &Uuid) -> RStrategyOrderManagerResult<&SStrategyOrder> {
        match self.order_strategy_order_index.get(order_id) {
            None => { Err(EStrategyOrderManagerError::OrderIdNotFound(*order_id)) }
            Some(id) => { self.peek_by_id(id) }
        }
    }

    /// 获取一个StrategyOrder的可变引用
    pub fn peek_mut_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerResult<&mut SStrategyOrder> {
        match self.order_strategy_order_index.get_mut(order_id).cloned() {
            None => { Err(EStrategyOrderManagerError::OrderIdNotFound(*order_id)) }
            Some(id) => { self.peek_mut_by_id(&id) }
        }
    }

    /// 将订单写入opened_orders
    fn push_into_opened_orders(&mut self, strategy_order_id: &Uuid) -> RStrategyOrderManagerResult<()> {
        let strategy_order = self.peek_by_id(strategy_order_id)?.clone();
        let mut opened_orders = match strategy_order.get_direction() {
            EOrderDirection::Long => { &mut self.long_opened_orders }
            EOrderDirection::Short => { &mut self.short_opened_orders }
        };
        let price_level = opened_orders.entry(strategy_order.get_open_price()).or_insert_with(Vec::new);
        price_level.push(strategy_order.get_id());
        Ok(())
    }

    /// 将订单移出opened_orders
    fn remove_from_opened_orders(&mut self, strategy_order_id: &Uuid) -> RStrategyOrderManagerResult<()> {
        let strategy_order = self.peek_by_id(strategy_order_id)?.clone();
        let mut opened_orders = match strategy_order.get_direction() {
            EOrderDirection::Long => { &mut self.long_opened_orders }
            EOrderDirection::Short => { &mut self.short_opened_orders }
        };
        let price_level = opened_orders.get_mut(&strategy_order.get_open_price());
        if let Some(level) = price_level {
            level.retain(|&id| id != *strategy_order_id);
            if level.is_empty() {
                opened_orders.remove(&strategy_order.get_open_price());
            }
        }
        Ok(())
    }

    // region --- 转发SStrategyOrder函数
    /// 取消开单 删除结构体自身（通过外部方式自动析构）
    pub fn cancel_open_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerResult<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        Ok(strategy_order.cancel_open())
    }

    /// 开单结算
    /// 需要将订单写入opened_orders
    pub fn opened_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerResult<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let result = strategy_order.opened();
        let strategy_order_id = strategy_order.get_id();
        if let Ok(()) = result {
            if let Err(e) = self.push_into_opened_orders(&strategy_order_id) {
                error!("{:?}", e);
            }
        }
        Ok(result)
    }

    /// 绑定平单
    /// 需要将订单移出opened_orders
    pub fn bind_close_by_order_id(&mut self, order_id: &Uuid, closing_order: &SOrder) -> RStrategyOrderManagerResult<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let result = strategy_order.bind_close(closing_order);
        let strategy_order_id = strategy_order.get_id();
        if let Ok(()) = result {
            if let Err(e) = self.remove_from_opened_orders(&strategy_order_id) {
                error!("{:?}", e);
            }
        }
        Ok(result)
    }

    /// 平单结算
    pub fn closed_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerResult<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        Ok(strategy_order.closed())
    }

    /// 取消平单
    /// 需要将订单写入opened_orders
    pub fn cancel_close_by_order_id(&mut self, order_id: &Uuid) -> RStrategyOrderManagerResult<RStrategyOrderResult<()>> {
        let strategy_order = self.peek_mut_by_order_id(order_id)?;
        let result = strategy_order.cancel_close();
        let strategy_order_id = strategy_order.get_id();
        if let Ok(()) = result {
            if let Err(e) = self.push_into_opened_orders(&strategy_order_id) {
                error!("{:?}", e);
            }
        }
        Ok(result)
    }
    // end region --- 转发SStrategyOrder函数

    ///  基于特定限定(Long/Short Lowest/Highest) 查看第一个已开仓待平仓订单 获取其引用
    fn peek_conditioned_first_opened_order(&self, is_long: bool, is_highest: bool) -> RStrategyOrderManagerResult<Option<&SStrategyOrder>> {
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
                    None => { Err(EStrategyOrderManagerError::OrdersUuidVecEmptyError(direction, price.clone())) }
                    Some(uuid) => {
                        match self.peek_by_id(uuid) {
                            Err(_) => { Err(EStrategyOrderManagerError::OrdersUuidCannotFindInOrdersError(direction, price.clone(), uuid.clone())) }
                            Ok(order) => { Ok(Some(order)) }
                        }
                    }
                }
            }
        }
    }

    /// 基于特定限定(Long/Short Lowest/Highest) 获取第一个已开仓待平仓订单
    fn pop_conditioned_first_opened_order(&mut self, is_long: bool, is_highest: bool) -> RStrategyOrderManagerResult<Option<SStrategyOrder>> {
        let id = match self.peek_conditioned_first_opened_order(is_long, is_highest)? {
            None => { None }
            Some(order) => { Some(order.get_id()) }
        };
        match id {
            None => { Ok(None) }
            Some(uuid) => {
                // todo 从open_orders中删除
                let order_set = if is_long { &self.long_opened_orders } else { &self.short_opened_orders };
                Ok(self.pop_by_id(&uuid))
            }
        }
    }

    ///  查看价格最高的做多已开仓待平仓订单 获取其引用
    pub fn peek_highest_long_opened_order(&self) -> RStrategyOrderManagerResult<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(true, true)
    }

    ///  获取价格最高的做多已开仓待平仓订单
    pub fn pop_highest_long_opened_order(&mut self) -> RStrategyOrderManagerResult<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(true, true)
    }

    ///  查看价格最低的做多已开仓待平仓订单 获取其引用
    pub fn peek_lowest_long_opened_order(&self) -> RStrategyOrderManagerResult<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(true, false)
    }

    ///  获取价格最低的做多已开仓待平仓订单
    pub fn pop_lowest_long_opened_order(&mut self) -> RStrategyOrderManagerResult<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(true, false)
    }

    ///  查看价格最高的做空已开仓待平仓订单 获取其引用
    pub fn peek_highest_short_opened_order(&self) -> RStrategyOrderManagerResult<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(false, true)
    }

    ///  获取价格最高的做空已开仓待平仓订单
    pub fn pop_highest_short_opened_order(&mut self) -> RStrategyOrderManagerResult<Option<SStrategyOrder>> {
        self.pop_conditioned_first_opened_order(false, true)
    }

    ///  查看价格最低的做空已开仓待平仓订单 获取其引用
    pub fn peek_lowest_short_opened_order(&self) -> RStrategyOrderManagerResult<Option<&SStrategyOrder>> {
        self.peek_conditioned_first_opened_order(false, false)
    }

    ///  获取价格最低的做空已开仓待平仓订单
    pub fn pop_lowest_short_opened_order(&mut self) -> RStrategyOrderManagerResult<Option<SStrategyOrder>> {
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
    use crate::strategy::order::order_manager::SStrategyOrderManager;

    pub fn get_test_data_origin(direction: EOrderDirection) -> (Vec<Uuid>, SOrderManager, SStrategyOrderManager) {
        let mut strategy_manager = SStrategyOrderManager::new();
        let mut manager = SOrderManager::new();

        let price_vec = vec![
            (Decimal::from_str("400").unwrap(), Decimal::from_f64(0.4).unwrap()),
            (Decimal::from_str("200").unwrap(), Decimal::from_f64(0.2).unwrap()),
            (Decimal::from_str("100").unwrap(), Decimal::from_f64(0.1).unwrap()),
            (Decimal::from_str("300").unwrap(), Decimal::from_f64(0.3).unwrap()),
            (Decimal::from_str("300").unwrap(), Decimal::from_f64(0.33).unwrap()),
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

    pub fn get_test_data_opened(direction: EOrderDirection) -> (Vec<Uuid>, SOrderManager, SStrategyOrderManager) {
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
        assert_eq!(*key, Decimal::from(100));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(200));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(300));
        assert_eq!(value.len(), 2);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(400));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(500));
        assert_eq!(value.len(), 1);
        let (key, value) = iter.next().unwrap();
        assert_eq!(*key, Decimal::from(600));
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

        let r = strategy_manager.long_opened_orders.get(&open_order.get_price());
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

        let r = strategy_manager.long_opened_orders.get(&open_order.get_price());
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
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));

        let strategy_order = strategy_manager.peek_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));

        // peek
        let r = strategy_manager.peek_highest_long_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        let r = strategy_manager.peek_lowest_long_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));

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
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));

        let r = strategy_manager.peek_highest_long_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));

        let r = strategy_manager.pop_highest_long_opened_order();
        assert!(matches!(r, Ok(_)));
        let strategy_order  = r.unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));
        let strategy_order  = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        let strategy_order  = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(200));
        let strategy_order  = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        let strategy_order  = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        let strategy_order  = strategy_manager.pop_lowest_long_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(400));

        // pop fail
        let r  = strategy_manager.pop_lowest_long_opened_order().unwrap();
        assert!(matches!(r, None));
        let r  = strategy_manager.pop_highest_long_opened_order().unwrap();
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
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));

        let strategy_order = strategy_manager.peek_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));

        // peek
        let r = strategy_manager.peek_highest_short_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));
        let r = strategy_manager.peek_lowest_short_opened_order().unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));

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
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(600));

        let r = strategy_manager.peek_highest_short_opened_order();
        assert!(matches!(r, Ok(_)));
        let r = r.unwrap();
        assert!(matches!(r, Some(_)));
        let strategy_order  = r.unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));

        let r = strategy_manager.pop_highest_short_opened_order();
        assert!(matches!(r, Ok(_)));
        let strategy_order  = r.unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(500));
        let strategy_order  = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(100));
        let strategy_order  = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(200));
        let strategy_order  = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        let strategy_order  = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(300));
        let strategy_order  = strategy_manager.pop_lowest_short_opened_order().unwrap().unwrap();
        assert_eq!(strategy_order.get_open_price(), Decimal::from(400));

        // pop fail
        let r  = strategy_manager.pop_lowest_short_opened_order().unwrap();
        assert!(matches!(r, None));
        let r  = strategy_manager.pop_highest_short_opened_order().unwrap();
        assert!(matches!(r, None));

        // peek fail
        let r = strategy_manager.peek_highest_short_opened_order().unwrap();
        assert!(matches!(r, None));
        let r = strategy_manager.peek_lowest_short_opened_order().unwrap();
        assert!(matches!(r, None));
    }
}