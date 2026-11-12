use std::io;

use anyhow::Result;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use super::Node;
use super::give_way::{GiveWay, give_way};

impl Node {
    pub(super) async fn gossip_over(
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
            && let Err(error) = replica.rumors.gossip(&mut read, &mut write).await
        {
            match give_way(&error) {
                None => return Err(error.into()),
                Some(GiveWay::Them) => {}
                Some(GiveWay::Us) => {
                    let current = self.current.lock().await;
                    let abandoning = current.borrow().as_ref().is_some_and(|held| {
                        held.rumors.network() == replica.rumors.network()
                            && held.rumors.snapshot().latest().is_empty()
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
