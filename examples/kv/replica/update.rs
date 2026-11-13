use rumors::EncodeError;

use crate::table::Update;

use super::Replica;

impl Replica {
    pub(crate) async fn update(&self, updates: &[Update]) -> Result<(), EncodeError> {
        if let Some(sent) = self.rumors.send_all(updates.iter().cloned())? {
            let mut table = self.table.subscribe();
            let applied = table.wait_for(|table| sent <= table.applied);
            self.stop.run_until_cancelled(applied).await;
        }
        Ok(())
    }
}
