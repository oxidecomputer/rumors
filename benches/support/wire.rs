//! Runtime-free asynchronous wire harness shared by reconciliation benchmarks.
//!
//! Benchmarks measure what ships: peers minted here run at the default
//! pipeline window, which is the production budget in every build shape.

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::link::MemoryLink;
use rumors::{Peer, Protocol, Rumors};

/// Bounded transport capacity; concurrent polling supplies the backpressure.
const CAPACITY: usize = 64 * 1024;

/// A persistent in-memory connection reusable at clean session boundaries.
pub struct Wire {
    a_link: MemoryLink,
    b_link: MemoryLink,
}

impl Wire {
    /// Allocate one bounded in-memory link pair, one end per side.
    pub fn new() -> Self {
        let (a_link, b_link) = rumors::link::memory_with_capacity(CAPACITY);
        Self { a_link, b_link }
    }

    /// Reconcile one pair while driving both endpoints concurrently.
    pub fn round_trip<T>(&mut self, a: Rumors<T>, b: Rumors<T>) -> (Rumors<T>, Rumors<T>)
    where
        T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
    {
        let (a_result, b_result) = pollster::block_on(async {
            tokio::join!(a.gossip(&mut self.a_link), b.gossip(&mut self.b_link))
        });
        a_result.expect("peer A gossip");
        b_result.expect("peer B gossip");
        (a, b)
    }
}

/// Mint one disjoint replica by serving a bootstrap over an ephemeral link.
pub fn bootstrap_fork<T>(parent: &Rumors<T>, protocol: Protocol) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    pollster::block_on(async {
        let (mut parent_link, mut newcomer_link) = rumors::link::memory_with_capacity(CAPACITY);
        let (served, newcomer) = tokio::join!(
            parent.gossip(&mut parent_link),
            Peer::<T>::bootstrap_with_protocol(protocol, &mut newcomer_link),
        );
        served.expect("serve bootstrap");
        newcomer
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .into_rumors()
    })
}
