use std::io;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

impl Node {
    async fn gossip_over(
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

        if let Some(replica) = replica {
            let ended = replica.gossip(&mut read, &mut write).await?;

            if let Ended::Overtaken = ended {
                let current = self.current.lock().await;
                let abandoning = current
                    .borrow()
                    .as_ref()
                    .is_some_and(|held| held.same_network(&replica) && held.is_empty());
                if abandoning {
                    current.send_replace(None);
                    replica.abandon();
                }
            }
        }

        write.shutdown().await?;
        Ok(())
    }
}
