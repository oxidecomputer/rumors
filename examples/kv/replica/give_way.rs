impl Replica {
    fn same_network(&self, other: &Replica) -> bool {
        self.rumors.network() == other.rumors.network()
    }

    fn is_empty(&self) -> bool {
        self.rumors.snapshot().latest().is_empty()
    }

    fn abandon(&self) {
        self.stop.cancel();
    }
}
