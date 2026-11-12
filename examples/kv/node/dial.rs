const RETRY_MIN: Duration = Duration::from_millis(500);
const RETRY_MAX: Duration = Duration::from_secs(30);

impl Node {
    async fn connect(&self, address: &Authority) -> Result<()> {
        let (read, write) = self.tls.connect(address).await?;
        self.gossip_over(read, write).await
    }

    async fn dial(self: Arc<Self>, address: Authority) {
        let mut retry = Duration::ZERO;
        while self.stop.run_until_cancelled(sleep(retry)).await.is_some() {
            retry = match self.connect(&address).await {
                Ok(()) => RETRY_MIN,
                Err(e) => {
                    eprintln!("connection to {address} ended: {e:#}");
                    (retry * 2).clamp(RETRY_MIN, RETRY_MAX)
                }
            };
        }
    }
}
