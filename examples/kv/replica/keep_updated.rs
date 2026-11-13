use futures::StreamExt;

use super::Replica;

impl Replica {
    pub(crate) async fn keep_updated(self) {
        let mut updates = self.rumors.causal_messages();
        while let Some(Some((version, update))) =
            self.stop.run_until_cancelled(updates.next()).await
        {
            self.table.send_modify(|table| {
                if let Some(obsolete) = table.update(version, &update) {
                    self.rumors.redact(&obsolete);
                }
            });
        }
    }
}
