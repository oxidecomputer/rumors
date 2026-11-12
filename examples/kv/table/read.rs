use super::{Key, Table, Value};

impl Table {
    pub(crate) fn get(&self, key: &Key) -> Option<&Value> {
        self.entries.get(key)?.1.as_ref()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries
            .iter()
            .filter_map(|(key, (_, value))| Some((key, value.as_ref()?)))
    }
}
