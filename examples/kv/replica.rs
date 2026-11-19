use rumors::{FileBookmark, Peer, TokioBlocking};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::table::{Table, Update};

#[derive(Clone)]
pub struct Replica {
    pub peer: Peer<Update, FileBookmark<TokioBlocking>>,
    table: watch::Sender<Table>,
    pub stop: CancellationToken,
}

impl Replica {
    pub fn new(peer: Peer<Update, FileBookmark<TokioBlocking>>, stop: CancellationToken) -> Self {
        Self {
            peer,
            table: watch::Sender::new(Table::default()),
            stop,
        }
    }
}

mod keep_updated;
mod update;
mod read;
