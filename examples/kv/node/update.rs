use tokio::time::timeout;

const REJOIN_WAIT: Duration = Duration::from_secs(5);

impl Node {
    async fn update(&self, updates: &[Update]) -> Result<()> {
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
