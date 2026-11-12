use rumors::error::Mismatch;
use rumors::{Count, FileBookmark, TokioBlocking};
use tokio::io::{AsyncBufRead, AsyncWrite};

use super::Replica;

pub(crate) enum Ended {
    Closed,
    Overtaken,
    Overtaking,
}

impl Replica {
    pub(crate) async fn gossip(
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
