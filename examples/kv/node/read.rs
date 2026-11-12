impl Node {
    fn get(&self, key: &Key) -> Option<Value> {
        self.replica.borrow().as_ref()?.get(key)
    }

    fn entries(&self) -> BTreeMap<Key, Value> {
        let replica = self.replica.borrow();
        replica.as_ref().map(Replica::entries).unwrap_or_default()
    }

    async fn wait_for_replica(&self) -> Option<Replica> {
        let mut replica = self.replica.clone();
        replica.wait_for(Option::is_some).await.ok()?.clone()
    }
}
