//! Single-peer correctness for a lone rumor set, with no gossip.
//!
//! Exercises the surface area of [`Batch`](rumors::Batch) commits:
//! live-leaf fan-out, distinctness of the [`Version`](rumors::Version)s
//! minted within a batch, and strict monotonicity of the local party's
//! component of each minted version.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Peer, Rumors, Version, causally};

use crate::common::wire::block_on;

/// Commit `values` to `peer` as one batch, returning the [`Version`]s it
/// minted (recovered as the live leaves above the pre-commit frontier).
fn batch_send(peer: &Rumors<u64>, values: &[u64]) -> Vec<Version> {
    let pre = peer.snapshot().latest().clone();
    {
        let mut batch = peer.batch();
        for v in values {
            batch.send(*v);
        }
    }
    peer.snapshot()
        .range(causally::since(&pre))
        .map(|(v, _)| v.clone())
        .collect()
}

proptest! {
    /// Every value committed in a batch becomes exactly one live leaf:
    /// no duplicates, no omissions.
    #[test]
    fn batch_mints_once_per_value(values in vec(any::<u64>(), 0..=32)) {
        let peer = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let minted = batch_send(&peer, &values);
        prop_assert_eq!(minted.len(), values.len());
        prop_assert_eq!(peer.snapshot().len(), values.len());
    }

    /// All `Version`s minted within a single batch are distinct, even
    /// when several values in the batch are equal.
    #[test]
    fn distinct_versions_per_batch(values in vec(any::<u64>(), 1..=32)) {
        let peer = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let minted = batch_send(&peer, &values);
        prop_assert_eq!(minted.len(), values.len());
        let unique: BTreeSet<_> =
            minted.iter().map(|v| v.as_bytes().to_vec()).collect();
        prop_assert_eq!(unique.len(), values.len(), "versions must be distinct");
    }

    /// The same value inserted `n` times in one batch still yields `n`
    /// distinct leaves — each send mints a fresh `Version`, so content
    /// equality does not collapse messages.
    #[test]
    fn duplicate_values_get_distinct_versions(n in 1usize..=16, value in any::<u64>()) {
        let peer = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let values: Vec<u64> = std::iter::repeat_n(value, n).collect();
        let minted = batch_send(&peer, &values);
        prop_assert_eq!(minted.len(), n);
        let unique: BTreeSet<_> =
            minted.iter().map(|v| v.as_bytes().to_vec()).collect();
        prop_assert_eq!(unique.len(), n);
    }

    /// Every `Version` minted by a lone peer is totally ordered against
    /// every other — both within a single batch (the batch docs promise
    /// strictly increasing versions per action) and across successive
    /// batches.
    ///
    /// With one party and no gossip there is no concurrency, so
    /// any incomparable or equal pair would betray a versioning bug.
    #[test]
    fn local_versions_form_a_chain(
        batches in vec(vec(any::<u64>(), 1..=8), 1..=8),
    ) {
        let peer = Peer::<u64>::seed().sync_window_floor().into_rumors();

        // Versions in commit order: per batch, the minted versions sorted
        // into their (total) causal order; batches concatenated in commit
        // order. Each batch's recovery is scoped by the pre-commit frontier.
        let mut versions: Vec<Version> = Vec::new();
        for batch in &batches {
            let mut minted: Vec<Version> = batch_send(&peer, batch);
            minted.sort_by(|a, b| {
                a.partial_cmp(b).expect("a lone peer's versions are totally ordered")
            });
            versions.extend(minted);
        }

        // Strict precedence on causal versions is transitive, so
        // adjacent-pair monotonicity implies the full chain.
        for window in versions.windows(2) {
            prop_assert!(
                window[0] < window[1],
                "{:?} must strictly precede {:?}", window[0], window[1],
            );
        }
    }

    /// Final state after a batch commit does not depend on the input
    /// order. Inserting `values` and a Fisher-Yates shuffle of `values`
    /// into two fresh peers yields equal live value multisets.
    #[test]
    fn batch_state_is_input_order_independent(
        values in vec(any::<u64>(), 0..=16),
        seed in any::<u64>(),
    ) {
        let shuffled = {
            let mut v = values.clone();
            // Fisher-Yates over an inline 64-bit LCG: deterministic
            // from `seed`, no extra dependency; any step function whose
            // high bits reduce to a uniform-enough draw over `0..=i`
            // would do.
            let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            for i in (1..v.len()).rev() {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let j = ((state >> 33) as usize) % (i + 1);
                v.swap(i, j);
            }
            v
        };

        // Each peer is its own fresh seed (the two never gossip, so they
        // need not share a universe). Read the live multiset directly off
        // the snapshot.
        let multiset_of = |values: &[u64]| -> BTreeMap<u64, usize> {
            let peer = Peer::<u64>::seed().sync_window_floor().into_rumors();
            batch_send(&peer, values);
            let mut out = BTreeMap::new();
            for (_, v) in peer.snapshot().iter() {
                *out.entry(**v).or_insert(0) += 1;
            }
            out
        };
        prop_assert_eq!(multiset_of(&values), multiset_of(&shuffled));
    }
}

/// A value whose serialization fails on demand, so a test can fire
/// [`rumors::Batch::send`]'s documented serialization panic at a chosen
/// point mid-batch.
#[derive(Debug)]
struct Explosive {
    value: u64,
    fail: bool,
}

impl borsh::BorshSerialize for Explosive {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        if self.fail {
            return Err(borsh::io::Error::other("detonated"));
        }
        borsh::BorshSerialize::serialize(&self.value, writer)
    }
}

/// A batch interrupted by a panic between its sends commits nothing.
///
/// [`Batch`](rumors::Batch) documents that a batch dropped by a panic's
/// unwind commits nothing: the caller never finished building it, so
/// nothing it holds may publish.
///
/// `Batch::send` panics when a value fails to serialize (its documented
/// panic), and the unwind drops the half-built batch. `Batch`'s `Drop`
/// consults `std::thread::panicking()` and commits nothing during an
/// unwind, so the prefix queued before the panic never publishes; this
/// test pins that guard.
#[test]
fn a_panicked_batch_commits_nothing() {
    let rumors: Rumors<Explosive> = Peer::seed().sync_window_floor().into_rumors();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut batch = rumors.batch();
        batch.send(Explosive {
            value: 1,
            fail: false,
        });
        // The documented serialization panic fires here, after the first
        // send is queued; the unwind drops the half-built batch.
        batch.send(Explosive {
            value: 2,
            fail: true,
        });
    }));
    assert!(unwound.is_err(), "the second send must panic");
    assert_eq!(
        rumors.snapshot().len(),
        0,
        "a batch interrupted by a panic must commit nothing: an unwound \
         batch aborts"
    );
}

/// Pins the drop semantics [`Batch`](rumors::Batch) documents for async
/// cancellation: a batch dropped mid-await commits its queued prefix.
///
/// Dropping the future holding a batch across an await runs no unwind, so
/// the drop is indistinguishable from an ordinary end-of-statement commit
/// and publishes the prefix queued before the cancellation point. This is
/// the documented hazard behind the rule that a batch must not be held
/// across an `.await` in a cancellable task: a batch is a performance
/// optimization, and all-or-nothing delivery bundles into one
/// application-level message instead.
#[test]
fn a_cancelled_batch_commits_its_prefix() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    // The select needs no runtime facilities, so the closed-future driver
    // suffices: the whole select completes on its first poll.
    block_on(async {
        let work = async {
            let mut batch = rumors.batch();
            batch.send(1);
            // The cancellation point: parked mid-build, holding the batch.
            std::future::pending::<()>().await;
            batch.send(2);
        };
        // A biased select polls `work` first (queuing the prefix, then
        // parking) and completes on the ready branch, dropping `work` (and
        // the batch it holds) mid-await.
        tokio::select! {
            biased;
            _ = work => unreachable!("the parked future never completes"),
            _ = std::future::ready(()) => {}
        }
    });

    // The documented behavior, exactly: the prefix committed as a batch
    // of one; the send queued after the cancellation point never ran.
    assert_eq!(
        rumors.snapshot().len(),
        1,
        "cancellation drop-commits the prefix queued before the await"
    );
}
