//! The opening question's room, charged before the session's tasks run.

use proptest::prelude::*;

use crate::default;
use crate::network::NetworkKeys;
use crate::tree::Root;
use crate::tree::arb::arb_tree_root;
use crate::tree::mirror::streaming::{
    Backend, Local,
    holding::Holding,
    ledger::ReplyBound,
    materialized::{Handshaking, PeerSummary, Start, parked_room},
    protocol::{CompleteConnect, Connect, Initiator},
};
use crate::tree::typed::Prefix;

/// The in-memory backend's erased node representation.
type Erased = <Local as Backend>::Erased;

/// A side of a session over `root`, in the serial configuration.
fn start(root: Root) -> Handshaking<Local, Start> {
    Handshaking::start(
        Local,
        root.into(),
        Some(NetworkKeys::fixture()),
        default::TARGET_MESSAGE_SIZE as u64,
    )
    .serial()
}

proptest! {
    /// Once a side opens its descent as the initiator, its ledger holds the
    /// room for the reply its opening question invites, before any of the
    /// session's tasks runs; until then, through the greeting exchange, it
    /// holds none.
    ///
    /// The opening question's listing crossed in the greeting, so the peer
    /// may have answered it already, and its reply's frames wait in the
    /// receive path, which never waits on the walk, from the moment the
    /// session's pipe starts reading. The room is the one the peer's count
    /// of its root's grandchildren bounds.
    #[test]
    fn the_opening_room_is_charged_before_the_session_runs(
        ours in arb_tree_root(0, 0..64usize),
        theirs in arb_tree_root(1, 0..64usize),
    ) {
        let (greeting, connecting) = pollster::block_on(start(ours).connect())
            .expect("an in-memory tree builds its greeting");
        prop_assert_eq!(connecting.ledger.held_for(Holding::ParkedReplies), 0);
        let (peer, _) = pollster::block_on(start(theirs).connect())
            .expect("an in-memory tree builds its greeting");
        let count = PeerSummary::from_greeting(peer.clone()).grandchildren;
        let connected = pollster::block_on(connecting.complete_connect(peer))
            .expect("completing the greeting exchange cannot fail");
        let ledger = connected.ledger.clone();
        prop_assert_eq!(ledger.held_for(Holding::ParkedReplies), 0);

        let descent = connected.initiator();
        let listed = greeting.listing.len();
        let bound = ReplyBound::of(Prefix::new().erase().height(), listed, count);
        prop_assert_eq!(
            ledger.held_for(Holding::ParkedReplies),
            parked_room::<Erased>(bound, listed),
        );
        drop(descent);
    }
}
