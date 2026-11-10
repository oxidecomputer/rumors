use rumors::UnorderedMessages;

use crate::table::{Table, Update};

struct View {
    updates: UnorderedMessages<Update>,
    table: Table,
}

use std::sync::{Arc, Mutex};

use rumors::{FileBookmark, Rumors, TokioBlocking};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Replica {
    rumors: Rumors<Update, FileBookmark<TokioBlocking>>,
    view: Arc<Mutex<View>>,
    stop: CancellationToken,
}

use futures::StreamExt;
use rumors::{EncodeError, Peer, TryNext};

impl Replica {
    pub fn new(peer: Peer<Update, FileBookmark<TokioBlocking>>, stop: &CancellationToken) -> Self {
        let stop = stop.child_token();
        let rumors = peer
            .gossip_until(stop.clone().cancelled_owned())
            .into_rumors();
        let updates = rumors.unordered_messages();
        Self {
            rumors,
            view: Arc::new(Mutex::new(View {
                updates,
                table: Table::default(),
            })),
            stop,
        }
    }

    fn catch_up(&self) {
        let mut view = self.view.lock().unwrap();
        while let TryNext::Message((version, update)) = view.updates.try_next() {
            if let Some(obsolete) = view.table.update(version, &update) {
                self.rumors.redact(&obsolete);
            }
        }
    }

    pub fn update(&self, updates: &[Update]) -> Result<(), EncodeError> {
        self.rumors.send_all(updates.iter().cloned())?;
        self.catch_up();
        Ok(())
    }

    pub async fn keep_updated(self) {
        let mut changes = self.rumors.changes();
        while let Some(Some(())) = self.stop.run_until_cancelled(changes.next()).await {
            self.catch_up();
        }
    }
}

use std::collections::BTreeMap;

use crate::table::{Key, Value};

impl Replica {
    pub fn get(&self, key: &Key) -> Option<Value> {
        self.view.lock().unwrap().table.get(key).cloned()
    }

    pub fn entries(&self) -> BTreeMap<Key, Value> {
        let view = self.view.lock().unwrap();
        view.table
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

pub enum Ended {
    Closed,
    Overtaken,
    Overtaking,
}

use rumors::Count;
use rumors::error::Mismatch;
use tokio::io::{AsyncBufRead, AsyncWrite};

impl Replica {
    pub async fn gossip(
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

impl Replica {
    pub fn same_network(&self, other: &Replica) -> bool {
        self.rumors.network() == other.rumors.network()
    }

    pub fn is_empty(&self) -> bool {
        self.rumors.snapshot().latest().is_empty()
    }

    pub fn abandon(&self) {
        self.stop.cancel();
    }
}

impl Replica {
    pub fn into_rumors(self) -> Rumors<Update, FileBookmark<TokioBlocking>> {
        self.rumors
    }
}
