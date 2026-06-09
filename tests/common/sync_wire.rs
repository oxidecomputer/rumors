//! Wire-equivalence helper for the *synchronous* gossip path: drive
//! `sync::Known::gossip` over a pair of `std::io::pipe`s with one peer
//! on each thread. Mirrors `wire.rs` for the async `Known::gossip`
//! path. Used by the `sync_wire` integration test.

use std::io::pipe;
use std::thread;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::sync::{self, Known};

/// Gossip two `Known`s through the synchronous wire protocol and
/// return the reconciled pair. After this returns, the two `Known`s
/// agree on live content.
pub fn sync_wire_gossip<T>(a: Known<T>, b: Known<T>) -> (Known<T>, Known<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + std::marker::Sync + 'static,
{
    let (mut a_to_b_r, mut a_to_b_w) = pipe().expect("pipe a→b");
    let (mut b_to_a_r, mut b_to_a_w) = pipe().expect("pipe b→a");

    let b_thread =
        thread::spawn(move || b.gossip(&mut a_to_b_r, &mut b_to_a_w).expect("sync wire B"));

    let a_out = a.gossip(&mut b_to_a_r, &mut a_to_b_w).expect("sync wire A");
    let b_out = b_thread.join().expect("join B thread");
    (a_out, b_out)
}

/// Mint a genuine, party-disjoint `sync::Known` from `parent` by serving it a
/// bootstrap over a pair of pipes: the synchronous counterpart of
/// `wire::bootstrap_fork`.
///
/// Now that `fork` is gone, this is how the sync tests obtain a second
/// *originator*. The returned peer descends from `parent`'s universe (same
/// [`Network`](rumors::Network)) with its own disjoint party region and a copy
/// of `parent`'s content. Use it — not [`rumors`](Known::rumors) — wherever the
/// second peer must go on to `message`/`redact`: two `rumors` snapshots share a
/// party and would originate non-concurrent versions, breaking the merge.
pub fn sync_bootstrap_fork<T>(parent: &Known<T>) -> Known<T>
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + std::marker::Sync + 'static,
{
    let (mut p_to_n_r, mut p_to_n_w) = pipe().expect("pipe parent→newcomer");
    let (mut n_to_p_r, mut n_to_p_w) = pipe().expect("pipe newcomer→parent");

    // The newcomer bootstraps on its own thread; the server is a `rumors`
    // snapshot sharing parent's party, so serving the bootstrap forks a disjoint
    // slice off parent in place and the two end up pairwise disjoint.
    let newcomer = thread::spawn(move || {
        sync::Known::<T>::bootstrap(&mut p_to_n_r, &mut n_to_p_w)
            .expect("bootstrap handshake")
            .expect("parent served the bootstrap")
    });
    let server = parent.rumors();
    server
        .gossip(&mut n_to_p_r, &mut p_to_n_w)
        .expect("bootstrap server gossip");
    newcomer.join().expect("join bootstrap thread")
}
