use std::net::SocketAddr;

use tokio::net::TcpListener;

const ACCEPT_BACKOFF: Duration = Duration::from_millis(500);

impl Node {
    async fn accept(&self, tcp: TcpStream, address: SocketAddr) {
        let accepted = async {
            let (read, write) = self.tls.accept(tcp).await?;
            self.gossip_over(read, write).await
        };
        if let Err(e) = accepted.await {
            eprintln!("connection from {address} ended: {e:#}");
        }
    }

    async fn serve(self: Arc<Self>, listener: TcpListener) {
        while let Some(accepted) = self.stop.run_until_cancelled(listener.accept()).await {
            let (tcp, address) = match accepted {
                Ok(accepted) => accepted,
                Err(e) => {
                    eprintln!("cannot accept connections: {e}");
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
