use anyhow::{Result, bail};
use rumors::{Join, Peer};
use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::time::sleep;
use tracing::debug;

use crate::replica::Replica;

use super::Node;
use super::seed::GOSSIP_TIMEOUT;

impl Node {
    pub async fn join(
        &self,
        read: impl AsyncBufRead + Unpin + Send,
        write: impl AsyncWrite + Unpin + Send,
    ) -> Result<Replica> {
        let current = self.current.lock().await;
        let existing = current.borrow().clone();
        if let Some(replica) = existing {
            return Ok(replica);
        }

        match Peer::joining()
            .gossip_timeout(|| sleep(GOSSIP_TIMEOUT))
            .bookmark(self.claim_bookmark().await?)
            .join(read, write)
            .await
        {
            Join::Connected { peer, connection } => {
                debug!(network = %peer.network(), "joined network");
                let replica = Replica::new(peer, &self.stop);
                current.send_replace(Some(replica.clone()));
                self.tasks.spawn(replica.clone().keep_updated());
                connection.into_halves()?;
                Ok(replica)
            }
            Join::Unbookmarked { unbookmarked, .. } => Err(unbookmarked.error.into()),
            Join::Bounced { .. } => bail!("both ends were waiting to join a network"),
            Join::Failed { error, .. } => Err(error.into()),
        }
    }
}
