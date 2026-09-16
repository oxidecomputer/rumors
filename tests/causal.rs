//! The [`rumors::CausalMessages`] observer: the causal-delivery contract on top
//! of everything [`UnorderedMessages`](rumors::UnorderedMessages) already
//! promises (exercised in `tests/listen.rs`).
//!
//! The contract: no message is ever delivered before a delivered message it
//! causally depends on, within a backlog and across live passes, and the
//! resume checkpoint lags the staged backlog so resumption never skips an
//! undelivered message.
//!
//! Driven step-by-step with `now_or_never`, as in `tests/listen.rs`: an
//! *item*, a *quiet* observer (no change to report, actors live), or an
//! *ended* one (final state fully delivered).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use futures::{FutureExt, StreamExt};
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Peer, Rumors, Version};

use crate::common::observer::{
    Step, arb_ops, drain, interleave, live_map, redact_during_pass, step,
};
use crate::common::wire::{bootstrap_fork, wire_gossip};

/// Assert the causal-delivery contract on a delivered sequence: no message
/// precedes a delivered message it causally dominates — for every pair, the
/// later-delivered version is never strictly less than the earlier one.
// `Version` is a partial order: `!(later < earlier)` also admits concurrent
// pairs, which `later >= earlier` would reject.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn assert_causal(items: &[(Version, u64)]) {
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            assert!(
                !(items[j].0 < items[i].0),
                "causal inversion: item {j} ({:?}) causally precedes item {i} ({:?})",
                items[j].0,
                items[i].0,
            );
        }
    }
}

/// A single party's sends form a causal chain, so a fresh observer must
/// deliver the whole backlog in exactly send order — the case path-ordered
/// delivery scrambles roughly half the time.
#[test]
fn single_party_backlog_replays_in_send_order() {
    let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
    for v in 0..8u64 {
        known.send(v).unwrap(); // one commit per send: strictly increasing versions
    }

    let mut obs = known.causal_messages();
    let (items, ended) = drain(&mut obs);
    assert!(!ended, "the set is live: quiet, not ended");
    assert_eq!(
        items.iter().map(|(_, m)| *m).collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>(),
        "a causal chain is delivered in chain order"
    );
    assert_causal(&items);
}

/// One captured backlog follows the staging order: rank, then version bytes.
/// This checks the internal ordering within a pass; concurrent messages
/// arriving in later passes need not follow it.
#[test]
fn converged_backlog_has_no_inversions() {
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    for v in 0..4u64 {
        a.send(v).unwrap();
    }
    for v in 10..14u64 {
        b.send(v).unwrap();
    }
    wire_gossip(&a, &b);

    let mut obs = a.causal_messages();
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 8, "both chains are in the converged backlog");
    assert_causal(&items);

    // The staging map orders one snapshot by rank and breaks concurrent ties
    // with version bytes.
    let ranks: Vec<_> = items
        .iter()
        .map(|(v, _)| (v.rank(), v.as_bytes().to_vec()))
        .collect();
    assert!(
        ranks.windows(2).all(|w| w[0] < w[1]),
        "a single backlog drains in strictly increasing (rank, bytes) order"
    );
}

/// Sorting a snapshot by `Version::ranked` places a shared cause before two
/// concurrent effects and gives those effects a deterministic order.
#[test]
fn snapshot_items_sort_by_ranked_version() {
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let cause = a.send(0).unwrap();
    let b = bootstrap_fork(&a);
    let left = a.send(1).unwrap();
    let right = b.send(2).unwrap();
    assert!(
        left.partial_cmp(&right).is_none(),
        "the effects are concurrent"
    );
    wire_gossip(&a, &b);

    let snapshot = a.snapshot();
    let mut items: Vec<_> = snapshot.iter().collect();
    items.sort_by(|a, b| a.0.ranked().cmp(&b.0.ranked()));
    assert_eq!(items.first().unwrap().0, &cause);
    assert_causal(
        &items
            .into_iter()
            .map(|(version, value)| (version.clone(), *value))
            .collect::<Vec<_>>(),
    );
}

/// Identical snapshots use the same internal staging order for a single pass.
/// The public contract allows concurrent messages to arrive in either order.
#[test]
fn identical_backlogs_use_the_same_staging_order() {
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    a.send_all([1, 2]).unwrap();
    b.send_all([3, 4]).unwrap();
    wire_gossip(&a, &b);
    a.send(5).unwrap();
    b.send(6).unwrap();
    wire_gossip(&a, &b);
    assert_eq!(a.snapshot(), b.snapshot(), "the replicas converged");

    let (from_a, _) = drain(&mut a.causal_messages());
    let (from_b, _) = drain(&mut b.causal_messages());
    assert_eq!(
        from_a, from_b,
        "identical snapshots have identical staging order"
    );
}

/// Causal order holds *across* passes, not just within one: messages
/// delivered live (pass by pass, interleaved with sends and gossip) never
/// invert against earlier deliveries.
///
/// Order holds because a later pass can never
/// contain a causal predecessor of an earlier pass's message.
#[test]
fn live_passes_preserve_causal_order_cumulatively() {
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    let mut obs = a.causal_messages();
    let mut delivered = Vec::new();

    a.send(1).unwrap();
    delivered.extend(drain(&mut obs).0);

    b.send_all([2, 3]).unwrap();
    wire_gossip(&a, &b);
    delivered.extend(drain(&mut obs).0);

    a.send(4).unwrap();
    b.send(5).unwrap();
    wire_gossip(&a, &b);
    delivered.extend(drain(&mut obs).0);

    assert_eq!(delivered.len(), 5, "every message was delivered live");
    assert_causal(&delivered);
}

/// The resume point lags the staged backlog: after delivering part of a
/// backlog, `checkpoint()` still names the batch's range start.
///
/// A resume therefore
/// re-delivers the partial batch (at-least-once) rather than losing the
/// undelivered remainder.
///
/// Once the backlog drains, the checkpoint catches up
/// and a resume observes nothing.
#[test]
fn checkpoint_lags_until_the_backlog_drains() {
    let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let genesis = known.snapshot().latest().clone();
    known.send(1).unwrap();
    known.send(2).unwrap();
    known.send(3).unwrap();

    let mut obs = known.causal_messages();
    let Step::Item(first) = step(&mut obs) else {
        panic!("a populated set delivers an item");
    };
    assert_eq!(
        obs.checkpoint(),
        &genesis,
        "mid-backlog, the checkpoint holds at the batch's range start"
    );

    // A resume from the lagging checkpoint re-delivers the whole batch,
    // including the already-delivered first item: re-delivery, never loss.
    let mut resumed = known.causal_messages_since(obs.checkpoint().clone());
    let (resumed_items, _) = drain(&mut resumed);
    assert_eq!(resumed_items.len(), 3, "the partial batch re-delivers");
    assert!(resumed_items.contains(&first));

    // Drain the original: the checkpoint catches up to the ingest frontier and
    // a fresh resume from it observes nothing.
    let (rest, _) = drain(&mut obs);
    assert_eq!(rest.len(), 2);
    assert_causal(&[vec![first], rest].concat());
    let mut from_drained = known.causal_messages_since(obs.checkpoint().clone());
    let (none, _) = drain(&mut from_drained);
    assert!(none.is_empty(), "a drained backlog's checkpoint is current");
}

/// Termination mirrors the plain observer: when every handle drops, the
/// observer delivers the complete final state — in causal order — then
/// ends, and ended is terminal.
#[test]
fn observer_drains_the_final_state_causally_then_ends() {
    let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
    known.send_all([1, 2, 3]).unwrap();
    let expected = live_map(&known);

    let mut obs = known.causal_messages();
    drop(known);

    let (items, ended) = drain(&mut obs);
    assert!(ended, "with every sender gone the observer ends");
    assert_causal(&items);
    assert_eq!(
        items
            .iter()
            .map(|(v, m)| (v.as_bytes().to_vec(), *m))
            .collect::<BTreeMap<_, _>>(),
        expected,
        "the complete final state is yielded before the end"
    );
    assert_eq!(step(&mut obs), Step::Ended, "ended is terminal");
}

/// The `Stream` face delivers in causal order and terminates with `None`
/// once the set closes.
#[test]
fn stream_face_is_causal_and_terminates() {
    let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
    for v in 0..6u64 {
        known.send(v).unwrap();
    }

    let mut obs = known.causal_messages();
    let mut items = Vec::new();
    while let Some(Some((v, m))) = obs.next().now_or_never() {
        items.push((v, *m));
    }
    assert_eq!(
        items.iter().map(|(_, m)| *m).collect::<Vec<_>>(),
        (0..6).collect::<Vec<_>>(),
        "the Stream face replays the chain in order"
    );

    drop(known);
    assert_eq!(
        obs.next().now_or_never(),
        Some(None),
        "the Stream ends once the set closes"
    );
}

proptest! {
    /// Later passes may deliver lower-ranked concurrent messages without a causal inversion.
    #[test]
    fn concurrent_later_arrivals_need_not_follow_rank_order(count in 2u64..10) {
        let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork(&a);
        a.send_all(0..count).unwrap();
        b.send(count).unwrap();
        let mut obs = a.causal_messages();
        let (mut delivered, _) = drain(&mut obs);
        wire_gossip(&a, &b);
        let (later, _) = drain(&mut obs);
        prop_assert_eq!(later.len(), 1);
        let earlier = &delivered.last().unwrap().0;
        let late = &later[0].0;
        prop_assert!(late.rank() < earlier.rank());
        prop_assert_eq!(late.partial_cmp(earlier), None, "the versions are concurrent");
        delivered.extend(later);
        assert_causal(&delivered);
    }

    /// A captured pass survives local and remote redactions while it is partly read.
    /// A message redacted before capture produces no delivery.
    #[test]
    fn captured_pass_survives_redaction(
        values in vec(any::<u64>(), 2..10),
        taken in any::<prop::sample::Index>(),
        redactions in vec(any::<prop::sample::Index>(), 1..10),
        remote in any::<bool>(),
    ) {
        redact_during_pass(values, taken, redactions, remote, Rumors::causal_messages)?;
    }

    /// Delivery remains duplicate-free and covers the final live set across
    /// sends, local and remote redactions, gossip, and partially drained passes.
    /// Every pair of delivered messages also respects causal order.
    #[test]
    fn causal_delivery_under_interleaving(ops in arb_ops()) {
        let delivered = interleave(&ops, Rumors::causal_messages)?;
        assert_causal(&delivered);
    }

    /// Each resumed run is causal and duplicate-free; together they cover the live set.
    /// Completing a backlog before taking its checkpoint prevents replay across runs.
    #[test]
    fn checkpoint_resume_loses_nothing(
        phase_one in vec(any::<u64>(), 1..8),
        phase_two in vec(any::<u64>(), 0..8),
        taken in any::<prop::sample::Index>(),
        complete_drain in any::<bool>(),
    ) {
        let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
        for v in &phase_one {
            known.send(*v).unwrap(); // separate commits: a strict causal chain
        }

        let mut obs = known.causal_messages();
        let mut first_run: Vec<(Version, u64)> = Vec::new();
        if complete_drain {
            first_run.extend(drain(&mut obs).0);
        } else {
            for _ in 0..(taken.index(phase_one.len() + 1)) {
                match step(&mut obs) {
                    Step::Item(item) => first_run.push(item),
                    other => panic!("the backlog has more items, got {other:?}"),
                }
            }
        }
        assert_causal(&first_run);
        let checkpoint = obs.checkpoint().clone();
        drop(obs);

        for v in &phase_two {
            known.send(*v).unwrap();
        }

        let mut resumed = known.causal_messages_since(checkpoint);
        let final_live = live_map(&known);
        drop(known);
        let (second_run, ended) = drain(&mut resumed);
        prop_assert!(ended);
        assert_causal(&second_run);

        // Nothing lost: the union of the two runs covers the final state.
        for (key, value) in &final_live {
            prop_assert!(
                first_run
                    .iter()
                    .chain(&second_run)
                    .any(|(v, m)| v.as_bytes() == key.as_slice() && m == value),
                "a live message fell between the stopped and resumed observers",
            );
        }

        // After a complete drain the checkpoint is current: no re-delivery.
        let first_versions: BTreeSet<Vec<u8>> =
            first_run.iter().map(|(v, _)| v.as_bytes().to_vec()).collect();
        let second_versions: BTreeSet<Vec<u8>> =
            second_run.iter().map(|(v, _)| v.as_bytes().to_vec()).collect();
        prop_assert_eq!(first_versions.len(), first_run.len(), "first run repeated a version");
        prop_assert_eq!(second_versions.len(), second_run.len(), "resumed run repeated a version");
        if complete_drain {
            prop_assert!(
                first_versions.is_disjoint(&second_versions),
                "a drained backlog's messages must not re-fire",
            );
        }
    }

    /// The restart shape, generalized over both observer faces: a resumed
    /// observer re-delivers every live message a crashed process had not
    /// handled — the in-flight last delivery included.
    ///
    /// The process follows the persist-after-delivery protocol — deliver a
    /// message, persist the checkpoint as bytes, handle the message — and
    /// crashes with the last delivered message still unhandled, dropping
    /// every handle it held. A rebuilt replica of the same network resumes
    /// each face from the deserialized checkpoint, and the resumed run
    /// must deliver every unhandled live message: at-least-once, so replay
    /// is permitted and loss never is. Both runs of the causal face are
    /// individually causal.
    #[test]
    fn restart_replays_every_unhandled_message(
        local in vec(any::<u64>(), 1..8),
        remote in vec(any::<u64>(), 0..4),
        taken in any::<prop::sample::Index>(),
    ) {
        // Always test the final delivery too: an empty backlog can still
        // leave an unhandled message in the caller's hands at the crash.
        let total = local.len() + remote.len();
        for taken in [taken.index(total + 1), total] {
            // Two replicas of one network: `known` is the crashing process,
            // `partner` survives it to seed the rebuild.
            let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
            let partner = bootstrap_fork(&known);
            for v in &local {
                known.send(*v).unwrap();
            }
            for v in &remote {
                partner.send(*v).unwrap(); // concurrent with `local`: a real partial order
            }
            wire_gossip(&known, &partner);
            let final_live = live_map(&known);

            // Deliver `taken` messages on each face. All but the last delivered
            // message count as handled; the last is in flight — delivered, its
            // checkpoint persisted, not yet handled — when the process dies.
            let mut causal = known.causal_messages();
            let mut causal_delivered: Vec<(Version, u64)> = Vec::new();
            for _ in 0..taken {
                match step(&mut causal) {
                    Step::Item(item) => causal_delivered.push(item),
                    other => panic!("the backlog has more items, got {other:?}"),
                }
            }
            assert_causal(&causal_delivered);
            let causal_checkpoint = causal.checkpoint().as_bytes().to_vec();

            let mut unordered = known.unordered_messages();
            let mut unordered_delivered: Vec<(Version, u64)> = Vec::new();
            for _ in 0..taken {
                match unordered.try_next() {
                    rumors::TryNext::Message((version, message)) => {
                        unordered_delivered.push((version, *message));
                    }
                    other => panic!("the pass has more items, got {other:?}"),
                }
            }
            let unordered_checkpoint = unordered.checkpoint().as_bytes().to_vec();

            let causal_handled: BTreeSet<Vec<u8>> = causal_delivered
                .iter()
                .rev()
                .skip(1)
                .map(|(v, _)| v.as_bytes().to_vec())
                .collect();
            let unordered_handled: BTreeSet<Vec<u8>> = unordered_delivered
                .iter()
                .rev()
                .skip(1)
                .map(|(v, _)| v.as_bytes().to_vec())
                .collect();

            // The crash: every handle the process held goes away at once.
            drop(causal);
            drop(unordered);
            drop(known);

            // The rebuild: a fresh replica of the same network (bootstrapped
            // converged from the survivor) resumes each face from the
            // persisted bytes.
            let rebuilt = bootstrap_fork(&partner);

            let since =
                Version::decode(&causal_checkpoint[..]).expect("a checkpoint deserializes");
            let mut resumed = rebuilt.causal_messages_since(since);
            let (replayed, _) = drain(&mut resumed);
            assert_causal(&replayed);
            for (key, value) in &final_live {
                prop_assert!(
                    causal_handled.contains(key)
                        || replayed
                            .iter()
                            .any(|(v, m)| v.as_bytes() == key.as_slice() && m == value),
                    "causal: unhandled live message {key:?} fell through the restart",
                );
            }

            let since =
                Version::decode(&unordered_checkpoint[..]).expect("a checkpoint deserializes");
            let mut resumed = rebuilt.unordered_messages_since(since);
            let replayed = drain(&mut resumed).0;
            for (key, value) in &final_live {
                prop_assert!(
                    unordered_handled.contains(key)
                        || replayed
                            .iter()
                            .any(|(v, m)| v.as_bytes() == key.as_slice() && m == value),
                    "unordered: unhandled live message {key:?} fell through the restart",
                );
            }
        }
    }
}

/// The crash boundary at the final pop: a checkpoint read after the last
/// staged message is handed over — but before the caller has durably
/// handled it — still replays that message on resume.
///
/// `checkpoint()` promises "resuming from this `Version` will never skip
/// messages" (at-least-once), the same promise the mid-backlog lag in
/// `checkpoint_lags_until_the_backlog_drains` pins, here at the
/// backlog's last item instead of its middle.
///
/// Both observers hold this boundary the same way: the checkpoint catches
/// up only on the call *after* the backlog (or pass) drains, never in the
/// step that hands over its last message, so a checkpoint persisted while
/// the caller still holds an unhandled message never covers that message.
/// This test pins the boundary on both faces.
#[test]
fn final_pop_checkpoint_still_replays_the_last_message() {
    let known = Peer::<u64>::seed().sync_window_floor().into_rumors();
    known.send(7).unwrap();

    // The unordered observer under the probe's protocol — deliver the only
    // message, persist the checkpoint, crash, resume — replays the message.
    let mut unordered = known.unordered_messages();
    assert!(
        unordered.next().now_or_never().flatten().is_some(),
        "a populated set delivers an item"
    );
    let persisted = unordered.checkpoint().clone();
    let mut resumed = known.unordered_messages_since(persisted);
    assert!(
        matches!(resumed.try_next(), rumors::TryNext::Message(_)),
        "unordered: the in-flight message replays after a crash-resume"
    );

    // The causal observer under the identical protocol.
    let mut causal = known.causal_messages();
    let Step::Item(item) = step(&mut causal) else {
        panic!("a populated set delivers an item");
    };
    let persisted = causal.checkpoint().clone();
    let mut resumed = known.causal_messages_since(persisted);
    let (replayed, _) = drain(&mut resumed);
    assert_eq!(
        replayed,
        vec![item],
        "causal: the in-flight message must replay after a crash-resume"
    );
}
