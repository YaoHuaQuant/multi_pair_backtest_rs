use uuid::Uuid;

#[derive(Debug)]
struct Order {
    id: Uuid,
    // 其他字段
}

#[derive(Debug)]
struct OrderPair<'a> {
    order1: &'a Order,
    order2: &'a Order,
}

fn main() {
    let order_a = Order { id: Uuid::new_v4() };
    let order_b = Order { id: Uuid::new_v4() };

    let pair = OrderPair {
        order1: &order_a,
        order2: &order_b,
    };

    println!("OrderPair: {:?}", pair);
}
