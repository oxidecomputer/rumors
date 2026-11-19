use std::time::Duration;

use anyhow::Result;
use rumors::Peer;
use tokio::time::sleep;
use tracing::debug;

use crate::replica::Replica;

use super::Node;

pub const GOSSIP_TIMEOUT: Duration = Duration::from_secs(30);

impl Node {
    pub async fn seed(&self) -> Result<()> {
        let current = self.current.lock().await;
        if current.borrow().is_some() {
            return Ok(());
        }

        let stop = self.stop.child_token();
        let until = stop.clone();
        let peer = Peer::builder()
            .gossip_timeout(|| sleep(GOSSIP_TIMEOUT))
            .gossip_until(move || until.clone().cancelled_owned())
            .bookmark(self.claim_bookmark().await?)
            .seed()
            .await
            .map_err(|unbookmarked| unbookmarked.error)?;

        debug!(network = %peer.network(), "seeded network");
        let replica = Replica::new(peer, stop);
        current.send_replace(Some(replica.clone()));
        self.tasks.spawn(replica.keep_updated());

        Ok(())
    }
}
