//! Wire-equivalence helper and test for the *synchronous* gossip
//! path: drive `sync::Local::gossip` over a pair of `std::io::pipe`s
//! with one peer on each thread. Mirrors `wire.rs` for the async
//! `Local::gossip` path; both paths must produce the same per-peer
//! state as bidirectional `Local::process`.

use std::io::pipe;
use std::thread;

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use rumors::sync::{self, Local, ignore};

use crate::action::{arb_local_actions, build_local};
use crate::oracle::readout;

/// Gossip two `Local`s through the synchronous wire protocol and
/// return the reconciled pair. After this returns, the two `Local`s
/// agree on live content.
fn sync_wire_gossip<T>(a: Local<T>, b: Local<T>) -> (Local<T>, Local<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + std::marker::Sync + 'static,
{
    let (mut a_to_b_r, mut a_to_b_w) = pipe().expect("pipe a→b");
    let (mut b_to_a_r, mut b_to_a_w) = pipe().expect("pipe b→a");

    let b_thread = thread::spawn(move || {
        b.gossip(&mut a_to_b_r, &mut b_to_a_w, sync::ignore)
            .expect("sync wire B")
    });

    let a_out = a
        .gossip(&mut b_to_a_r, &mut a_to_b_w, ignore)
        .expect("sync wire A");
    let b_out = b_thread.join().expect("join B thread");
    (a_out, b_out)
}

proptest! {
    /// `sync::Local::gossip` over `std::io::pipe`s yields the same live
    /// content as bidirectional `Local::process`. Exercised with the
    /// shared `Insert`/`Redact` action shape so redactions cross the
    /// wire too (not just inserts).
    #[test]
    fn sync_gossip_matches_local_process(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let mut a_proc = a0.clone();
        let mut b_proc = b0.clone();
        let a_snap = a_proc.clone();
        let b_snap = b_proc.clone();
        a_proc.process(b_snap, ignore);
        b_proc.process(a_snap, ignore);

        let (a_wire, b_wire) = sync_wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
    }
}
