//! The [`CausalMessages`] observer: the causal-delivery contract on top of
//! everything [`Messages`](rumors::Messages) already promises (exercised in
//! `tests/listen.rs`) — no message is ever delivered before a delivered
//! message it causally depends on, within a backlog and across live passes,
//! with the resume checkpoint lagging the staged backlog so resumption never
//! skips an undelivered message.
//!
//! Driven step-by-step with `now_or_never`, as in `tests/listen.rs`: an
//! *item*, a *quiet* observer (no change to report, actors live), or an
//! *ended* one (final state fully delivered).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use futures::FutureExt;
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{CausalMessages, Key, Known, Version};

use crate::common::action::minted_key;
use crate::common::wire::{bootstrap_fork, wire_gossip};

/// One observer step, with the borrowed faces cloned out.
#[derive(Debug, PartialEq)]
enum Step {
    /// The observer yielded a message.
    Item((Key, Version, u64)),
    /// The observer is quiet: nothing new, actors still live.
    Quiet,
    /// The observer ended: every sender is gone and the complete final
    /// state has been yielded.
    Ended,
}

/// Poll `borrow_next` exactly once without an executor.
fn step(obs: &mut CausalMessages<u64>) -> Step {
    match obs.borrow_next().now_or_never() {
        None => Step::Quiet,
        Some(None) => Step::Ended,
        Some(Some((k, v, m))) => Step::Item((k, v.clone(), **m)),
    }
}

/// Drain the observer until it goes quiet or ends, returning the items in
/// delivery order and whether it ended.
fn drain(obs: &mut CausalMessages<u64>) -> (Vec<(Key, Version, u64)>, bool) {
    let mut items = Vec::new();
    loop {
        match step(obs) {
            Step::Item(item) => items.push(item),
            Step::Quiet => return (items, false),
            Step::Ended => return (items, true),
        }
    }
}

/// Assert the causal-delivery contract on a delivered sequence: no message
/// precedes a delivered message it causally dominates — for every pair, the
/// later-delivered version is never strictly less than the earlier one.
// `Version` is a partial order: `!(later < earlier)` also admits concurrent
// pairs, which `later >= earlier` would reject.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn assert_causal(items: &[(Key, Version, u64)]) {
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            assert!(
                !(items[j].1 < items[i].1),
                "causal inversion: item {j} ({:?}) causally precedes item {i} ({:?})",
                items[j].0,
                items[i].0,
            );
        }
    }
}

/// The live `Key → value` map, for comparing against deliveries.
fn live_map(known: &Known<u64>) -> BTreeMap<Key, u64> {
    known.snapshot().iter().map(|(k, _, m)| (k, **m)).collect()
}

/// A single party's sends form a causal chain, so a fresh observer must
/// deliver the whole backlog in exactly send order — the case key-ordered
/// delivery scrambles roughly half the time.
#[test]
fn single_party_backlog_replays_in_send_order() {
    let known = Known::<u64>::seed();
    for v in 0..8u64 {
        known.send(v); // one batch per send: strictly increasing versions
    }

    let mut obs = known.causal_messages();
    let (items, ended) = drain(&mut obs);
    assert!(!ended, "the Known is live: quiet, not ended");
    assert_eq!(
        items.iter().map(|(_, _, m)| *m).collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>(),
        "a causal chain is delivered in chain order"
    );
    assert_causal(&items);
}

/// A backlog mixing one party's chain with a concurrent peer's (learned via
/// gossip) is delivered without causal inversions, and concurrent messages
/// come out in the deterministic `(area, key)` rank order.
#[test]
fn converged_backlog_has_no_inversions() {
    let mut a = Known::<u64>::seed();
    let mut b = bootstrap_fork(&mut a);

    for v in 0..4u64 {
        a.send(v);
    }
    for v in 10..14u64 {
        b.send(v);
    }
    wire_gossip(&mut a, &mut b);

    let mut obs = a.causal_messages();
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 8, "both chains are in the converged backlog");
    assert_causal(&items);

    // One ingest batch pops in (area, key) order: the delivered sequence is
    // sorted by causal rank, which is what makes it deterministic.
    let ranks: Vec<_> = items.iter().map(|(k, v, _)| (v.area(), *k)).collect();
    assert!(
        ranks.windows(2).all(|w| w[0] < w[1]),
        "a single backlog drains in strictly increasing (area, key) order"
    );
}

/// Two converged replicas deliver the same backlog in the *same* order to
/// fresh observers: the rank order is a property of the set, not of the
/// replica, the insertion order, or the gossip schedule.
#[test]
fn delivery_order_is_replica_independent() {
    let mut a = Known::<u64>::seed();
    let mut b = bootstrap_fork(&mut a);

    a.batch().send(1).send(2);
    b.batch().send(3).send(4);
    wire_gossip(&mut a, &mut b);
    a.send(5);
    b.send(6);
    wire_gossip(&mut a, &mut b);
    assert_eq!(a.hash(), b.hash(), "the replicas converged");

    let (from_a, _) = drain(&mut a.causal_messages());
    let (from_b, _) = drain(&mut b.causal_messages());
    assert_eq!(
        from_a, from_b,
        "identical sets replay identically, replica notwithstanding"
    );
}

/// Causal order holds *across* passes, not just within one: messages
/// delivered live (pass by pass, interleaved with sends and gossip) never
/// invert against earlier deliveries, because a later pass can never
/// contain a causal predecessor of an earlier pass's message.
#[test]
fn live_passes_preserve_causal_order_cumulatively() {
    let mut a = Known::<u64>::seed();
    let mut b = bootstrap_fork(&mut a);

    let mut obs = a.causal_messages();
    let mut delivered = Vec::new();

    a.send(1);
    delivered.extend(drain(&mut obs).0);

    b.batch().send(2).send(3);
    wire_gossip(&mut a, &mut b);
    delivered.extend(drain(&mut obs).0);

    a.send(4);
    b.send(5);
    wire_gossip(&mut a, &mut b);
    delivered.extend(drain(&mut obs).0);

    assert_eq!(delivered.len(), 5, "every message was delivered live");
    assert_causal(&delivered);
}

/// The resume point lags the staged backlog: after delivering part of a
/// backlog, `checkpoint()` still names the batch's range start, so a resume
/// re-delivers the partial batch (at-least-once) rather than losing the
/// undelivered remainder; once the backlog drains, the checkpoint catches up
/// and a resume observes nothing.
#[test]
fn checkpoint_lags_until_the_backlog_drains() {
    let known = Known::<u64>::seed();
    let genesis = known.latest();
    known.send(1);
    known.send(2);
    known.send(3);

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

/// A message staged and then redacted before delivery is still delivered —
/// the same exactly-once-per-observed-liveness contract as the plain
/// observer, where "observed" is the ingest — while a message redacted
/// wholly before its first ingest never appears.
#[test]
fn staged_then_redacted_is_still_delivered() {
    let known = Known::<u64>::seed();
    let pre = known.latest();
    known.send(1);
    let key_1 = minted_key(&known.snapshot(), &pre);
    let pre = known.latest();
    known.send(2);
    let key_2 = minted_key(&known.snapshot(), &pre);

    // First step ingests the whole pass (both messages) and delivers the
    // causally least; the other is staged.
    let mut obs = known.causal_messages();
    let Step::Item((delivered_key, ..)) = step(&mut obs) else {
        panic!("a populated set delivers an item");
    };
    let staged_key = if delivered_key == key_1 { key_2 } else { key_1 };

    // Redact the staged message, then drain: it is delivered anyway (it was
    // live at its ingest), and nothing fires after.
    known.redact(staged_key);
    let (items, _) = drain(&mut obs);
    assert_eq!(
        items.iter().map(|(k, _, _)| *k).collect::<Vec<_>>(),
        vec![staged_key],
        "the staged message outlives its redaction by exactly one delivery"
    );

    // Redacted wholly before any ingest: never delivered.
    let pre = known.latest();
    known.send(3);
    known.redact(minted_key(&known.snapshot(), &pre));
    let (items, _) = drain(&mut obs);
    assert!(items.is_empty(), "pre-ingest redactions never fire");
}

/// Termination mirrors the plain observer: when every handle drops, the
/// observer delivers the complete final state — in causal order — then
/// ends, and ended is terminal.
#[test]
fn observer_drains_the_final_state_causally_then_ends() {
    let known = Known::<u64>::seed();
    known.batch().send(1).send(2).send(3);
    let expected = live_map(&known);

    let mut obs = known.causal_messages();
    drop(known);

    let (items, ended) = drain(&mut obs);
    assert!(ended, "with every sender gone the observer ends");
    assert_causal(&items);
    assert_eq!(
        items
            .iter()
            .map(|(k, _, m)| (*k, *m))
            .collect::<BTreeMap<_, _>>(),
        expected,
        "the complete final state is yielded before the end"
    );
    assert_eq!(step(&mut obs), Step::Ended, "ended is terminal");
}

/// The owned-item face delivers the same causal order as `borrow_next` and
/// terminates with `None` once the set closes.
#[test]
fn stream_face_is_causal_and_terminates() {
    use futures::StreamExt;

    let known = Known::<u64>::seed();
    for v in 0..6u64 {
        known.send(v);
    }

    let mut obs = known.causal_messages();
    let mut items = Vec::new();
    while let Some(Some((k, v, m))) = obs.next().now_or_never() {
        items.push((k, v, *m));
    }
    assert_eq!(
        items.iter().map(|(_, _, m)| *m).collect::<Vec<_>>(),
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

/// The synchronous mirror: `sync::CausalMessages` is an `Iterator` whose
/// items arrive in causal order and whose end is the set closing.
#[test]
fn sync_iterator_face_is_causal() {
    let known = rumors::sync::Known::<u64>::seed();
    for v in 0..5u64 {
        known.send(v);
    }

    let obs = known.causal_messages();
    drop(known);

    let values: Vec<u64> = obs.map(|(_, _, m)| *m).collect();
    assert_eq!(
        values,
        (0..5).collect::<Vec<_>>(),
        "the sync iterator replays the chain in order, then ends"
    );
}

const MAX_OPS: usize = 32;

/// One scripted action against a two-replica universe observed at `a`.
#[derive(Debug, Clone)]
enum Op {
    /// `a` sends this value.
    SendA(u64),
    /// `b` sends this value (concurrent to `a` until a gossip).
    SendB(u64),
    /// Redact the `idx % minted`-th key minted at `a` so far (dropped if
    /// none).
    Redact(usize),
    /// Converge the replicas.
    Gossip,
    /// Drain the observer to quiescence.
    Drain,
}

fn arb_ops() -> impl Strategy<Value = Vec<Op>> {
    vec(
        prop_oneof![
            3 => any::<u64>().prop_map(Op::SendA),
            3 => any::<u64>().prop_map(Op::SendB),
            1 => any::<usize>().prop_map(Op::Redact),
            2 => Just(Op::Gossip),
            3 => Just(Op::Drain),
        ],
        0..=MAX_OPS,
    )
}

proptest! {
    /// The whole contract under arbitrary interleaving of local sends,
    /// concurrent peer sends, redactions, gossip, and partial drains: the
    /// cumulative delivered sequence has no causal inversion, no key fires
    /// twice, and the deliveries cover the final live set — causal order
    /// costs nothing in coverage relative to the plain observer.
    #[test]
    fn causal_delivery_under_interleaving(ops in arb_ops()) {
        let mut a = Known::<u64>::seed();
        let mut b = bootstrap_fork(&mut a);

        let mut obs = a.causal_messages();
        let mut minted: Vec<Key> = Vec::new();
        let mut delivered: Vec<(Key, Version, u64)> = Vec::new();

        for op in &ops {
            match op {
                Op::SendA(v) => {
                    let pre = a.latest();
                    a.send(*v);
                    minted.push(minted_key(&a.snapshot(), &pre));
                }
                Op::SendB(v) => {
                    b.send(*v);
                }
                Op::Redact(idx) => {
                    if !minted.is_empty() {
                        a.redact(minted[idx % minted.len()]);
                    }
                }
                Op::Gossip => wire_gossip(&mut a, &mut b),
                Op::Drain => delivered.extend(drain(&mut obs).0),
            }
        }

        // Close `a`'s side and take the final drain. (`b` stays alive; it
        // holds no handle on `a`'s set.)
        let final_live = live_map(&a);
        drop(a);
        let (final_items, ended) = drain(&mut obs);
        prop_assert!(ended, "all handles gone: the observer ends");
        delivered.extend(final_items);

        // No inversion across the entire delivered history.
        assert_causal(&delivered);

        // Exactly-once and coverage, as the plain observer promises.
        let mut seen = BTreeSet::new();
        for (key, _, _) in &delivered {
            prop_assert!(seen.insert(*key), "key {key:?} delivered twice");
        }
        for (key, value) in &final_live {
            prop_assert!(
                delivered.iter().any(|(k, _, m)| k == key && m == value),
                "a final live message was never delivered",
            );
        }
    }

    /// Checkpoint-resume discipline: stop at an arbitrary point in the backlog
    /// (or after a complete drain) and resume a fresh observer from
    /// `checkpoint()`; nothing is lost, both runs are individually causal, and
    /// after a *complete* drain nothing re-delivers.
    #[test]
    fn checkpoint_resume_loses_nothing(
        phase_one in vec(any::<u64>(), 1..8),
        phase_two in vec(any::<u64>(), 0..8),
        taken in any::<usize>(),
        complete_drain in any::<bool>(),
    ) {
        let known = Known::<u64>::seed();
        for v in &phase_one {
            known.send(*v); // separate batches: a strict causal chain
        }

        let mut obs = known.causal_messages();
        let mut first_run: Vec<(Key, Version, u64)> = Vec::new();
        if complete_drain {
            first_run.extend(drain(&mut obs).0);
        } else {
            for _ in 0..(taken % (phase_one.len() + 1)) {
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
            known.send(*v);
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
                    .any(|(k, _, m)| k == key && m == value),
                "a live message fell between the stopped and resumed observers",
            );
        }

        // After a complete drain the checkpoint is current: no re-delivery.
        let first_keys: BTreeSet<Key> = first_run.iter().map(|(k, _, _)| *k).collect();
        let second_keys: BTreeSet<Key> = second_run.iter().map(|(k, _, _)| *k).collect();
        if complete_drain {
            prop_assert!(
                first_keys.is_disjoint(&second_keys),
                "a drained backlog's messages must not re-fire",
            );
        }
    }
}
