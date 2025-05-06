use std::collections::BTreeMap;

#[derive(Debug)]
pub struct StrategyOrder {
    pub open_price: i64,
    pub close_price: Option<i64>,
}

impl StrategyOrder {
    pub fn new(open_price: i64) -> Self {
        StrategyOrder {
            open_price,
            close_price: None,
        }
    }
}

pub struct StrategyOrderManager {
    orders: BTreeMap<i64, StrategyOrder>,
}

impl StrategyOrderManager {
    pub fn new() -> Self {
        StrategyOrderManager {
            orders: BTreeMap::new(),
        }
    }

    pub fn insert_order(&mut self, mut order: StrategyOrder) {
        // 收集所有已有订单的 close_price 值
        let existing_close_prices: Vec<i64> = self
            .orders
            .values()
            .filter_map(|o| o.close_price)
            .collect();

        // 如果没有已有订单，设置 close_price 为 open_price
        if existing_close_prices.is_empty() {
            order.close_price = Some(order.open_price);
        } else {
            // 查找最小的 n，使得 open_price + n 与所有已有 close_price 的差值都大于等于 1
            let mut n = 0;
            loop {
                let candidate = order.open_price + n;
                let conflict = existing_close_prices.iter().any(|&cp| (cp - candidate).abs() < 2);
                if !conflict {
                    order.close_price = Some(candidate);
                    break;
                }
                n += 1;
            }
        }

        // 将订单插入 BTreeMap
        self.orders.insert(order.open_price, order);
    }

    // 其他方法，例如遍历订单等
    pub fn iter(&self) -> impl Iterator<Item = &StrategyOrder> {
        self.orders.values()
    }
}

fn main() {
    let mut manager = StrategyOrderManager::new();

    let orders = vec![
        StrategyOrder::new(9),
        StrategyOrder::new(10),
        StrategyOrder::new(11),
        StrategyOrder::new(12),
        StrategyOrder::new(13),
    ];

    for order in orders {
        manager.insert_order(order);
    }

    for order in manager.iter() {
        println!(
            "Open: {:.2}, Close: {:.2}",
            order.open_price,
            order.close_price.unwrap()
        );
    }
}

