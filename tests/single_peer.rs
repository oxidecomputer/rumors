//! Single-peer write correctness for a lone set of messages, with no gossip.
//!
//! Exercises individual send versions and the [`Batch`](rumors::Batch) commit
//! surface: live-leaf fan-out, distinct versions within a batch, and strict
//! monotonicity of the local party's component.

use std::collections::{BTreeMap, BTreeSet};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Peer, Version, causally};

use rumors_testkit::common::property::LOCAL_DIVISOR;
use rumors_testkit::common::seed::{distinct_network_seeds, network_seed};
use rumors_testkit::divided_proptest;
use serde::{Deserialize, Serialize, Serializer};

/// Commit `values` to `peer` as one batch, returning the [`Version`]s it
/// created (recovered as the live leaves above the pre-commit frontier).
fn batch_send(peer: &Peer<u64>, values: &[u64]) -> Vec<Version> {
    let pre = peer.snapshot().latest().clone();
    peer.send_all(values.iter().copied()).unwrap();
    peer.snapshot()
        .range(causally::since(&pre))
        .map(|(v, _)| v.clone())
        .collect()
}

proptest! {
    /// Each individual send returns the version stored with that message;
    /// equal payloads sent twice remain distinct messages.
    #[test]
    fn send_returns_its_stamped_version(value in any::<u64>(), network in network_seed()) {
        let peer = network.builder::<u64>().sync_memory_budget(0).seed();
        let first = peer.send(value).unwrap();
        let second = peer.send(value).unwrap();
        let snapshot = peer.snapshot();

        prop_assert_ne!(&first, &second);
        prop_assert_eq!(snapshot.get(&first).map(|message| *message), Some(value));
        prop_assert_eq!(snapshot.get(&second).map(|message| *message), Some(value));
        prop_assert_eq!(snapshot.latest(), &second);
    }

    /// Every value committed in a batch becomes exactly one live leaf:
    /// no duplicates, no omissions.
    #[test]
    fn batch_commits_one_leaf_per_value(
        values in vec(any::<u64>(), 0..=32),
        network in network_seed(),
    ) {
        let peer = network.builder::<u64>().sync_memory_budget(0).seed();
        let created = batch_send(&peer, &values);
        let snapshot = peer.snapshot();
        prop_assert_eq!(created.len(), values.len());
        prop_assert_eq!(snapshot.len(), values.len());
    }

    /// The snapshot readers describe one exact live set after arbitrary
    /// redactions: `versions` enumerates it, `contains` tests it, and
    /// `earliest` is absent exactly when it is empty and otherwise precedes
    /// every member.
    #[test]
    fn snapshot_readers_describe_exact_live_membership(
        items in vec((any::<u64>(), any::<bool>()), 0..=32),
        network in network_seed(),
    ) {
        let peer = network.builder::<u64>().sync_memory_budget(0).seed();
        let values: Vec<_> = items.iter().map(|(value, _)| *value).collect();
        let created = batch_send(&peer, &values);
        peer.redact_all(
            created
                .iter()
                .zip(&items)
                .filter_map(|(version, (_, redact))| redact.then_some(version)),
        );

        let snapshot = peer.snapshot();
        let actual: BTreeSet<_> = snapshot
            .versions()
            .map(|version| version.as_bytes().to_vec())
            .collect();
        let expected: BTreeSet<_> = created
            .iter()
            .zip(&items)
            .filter(|(_, (_, redact))| !redact)
            .map(|(version, _)| version.as_bytes().to_vec())
            .collect();
        prop_assert_eq!(actual, expected);
        for (version, (_, redacted)) in created.iter().zip(&items) {
            prop_assert_eq!(snapshot.contains(version), !redacted);
        }
        prop_assert_eq!(snapshot.earliest().is_none(), snapshot.is_empty());
        if let Some(earliest) = snapshot.earliest() {
            for version in snapshot.versions() {
                prop_assert!(earliest <= version, "the live floor precedes every live version");
            }
        }

        let later = peer.send(u64::MAX).unwrap();
        prop_assert!(!snapshot.contains(&later));
    }

    /// All `Version`s created within a single batch are distinct, even
    /// when several values in the batch are equal.
    #[test]
    fn distinct_versions_per_batch(values in vec(any::<u64>(), 1..=32), network in network_seed()) {
        let peer = network.builder::<u64>().sync_memory_budget(0).seed();
        let created = batch_send(&peer, &values);
        prop_assert_eq!(created.len(), values.len());
        let unique: BTreeSet<_> =
            created.iter().map(|v| v.as_bytes().to_vec()).collect();
        prop_assert_eq!(unique.len(), values.len(), "versions must be distinct");
    }

    /// The same value inserted `n` times in one batch still yields `n`
    /// distinct leaves — each send creates a fresh `Version`, so content
    /// equality does not collapse messages.
    #[test]
    fn duplicate_values_get_distinct_versions(
        n in 1usize..=16,
        value in any::<u64>(),
        network in network_seed(),
    ) {
        let peer = network.builder::<u64>().sync_memory_budget(0).seed();
        let values: Vec<u64> = std::iter::repeat_n(value, n).collect();
        let created = batch_send(&peer, &values);
        prop_assert_eq!(created.len(), n);
        let unique: BTreeSet<_> =
            created.iter().map(|v| v.as_bytes().to_vec()).collect();
        prop_assert_eq!(unique.len(), n);
    }

    /// Every `Version` created by a lone peer is totally ordered against
    /// every other — both within a single batch (the batch docs promise
    /// strictly increasing versions per action) and across successive
    /// batches.
    ///
    /// With one party and no gossip there is no concurrency, so
    /// any incomparable or equal pair would betray a versioning bug.
    #[test]
    fn local_versions_form_a_chain(
        batches in vec(vec(any::<u64>(), 1..=8), 1..=8),
        network in network_seed(),
    ) {
        let peer = network.builder::<u64>().sync_memory_budget(0).seed();

        // Versions in commit order: per batch, the created versions sorted
        // into their (total) causal order; batches concatenated in commit
        // order. Each batch's recovery is scoped by the pre-commit frontier.
        let mut versions: Vec<Version> = Vec::new();
        for batch in &batches {
            let mut created: Vec<Version> = batch_send(&peer, batch);
            created.sort_by(|a, b| {
                a.partial_cmp(b).expect("a lone peer's versions are totally ordered")
            });
            versions.extend(created);
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

    /// A batch's live-value multiset does not depend on input order. Inserting
    /// a generated sequence and a generated permutation into two fresh peers
    /// yields equal multisets, although their assigned versions may differ.
    #[test]
    fn batch_live_values_are_input_order_independent(
        (values, shuffled) in vec(any::<u64>(), 0..=16).prop_flat_map(|values| {
            let shuffled = Just(values.clone()).prop_shuffle();
            (Just(values), shuffled)
        }),
        network in network_seed(),
    ) {
        // Each peer is its own fresh seed (the two never gossip, so they
        // need not share a network). Read the live multiset directly off
        // the snapshot.
        let multiset_of = |values: &[u64]| -> BTreeMap<u64, usize> {
            let peer = network.builder::<u64>().sync_memory_budget(0).seed();
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

/// A payload that satisfies the storage contract but is neither `Clone` nor
/// `Debug`.
#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct OpaquePayload(u64);

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// Snapshot traits inspect the stored representation rather than requiring
    /// the payload to implement the corresponding traits. Its iterator also has
    /// the public, nameable `Iter` type.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn snapshot_traits_do_not_inherit_payload_bounds(network in network_seed()) {
        let peer = network.peer::<OpaquePayload>();
        peer.send(OpaquePayload(7)).unwrap();
        let snapshot = peer.snapshot();
        let clone = snapshot.clone();

        assert!(snapshot == clone);
        let rendered = format!("{snapshot:?}");
        assert!(rendered.starts_with("Snapshot"));
        let _: rumors::Iter<'_, OpaquePayload> = snapshot.iter();
    }

    /// Snapshot equality covers the network, live set, and causal frontier.
    ///
    /// The claim holds for every pair of distinct network seeds.
    #[test]
    fn snapshot_equality_compares_complete_replica_state(
        (network, other_network) in distinct_network_seeds(),
    ) {
        let peer = network.peer::<u64>();
        let initial = peer.snapshot();
        assert_eq!(initial, initial.clone());

        let version = peer.send(1).unwrap();
        assert_ne!(initial, peer.snapshot(), "the live set changed");
        peer.redact(&version);
        let advanced_empty = peer.snapshot();
        assert!(initial.is_empty() && advanced_empty.is_empty());
        assert_ne!(initial, advanced_empty, "the causal frontier changed");

        let other = other_network.peer::<u64>().snapshot();
        assert_ne!(initial, other, "the network changed");
    }
}

/// A value whose serialization fails on demand, so a test can make admission
/// reject a chosen message with
/// [`EncodeError::Serialize`](rumors::error::EncodeError::Serialize).
#[derive(Debug, PartialEq, Eq)]
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

// Peer construction builds the payload deserializer up front, so even this
// send-only payload type states how its wire form reads back.
impl<'de> serde::Deserialize<'de> for Explosive {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Explosive {
            value: u64::deserialize(deserializer)?,
            fail: false,
        })
    }
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// A batch whose closure panics commits nothing, earlier-queued sends
    /// included.
    ///
    /// [`Peer::batch`](rumors::Peer::batch) commits iff the closure
    /// returns `Ok`; a panic's unwind exits the closure without returning, so
    /// the commit call never runs and nothing publishes — structurally, with
    /// no unwind detection anywhere. The closure queues one send, then
    /// panics.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_panicked_batch_commits_nothing(network in network_seed()) {
        let peer: Peer<u64> = network.builder().sync_memory_budget(0).seed();
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            peer.batch(|batch| {
                batch.send(1)?;
                panic!("the closure unwinds after queueing a send");
                #[allow(unreachable_code)]
                Ok::<(), rumors::error::EncodeError>(())
            })
        }));
        assert!(unwound.is_err(), "the closure must panic");
        assert_eq!(
            peer.snapshot().len(),
            0,
            "a batch whose closure panicked must commit nothing"
        );
    }

    /// [`Peer::send_all`](rumors::Peer::send_all) commits nothing when a
    /// message fails to serialize.
    ///
    /// Wherever the failing message sits in the iterator, it is the returned
    /// [`EncodeError::Serialize`](rumors::error::EncodeError::Serialize).
    /// The messages admitted before the failing one are cancelled with it,
    /// and admission stops at the failing message rather than draining the
    /// iterator.
    ///
    /// The claim holds for every network seed, batch length, and position of
    /// the failing message.
    #[test]
    fn send_all_commits_nothing_when_a_message_fails_to_serialize(
        network in network_seed(),
        (len, fail_at) in (1..8_u64).prop_flat_map(|len| (Just(len), 0..len)),
    ) {
        let peer: Peer<Explosive> = network.builder().sync_memory_budget(0).seed();

        let mut drawn = 0_u64;
        let error = peer
            .send_all(
                (0..len)
                    .map(|i| Explosive {
                        value: i,
                        fail: i == fail_at,
                    })
                    .inspect(|_| drawn += 1),
            )
            .expect_err("one message fails to serialize");
        assert!(
            matches!(error, rumors::error::EncodeError::Serialize(_)),
            "the rejection is the serialization case: {error:?}"
        );
        assert_eq!(
            peer.snapshot().len(),
            0,
            "a rejected send_all commits nothing, earlier-admitted messages included"
        );
        assert_eq!(drawn, fail_at + 1, "admission stops at the failing message");
    }

    /// A batch commits nothing until — and everything once — the closure
    /// returns `Ok`: mid-closure the set is untouched (queueing publishes
    /// nothing), and the `Ok` return lands sends and redactions together,
    /// all-or-nothing.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_batch_commits_iff_the_closure_returns_ok(network in network_seed()) {
        let peer: Peer<u64> = network.builder().sync_memory_budget(0).seed();
        let doomed = peer.send(7).unwrap();

        peer
            .batch(|batch| {
                batch.send(1)?;
                batch.send(2)?;
                batch.redact(&doomed);
                // Nothing queued is visible while the closure runs: the batch
                // publishes only at its commit.
                assert_eq!(peer.snapshot().len(), 1, "queueing publishes nothing");
                Ok::<(), rumors::error::EncodeError>(())
            })
            .expect("flat payloads are within any depth limit");

        let live: Vec<u64> = peer.snapshot().iter().map(|(_, m)| *m).collect();
        assert_eq!(live.len(), 2, "the sends landed and the redaction took");
        assert!(live.contains(&1) && live.contains(&2));
    }
}

/// Pure CBOR array nesting from a type meeting every message type
/// requirement: each layer is a one-element array, the innermost empty.
/// `nested(n)` is `n` layers deep.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Arr(Vec<Arr>);

impl Arr {
    fn nested(layers: usize) -> Self {
        (0..layers).fold(Arr(vec![]), |a, _| Arr(vec![a]))
    }
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// A depth-rejected send, `?`-propagated out of the closure, cancels
    /// the whole batch: earlier-queued actions included — the
    /// cancel-on-error pin.
    ///
    /// The error carries the configured limit, and the tree is
    /// untouched.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_depth_error_cancels_the_whole_batch(network in network_seed()) {
        let limit = 4;
        let peer: Peer<Arr> = network.builder()
            .message_depth_limit(limit)
            .sync_memory_budget(0).seed();
        let deep = Arr::nested(4);

        let error = peer
            .batch(|batch| {
                batch.send(Arr(vec![]))?;
                batch.send(deep.clone())?;
                Ok::<(), rumors::error::EncodeError>(())
            })
            .expect_err("the second send exceeds the limit");
        assert!(
            matches!(error, rumors::error::EncodeError::TooDeep { limit: l } if l == limit),
            "the rejection is the depth case naming the limit: {error:?}"
        );
        assert_eq!(
            peer.snapshot().len(),
            0,
            "a cancelled batch commits nothing, earlier-queued sends included"
        );
    }

    /// A user `Err` return cancels the batch — the deliberate abort
    /// affordance — and comes back verbatim.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_user_error_cancels_the_batch(network in network_seed()) {
        let peer: Peer<u64> = network.builder().sync_memory_budget(0).seed();
        let error = peer
            .batch(|batch| {
                batch.send(1).unwrap();
                batch.send(2).unwrap();
                Err::<(), &str>("changed my mind")
            })
            .expect_err("the closure aborts deliberately");
        assert_eq!(error, "changed my mind");
        assert_eq!(
            peer.snapshot().len(),
            0,
            "an aborted batch commits nothing"
        );
    }

    /// [`Peer::send_all`](rumors::Peer::send_all) is one all-or-nothing
    /// commit: a depth-rejected message anywhere in the iterator is the
    /// returned error, and nothing lands.
    ///
    /// The messages admitted before the rejected one are cancelled with it,
    /// and admission stops at the rejected message rather than draining the
    /// iterator.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn send_all_commits_nothing_when_a_message_is_rejected(network in network_seed()) {
        let limit = 4;
        let peer: Peer<Arr> = network.builder()
            .message_depth_limit(limit)
            .sync_memory_budget(0).seed();

        let mut drawn = 0;
        let error = peer
            .send_all(
                [
                    Arr::nested(0),
                    Arr::nested(1),
                    Arr::nested(4),
                    Arr::nested(2),
                ]
                .into_iter()
                .inspect(|_| drawn += 1),
            )
            .expect_err("the third message exceeds the limit");
        assert!(
            matches!(error, rumors::error::EncodeError::TooDeep { limit: l } if l == limit),
            "the rejection is the depth case naming the limit: {error:?}"
        );
        assert_eq!(
            peer.snapshot().len(),
            0,
            "a rejected send_all commits nothing, earlier-admitted messages included"
        );
        assert_eq!(drawn, 3, "admission stops at the rejected message");
    }

    /// [`Peer::redact_all`](rumors::Peer::redact_all) removes every held
    /// version it names, skips versions not currently held, and lands as
    /// one commit: a change observer sees exactly one tick for the whole
    /// redaction.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn redact_all_removes_the_held_skips_the_unheld_and_commits_once(network in network_seed()) {
        use futures::{FutureExt, StreamExt};

        let peer: Peer<u64> = network.builder().sync_memory_budget(0).seed();
        batch_send(&peer, &[1, 2, 3, 4]);
        // Versions come back through observation, by payload: a batch's
        // versions carry no input-order correspondence.
        let version_of = |payload: u64| -> Version {
            peer
                .snapshot()
                .iter()
                .find(|(_, m)| **m == payload)
                .map(|(v, _)| v.clone())
                .expect("the payload is live")
        };
        let targets = [version_of(1), version_of(2), version_of(3)];
        // Redacting the first version singly makes it a version not held
        // when the bulk redaction names it again.
        peer.redact(&targets[0]);

        let mut changes = peer.changes();
        assert_eq!(
            changes.next().now_or_never(),
            Some(Some(())),
            "a fresh observer's first tick is immediate"
        );
        assert_eq!(changes.next().now_or_never(), None, "then quiet");

        peer.redact_all(&targets);
        assert_eq!(
            changes.next().now_or_never(),
            Some(Some(())),
            "the whole redaction is one commit, one tick"
        );
        assert_eq!(changes.next().now_or_never(), None, "and only one");

        let live: Vec<u64> = peer.snapshot().iter().map(|(_, m)| *m).collect();
        assert_eq!(
            live,
            vec![4],
            "the two held versions are gone, the unheld one skipped, the rest untouched"
        );
    }

    /// A rejected [`Batch::send_all`](rumors::Batch::send_all) handled inside
    /// the closure leaves the batch alive with the admitted prefix queued.
    ///
    /// A [`Batch::redact_all`] in the same closure lands with that prefix as
    /// one commit.
    ///
    /// [`Batch::redact_all`]: rumors::Batch::redact_all
    ///
    /// The claim holds for every network seed.
    #[test]
    fn batch_send_all_handled_locally_keeps_the_admitted_prefix(network in network_seed()) {
        let limit = 4;
        let peer: Peer<Arr> = network.builder()
            .message_depth_limit(limit)
            .sync_memory_budget(0).seed();
        peer.send_all([Arr::nested(3)]).unwrap();
        let doomed: Vec<Version> = peer.snapshot().versions().cloned().collect();

        peer
            .batch(|batch| {
                let error = batch
                    .send_all([
                        Arr::nested(0),
                        Arr::nested(1),
                        Arr::nested(4),
                        Arr::nested(2),
                    ])
                    .expect_err("the third message exceeds the limit");
                assert!(matches!(error, rumors::error::EncodeError::TooDeep { .. }));
                batch.redact_all(&doomed);
                Ok::<(), rumors::error::EncodeError>(())
            })
            .expect("the closure handles the rejection itself");

        let mut live: Vec<Arr> = peer
            .snapshot()
            .iter()
            .map(|(_, m)| (*m).clone())
            .collect();
        live.sort_by_key(|a| a.0.len());
        assert_eq!(
            live,
            vec![Arr::nested(0), Arr::nested(1)],
            "the admitted prefix and the redaction land together; the rest never queued"
        );
    }

    /// Batches nest: the closure may use the same handle, and the nested
    /// operations commit first, the outer batch after, as separate commits
    /// (inner-before-outer).
    ///
    /// The claim holds for every network seed.
    #[test]
    fn nested_batches_commit_inner_before_outer(network in network_seed()) {
        let peer: Peer<u64> = network.builder().sync_memory_budget(0).seed();
        peer
            .batch(|batch| {
                batch.send(1)?;
                // Re-entrant single send and nested batch, both on the same
                // handle: each commits immediately, before the outer batch.
                peer.send(2)?;
                peer.batch(|inner| inner.send(3))?;
                let mid: Vec<u64> = peer.snapshot().iter().map(|(_, m)| *m).collect();
                assert!(
                    mid.contains(&2) && mid.contains(&3) && !mid.contains(&1),
                    "inner commits land before the outer batch: {mid:?}"
                );
                Ok::<(), rumors::error::EncodeError>(())
            })
            .expect("flat payloads are within any depth limit");
        assert_eq!(peer.snapshot().len(), 3, "the outer batch lands last");
    }
}
