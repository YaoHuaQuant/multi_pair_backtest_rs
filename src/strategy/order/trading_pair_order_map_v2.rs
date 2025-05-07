use std::collections::HashMap;
use crate::data_source::trading_pair::ETradingPairType;
use crate::strategy::order::order_manager_v2::SStrategyOrderManagerV2;

#[derive(Default)]
pub struct SStrategyTradingPairOrderMapV2 {
    pub inner:HashMap<ETradingPairType, SStrategyOrderManagerV2>,
}
 impl SStrategyTradingPairOrderMapV2 {
     pub fn insert(&mut self, tp_type: ETradingPairType, order_manager: SStrategyOrderManagerV2) -> Option<SStrategyOrderManagerV2> {
         self.inner.insert(tp_type, order_manager)
     }

     pub fn get(&self, key: &ETradingPairType) -> Option<&SStrategyOrderManagerV2> {
         self.inner.get(key)
     }

     pub fn get_mut(&mut self, key: &ETradingPairType) -> Option<&mut SStrategyOrderManagerV2> {
         self.inner.get_mut(key)
     }
 }
