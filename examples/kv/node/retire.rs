use tokio::io::AsyncWriteExt;

use super::Node;

impl Node {
    pub(super) async fn retire(&self) {
        let replica = self.current.lock().await.send_replace(None);
        if let Some(replica) = replica
            && let Some(mut peer) = replica.into_rumors().try_into_peer().await
        {
            for address in &self.peers {
                if let Ok((read, write)) = self.tls.connect(address).await {
                    use rumors::Retire::*;
                    peer = match peer.retire(read, write).await {
                        Retired { connection } => {
                            if let Ok((_, mut write)) = connection.into_halves() {
                                let _ = write.shutdown().await;
                            }
                            return;
                        }
                        Declined {
                            peer, mut write, ..
                        } => {
                            let _ = write.shutdown().await;
                            peer
                        }
                        Recovered { peer, .. } => peer,
                    };
                }
            }
        }
    }
}
