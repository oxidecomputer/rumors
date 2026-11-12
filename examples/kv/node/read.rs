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

    pub(super) async fn wait_for_replica(&self) -> Option<Replica> {
        let mut replica = self.replica.clone();
        replica.wait_for(Option::is_some).await.ok()?.clone()
    }
}
