use std::cmp::Ordering;
use std::collections::btree_map::Entry;

use rumors::Version;

use super::{Key, Table, Value};

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Update {
    pub(crate) key: Key,
    pub(crate) value: Option<Value>,
}

impl Table {
    pub(crate) fn update(&mut self, version: Version, update: &Update) -> Option<Version> {
        self.applied |= &version;
        let value = update.value.clone();
        match self.entries.entry(update.key.clone()) {
            Entry::Vacant(entry) => {
                entry.insert((version, value));
                None
            }
            Entry::Occupied(mut entry) => match version.ranked().cmp(&entry.get().0.ranked()) {
                Ordering::Greater => Some(entry.insert((version, value)).0),
                Ordering::Equal => None,
                Ordering::Less => Some(version),
            },
        }
    }
}
