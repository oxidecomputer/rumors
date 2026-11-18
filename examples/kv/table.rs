mod key;
mod value;
mod update;

use std::collections::BTreeMap;

use rumors::Version;

pub use key::Key;
pub use update::Update;
pub use value::Value;

#[derive(Default)]
pub struct Table {
    entries: BTreeMap<Key, (Version, Option<Value>)>,
    pub applied: Version,
}

mod apply;
mod read;
