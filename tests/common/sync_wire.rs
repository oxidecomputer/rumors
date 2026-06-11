//! Wire helpers for the *synchronous* gossip path: drive
//! `sync::Known::gossip` over a pair of `std::io::pipe`s with one peer
//! on each thread. Mirrors `wire.rs` for the async `Known::gossip`
//! path. Used by the `sync_wire` integration test.

use std::io::pipe;
use std::thread;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::sync::Known;

/// Gossip two `Known`s through the synchronous wire protocol. After this
/// returns, the two `Known`s agree on live content. One side blocks on
/// this thread, the other on a spawned thread; the thread is scoped so
/// `b` can be borrowed rather than moved.
pub fn sync_wire_gossip<T>(a: &mut Known<T>, b: &mut Known<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + std::marker::Sync + 'static,
{
    let (mut a_to_b_r, mut a_to_b_w) = pipe().expect("pipe a→b");
    let (mut b_to_a_r, mut b_to_a_w) = pipe().expect("pipe b→a");

    thread::scope(|s| {
        let b_thread =
            s.spawn(move || b.gossip(&mut a_to_b_r, &mut b_to_a_w).expect("sync wire B"));
        a.gossip(&mut b_to_a_r, &mut a_to_b_w).expect("sync wire A");
        b_thread.join().expect("join B thread");
    });
}

/// Mint a genuine, party-disjoint `sync::Known` from `parent` by serving it
/// a bootstrap over a pair of pipes: the synchronous counterpart of
/// `wire::bootstrap_fork`.
///
/// This is how the sync tests obtain a second *originator*: the returned
/// peer descends from `parent`'s universe (same
/// [`Network`](rumors::Network)) with its own disjoint party region and a
/// copy of `parent`'s content. Serving the bootstrap forks the slice off
/// `parent`'s party in the same critical section that snapshots the served
/// tree, so the two end up pairwise disjoint.
pub fn sync_bootstrap_fork<T>(parent: &mut Known<T>) -> Known<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + std::marker::Sync + 'static,
{
    let (mut p_to_n_r, mut p_to_n_w) = pipe().expect("pipe parent→newcomer");
    let (mut n_to_p_r, mut n_to_p_w) = pipe().expect("pipe newcomer→parent");

    thread::scope(|s| {
        let newcomer = s.spawn(move || {
            Known::<T>::bootstrap(&mut p_to_n_r, &mut n_to_p_w)
                .expect("bootstrap handshake")
                .expect("parent served the bootstrap")
        });
        parent
            .gossip(&mut n_to_p_r, &mut p_to_n_w)
            .expect("bootstrap server gossip");
        newcomer.join().expect("join bootstrap thread")
    })
}
