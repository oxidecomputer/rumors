impl Table {
    fn get(&self, key: &Key) -> Option<&Value> {
        self.entries.get(key)?.1.as_ref()
    }
    fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries
            .iter()
            .filter_map(|(key, (_, value))| Some((key, value.as_ref()?)))
    }
}
