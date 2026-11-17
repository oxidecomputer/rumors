mod key;
mod value;

use std::collections::BTreeMap;

use rumors::Version;

pub use key::Key;
pub use value::Value;

#[derive(Default)]
pub struct Table {
    entries: BTreeMap<Key, (Version, Option<Value>)>,
    pub applied: Version,
}

mod update;
mod read;

pub use update::Update;
