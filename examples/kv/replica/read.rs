use std::collections::BTreeMap;

use crate::table::{Key, Value};

use super::Replica;

impl Replica {
    pub(crate) fn get(&self, key: &Key) -> Option<Value> {
        self.view.lock().unwrap().table.get(key).cloned()
    }
    pub(crate) fn entries(&self) -> BTreeMap<Key, Value> {
        let view = self.view.lock().unwrap();
        view.table
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}
