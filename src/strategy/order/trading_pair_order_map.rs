use std::collections::HashMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::strategy::order::order_manager::SStrategyOrderManager;

#[derive(Default)]
pub struct SStrategyTradingPairOrderMap {
    pub inner:HashMap<ETradingPairType, SStrategyOrderManager>,
}
 impl SStrategyTradingPairOrderMap {
     pub fn insert(&mut self, tp_type: ETradingPairType, order_manager: SStrategyOrderManager) -> Option<SStrategyOrderManager> {
         self.inner.insert(tp_type, order_manager)
     }

     pub fn get(&self, key: &ETradingPairType) -> Option<&SStrategyOrderManager> {
         self.inner.get(key)
     }

     pub fn get_mut(&mut self, key: &ETradingPairType) -> Option<&mut SStrategyOrderManager> {
         self.inner.get_mut(key)
     }
 }
