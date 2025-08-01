use std::collections::{BTreeMap, HashMap};

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::data_runtime::asset::asset_map::SAssetMap;
use crate::data_runtime::asset::asset_map_v3::SAssetMapV3;
use crate::data_runtime::asset::asset_union::EAssetUnion;
use crate::data_runtime::order::EOrderAction;
use crate::data_runtime::order::order_v3::{EOrderState, SAddOrder, SOrderV3};
use crate::data_source::trading_pair::ETradingPairType;

pub type ROrderManagerV3Result<T> = Result<T, EOrderManagerV3Error>;

/// 订单管理器异常
#[derive(Debug)]
pub enum EOrderManagerV3Error {
    /// 找不到Uuid对应的订单
    UuidNotFound(Uuid),
    /// 插入订单失败
    InsertOrderFail(SOrderV3),
    /// BuyOrders/SellOrders订单索引 在某个价格的UuidVec为空
    OrdersUuidVecEmptyError(EOrderAction, Decimal),
    /// BuyOrders/SellOrders订单索引中的Uuid无法在Orders上找到
    OrdersUuidCannotFindInOrdersError(EOrderAction, Decimal, Uuid),
    /// 插入已完成的订单时 订单状态错误
    InsertFinishedOrderStateError(EOrderState),
}

/// 订单管理器
#[derive(Debug)]
pub struct SOrderManagerV3 {
    /// 交易对类型
    pub tp_type: ETradingPairType,
    
    /// 订单池
    pub orders: HashMap<Uuid, SOrderV3>,

    /// 买单索引集合 K-价格 V-订单uuid列表
    pub buy_orders: BTreeMap<Decimal, Vec<Uuid>>,

    /// 卖单索引集合 K-价格 V-订单uuid列表
    pub sell_orders: BTreeMap<Decimal, Vec<Uuid>>,

    /// 累计手续费
    pub total_fee_asset_map: SAssetMap,
}

impl SOrderManagerV3 {
    pub fn new(tp_type: ETradingPairType) -> Self {
        Self {
            tp_type,
            orders: Default::default(),
            buy_orders: Default::default(),
            sell_orders: Default::default(),
            total_fee_asset_map: Default::default(),
        }
    }

    /// 直接插入一个Order对象
    /// 同时调整索引
    pub fn insert_order(&mut self, order: SOrderV3) -> ROrderManagerV3Result<()> {
        match self.orders.insert(order.get_id(), order.clone()) {
            None => {
                // 插入成功的场景
                let price_level = match order.get_action() {
                    EOrderAction::Buy => self.buy_orders.entry(order.get_price()).or_insert_with(Vec::new),
                    EOrderAction::Sell => self.sell_orders.entry(order.get_price()).or_insert_with(Vec::new),
                };
                price_level.push(order.get_id());
                Ok(())
            }
            Some(order) => {
                // 插入失败的场景
                Err(EOrderManagerV3Error::InsertOrderFail(order))
            }
        }
    }

    /// 添加订单
    /// 直接构筑一个对象 并调用insert_order
    pub fn add_new_order(&mut self, add_order: SAddOrder) -> ROrderManagerV3Result<Uuid> {
        let action = add_order.action;
        let price = add_order.price;
        let quantity = add_order.quantity;
        let order = SOrderV3::new(self.tp_type, price, quantity, action);
        let id = order.get_id();
        self.insert_order(order)?;
        Ok(id)
    }

    /// 添加已完成的订单
    pub fn add_finished_order(&mut self, order: SOrderV3) -> ROrderManagerV3Result<()> {
        let state = order.get_state();
        match state {
            EOrderState::Executed => {
                if let Some(fee) =  order.get_paid_fee_asset() {
                    self.total_fee_asset_map.merge_asset(fee.clone())
                }
                Ok(())
            }
            _ => { Err(EOrderManagerV3Error::InsertFinishedOrderStateError(state)) }
        }
    }

    /// 查询订单
    /// 根据id从订单池中查询
    pub fn peek_order(&self, uuid: &Uuid) -> Option<&SOrderV3> {
        self.orders.get(uuid)
    }

    /// 删除订单 同时调整索引
    pub fn remove_order(&mut self, uuid: Uuid) -> Option<SOrderV3> {
        // 删除订单
        match self.orders.remove(&uuid) {
            None => { None }
            Some(order) => {
                // 调整索引
                let price_level = match order.get_action() {
                    EOrderAction::Buy => self.buy_orders.get_mut(&order.get_price()),
                    EOrderAction::Sell => self.sell_orders.get_mut(&order.get_price()),
                };
                if let Some(level) = price_level {
                    level.retain(|&id| id != uuid);
                    if level.is_empty() {
                        match order.get_action() {
                            EOrderAction::Buy => self.buy_orders.remove(&order.get_price()),
                            EOrderAction::Sell => self.sell_orders.remove(&order.get_price()),
                        };
                    }
                }
                Some(order)
            }
        }
    }

    /// 删除多个订单
    pub fn remove_orders(&mut self, uuid_list: Vec<Uuid>) -> ROrderManagerV3Result<Vec<SOrderV3>> {
        // 保存执行成功的订单 用于返回
        let mut removed_order_vec: Vec<SOrderV3> = Vec::new();
        for uuid in uuid_list {
            if let Some(order) = self.remove_order(uuid) {
                removed_order_vec.push(order);
            }
        }
        Ok(removed_order_vec)
    }

    /// 查看最高价的买单 获取其引用
    pub fn peek_highest_buy_order(&self) -> ROrderManagerV3Result<Option<&SOrderV3>> {
        match self.buy_orders.last_key_value() {
            None => { Ok(None) }
            Some((price, uuid_list)) => {
                match uuid_list.get(0) {
                    None => { Err(EOrderManagerV3Error::OrdersUuidVecEmptyError(EOrderAction::Buy, price.clone())) }
                    Some(uuid) => {
                        match self.peek_order(uuid) {
                            None => { Err(EOrderManagerV3Error::OrdersUuidCannotFindInOrdersError(EOrderAction::Buy, price.clone(), uuid.clone())) }
                            Some(order) => { Ok(Some(order)) }
                        }
                    }
                }
            }
        }
    }

    /// 获取最高价的买单
    pub fn pop_highest_buy_order(&mut self) -> ROrderManagerV3Result<Option<SOrderV3>> {
        let id = match self.peek_highest_buy_order()? {
            None => { None }
            Some(order) => { Some(order.get_id()) }
        };
        match id {
            None => { Ok(None) }
            Some(uuid) => {
                Ok(self.remove_order(uuid))
            }
        }
    }

    /// 查看最低价的卖单 获取其引用
    pub fn peek_lowest_sell_order(&self) -> ROrderManagerV3Result<Option<&SOrderV3>> {
        match self.sell_orders.first_key_value() {
            None => { Ok(None) }
            Some((price, uuid_list)) => {
                match uuid_list.get(0) {
                    None => { Err(EOrderManagerV3Error::OrdersUuidVecEmptyError(EOrderAction::Sell, price.clone())) }
                    Some(uuid) => {
                        match self.peek_order(uuid) {
                            None => { Err(EOrderManagerV3Error::OrdersUuidCannotFindInOrdersError(EOrderAction::Sell, price.clone(), uuid.clone())) }
                            Some(order) => { Ok(Some(order)) }
                        }
                    }
                }
            }
        }
    }

    /// 获取最低价的卖单
    pub fn pop_lowest_sell_order(&mut self) -> ROrderManagerV3Result<Option<SOrderV3>> {
        let id = match self.peek_lowest_sell_order()? {
            None => { None }
            Some(order) => { Some(order.get_id()) }
        };
        match id {
            None => { Ok(None) }
            Some(uuid) => {
                Ok(self.remove_order(uuid))
            }
        }
    }

    /// 统计每种资产的总锁定量
    pub fn calculate_total_assets(&self) -> SAssetMapV3 {
        let mut result = SAssetMapV3::new();
        for (_, order) in self.orders.iter() {
            if let Some(asset) = order.get_locked_asset() {
                result.merge_asset(EAssetUnion::from(asset.clone()))
            }
        }
        result
    }

    /// 统计总手续费量
    pub fn calculate_total_fee(&self) -> SAssetMap {
        self.total_fee_asset_map.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;
    use uuid::Uuid;

    use crate::data_runtime::asset::asset::SAsset;
    use crate::data_runtime::asset::asset_union::EAssetUnion;
    use crate::data_runtime::asset::EAssetType;
    use crate::data_runtime::order::EOrderAction;
    use crate::data_runtime::order::order_v3::{SAddOrder, SOrderV3};
    use crate::data_runtime::order::order_manager_v3::SOrderManagerV3;
    use crate::data_source::trading_pair::ETradingPairType;

    fn get_test_data(action: EOrderAction) -> (SOrderManagerV3, Vec<Uuid>) {
        let mut manager = SOrderManagerV3::new(ETradingPairType::BtcUsdt);

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
            let id = manager.add_new_order(SAddOrder {
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
        let mut pop_vec: Vec<SOrderV3> = vec![];
        let mut order = manager.pop_highest_buy_order().unwrap();
        while order.is_some() {
            let order2 = order.unwrap();
            println!("{:?}", &order2);
            pop_vec.push(order2);
            order = manager.pop_highest_buy_order().unwrap();
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
        let mut pop_vec: Vec<SOrderV3> = vec![];
        let mut order = manager.pop_lowest_sell_order().unwrap();
        while order.is_some() {
            let order2 = order.unwrap();
            println!("{:?}", &order2);
            pop_vec.push(order2);
            order = manager.pop_lowest_sell_order().unwrap();
        }

        assert_eq!(pop_vec.get(0).unwrap().get_price(), Decimal::from_str("100").unwrap());
        assert_eq!(pop_vec.get(1).unwrap().get_price(), Decimal::from_str("200").unwrap());
        assert_eq!(pop_vec.get(2).unwrap().get_price(), Decimal::from_str("400").unwrap());
        assert_eq!(pop_vec.get(3).unwrap().get_price(), Decimal::from_str("500").unwrap());
        assert_eq!(pop_vec.get(4).unwrap().get_price(), Decimal::from_str("600").unwrap());
    }

    #[test]
    pub fn test_calculate_total_assets() {
        let mut manager = SOrderManagerV3::new(ETradingPairType::BtcUsdt);

        let price_vec_buy = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("2").unwrap(),
            Decimal::from_str("3").unwrap(),
            Decimal::from_str("5").unwrap(),
        ];

        for price in price_vec_buy {
            let id = manager.add_new_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let asset = SAsset {
                as_type: EAssetType::Usdt,
                balance: Decimal::from(price),
            };
            let _r = order.submit(asset);
        }

        let price_vec_sell = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
        ];

        for price in price_vec_sell {
            let id = manager.add_new_order(SAddOrder {
                action: EOrderAction::Sell,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let asset = SAsset {
                as_type: EAssetType::Btc,
                balance: Decimal::from(price),
            };
            let _r = order.submit(asset);
        }

        let r = manager.calculate_total_assets();
        println!("result:{:?}", r);
        let usdt = r.get(&EAssetType::Usdt);
        assert!(usdt.is_ok());
        assert!(matches!(usdt, Ok(EAssetUnion::Usdt(_))));
        if let EAssetUnion::Usdt(asset) = usdt.unwrap() {
            let SAsset { balance, as_type } = asset;
            assert_eq!(*balance, Decimal::from(12));
            assert_eq!(*as_type, EAssetType::Usdt);
        }

        let btc = r.get(&EAssetType::Btc);
        assert!(btc.is_ok());
        assert!(matches!(btc, Ok(EAssetUnion::Btc(_))));
        if let EAssetUnion::Btc(asset) = btc.unwrap() {
            let SAsset { balance, as_type } = asset;
            assert_eq!(*balance, Decimal::from(5));
            assert_eq!(*as_type, EAssetType::Btc);
        }
    }

    #[test]
    pub fn test_buy_pop() {
        let mut manager = SOrderManagerV3::new(ETradingPairType::BtcUsdt);

        let price_vec_buy = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("2").unwrap(),
            Decimal::from_str("3").unwrap(),
            Decimal::from_str("5").unwrap(),
        ];

        for price in price_vec_buy {
            let id = manager.add_new_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let asset = SAsset {
                as_type: EAssetType::Usdt,
                balance: Decimal::from(price),
            };
            let _r = order.submit(asset);
        }
        dbg!(&manager);

        let r1 = manager.calculate_total_assets();
        println!("result:{:?}", r1);
        let usdt = r1.get(&EAssetType::Usdt);
        assert!(usdt.is_ok());
        assert_eq!(usdt.unwrap().get_balance(), Decimal::from(12));

        let r2 = manager.pop_highest_buy_order().unwrap().unwrap();
        let r1 = manager.calculate_total_assets();
        assert_eq!(r2.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("5").unwrap());
        assert_eq!(r1.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(7));
    }

    #[test]
    pub fn test_sell_pop() {
        let mut manager = SOrderManagerV3::new(ETradingPairType::BtcUsdt);

        let price_vec_sell = vec![
            Decimal::from_str("5").unwrap(),
            Decimal::from_str("5").unwrap(),
            Decimal::from_str("5").unwrap(),
            Decimal::from_str("9").unwrap(),
            Decimal::from_str("9").unwrap(),
        ];

        for price in price_vec_sell {
            let id = manager.add_new_order(SAddOrder {
                action: EOrderAction::Sell,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let asset = SAsset {
                as_type: EAssetType::Btc,
                balance: Decimal::from(price),
            };
            let r = order.submit(asset);
            assert!(r.is_ok())
        }
        let r1 = manager.calculate_total_assets();
        let btc = r1.get(&EAssetType::Btc);
        assert!(btc.is_ok());
        assert_eq!(btc.unwrap().get_balance(), Decimal::from(33));

        let r2 = manager.pop_lowest_sell_order().unwrap().unwrap();
        let r1 = manager.calculate_total_assets();
        assert_eq!(r2.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("5").unwrap());
        assert_eq!(r1.get(&EAssetType::Btc).unwrap().get_balance(), Decimal::from(28));
    }

    #[test]
    pub fn test_buy_sell_pop() {
        let mut manager = SOrderManagerV3::new(ETradingPairType::BtcUsdt);


        let price_vec_buy = vec![
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("1").unwrap(),
            Decimal::from_str("2").unwrap(),
            Decimal::from_str("3").unwrap(),
            Decimal::from_str("5").unwrap(),
        ];

        for price in price_vec_buy {
            let id = manager.add_new_order(SAddOrder {
                action: EOrderAction::Buy,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let asset = SAsset {
                as_type: EAssetType::Usdt,
                balance: Decimal::from(price),
            };
            let _r = order.submit(asset);
        }

        let price_vec_sell = vec![
            Decimal::from_str("5").unwrap(),
            Decimal::from_str("5").unwrap(),
            Decimal::from_str("5").unwrap(),
            Decimal::from_str("9").unwrap(),
            Decimal::from_str("9").unwrap(),
        ];

        for price in price_vec_sell {
            let id = manager.add_new_order(SAddOrder {
                action: EOrderAction::Sell,
                price,
                quantity: Decimal::from_str("1").unwrap(),
            }).unwrap();
            let order = manager.orders.get_mut(&id).unwrap();
            let asset = SAsset {
                as_type: EAssetType::Btc,
                balance: Decimal::from(price),
            };
            let _r = order.submit(asset);
        }
        dbg!(&manager);

        let asset_total = manager.calculate_total_assets();
        println!("result:{:?}", asset_total);
        let asset_btc = asset_total.get(&EAssetType::Btc);
        assert!(asset_btc.is_ok());
        assert_eq!(asset_btc.unwrap().get_balance(), Decimal::from(33));
        let asset_usdt = asset_total.get(&EAssetType::Usdt);
        assert!(asset_usdt.is_ok());
        assert_eq!(asset_usdt.unwrap().get_balance(), Decimal::from(12));

        let pop_sell = manager.pop_lowest_sell_order().unwrap().unwrap();
        let asset_total = manager.calculate_total_assets();
        assert_eq!(pop_sell.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("5").unwrap());
        assert_eq!(asset_total.get(&EAssetType::Btc).unwrap().get_balance(), Decimal::from(28));
        assert_eq!(asset_total.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(12));

        let pop_sell = manager.pop_lowest_sell_order().unwrap().unwrap();
        let asset_total = manager.calculate_total_assets();
        assert_eq!(pop_sell.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("5").unwrap());
        assert_eq!(asset_total.get(&EAssetType::Btc).unwrap().get_balance(), Decimal::from(23));
        assert_eq!(asset_total.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(12));

        let pop_sell = manager.pop_lowest_sell_order().unwrap().unwrap();
        let asset_total = manager.calculate_total_assets();
        assert_eq!(pop_sell.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("5").unwrap());
        assert_eq!(asset_total.get(&EAssetType::Btc).unwrap().get_balance(), Decimal::from(18));
        assert_eq!(asset_total.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(12));

        let pop_sell = manager.pop_lowest_sell_order().unwrap().unwrap();
        let asset_total = manager.calculate_total_assets();
        assert_eq!(pop_sell.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("9").unwrap());
        assert_eq!(asset_total.get(&EAssetType::Btc).unwrap().get_balance(), Decimal::from(9));
        assert_eq!(asset_total.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(12));

        let pop_sell = manager.pop_lowest_sell_order().unwrap().unwrap();
        let asset_total = manager.calculate_total_assets();
        assert_eq!(pop_sell.get_locked_asset().clone().unwrap().get_balance(), Decimal::from_str("9").unwrap());
        assert!(asset_total.get(&EAssetType::Btc).is_err());
        assert_eq!(asset_total.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(12));

        let pop_sell = manager.pop_lowest_sell_order().unwrap();
        let asset_total = manager.calculate_total_assets();
        assert!(pop_sell.is_none());
        assert!(asset_total.get(&EAssetType::Btc).is_err());
        assert_eq!(asset_total.get(&EAssetType::Usdt).unwrap().get_balance(), Decimal::from(12));
    }
}