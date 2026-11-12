use super::Replica;

impl Replica {
    pub(crate) fn same_network(&self, other: &Replica) -> bool {
        self.rumors.network() == other.rumors.network()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rumors.snapshot().latest().is_empty()
    }

    pub(crate) fn abandon(&self) {
        self.stop.cancel();
    }
}
