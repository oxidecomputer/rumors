use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tracing::warn;

use super::Node;

const ACCEPT_BACKOFF: Duration = Duration::from_millis(500);

impl Node {
    async fn accept(&self, tcp: TcpStream, address: SocketAddr) {
        let accepted = async {
            let handshake = self.tls.accept(tcp);
            let Some(halves) = self.stop.run_until_cancelled(handshake).await else {
                return Ok(());
            };
            let (read, write) = halves?;
            self.gossip_over(read, write).await
        };
        if let Err(e) = accepted.await {
            warn!(%address, error = &*e, "connection from peer ended");
        }
    }

    pub(super) async fn serve(self: Arc<Self>, listener: TcpListener) {
        while let Some(accepted) = self.stop.run_until_cancelled(listener.accept()).await {
            let (tcp, address) = match accepted {
                Ok(accepted) => accepted,
                Err(e) => {
                    warn!(error = &e as &dyn Error, "cannot accept connections");
                    self.stop.run_until_cancelled(sleep(ACCEPT_BACKOFF)).await;
                    continue;
                }
            };
            self.tasks.spawn({
                let node = self.clone();
                async move { node.accept(tcp, address).await }
            });
        }
    }
}
