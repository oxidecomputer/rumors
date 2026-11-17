use rumors::{FileBookmark, Peer, Rumors, TokioBlocking};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::table::{Table, Update};

#[derive(Clone)]
pub struct Replica {
    pub rumors: Rumors<Update, FileBookmark<TokioBlocking>>,
    table: watch::Sender<Table>,
    pub stop: CancellationToken,
}

impl Replica {
    pub fn new(peer: Peer<Update, FileBookmark<TokioBlocking>>, stop: &CancellationToken) -> Self {
        let stop = stop.child_token();
        let rumors = peer
            .gossip_until(stop.clone().cancelled_owned())
            .into_rumors();
        Self {
            rumors,
            table: watch::Sender::new(Table::default()),
            stop,
        }
    }
}

mod keep_updated;
mod update;
mod read;
