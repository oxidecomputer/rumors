mod key;
mod value;

use std::collections::BTreeMap;

use rumors::Version;

pub(crate) use key::Key;
pub(crate) use value::Value;

#[derive(Default)]
pub(crate) struct Table {
    entries: BTreeMap<Key, (Version, Option<Value>)>,
    pub(crate) applied: Version,
}

mod update;
mod read;

pub(crate) use update::Update;
