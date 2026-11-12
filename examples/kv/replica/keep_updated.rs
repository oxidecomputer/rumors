use futures::StreamExt;

use super::Replica;

impl Replica {
    pub(crate) async fn keep_updated(self) {
        let mut changes = self.rumors.changes();
        while let Some(Some(())) = self.stop.run_until_cancelled(changes.next()).await {
            self.catch_up();
        }
    }
}
