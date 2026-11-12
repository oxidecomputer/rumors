use rumors::TryNext;

use super::Replica;

impl Replica {
    pub(super) fn catch_up(&self) {
        let mut view = self.view.lock().unwrap();
        while let TryNext::Message((version, update)) = view.updates.try_next() {
            if let Some(obsolete) = view.table.update(version, &update) {
                self.rumors.redact(&obsolete);
            }
        }
    }
}
