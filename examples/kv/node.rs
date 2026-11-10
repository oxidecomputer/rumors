use std::path::PathBuf;
use std::sync::Arc;

use http::uri::Authority;
use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::replica::Replica;
use crate::tls::Tls;

pub struct Node {
    replica: watch::Receiver<Option<Replica>>,
    current: Mutex<watch::Sender<Option<Replica>>>,
    bookmarks_dir: PathBuf,
    tls: Tls,
    peers: Vec<Authority>,
    stop: CancellationToken,
    tasks: TaskTracker,
}

impl Node {
    pub fn new(tls: Tls, bookmarks_dir: PathBuf, peers: Vec<Authority>) -> Arc<Self> {
        let (current, replica) = watch::channel(None);
        Arc::new(Self {
            replica,
            current: Mutex::new(current),
            bookmarks_dir,
            tls,
            peers,
            stop: CancellationToken::new(),
            tasks: TaskTracker::new(),
        })
    }
}

use std::collections::BTreeMap;

use crate::table::{Key, Value};

impl Node {
    pub fn get(&self, key: &Key) -> Option<Value> {
        self.replica.borrow().as_ref()?.get(key)
    }

    pub fn entries(&self) -> BTreeMap<Key, Value> {
        let replica = self.replica.borrow();
        replica.as_ref().map(Replica::entries).unwrap_or_default()
    }

    async fn wait_for_replica(&self) -> Option<Replica> {
        let mut replica = self.replica.clone();
        replica.wait_for(Option::is_some).await.ok()?.clone()
    }
}

use std::fs::{self, TryLockError};

use anyhow::Result;
use rumors::{FileBookmark, TokioBlocking};

impl Node {
    async fn claim_bookmark(&self) -> Result<FileBookmark<TokioBlocking>> {
        fs::create_dir_all(&self.bookmarks_dir)?;
        let mut slot = 0;
        loop {
            let path = self.bookmarks_dir.join(format!("{slot}.bookmark"));
            match FileBookmark::new(path, TokioBlocking).await {
                Ok(bookmark) => return Ok(bookmark),
                Err(TryLockError::WouldBlock) => slot += 1,
                Err(TryLockError::Error(error)) => return Err(error.into()),
            }
        }
    }
}

use std::time::Duration;

use anyhow::bail;
use rumors::{Joined, Peer};
use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::time::sleep;

const SESSION_DEADLINE: Duration = Duration::from_secs(30);

impl Node {
    async fn seed(&self) -> Result<()> {
        let current = self.current.lock().await;
        if current.borrow().is_some() {
            return Ok(());
        }

        let peer = Peer::seed()
            .session_deadline(|| sleep(SESSION_DEADLINE))
            .bookmark(self.claim_bookmark().await?)
            .await
            .map_err(|unbookmarked| unbookmarked.error)?;

        let replica = Replica::new(peer, &self.stop);
        current.send_replace(Some(replica.clone()));
        self.tasks.spawn(replica.keep_updated());

        Ok(())
    }

    async fn join(
        &self,
        read: impl AsyncBufRead + Unpin + Send,
        write: impl AsyncWrite + Unpin + Send,
    ) -> Result<Replica> {
        let current = self.current.lock().await;
        let existing = current.borrow().clone();
        if let Some(replica) = existing {
            return Ok(replica);
        }

        match Peer::bootstrap()
            .session_deadline(|| sleep(SESSION_DEADLINE))
            .bookmark(self.claim_bookmark().await?)
            .join(read, write)
            .await
        {
            Joined::Joined { peer, connection } => {
                let replica = Replica::new(peer, &self.stop);
                current.send_replace(Some(replica.clone()));
                self.tasks.spawn(replica.clone().keep_updated());
                connection.into_halves()?;
                Ok(replica)
            }
            Joined::Bailed { .. } => bail!("both ends were waiting to join a network"),
            Joined::Unbookmarked { unbookmarked, .. } => Err(unbookmarked.error.into()),
            Joined::Failed { error, .. } => Err(error.into()),
        }
    }
}

use anyhow::Context;
use tokio::time::timeout;

use crate::table::Update;

const REJOIN_WAIT: Duration = Duration::from_secs(5);

impl Node {
    pub async fn update(&self, updates: &[Update]) -> Result<()> {
        if timeout(REJOIN_WAIT, self.wait_for_replica()).await.is_err() {
            self.seed().await?;
        }

        let current = self.current.lock().await;
        let replica = current.borrow().clone();
        replica
            .context("not replicating any network right now")?
            .update(updates)?;

        Ok(())
    }
}

use std::io;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::replica::Ended;

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

const RETRY_MIN: Duration = Duration::from_millis(500);
const RETRY_MAX: Duration = Duration::from_secs(30);

impl Node {
    async fn connect(&self, address: &Authority) -> Result<()> {
        let (read, write) = self.tls.connect(address).await?;
        self.gossip_over(read, write).await
    }

    async fn dial(self: Arc<Self>, address: Authority) {
        let mut retry = Duration::ZERO;
        while self.stop.run_until_cancelled(sleep(retry)).await.is_some() {
            retry = match self.connect(&address).await {
                Ok(()) => RETRY_MIN,
                Err(e) => {
                    eprintln!("connection to {address} ended: {e:#}");
                    (retry * 2).clamp(RETRY_MIN, RETRY_MAX)
                }
            };
        }
    }
}

use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};

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

impl Node {
    pub async fn start(self: &Arc<Self>, listener: TcpListener) -> Result<()> {
        self.seed().await?;
        self.tasks.spawn(self.clone().serve(listener));
        for address in &self.peers {
            self.tasks.spawn(self.clone().dial(address.clone()));
        }
        Ok(())
    }

    async fn retire(&self) {
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

    pub async fn shutdown(&self) {
        self.stop.cancel();
        self.tasks.close();
        self.tasks.wait().await;
        self.retire().await;
    }
}
