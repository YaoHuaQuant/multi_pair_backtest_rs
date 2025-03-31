use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap, HashSet};
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::asset::EAssetType;
use crate::order::EOrderAction;
use crate::runner::strategy_runner::order::order::{ROrderResult, SAddOrder, SOrder};

pub type ROrderManagerResult<T> = Result<T, EOrderManagerError>;

/// 订单管理器异常
#[derive(Debug)]
pub enum EOrderManagerError {
    UuidNotFound(Uuid),
    InsertOrderFail(SOrder),
    // todo
}

// /// 订单管理器更新
// #[derive(Debug, Copy, Clone)]
// pub enum EOrderManagerUpdate {
//     Update(EOrderUpdate),
//     Remove,
// }

// ----- 为订单提供排序功能 -----
impl PartialEq<Self> for SOrder {
    fn eq(&self, other: &Self) -> bool {
        self.get_id().eq(&other.get_id())
    }
}

impl Eq for SOrder {}

impl Ord for SOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_price().cmp(&other.get_price())
    }
}

impl PartialOrd for SOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 订单管理器
#[derive(Debug, Default)]
pub struct SOrderManager {
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

// /// 保存uuid和EOrderManagerUpdate的映射
// #[derive(Debug)]
// pub struct SOrderUuidAndUpdate {
//     pub uuid: Uuid,
//     pub update: EOrderManagerUpdate,
// }

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
    pub fn add_order(&mut self, add_order: SAddOrder) -> ROrderManagerResult<Uuid> {
        let action = add_order.action;
        let price = add_order.price;
        let quantity = add_order.quantity;
        let order = SOrder::new(price, quantity, action);
        self.insert_order(order)?;
        Ok(order.get_id())
    }

    /// 直接插入一个Order对象
    pub fn insert_order(&mut self, order: SOrder) -> ROrderManagerResult<()> {
        match self.orders.insert(order.get_id(), order) {
            None => {
                // 插入成功的场景
                match order.get_action() {
                    EOrderAction::Buy => {
                        self.buying_order_heap.push(order);
                    }
                    EOrderAction::Sell => {
                        self.selling_order_heap.push(Reverse(order));
                    }
                }
                Ok(())
            }
            Some(order) => {
                // 插入失败的场景
                Err(EOrderManagerError::InsertOrderFail(order))
            }
        }
    }

    /// 查询订单
    /// 根据id从订单池中查询
    pub fn peek_order(&self, uuid: Uuid) -> Option<&SOrder> {
        self.orders.get(&uuid)
    }

    // ///（非pub）更新单个订单 不做堆重构
    // fn update_order(&mut self, uuid: Uuid, update: EOrderUpdate) -> ROrderManagerResult<()> {
    //     let order = self.orders.get_mut(&uuid);
    //     match order {
    //         None => {
    //             Err(EOrderManagerError::UuidNotFound(uuid))
    //         }
    //         Some(order) => {
    //             order.update(update);
    //             Ok(())
    //         }
    //     }
    // }

    /// （非pub）删除订单 不做堆重构
    fn remove_order(&mut self, uuid: Uuid) -> Option<SOrder> {
        self.orders.remove(&uuid)
    }

    /// （pub）删除多个订单 需要做堆重构
    pub fn remove_orders(&mut self, uuid_list: Vec<Uuid>) -> ROrderManagerResult<Vec<SOrder>> {
        // 标记需要从堆中剔除的元素
        let mut buy_delete_set = HashSet::new();
        let mut sell_delete_set = HashSet::new();

        // 保存执行成功的订单 用于返回
        let mut removed_order_vec: Vec<SOrder> = Vec::new();

        for uuid in uuid_list {
            if let Some(order) = self.remove_order(uuid) {
                match order.get_action() {
                    EOrderAction::Buy => { buy_delete_set.insert(uuid) }
                    EOrderAction::Sell => { sell_delete_set.insert(uuid) }
                };
                removed_order_vec.push(order);
            }
        }

        // 堆重构
        if !buy_delete_set.is_empty() {
            let new_heap = self.buying_order_heap.clone().into_iter();
            self.buying_order_heap.clear();
            for item in new_heap {
                if !buy_delete_set.contains(&item.get_id()) {
                    self.buying_order_heap.push(item);
                }
            }
        }
        if !sell_delete_set.is_empty() {
            let new_heap = self.selling_order_heap.clone().into_iter();
            self.selling_order_heap.clear();
            for Reverse(item) in new_heap {
                if !sell_delete_set.contains(&item.get_id()) {
                    self.selling_order_heap.push(Reverse(item));
                }
            }
        }
        Ok(removed_order_vec)
    }

    // /// （pub）更新或删除多个订单 需要做堆重构
    // /// 如果订单价格被修改 或者 订单被删除 则需要做堆重构
    // pub fn update_or_remove_orders(&mut self, uuid_update_list: Vec<SOrderUuidAndUpdate>) -> ROrderManagerResult<Vec<SOrderUuidAndUpdate>> {
    //     // 标记需要从堆中剔除的元素
    //     let mut buy_delete_set = HashSet::new();
    //     let mut sell_delete_set = HashSet::new();
    //
    //     // 保存执行成功的订单 用于返回
    //     let mut success_list: Vec<SOrderUuidAndUpdate> = Vec::new();
    //
    //     for item in uuid_update_list {
    //         let SOrderUuidAndUpdate { uuid, update: manager_update } = item;
    //         // 变更订单
    //         match manager_update {
    //             EOrderManagerUpdate::Update(order_update) => {
    //                 let result = self.update_order(uuid, order_update);
    //                 match result {
    //                     Err(err) => {
    //                         error!("{:?}", err);
    //                     }
    //                     Ok(_) => {
    //                         if let EOrderUpdate::Price(_) = order_update {
    //                             match self.peek_order(uuid) {
    //                                 None => {}
    //                                 Some(order) => {
    //                                     match order.get_action() {
    //                                         EOrderAction::Buy => {
    //                                             buy_delete_set.insert(uuid);
    //                                         }
    //                                         EOrderAction::Sell => {
    //                                             sell_delete_set.insert(uuid);
    //                                         }
    //                                     }
    //                                 }
    //                             }
    //                             success_list.push(SOrderUuidAndUpdate {
    //                                 uuid,
    //                                 update: EOrderManagerUpdate::Update(order_update),
    //                             })
    //                         }
    //                     }
    //                 }
    //             }
    //             EOrderManagerUpdate::Remove => {
    //                 if !self.remove_order(&uuid).is_none() {
    //                     success_list.push(SOrderUuidAndUpdate {
    //                         uuid,
    //                         update: EOrderManagerUpdate::Remove,
    //                     })
    //                 }
    //             }
    //         };
    //     }
    //
    //     // 堆重构
    //     if !buy_delete_set.is_empty() {
    //         let new_heap = self.buying_order_heap.clone().into_iter();
    //         self.buying_order_heap.clear();
    //         for item in new_heap {
    //             if buy_delete_set.contains(&item.get_id()) {
    //                 self.buying_order_heap.push(self.peek_order(item.get_id()).unwrap().clone());
    //             } else {
    //                 self.buying_order_heap.push(item);
    //             }
    //         }
    //     }
    //     if !sell_delete_set.is_empty() {
    //         let new_heap = self.selling_order_heap.clone().into_iter();
    //         self.selling_order_heap.clear();
    //         for Reverse(item) in new_heap {
    //             if sell_delete_set.contains(&item.get_id()) {
    //                 self.selling_order_heap.push(Reverse(self.peek_order(item.get_id()).unwrap().clone()));
    //             } else {
    //                 self.selling_order_heap.push(Reverse(item));
    //             }
    //         }
    //     }
    //     Ok(success_list)
    // }

    /// 查看最高价的买单 获取其引用
    pub fn peek_highest_buy_order(&self) -> Option<&SOrder> {
        self.buying_order_heap.peek()
    }

    /// 获取最高价的买单
    pub fn pop_highest_buy_order(&mut self) -> Option<SOrder> {
        match self.buying_order_heap.pop() {
            None => { None }
            Some(order) => {
                self.remove_order(order.get_id());
                Some(order)
            }
        }
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
                self.remove_order(order.get_id());
                Some(order)
            }
        }
    }

    /// 统计每种资产的总锁定量
    pub fn calculate_total_assets(&self) -> HashMap<EAssetType, Decimal> {
        let mut result = HashMap::new();
        for (_, order) in self.orders.iter() {
            if let Some(asset) = order.get_asset() {
                let balance = result.entry(asset.as_type).or_insert(Decimal::from(0));
                *balance += asset.balance;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Reverse;
    use std::str::FromStr;
    use rust_decimal::Decimal;
    use uuid::Uuid;
    use crate::asset::asset_v2::SAssetV2;
    use crate::asset::EAssetType;
    use crate::order::EOrderAction;
    use crate::runner::strategy_runner::order::order::{SAddOrder, SOrder};
    use crate::runner::strategy_runner::order::order_manager::{SOrderManager};

    fn get_test_data(action: EOrderAction) -> (SOrderManager, Vec<Uuid>) {
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
            let id = manager.add_order(SAddOrder {
                action,
                price,
                quantity: Decimal::from_str("0.01").unwrap(),
            });
            id_vec.push(id.unwrap())
        }
        (manager, id_vec)
    }

    #[test]
    pub fn test_remove_orders_buy() {
        let (mut manager, id_vec) = get_test_data(EOrderAction::Buy);

        let id_for_change = id_vec.get(2).unwrap().clone();
        println!("id_for_change: {}", &id_for_change);

        println!("Before update:");
        dbg!(&manager);

        let remove_list = vec![id_for_change];

        let result = manager.remove_orders(remove_list);
        assert!(result.is_ok());
        println!("removed list: {:?}", result.unwrap());

        println!("After update:");
        dbg!(&manager);

        println!("Pop result:");
        let mut pop_vec: Vec<SOrder> = vec![];
        while !manager.buying_order_heap.is_empty() {
            let order = manager.buying_order_heap.pop().unwrap();
            println!("{:?}", &order);
            pop_vec.push(order);
        }

        assert_eq!(pop_vec.get(0).unwrap().get_price(), Decimal::from_str("600").unwrap());
        assert_eq!(pop_vec.get(1).unwrap().get_price(), Decimal::from_str("500").unwrap());
        assert_eq!(pop_vec.get(2).unwrap().get_price(), Decimal::from_str("400").unwrap());
        assert_eq!(pop_vec.get(3).unwrap().get_price(), Decimal::from_str("200").unwrap());
        assert_eq!(pop_vec.get(4).unwrap().get_price(), Decimal::from_str("100").unwrap());
    }

    #[test]
    pub fn test_remove_orders_sell() {
        let (mut manager, id_vec) = get_test_data(EOrderAction::Sell);

        let id_for_change = id_vec.get(2).unwrap().clone();
        println!("id_for_change: {}", &id_for_change);

        println!("Before update:");
        dbg!(&manager);

        let remove_list = vec![id_for_change];

        let result = manager.remove_orders(remove_list);
        assert!(result.is_ok());
        println!("removed list: {:?}", result.unwrap());

        println!("After update:");
        dbg!(&manager);

        println!("Pop result:");
        let mut pop_vec: Vec<SOrder> = vec![];
        while !manager.selling_order_heap.is_empty() {
            let Reverse(order) = manager.selling_order_heap.pop().unwrap();
            println!("{:?}", &order);
            pop_vec.push(order);
        }

        assert_eq!(pop_vec.get(0).unwrap().get_price(), Decimal::from_str("100").unwrap());
        assert_eq!(pop_vec.get(1).unwrap().get_price(), Decimal::from_str("200").unwrap());
        assert_eq!(pop_vec.get(2).unwrap().get_price(), Decimal::from_str("400").unwrap());
        assert_eq!(pop_vec.get(3).unwrap().get_price(), Decimal::from_str("500").unwrap());
        assert_eq!(pop_vec.get(4).unwrap().get_price(), Decimal::from_str("600").unwrap());
    }

    #[test]
    pub fn test_calculate_total_assets() {
        let mut manager = SOrderManager::new();

        let price_vec_buy = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("2").unwrap(),
            Decimal::from_str("3").unwrap(),
            Decimal::from_str("5").unwrap(),
        ];

        for price in price_vec_buy {
            let id = manager.add_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let mut order = manager.orders.get_mut(&id).unwrap();
            let asset = SAssetV2 {
                as_type: EAssetType::Usdt,
                balance: Decimal::from(price),
            };
            let r = order.submit(asset);
        }

        let price_vec_sell = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
        ];

        for price in price_vec_sell {
            let id = manager.add_order(SAddOrder {
                action: EOrderAction::Sell,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let mut order = manager.orders.get_mut(&id).unwrap();
            let asset = SAssetV2 {
                as_type: EAssetType::Btc,
                balance: Decimal::from(price),
            };
            let r = order.submit(asset);
        }

        let r = manager.calculate_total_assets();
        println!("result:{:?}", r);
        let usdt = r.get(&EAssetType::Usdt);
        assert!(usdt.is_some());
        assert_eq!(*usdt.unwrap(), Decimal::from(12));

        let btc = r.get(&EAssetType::Btc);
        assert!(btc.is_some());
        assert_eq!(*btc.unwrap(), Decimal::from(5));
    }

    #[test]
    pub fn test_buy_pop() {
        let mut manager = SOrderManager::new();

        let price_vec_buy = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("2").unwrap(),
            Decimal::from_str("3").unwrap(),
            Decimal::from_str("5").unwrap(),
        ];

        for price in price_vec_buy {
            let id = manager.add_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let mut order = manager.orders.get_mut(&id).unwrap();
            let asset = SAssetV2 {
                as_type: EAssetType::Usdt,
                balance: Decimal::from(price),
            };
            let r = order.submit(asset);
        }
        dbg!(&manager);

        let r1 = manager.calculate_total_assets();
        println!("result:{:?}", r1);
        let usdt = r1.get(&EAssetType::Usdt);
        assert!(usdt.is_some());
        assert_eq!(*usdt.unwrap(), Decimal::from(12));

        let r2 = manager.pop_highest_buy_order().unwrap();
        assert_eq!(r2.get_asset().unwrap().balance, Decimal::from_str("5").unwrap());
        assert_eq!(*r1.get(&EAssetType::Usdt).unwrap(), Decimal::from(7));
    }
}