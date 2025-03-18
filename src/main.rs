use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

/// 订单方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderDirection {
    Buy,
    Sell,
}

/// 订单结构
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Order {
    id: u64,
    price: u64,
    direction: OrderDirection,
}

/// 买单排序（最大堆）
impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price.cmp(&other.price) // 默认升序
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Manager 订单管理器
struct Manager {
    orders: Vec<Order>,                      // 订单存储
    order_map: HashMap<u64, usize>,          // id -> orders索引
    buy_orders: BinaryHeap<Order>,           // 价格降序买单（最大堆）
    sell_orders: BinaryHeap<std::cmp::Reverse<Order>>, // 价格升序卖单（最小堆）
}

impl Manager {
    /// 创建新的 Manager
    fn new() -> Self {
        Self {
            orders: Vec::new(),
            order_map: HashMap::new(),
            buy_orders: BinaryHeap::new(),
            sell_orders: BinaryHeap::new(),
        }
    }

    /// 添加订单
    fn add_order(&mut self, id: u64, price: u64, direction: OrderDirection) {
        let order = Order { id, price, direction };
        self.orders.push(order);
        let index = self.orders.len() - 1;
        self.order_map.insert(id, index);

        match direction {
            OrderDirection::Buy => self.buy_orders.push(order),
            OrderDirection::Sell => self.sell_orders.push(std::cmp::Reverse(order)),
        }
    }

    /// 根据订单 ID 查找订单
    fn get_order(&self, id: u64) -> Option<&Order> {
        self.order_map.get(&id).map(|&index| &self.orders[index])
    }

    /// 获取最高买单
    fn get_highest_buy_order(&self) -> Option<Order> {
        self.buy_orders.peek().copied()
    }

    /// 获取最低卖单
    fn get_lowest_sell_order(&self) -> Option<Order> {
        self.sell_orders.peek().map(|r| r.0)
    }

    /// 删除最高买单
    fn remove_highest_buy_order(&mut self) {
        if let Some(order) = self.buy_orders.pop() {
            if let Some(&index) = self.order_map.get(&order.id) {
                self.orders.swap_remove(index);
                self.order_map.remove(&order.id);
            }
        }
    }

    /// 删除最低卖单
    fn remove_lowest_sell_order(&mut self) {
        if let Some(order) = self.sell_orders.pop().map(|r| r.0) {
            if let Some(&index) = self.order_map.get(&order.id) {
                self.orders.swap_remove(index);
                self.order_map.remove(&order.id);
            }
        }
    }

    /// 修改订单价格
    fn update_order_price(&mut self, id: u64, new_price: u64) {
        if let Some(&index) = self.order_map.get(&id) {
            let order = &mut self.orders[index];
            order.price = new_price;

            // 重新添加到堆
            match order.direction {
                OrderDirection::Buy => {
                    self.buy_orders.push(*order);
                }
                OrderDirection::Sell => {
                    self.sell_orders.push(std::cmp::Reverse(*order));
                }
            }
        }
    }
}

fn main() {
    let mut manager = Manager::new();

    // 添加订单
    manager.add_order(1, 100, OrderDirection::Buy);
    manager.add_order(2, 120, OrderDirection::Buy);
    manager.add_order(3, 80, OrderDirection::Sell);
    manager.add_order(4, 90, OrderDirection::Sell);

    // 获取最高买单 & 最低卖单
    println!("Highest Buy Order: {:?}", manager.get_highest_buy_order());
    println!("Lowest Sell Order: {:?}", manager.get_lowest_sell_order());

    // 修改订单价格
    manager.update_order_price(1, 130);
    println!("Updated Highest Buy Order: {:?}", manager.get_highest_buy_order());

    // 删除最高买单 & 最低卖单
    manager.remove_highest_buy_order();
    manager.remove_lowest_sell_order();

    println!("After Removal - Highest Buy Order: {:?}", manager.get_highest_buy_order());
    println!("After Removal - Lowest Sell Order: {:?}", manager.get_lowest_sell_order());
}
