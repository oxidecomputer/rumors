use nutype::nutype;

#[rustfmt::skip]
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, TryFrom, Into, Serialize, Deserialize)
)]
pub struct Key(String);

#[rustfmt::skip]
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, TryFrom, Into, Serialize, Deserialize)
)]
pub struct Value(String);

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Update {
    pub key: Key,
    pub value: Option<Value>,
}

use std::collections::BTreeMap;

use rumors::Version;

#[derive(Default)]
pub struct Table {
    entries: BTreeMap<Key, (Version, Option<Value>)>,
}

use std::cmp::Ordering;
use std::collections::btree_map::Entry;

impl Table {
    pub fn update(&mut self, version: Version, update: &Update) -> Option<Version> {
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

    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.entries.get(key)?.1.as_ref()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries
            .iter()
            .filter_map(|(key, (_, value))| Some((key, value.as_ref()?)))
    }
}
