use crate::replica::Replica;

use super::Node;

impl Node {
    pub(super) async fn wait_for_replica(&self) -> Option<Replica> {
        let mut replica = self.replica.clone();
        replica.wait_for(Option::is_some).await.ok()?.clone()
    }
}
