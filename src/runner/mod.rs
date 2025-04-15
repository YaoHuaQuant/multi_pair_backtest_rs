//! 执行器
//! 用于执行strategy中的量化算法，并且应用在回测平台或者交易所。

use crate::strategy::TStrategy;
use crate::data_runtime::user::SUser;

pub mod back_trade;
pub mod logger;

pub trait TRunner<S: TStrategy> {
    fn run(&mut self, users: &mut Vec<SUser<S>>);
}