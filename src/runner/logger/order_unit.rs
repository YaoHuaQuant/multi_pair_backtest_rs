use rust_decimal::Decimal;

#[derive(Clone, Debug)]
pub struct  SDataLogOrderUnit{
    pub btc_usdt_highest_buy_price:Option<Decimal>,
    pub btc_usdt_lowest_sell_price:Option<Decimal>,
}