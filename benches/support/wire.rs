//! Runtime-free asynchronous wire harness shared by reconciliation benchmarks.

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Peer, Protocol, Rumors};
use tokio::io::{DuplexStream, ReadHalf, WriteHalf};

/// Bounded transport capacity; concurrent polling supplies the backpressure.
const CAPACITY: usize = 64 * 1024;

/// A persistent in-memory connection reusable at clean session boundaries.
pub struct Wire {
    a_read: ReadHalf<DuplexStream>,
    a_write: WriteHalf<DuplexStream>,
    b_read: ReadHalf<DuplexStream>,
    b_write: WriteHalf<DuplexStream>,
}

impl Wire {
    /// Allocate and split one bounded full-duplex connection.
    pub fn new() -> Self {
        let (a, b) = tokio::io::duplex(CAPACITY);
        let (a_read, a_write) = tokio::io::split(a);
        let (b_read, b_write) = tokio::io::split(b);
        Self {
            a_read,
            a_write,
            b_read,
            b_write,
        }
    }

    /// Reconcile one pair while driving both endpoints concurrently.
    pub fn round_trip<T>(&mut self, a: Rumors<T>, b: Rumors<T>) -> (Rumors<T>, Rumors<T>)
    where
        T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
    {
        let (a_result, b_result) = pollster::block_on(async {
            tokio::join!(
                a.gossip(&mut self.a_read, &mut self.a_write),
                b.gossip(&mut self.b_read, &mut self.b_write),
            )
        });
        a_result.expect("peer A gossip");
        b_result.expect("peer B gossip");
        (a, b)
    }
}

/// Mint one disjoint replica by serving a bootstrap over an ephemeral wire.
pub fn bootstrap_fork<T>(parent: &Rumors<T>, protocol: Protocol) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    pollster::block_on(async {
        let (parent_side, newcomer_side) = tokio::io::duplex(CAPACITY);
        let (mut parent_read, mut parent_write) = tokio::io::split(parent_side);
        let (mut newcomer_read, mut newcomer_write) = tokio::io::split(newcomer_side);
        let (served, newcomer) = tokio::join!(
            parent.gossip(&mut parent_read, &mut parent_write),
            Peer::<T>::bootstrap_with_protocol(protocol, &mut newcomer_read, &mut newcomer_write,),
        );
        served.expect("serve bootstrap");
        newcomer
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .into_rumors()
    })
}
