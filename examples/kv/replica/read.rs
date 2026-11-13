use std::collections::BTreeMap;

use crate::table::{Key, Value};

use super::Replica;

impl Replica {
    pub(crate) fn get(&self, key: &Key) -> Option<Value> {
        self.table.borrow().get(key).cloned()
    }

    pub(crate) fn entries(&self) -> BTreeMap<Key, Value> {
        self.table
            .borrow()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}
