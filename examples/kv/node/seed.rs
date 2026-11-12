use std::time::Duration;

use tokio::time::sleep;

const SESSION_DEADLINE: Duration = Duration::from_secs(30);

impl Node {
    async fn seed(&self) -> Result<()> {
        let current = self.current.lock().await;
        if current.borrow().is_some() {
            return Ok(());
        }

        let peer = Peer::seed()
            .session_deadline(|| sleep(SESSION_DEADLINE))
            .bookmark(self.claim_bookmark().await?)
            .await
            .map_err(|unbookmarked| unbookmarked.error)?;

        let replica = Replica::new(peer, &self.stop);
        current.send_replace(Some(replica.clone()));
        self.tasks.spawn(replica.keep_updated());

        Ok(())
    }
}
