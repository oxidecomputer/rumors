use std::collections::BTreeMap;

use nutype::nutype;
use rumors::Version;

#[rustfmt::skip]
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, TryFrom, Into, Serialize, Deserialize)
)]
pub(crate) struct Key(String);

#[rustfmt::skip]
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, TryFrom, Into, Serialize, Deserialize)
)]
pub(crate) struct Value(String);

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Update {
    pub(crate) key: Key,
    pub(crate) value: Option<Value>,
}

#[derive(Default)]
pub(crate) struct Table {
    entries: BTreeMap<Key, (Version, Option<Value>)>,
}

mod apply;
mod read;
