use std::collections::HashMap;
use std::fmt;

// 定义订单类型的枚举
#[derive(Debug, Eq, PartialEq, Hash)]
enum OrderType {
    Buy,
    Sell,
    // 可以根据需要添加其他订单类型
}

// 为 OrderType 实现 Display trait，以便后续打印输出
impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Buy => write!(f, "Buy"),
            OrderType::Sell => write!(f, "Sell"),
            // 为新增的订单类型添加匹配分支
        }
    }
}

// 定义订单结构体
#[derive(Debug)]
struct Order {
    order_type: OrderType,
    balance: f64,
}

fn main() {
    // 创建示例订单数据
    let orders = vec![
        Order { order_type: OrderType::Buy, balance: 100.0 },
        Order { order_type: OrderType::Sell, balance: 150.0 },
        Order { order_type: OrderType::Buy, balance: 200.0 },
        Order { order_type: OrderType::Sell, balance: 50.0 },
        // 可以根据需要添加更多订单
    ];

    // 使用 HashMap 存储每种 order_type 的累计 balance
    let mut totals: HashMap<OrderType, f64> = HashMap::new();

    // 遍历订单，累加相同 order_type 的 balance
    for order in orders {
        let counter = totals.entry(order.order_type).or_insert(0.0);
        *counter += order.balance;
    }

    // 输出每种 order_type 的总 balance
    for (order_type, total_balance) in &totals {
        println!("订单类型: {}, 总余额: {:.2}", order_type, total_balance);
    }
}
