//! Runtime-free asynchronous wire harness shared by reconciliation benchmarks.
//!
//! Benchmarks measure what ships: peers created here run at the default
//! memory budget in every build shape.

use rumors::testing::MemoryEnd;
use rumors::{Peer, Rumors};

/// Bounded transport capacity; concurrent polling supplies the backpressure.
const CAPACITY: usize = 64 * 1024;

/// A persistent in-memory connection reusable at clean session boundaries.
pub struct Wire {
    a_end: MemoryEnd,
    b_end: MemoryEnd,
}

/// Allocate a fresh in-memory connection.
impl Default for Wire {
    /// Allocate a fresh in-memory connection.
    fn default() -> Self {
        Self::new()
    }
}

impl Wire {
    /// Allocate one bounded in-memory connection, one end per side.
    pub fn new() -> Self {
        let (a_end, b_end) = rumors::testing::memory_connection(CAPACITY);
        Self { a_end, b_end }
    }

    /// Reconcile one pair while driving both endpoints concurrently.
    pub fn round_trip<T>(&mut self, a: Rumors<T>, b: Rumors<T>) -> (Rumors<T>, Rumors<T>)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
    {
        let (a_result, b_result) = pollster::block_on(async {
            tokio::join!(
                a.gossip_once(&mut self.a_end.read, &mut self.a_end.write),
                b.gossip_once(&mut self.b_end.read, &mut self.b_end.write)
            )
        });
        a_result.expect("peer A gossip");
        b_result.expect("peer B gossip");
        (a, b)
    }
}

/// Create one disjoint replica by serving a bootstrap over an ephemeral
/// connection.
pub fn bootstrap_fork<T>(parent: &Rumors<T>) -> Rumors<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
{
    pollster::block_on(async {
        let (mut parent_end, mut newcomer_end) = rumors::testing::memory_connection(CAPACITY);
        let (served, newcomer) = tokio::join!(
            parent.gossip_once(&mut parent_end.read, &mut parent_end.write),
            Peer::<T>::bootstrap().join(&mut newcomer_end.read, &mut newcomer_end.write),
        );
        served.expect("serve bootstrap");
        (match newcomer {
            rumors::Joined::Joined {
                peer,
                connection: rumors::Connection::Reusable { .. },
            } => peer,
            _ => panic!("provider is established"),
        })
        .into_rumors()
    })
}
