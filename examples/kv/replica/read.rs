impl Replica {
    fn get(&self, key: &Key) -> Option<Value> {
        self.view.lock().unwrap().table.get(key).cloned()
    }
    fn entries(&self) -> BTreeMap<Key, Value> {
        let view = self.view.lock().unwrap();
        view.table
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}
