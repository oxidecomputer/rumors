use std::time::Duration;

use anyhow::Result;
use rumors::Peer;
use tokio::time::sleep;
use tracing::debug;

use crate::replica::Replica;

use super::Node;

pub(super) const SESSION_DEADLINE: Duration = Duration::from_secs(30);

impl Node {
    pub(super) async fn seed(&self) -> Result<()> {
        let current = self.current.lock().await;
        if current.borrow().is_some() {
            return Ok(());
        }

        let peer = Peer::seed()
            .session_deadline(|| sleep(SESSION_DEADLINE))
            .bookmark(self.claim_bookmark().await?)
            .await
            .map_err(|unbookmarked| unbookmarked.error)?;

        debug!(network = %peer.network(), "seeded network");
        let replica = Replica::new(peer, &self.stop);
        current.send_replace(Some(replica.clone()));
        self.tasks.spawn(replica.keep_updated());

        Ok(())
    }
}
