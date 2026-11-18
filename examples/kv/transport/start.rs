use std::sync::Arc;

use anyhow::Result;
use http::uri::Authority;
use tokio::net::TcpListener;

use crate::node::Node;
use crate::tls::Tls;

impl Node {
    pub async fn start(
        self: &Arc<Self>,
        tls: &Tls,
        listener: TcpListener,
        peers: &[Authority],
    ) -> Result<()> {
        self.seed().await?;
        self.tasks.spawn(self.clone().serve(tls.clone(), listener));
        for address in peers {
            self.tasks
                .spawn(self.clone().dial(tls.clone(), address.clone()));
        }
        Ok(())
    }
}
