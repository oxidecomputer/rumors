//! Wire-equivalence helper for the *synchronous* gossip path: drive
//! `sync::Local::gossip` over a pair of `std::io::pipe`s with one peer
//! on each thread. Mirrors `wire.rs` for the async `Local::gossip`
//! path. Used by the `sync_wire` integration test.

use std::io::pipe;
use std::thread;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::sync::{self, Local, ignore};

/// Gossip two `Local`s through the synchronous wire protocol and
/// return the reconciled pair. After this returns, the two `Local`s
/// agree on live content.
pub fn sync_wire_gossip<T>(a: Local<T>, b: Local<T>) -> (Local<T>, Local<T>)
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
