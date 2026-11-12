use futures::StreamExt;
use rumors::{EncodeError, TryNext};

impl Replica {
    fn catch_up(&self) {
        let mut view = self.view.lock().unwrap();
        while let TryNext::Message((version, update)) = view.updates.try_next() {
            if let Some(obsolete) = view.table.update(version, &update) {
                self.rumors.redact(&obsolete);
            }
        }
    }
    fn update(&self, updates: &[Update]) -> Result<(), EncodeError> {
        self.rumors.send_all(updates.iter().cloned())?;
        self.catch_up();
        Ok(())
    }
    async fn keep_updated(self) {
        let mut changes = self.rumors.changes();
        while let Some(Some(())) = self.stop.run_until_cancelled(changes.next()).await {
            self.catch_up();
        }
    }
}
