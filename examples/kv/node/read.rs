use std::collections::BTreeMap;

use crate::replica::Replica;
use crate::table::{Key, Value};

use super::Node;

impl Node {
    pub(crate) fn get(&self, key: &Key) -> Option<Value> {
        self.replica.borrow().as_ref()?.get(key)
    }

    pub(crate) fn entries(&self) -> BTreeMap<Key, Value> {
        let replica = self.replica.borrow();
        replica.as_ref().map(Replica::entries).unwrap_or_default()
    }
}
