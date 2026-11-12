enum Ended {
    Closed,
    Overtaken,
    Overtaking,
}

use rumors::Count;
use rumors::error::Mismatch;
use tokio::io::{AsyncBufRead, AsyncWrite};

impl Replica {
    async fn gossip(
        &self,
        read: impl AsyncBufRead + Unpin + Send,
        write: impl AsyncWrite + Unpin + Send,
    ) -> Result<Ended, rumors::Error<FileBookmark<TokioBlocking>>> {
        let Err(error) = self.rumors.gossip(read, write).await else {
            return Ok(Ended::Closed);
        };
        if let rumors::Error::Mismatch(Mismatch::Network {
            local_network,
            local_min_events,
            remote_network,
            remote_min_events,
            ..
        }) = &error
        {
            let ours = (local_min_events, local_network);
            let theirs = (remote_min_events, remote_network);
            if *ours.min(theirs).0 == Count::ZERO {
                return Ok(if ours < theirs {
                    Ended::Overtaken
                } else {
                    Ended::Overtaking
                });
            }
        }
        Err(error)
    }
}
