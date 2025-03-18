use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap, HashSet};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::order::EOrderAction;
use crate::runner::strategy_runner::runner_order::{EOrderUpdate, SOrder};

/// 订单管理器异常
#[derive(Debug)]
pub enum EOrderManagerError {
    UuidNotFound(Uuid)
    // todo
}

/// 订单管理器更新
pub enum EOrderManagerUpdate {
    Update(EOrderUpdate),
    Remove(),
}

// ----- 为订单提供排序功能 -----
impl PartialEq<Self> for SOrder {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Eq for SOrder {}

impl Ord for SOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price.cmp(&other.price)
    }
}

impl PartialOrd for SOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 订单管理器
#[derive(Debug)]
struct SOrderManager {
    // 订单池
    pub orders: HashMap<Uuid, SOrder>,

    // 买入订单堆 大顶堆
    pub buying_order_heap: BinaryHeap<SOrder>,

    // 卖出订单堆 小顶堆
    pub selling_order_heap: BinaryHeap<Reverse<SOrder>>,

    // 已买入订单列表
    pub bought_order_list: Vec<SOrder>,

    // 已卖出订单列表
    pub sold_order_list: Vec<SOrder>,
}

/// 保存uuid和EOrderManagerUpdate的映射
pub struct SOrderUuidAndUpdate {
    pub uuid: Uuid,
    pub update: EOrderManagerUpdate,
}

impl SOrderManager {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            buying_order_heap: BinaryHeap::new(),
            selling_order_heap: BinaryHeap::new(),
            bought_order_list: vec![],
            sold_order_list: vec![],
        }
    }

    /// 添加订单
    /// 分别添加至 订单池 和 买单堆/卖单堆 中
    pub fn add_order(&mut self, price: Decimal, quantity: Decimal, action: EOrderAction) -> Uuid {
        let order = SOrder {
            id: Uuid::new_v4(),
            state: Default::default(),
            action,
            price,
            quantity,
        };
        self.orders.insert(order.id, order);
        match action {
            EOrderAction::Buy => {
                self.buying_order_heap.push(order);
            }
            EOrderAction::Sell => {
                self.selling_order_heap.push(Reverse(order));
            }
        }
        order.id
    }

    /// 查询订单
    /// 根据id从订单池中查询
    pub fn peek_order(&self, uuid: Uuid) -> Option<&SOrder> {
        self.orders.get(&uuid)
    }

    ///（非pub）更新单个订单 不做堆重构
    fn update_order(&mut self, uuid: Uuid, update: EOrderUpdate) -> Result<(), EOrderManagerError> {
        let order = self.orders.get_mut(&uuid);
        match order {
            None => {
                Err(EOrderManagerError::UuidNotFound(uuid))
            }
            Some(order) => {
                order.update(update);
                Ok(())
            }
        }
    }

    /// （非pub）删除订单 不做堆重构
    fn remove_order(&mut self, uuid: &Uuid) -> Option<SOrder> {
        self.orders.remove(uuid)
    }

    /// （pub）更新或删除多个订单 需要做堆重构
    /// 如果订单价格被修改 或者 订单被删除 则需要做堆重构
    pub fn update_or_remove_orders(&mut self, uuid_update_list: Vec<SOrderUuidAndUpdate>) -> Result<(), EOrderManagerError> {
        // 标记需要从堆中剔除的元素
        let mut buy_delete_set = HashSet::new();
        let mut sell_delete_set = HashSet::new();

        for item in uuid_update_list {
            let SOrderUuidAndUpdate { uuid, update } = item;
            // 变更订单
            let _result = match update {
                EOrderManagerUpdate::Update(update) => {
                    let result = self.update_order(uuid, update);
                    match result {
                        Err(err) => {
                            eprintln!("{:?}", err);
                            Err(err)
                        }
                        Ok(_) => {
                            if let EOrderUpdate::Price(_) = update {
                                match self.peek_order(uuid) {
                                    None => {}
                                    Some(order) => {
                                        match order.action {
                                            EOrderAction::Buy => { buy_delete_set.insert(uuid); }
                                            EOrderAction::Sell => { sell_delete_set.insert(uuid); }
                                        }
                                    }
                                }
                            }
                            self.update_order(uuid, update)
                        }
                    }
                }
                EOrderManagerUpdate::Remove() => {
                    let _removed_order = self.remove_order(&uuid);
                    Ok(())
                }
            };
        }

        // 堆重构
        if !buy_delete_set.is_empty() {
            let new_heap = self.buying_order_heap.clone().into_iter();
            self.buying_order_heap.clear();
            for item in new_heap {
                if buy_delete_set.contains(&item.id) {
                    self.buying_order_heap.push(self.peek_order(item.id).unwrap().clone());
                } else {
                    self.buying_order_heap.push(item);
                }
            }
        }
        if !sell_delete_set.is_empty() {
            let new_heap = self.selling_order_heap.clone().into_iter();
            self.selling_order_heap.clear();
            for Reverse(item) in new_heap {
                if sell_delete_set.contains(&item.id) {
                    self.selling_order_heap.push(Reverse(self.peek_order(item.id).unwrap().clone()));
                } else {
                    self.selling_order_heap.push(Reverse(item));
                }
            }
        }
        Ok(())
    }

    /// 查看最高价的买单 获取其引用
    pub fn peek_highest_buy_order(&self) -> Option<&SOrder> {
        self.buying_order_heap.peek()
    }

    /// 获取最高价的买单
    pub fn pop_highest_buy_order(&mut self) -> Option<SOrder> {
        self.buying_order_heap.pop()
    }

    /// 查看最低价的卖单 获取其引用
    pub fn peek_lowest_sell_order(&self) -> Option<&SOrder> {
        match self.selling_order_heap.peek() {
            None => { None }
            Some(Reverse(order)) => {
                Some(order)
            }
        }
    }

    /// 获取最低价的卖单
    pub fn pop_lowest_sell_order(&mut self) -> Option<SOrder> {
        match self.selling_order_heap.pop() {
            None => { None }
            Some(Reverse(order)) => {
                Some(order)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Reverse;
    use std::str::FromStr;
    use rust_decimal::Decimal;
    use crate::order::EOrderAction;
    use crate::runner::strategy_runner::runner_order::EOrderUpdate::Price;
    use crate::runner::strategy_runner::runner_order::SOrder;
    use crate::runner::strategy_runner::runner_order_manager::{SOrderManager, SOrderUuidAndUpdate};
    use crate::runner::strategy_runner::runner_order_manager::EOrderManagerUpdate::Update;

    #[test]
    pub fn test_update_or_remove_orders_buy() {
        let mut manager = SOrderManager::new();

        let price_vec = vec![
            Decimal::from_str("100").unwrap(),
            Decimal::from_str("200").unwrap(),
            Decimal::from_str("300").unwrap(),
            Decimal::from_str("400").unwrap(),
            Decimal::from_str("500").unwrap(),
            Decimal::from_str("600").unwrap(),
        ];

        let mut id_vec = vec![];

        for price in price_vec {
            let id = manager.add_order(price, Decimal::from_str("0.01").unwrap(), EOrderAction::Buy);
            id_vec.push(id)
        }

        let id_for_change = id_vec.get(2).unwrap().clone();
        println!("id_for_change: {}", &id_for_change);

        println!("Before update:");
        dbg!(&manager);

        let update_list = vec![SOrderUuidAndUpdate {
            uuid: id_for_change,
            update: Update(Price(Decimal::from_str("150").unwrap())),
        }];

        let _result = manager.update_or_remove_orders(update_list);

        println!("After update:");
        dbg!(&manager);

        println!("Pop result:");
        let mut pop_vec:Vec<SOrder> = vec![];
        while !manager.buying_order_heap.is_empty() {
            let order = manager.buying_order_heap.pop().unwrap();
            println!("{:?}", &order);
            pop_vec.push(order);
        }

        assert_eq!(pop_vec.get(0).unwrap().price, Decimal::from_str("600").unwrap());
        assert_eq!(pop_vec.get(1).unwrap().price, Decimal::from_str("500").unwrap());
        assert_eq!(pop_vec.get(2).unwrap().price, Decimal::from_str("400").unwrap());
        assert_eq!(pop_vec.get(3).unwrap().price, Decimal::from_str("200").unwrap());
        assert_eq!(pop_vec.get(4).unwrap().price, Decimal::from_str("150").unwrap());
        assert_eq!(pop_vec.get(5).unwrap().price, Decimal::from_str("100").unwrap());
    }

    #[test]
    pub fn test_update_or_remove_orders_sell() {
        let mut manager = SOrderManager::new();

        let price_vec = vec![
            Decimal::from_str("100").unwrap(),
            Decimal::from_str("200").unwrap(),
            Decimal::from_str("300").unwrap(),
            Decimal::from_str("400").unwrap(),
            Decimal::from_str("500").unwrap(),
            Decimal::from_str("600").unwrap(),
        ];

        let mut id_vec = vec![];

        for price in price_vec {
            let id = manager.add_order(price, Decimal::from_str("0.01").unwrap(), EOrderAction::Sell);
            id_vec.push(id)
        }

        let id_for_change = id_vec.get(2).unwrap().clone();
        println!("id_for_change: {}", &id_for_change);

        println!("Before update:");
        dbg!(&manager);

        let update_list = vec![SOrderUuidAndUpdate {
            uuid: id_for_change,
            update: Update(Price(Decimal::from_str("150").unwrap())),
        }];

        let _result = manager.update_or_remove_orders(update_list);

        println!("After update:");
        dbg!(&manager);

        println!("Pop result:");
        let mut pop_vec:Vec<SOrder> = vec![];
        while !manager.selling_order_heap.is_empty() {
            let Reverse(order) = manager.selling_order_heap.pop().unwrap();
            println!("{:?}", &order);
            pop_vec.push(order);
        }

        assert_eq!(pop_vec.get(0).unwrap().price, Decimal::from_str("100").unwrap());
        assert_eq!(pop_vec.get(1).unwrap().price, Decimal::from_str("150").unwrap());
        assert_eq!(pop_vec.get(2).unwrap().price, Decimal::from_str("200").unwrap());
        assert_eq!(pop_vec.get(3).unwrap().price, Decimal::from_str("400").unwrap());
        assert_eq!(pop_vec.get(4).unwrap().price, Decimal::from_str("500").unwrap());
        assert_eq!(pop_vec.get(5).unwrap().price, Decimal::from_str("600").unwrap());
    }
}