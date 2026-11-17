use std::collections::BTreeMap;

use crate::table::{Key, Value};

use super::Replica;

impl Replica {
    pub fn get(&self, key: &Key) -> Option<Value> {
        self.table.borrow().get(key).cloned()
    }

    pub fn entries(&self) -> BTreeMap<Key, Value> {
        self.table
            .borrow()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}
