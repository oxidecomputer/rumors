use std::io;

use anyhow::Result;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use rumors::Participant;
use rumors::error::{DataLoss, Mismatch};

use super::Node;

impl Node {
    pub async fn gossip_over(
        &self,
        mut read: impl AsyncBufRead + Unpin + Send,
        mut write: impl AsyncWrite + Unpin + Send,
    ) -> Result<()> {
        async fn remote_spoke(read: &mut (impl AsyncBufRead + Unpin)) -> io::Result<bool> {
            Ok(!read.fill_buf().await?.is_empty())
        }

        let replica = tokio::select! {
            Some(replica) = self.wait_for_replica() => Some(replica),
            spoke = remote_spoke(&mut read) => match spoke? {
                true => Some(self.join(&mut read, &mut write).await?),
                false => None,
            },
            () = self.stop.cancelled() => None,
        };

        if let Some(replica) = replica
            && let Err(error) = replica.peer.gossip(&mut read, &mut write).await
        {
            let yielding = match &error {
                rumors::Error::Mismatch(Mismatch::Network(mismatch)) => {
                    mismatch.yielding_peer(DataLoss::Forbid)
                }
                _ => None,
            };
            match yielding {
                None => return Err(error.into()),
                Some(Participant::Remote) => {}
                Some(Participant::Local) => {
                    let current = self.current.lock().await;
                    let abandoning = current.borrow().as_ref().is_some_and(|held| {
                        held.peer.network() == replica.peer.network()
                            && held.peer.snapshot().latest().is_empty()
                    });
                    if abandoning {
                        current.send_replace(None);
                        replica.stop.cancel();
                    }
                }
            }
        }

        write.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
