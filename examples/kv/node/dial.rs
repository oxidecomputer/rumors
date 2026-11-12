use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use http::uri::Authority;
use tokio::time::sleep;
use tracing::warn;

use super::Node;

const RETRY_MIN: Duration = Duration::from_millis(500);
const RETRY_MAX: Duration = Duration::from_secs(30);

impl Node {
    async fn connect(&self, address: &Authority) -> Result<()> {
        let handshake = self.tls.connect(address);
        let Some(halves) = self.stop.run_until_cancelled(handshake).await else {
            return Ok(());
        };
        let (read, write) = halves?;
        self.gossip_over(read, write).await
    }

    pub(super) async fn dial(self: Arc<Self>, address: Authority) {
        let mut retry = Duration::ZERO;
        while self.stop.run_until_cancelled(sleep(retry)).await.is_some() {
            retry = match self.connect(&address).await {
                Ok(()) => RETRY_MIN,
                Err(e) => {
                    warn!(%address, error = &*e, "connection to peer ended");
                    (retry * 2).clamp(RETRY_MIN, RETRY_MAX)
                }
            };
        }
    }
}
