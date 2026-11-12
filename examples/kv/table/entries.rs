use std::collections::BTreeMap;

use rumors::Version;

#[derive(Default)]
struct Table {
    entries: BTreeMap<Key, (Version, Option<Value>)>,
}
