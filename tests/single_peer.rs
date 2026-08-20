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

use serde::Serialize;
use serde::Serializer;
/// Commit `values` to `peer` as one batch, returning the [`Version`]s it
/// minted (recovered as the live leaves above the pre-commit frontier).
fn batch_send(peer: &Rumors<u64>, values: &[u64]) -> Vec<Version> {
    let pre = peer.snapshot().latest().clone();
    peer.batch(|batch| {
        for v in values {
            batch.send(*v)?;
        }
        Ok::<(), rumors::EncodeError>(())
    })
    .expect("flat test payloads are within any depth limit");
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
                *out.entry(*v).or_insert(0) += 1;
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

impl Serialize for Explosive {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.fail {
            return Err(serde::ser::Error::custom("detonated"));
        }
        self.value.serialize(serializer)
    }
}

// Peer construction mints the payload deserializer up front, so even this
// send-only payload type states how its wire form reads back.
impl<'de> serde::Deserialize<'de> for Explosive {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Explosive {
            value: u64::deserialize(deserializer)?,
            fail: false,
        })
    }
}

/// A batch whose closure panics commits nothing, earlier-queued sends
/// included.
///
/// [`Rumors::batch`](rumors::Rumors::batch) commits iff the closure
/// returns `Ok`; a panic's unwind exits the closure without returning, so
/// the commit call never runs and nothing publishes — structurally, with
/// no unwind detection anywhere.
///
/// `Batch::send` panics when a value fails to serialize (its documented
/// panic contract), which is this test's panic source: the first send is
/// queued, the second detonates.
#[test]
fn a_panicked_batch_commits_nothing() {
    let rumors: Rumors<Explosive> = Peer::seed().sync_window_floor().into_rumors();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rumors.batch(|batch| {
            batch.send(Explosive {
                value: 1,
                fail: false,
            })?;
            // The documented serialization panic fires here, after the
            // first send is queued; the unwind exits the closure.
            batch.send(Explosive {
                value: 2,
                fail: true,
            })?;
            Ok::<(), rumors::EncodeError>(())
        })
    }));
    assert!(unwound.is_err(), "the second send must panic");
    assert_eq!(
        rumors.snapshot().len(),
        0,
        "a batch whose closure panicked must commit nothing"
    );
}

/// A batch commits nothing until — and everything once — the closure
/// returns `Ok`: mid-closure the set is untouched (queueing publishes
/// nothing), and the `Ok` return lands sends and redactions together,
/// all-or-nothing.
#[test]
fn a_batch_commits_iff_the_closure_returns_ok() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    rumors.send(7).unwrap();
    let doomed = rumors
        .snapshot()
        .iter()
        .map(|(v, _)| v.clone())
        .next()
        .expect("the pre-batch send is live");

    rumors
        .batch(|batch| {
            batch.send(1)?;
            batch.send(2)?;
            batch.redact(&doomed);
            // Nothing queued is visible while the closure runs: the batch
            // publishes only at its commit.
            assert_eq!(rumors.snapshot().len(), 1, "queueing publishes nothing");
            Ok::<(), rumors::EncodeError>(())
        })
        .expect("flat payloads are within any depth limit");

    let live: Vec<u64> = rumors.snapshot().iter().map(|(_, m)| *m).collect();
    assert_eq!(live.len(), 2, "the sends landed and the redaction took");
    assert!(live.contains(&1) && live.contains(&2));
}

/// A depth-rejected send, `?`-propagated out of the closure, cancels
/// the whole batch: earlier-queued actions included — the
/// cancel-on-error pin.
///
/// The typed error carries the configured limit, and the tree is
/// untouched.
#[test]
fn a_depth_error_cancels_the_whole_batch() {
    use ciborium::Value;
    let limit = rumors::PayloadDepthLimit::new(4);
    let rumors: Rumors<Value> = Peer::seed()
        .payload_depth_limit(limit)
        .sync_window_floor()
        .into_rumors();
    let deep = (0..5).fold(Value::Integer(0.into()), |v, _| Value::Array(vec![v]));

    let error = rumors
        .batch(|batch| {
            batch.send(Value::Bool(true))?;
            batch.send(deep.clone())?;
            Ok::<(), rumors::EncodeError>(())
        })
        .expect_err("the second send exceeds the limit");
    assert!(
        matches!(error, rumors::EncodeError::Depth { limit: l } if l == limit),
        "the rejection is the typed depth case naming the limit: {error:?}"
    );
    assert_eq!(
        rumors.snapshot().len(),
        0,
        "a cancelled batch commits nothing, earlier-queued sends included"
    );
}

/// A user `Err` return cancels the batch — the deliberate abort
/// affordance — and comes back verbatim.
#[test]
fn a_user_error_cancels_the_batch() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let error = rumors
        .batch(|batch| {
            batch.send(1).unwrap();
            batch.send(2).unwrap();
            Err::<(), &str>("changed my mind")
        })
        .expect_err("the closure aborts deliberately");
    assert_eq!(error, "changed my mind");
    assert_eq!(
        rumors.snapshot().len(),
        0,
        "an aborted batch commits nothing"
    );
}

/// Batches nest: the closure may use the same handle, and the nested
/// operations commit first, the outer batch after, as separate commits
/// (inner-before-outer).
#[test]
fn nested_batches_commit_inner_before_outer() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    rumors
        .batch(|batch| {
            batch.send(1)?;
            // Re-entrant single send and nested batch, both on the same
            // handle: each commits immediately, before the outer batch.
            rumors.send(2)?;
            rumors.batch(|inner| inner.send(3))?;
            let mid: Vec<u64> = rumors.snapshot().iter().map(|(_, m)| *m).collect();
            assert!(
                mid.contains(&2) && mid.contains(&3) && !mid.contains(&1),
                "inner commits land before the outer batch: {mid:?}"
            );
            Ok::<(), rumors::EncodeError>(())
        })
        .expect("flat payloads are within any depth limit");
    assert_eq!(rumors.snapshot().len(), 3, "the outer batch lands last");
}
