use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;

use crate::table::Update;

use super::Node;

const REJOIN_WAIT: Duration = Duration::from_secs(5);

impl Node {
    pub(crate) async fn update(&self, updates: &[Update]) -> Result<()> {
        if timeout(REJOIN_WAIT, self.wait_for_replica()).await.is_err() {
            self.seed().await?;
        }

        let current = self.current.lock().await;
        let replica = current.borrow().clone();
        replica
            .context("not replicating any network right now")?
            .update(updates)?;

        Ok(())
    }
}
