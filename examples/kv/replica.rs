use std::sync::{Arc, Mutex};

use rumors::{FileBookmark, Peer, Rumors, TokioBlocking, UnorderedMessages};
use tokio_util::sync::CancellationToken;

use crate::table::{Table, Update};

struct View {
    updates: UnorderedMessages<Update>,
    table: Table,
}

#[derive(Clone)]
pub(crate) struct Replica {
    rumors: Rumors<Update, FileBookmark<TokioBlocking>>,
    view: Arc<Mutex<View>>,
    stop: CancellationToken,
}

impl Replica {
    pub(crate) fn new(
        peer: Peer<Update, FileBookmark<TokioBlocking>>,
        stop: &CancellationToken,
    ) -> Self {
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
}

mod catch_up;
mod read;
mod gossip;
mod give_way;
mod into_rumors;

pub(crate) use gossip::Ended;
