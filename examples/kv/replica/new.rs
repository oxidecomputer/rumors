use std::sync::{Arc, Mutex};

use rumors::{FileBookmark, Rumors, TokioBlocking};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Replica {
    rumors: Rumors<Update, FileBookmark<TokioBlocking>>,
    view: Arc<Mutex<View>>,
    stop: CancellationToken,
}

use rumors::Peer;

impl Replica {
    fn new(peer: Peer<Update, FileBookmark<TokioBlocking>>, stop: &CancellationToken) -> Self {
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
