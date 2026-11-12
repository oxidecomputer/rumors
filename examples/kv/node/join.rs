use anyhow::{Result, bail};
use rumors::{Joined, Peer};
use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::time::sleep;
use tracing::info;

use crate::replica::Replica;

use super::Node;
use super::seed::SESSION_DEADLINE;

impl Node {
    pub(super) async fn join(
        &self,
        read: impl AsyncBufRead + Unpin + Send,
        write: impl AsyncWrite + Unpin + Send,
    ) -> Result<Replica> {
        let current = self.current.lock().await;
        let existing = current.borrow().clone();
        if let Some(replica) = existing {
            return Ok(replica);
        }

        match Peer::bootstrap()
            .session_deadline(|| sleep(SESSION_DEADLINE))
            .bookmark(self.claim_bookmark().await?)
            .join(read, write)
            .await
        {
            Joined::Joined { peer, connection } => {
                info!(network = %peer.network(), "joined network");
                let replica = Replica::new(peer, &self.stop);
                current.send_replace(Some(replica.clone()));
                self.tasks.spawn(replica.clone().keep_updated());
                connection.into_halves()?;
                Ok(replica)
            }
            Joined::Bailed { .. } => bail!("both ends were waiting to join a network"),
            Joined::Unbookmarked { unbookmarked, .. } => Err(unbookmarked.error.into()),
            Joined::Failed { error, .. } => Err(error.into()),
        }
    }
}
