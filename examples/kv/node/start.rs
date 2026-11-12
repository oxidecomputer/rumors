use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;

use super::Node;

impl Node {
    pub(crate) async fn start(self: &Arc<Self>, listener: TcpListener) -> Result<()> {
        self.seed().await?;
        self.tasks.spawn(self.clone().serve(listener));
        for address in &self.peers {
            self.tasks.spawn(self.clone().dial(address.clone()));
        }
        Ok(())
    }
}
