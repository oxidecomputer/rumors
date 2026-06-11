//! The [`Messages`] observer: delivery contract, checkpoint semantics,
//! termination, and non-interference with the actor handles
//! (plan: `plans/broadcast-listen.md` §6; the `Snapshot::range`
//! differential proptest lives with the walk machinery in
//! `src/tree/test.rs`).
//!
//! The observer is pull-based, so "the listener is parked" is simply "the
//! caller has not asked": these tests drive observers step-by-step with
//! `now_or_never`, distinguishing an *item*, a *quiet* observer (pending:
//! no change to report, actors still live), and an *ended* one (no further
//! change possible, complete final state already yielded).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use futures::FutureExt;
use proptest::collection::vec;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::{Key, Known, Messages, Retire, Version, causally};

use crate::common::action::minted_key;
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

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
fn step(obs: &mut Messages<u64>) -> Step {
    match obs.borrow_next().now_or_never() {
        None => Step::Quiet,
        Some(None) => Step::Ended,
        Some(Some((k, v, m))) => Step::Item((k, v.clone(), **m)),
    }
}

/// Drain the observer until it goes quiet or ends, returning the items in
/// delivery order and whether it ended.
fn drain(obs: &mut Messages<u64>) -> (Vec<(Key, Version, u64)>, bool) {
    let mut items = Vec::new();
    loop {
        match step(obs) {
            Step::Item(item) => items.push(item),
            Step::Quiet => return (items, false),
            Step::Ended => return (items, true),
        }
    }
}

/// The live `Key → value` map, for comparing against deliveries. (Keys
/// identify messages uniquely; `Version` is only partially ordered, so it
/// can't key a comparison set.)
fn live_map(known: &Known<u64>) -> BTreeMap<Key, u64> {
    known.snapshot().iter().map(|(k, _, m)| (k, **m)).collect()
}

/// §6.1 Genesis replay: a from-genesis observer on a populated set yields
/// exactly the live set, each message once, then goes quiet; after the
/// completed pass its checkpoint dominates every observed version.
#[test]
fn genesis_replay_observes_the_live_set_once() {
    let known = Known::<u64>::seed();
    {
        let mut batch = known.batch();
        for v in 0..8u64 {
            batch.send(v);
        }
    }

    let mut obs = known.messages();
    let (items, ended) = drain(&mut obs);
    assert!(
        !ended,
        "actors are live, so the observer goes quiet, not ended"
    );

    let observed: BTreeMap<Key, u64> = items.iter().map(|(k, _, m)| (*k, *m)).collect();
    assert_eq!(observed.len(), items.len(), "no message is observed twice");
    assert_eq!(
        observed,
        live_map(&known),
        "exactly the live set is observed"
    );

    for (_, version, _) in &items {
        assert!(
            version <= obs.checkpoint(),
            "the post-pass checkpoint dominates every observed version"
        );
    }
}

/// §6.2 Arbitrary start: `messages_from(v_mid)` observes exactly the
/// messages `v_mid` does not causally contain.
#[test]
fn checkpoint_start_observes_only_what_it_does_not_contain() {
    let known = Known::<u64>::seed();
    known.batch().send(1).send(2).send(3);
    let v_mid = known.latest();
    known.batch().send(4).send(5).send(6);

    let mut obs = known.messages_from(v_mid.clone());
    let (items, _) = drain(&mut obs);

    let observed: BTreeSet<u64> = items.iter().map(|(_, _, m)| *m).collect();
    assert_eq!(
        observed,
        BTreeSet::from([4, 5, 6]),
        "exactly the leaves above v_mid fire"
    );
    for (_, version, _) in &items {
        // The causal membership predicate itself: `since(&v_mid)` keeps
        // exactly the versions v_mid does not contain.
        assert!(
            causally::since(&v_mid).contains(version),
            "no observed version is contained in the starting checkpoint"
        );
    }
}

/// §6.3 Live delivery: messages sent through a sibling `Broadcast` clone
/// after subscription are observed, as are messages learned via gossip.
#[test]
fn live_sends_and_gossip_learned_messages_are_observed() {
    let mut a = Known::<u64>::seed();
    let mut b = bootstrap_fork(&mut a);

    let broadcast = a.broadcast();
    let sibling = broadcast.clone();

    let mut obs = broadcast.messages();
    let (initial, _) = drain(&mut obs);
    assert!(initial.is_empty(), "nothing to observe yet");

    // A local send through a sibling clone.
    sibling.send(10);
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 1, "the sibling's send is observed");
    assert_eq!(items[0].2, 10);

    // A message learned through gossip.
    b.send(20);
    let mut a = block_on(async {
        drop(sibling);
        broadcast.reunite().await.expect("sole reuniter")
    });
    wire_gossip(&mut a, &mut b);
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 1, "the gossip-learned message is observed");
    assert_eq!(items[0].2, 20);
}

/// §6.4 Redaction honored: an observed-then-redacted message fires nothing
/// further; one redacted before subscription never fires; one inserted and
/// redacted wholly between passes is never delivered; a from-now observer
/// does not see pre-subscription content.
#[test]
fn redactions_are_honored_silently() {
    let known = Known::<u64>::seed();

    // Redacted before subscription: never fires.
    let pre = known.latest();
    known.send(1);
    let key_1 = minted_key(&known.snapshot(), &pre);
    known.redact(key_1);
    let mut obs = known.messages();
    let (items, _) = drain(&mut obs);
    assert!(items.is_empty(), "a pre-subscription redaction never fires");

    // Observed, then redacted: nothing further fires.
    let pre = known.latest();
    known.send(2);
    let key_2 = minted_key(&known.snapshot(), &pre);
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 1, "the live message fires once");
    known.redact(key_2);
    let (items, _) = drain(&mut obs);
    assert!(items.is_empty(), "a redaction fires no further observation");

    // Inserted and redacted wholly between passes: never delivered.
    let pre = known.latest();
    known.send(3);
    let key_3 = minted_key(&known.snapshot(), &pre);
    known.redact(key_3);
    let (items, _) = drain(&mut obs);
    assert!(
        items.is_empty(),
        "content already redacted is never delivered"
    );

    // A from-now observer does not see pre-subscription content.
    known.send(4);
    let mut from_now = known.messages_from(known.latest());
    let (items, _) = drain(&mut from_now);
    assert!(items.is_empty(), "a from-now observer starts quiet");
    known.send(5);
    let (items, _) = drain(&mut from_now);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].2, 5, "only post-subscription content fires");
}

/// §6.9 Termination: when the `Known` (and every other handle) drops, the
/// observer yields the complete final state and then ends.
#[test]
fn observer_drains_the_final_state_then_ends() {
    let known = Known::<u64>::seed();
    known.batch().send(1).send(2);
    let expected = live_map(&known);

    let mut obs = known.messages();
    drop(known);

    let (items, ended) = drain(&mut obs);
    assert!(ended, "with every sender gone the observer ends");
    assert_eq!(
        items
            .into_iter()
            .map(|(k, _, m)| (k, m))
            .collect::<BTreeMap<_, _>>(),
        expected,
        "the complete final state is yielded before the end"
    );

    // Ended is terminal.
    assert_eq!(step(&mut obs), Step::Ended);
}

/// §6.9 (retire variant): retiring the rumor set ends its observers. The
/// retire session's write-back lands the reconciled state first, so the
/// observer's final drain includes everything the session learned.
#[test]
fn retire_ends_the_observer() {
    let mut survivor = Known::<u64>::seed();
    let retiree = bootstrap_fork(&mut survivor);
    retiree.send(7);

    let mut obs = retiree.messages();

    let outcome = block_on(async {
        let (a_side, b_side) = tokio::io::duplex(64 * 1024);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (retire_out, gossip_out) = tokio::join!(
            retiree.retire(&mut a_r, &mut a_w),
            survivor.gossip(&mut b_r, &mut b_w),
        );
        gossip_out.expect("survivor gossip");
        retire_out
    });
    assert!(matches!(outcome, Retire::Retired));

    let (items, ended) = drain(&mut obs);
    assert!(ended, "retiring the set ends its observers");
    assert!(
        items.iter().any(|(_, _, m)| *m == 7),
        "the final drain delivered the retiree's own message"
    );
}

/// §6.10 Pending while actors live: with a `Known` or `Broadcast` alive, a
/// drained observer is quiet, not ended.
#[test]
fn observer_stays_quiet_while_actors_live() {
    let known = Known::<u64>::seed();
    known.send(1);

    let mut obs = known.messages();
    let (_, ended) = drain(&mut obs);
    assert!(!ended, "a live Known keeps the observer open");

    let broadcast = known.broadcast();
    let (_, ended) = drain(&mut obs);
    assert!(!ended, "a live Broadcast keeps the observer open");

    drop(broadcast);
    let (items, ended) = drain(&mut obs);
    assert!(ended, "dropping the last handle ends the observer");
    assert!(items.is_empty());
}

/// §6.11 Reunite non-interference: an outstanding observer is not an actor —
/// it neither blocks [`Broadcast::reunite`](rumors::Broadcast::reunite) nor
/// is ended by it, and it keeps observing the reunited `Known`.
#[test]
fn observer_does_not_block_reunite_and_survives_it() {
    let known = Known::<u64>::seed();
    let broadcast = known.broadcast();

    let mut obs = broadcast.messages();
    let (_, ended) = drain(&mut obs);
    assert!(!ended);

    // Reunite resolves immediately despite the outstanding observer.
    let known = broadcast
        .reunite()
        .now_or_never()
        .expect("an observer does not count against quiescence")
        .expect("the sole reuniter reclaims the Known");

    known.send(42);
    let (items, ended) = drain(&mut obs);
    assert!(!ended, "the reunited Known keeps the set open");
    assert_eq!(
        items.len(),
        1,
        "the observer keeps observing across reunite"
    );
    assert_eq!(items[0].2, 42);
}

/// §6.12 Non-blocking observer: an observer mid-pass — its most recent item
/// still lent out — holds no lock, so sends on the set proceed and the
/// observer sees their effects on its next passes.
#[test]
fn lent_borrows_do_not_block_senders() {
    let known = Known::<u64>::seed();
    known.batch().send(1).send(2);

    let mut obs = known.messages();
    let lent = block_on(obs.borrow_next()).expect("first item of the pass");
    let lent_value = *lent.2.clone();

    // With the borrow conceptually outstanding (the observer is mid-pass),
    // a send must not deadlock.
    known.send(3);

    let (rest, _) = drain(&mut obs);
    assert!(
        rest.iter().any(|(_, _, m)| *m == 3),
        "the mid-pass send is observed by a later pass"
    );
    assert!(
        [1, 2].contains(&lent_value),
        "the lent item was a first-pass message"
    );
}

/// §6.7 Checkpoint round-trip: a checkpoint earned by a completed pass, fed to a
/// fresh `messages_from` on an unchanged set, observes nothing and earns an
/// equal checkpoint.
#[test]
fn checkpoint_round_trips_on_an_unchanged_set() {
    let known = Known::<u64>::seed();
    known.batch().send(1).send(2).send(3);

    let mut obs = known.messages();
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 3);
    let checkpoint = obs.checkpoint().clone();

    let mut resumed = known.messages_from(checkpoint.clone());
    let (items, _) = drain(&mut resumed);
    assert!(items.is_empty(), "nothing fires on an unchanged set");
    assert_eq!(
        resumed.checkpoint(),
        &checkpoint,
        "the resumed observer's completed pass earns an equal checkpoint"
    );
}

/// §6.8 Replica portability: a checkpoint earned against replica A is a valid
/// `since` against replica B of the same universe — messages observed via A
/// are skipped, messages B holds that A never saw fire.
#[test]
fn checkpoint_is_portable_across_replicas() {
    let mut a = Known::<u64>::seed();
    let mut b = bootstrap_fork(&mut a);

    a.send(1);
    b.send(2);

    // Observe everything A has, completing the pass to earn the checkpoint.
    let mut obs_a = a.messages();
    let (items, _) = drain(&mut obs_a);
    assert_eq!(items.len(), 1);
    let checkpoint = obs_a.checkpoint().clone();

    // Converge the replicas, then resume against B.
    wire_gossip(&mut a, &mut b);
    let mut obs_b = b.messages_from(checkpoint);
    let (items, _) = drain(&mut obs_b);
    assert_eq!(items.len(), 1, "only the message A never observed fires");
    assert_eq!(items[0].2, 2, "A-observed messages are skipped at B");
}

/// The sync face's non-blocking step: `try_next` lends exactly as
/// `borrow_next` does, and distinguishes a *quiet* observer (nothing new,
/// actors live — where `borrow_next` would block) from an *ended* one.
#[test]
fn sync_try_next_distinguishes_quiet_from_ended() {
    use rumors::sync::{Known as SyncKnown, TryNext};

    let known = SyncKnown::<u64>::seed();
    known.batch().send(1).send(2);

    let mut obs = known.messages();
    let mut seen = BTreeSet::new();
    while let TryNext::Message((_, _, m)) = obs.try_next() {
        seen.insert(**m);
    }
    assert_eq!(seen, BTreeSet::from([1, 2]), "the pending pass drains");
    assert!(
        matches!(obs.try_next(), TryNext::Quiet),
        "with the Known live, a drained observer is quiet, not ended"
    );

    known.send(3);
    let TryNext::Message((_, _, m)) = obs.try_next() else {
        panic!("the new send is immediately available");
    };
    assert_eq!(**m, 3);

    drop(known);
    assert!(matches!(obs.try_next(), TryNext::Ended));
    assert!(
        matches!(obs.try_next(), TryNext::Ended),
        "ended is terminal"
    );
}

/// The owned-item face: the `Stream` impl yields the same messages as
/// `borrow_next`, owned, and terminates with `None` once the set closes.
#[test]
fn stream_face_matches_and_terminates() {
    use futures::StreamExt;

    let known = Known::<u64>::seed();
    known.batch().send(1).send(2);
    let expected = live_map(&known);

    let mut obs = known.messages();
    let mut items = BTreeMap::new();
    while let Some(Some((k, _, m))) = obs.next().now_or_never() {
        items.insert(k, *m);
    }
    assert_eq!(items, expected, "the Stream face yields the live set");

    drop(known);
    assert_eq!(
        obs.next().now_or_never(),
        Some(None),
        "the Stream ends once the set closes"
    );
}

/// The synchronous mirror: `sync::Messages` is an `Iterator` whose end is
/// the set closing; it yields the complete final state first.
#[test]
fn sync_iterator_face_drains_then_ends() {
    let known = rumors::sync::Known::<u64>::seed();
    known.batch().send(1).send(2).send(3);
    let expected: BTreeSet<u64> = known.snapshot().iter().map(|(_, _, m)| **m).collect();

    let obs = known.messages();
    drop(known);

    let observed: BTreeSet<u64> = obs.map(|(_, _, m)| *m).collect();
    assert_eq!(
        observed, expected,
        "the sync iterator yields the final state, then ends"
    );
}

/// §6.6 (negative control): folding *delivered* versions is not a sound
/// resume point. Delivery is in key order, not causal order, so a stopped
/// pass can have delivered `m2` (later version) but not `m1` (earlier);
/// the fold then causally contains `m1`, and resuming from it skips `m1`
/// forever — loss, not re-delivery. `Messages::checkpoint()` (the last
/// *completed* pass's frontier) re-delivers instead, which is why the API
/// exposes the pass checkpoint and not a per-item fold.
#[test]
fn folding_delivered_versions_can_lose_a_message() {
    // Search deterministic universes for the counterexample shape: the
    // *later*-minted of two messages is delivered first (content-addressed
    // keys vs. causal versions disagree about order roughly half the time).
    let (known, later_value) = (1u64..256)
        .find_map(|candidate| {
            let known = Known::<u64>::seed_rng(&mut SmallRng::seed_from_u64(0));
            known.send(0);
            known.send(candidate);
            let snapshot = known.snapshot();
            let first_yielded = snapshot.iter().next().expect("two live messages");
            let later_first = **first_yielded.2 == candidate;
            drop(snapshot);
            later_first.then_some((known, candidate))
        })
        .expect("some candidate must collide into key-before-version order");

    // Deliver exactly one item — the later version — and stop mid-pass.
    let mut obs = known.messages();
    let Step::Item((_, delivered_version, delivered_value)) = step(&mut obs) else {
        panic!("the populated set delivers an item");
    };
    assert_eq!(delivered_value, later_value, "the later version came first");

    // The unsound resume: fold the delivered version into genesis.
    let fold = {
        let mut fold = Version::new();
        fold |= &delivered_version;
        fold
    };
    let mut resumed_from_fold = known.messages_from(fold);
    let (items, _) = drain(&mut resumed_from_fold);
    assert!(
        items.is_empty(),
        "the fold causally contains the never-delivered message: it is lost"
    );

    // The sound resume: the observer's pass checkpoint (genesis — no pass
    // completed) re-delivers both messages. At-least-once, never loss.
    let mut resumed_from_checkpoint = known.messages_from(obs.checkpoint().clone());
    let (items, _) = drain(&mut resumed_from_checkpoint);
    assert_eq!(
        items.len(),
        2,
        "the pass checkpoint re-delivers the interrupted pass instead of losing"
    );
}

const MAX_OPS: usize = 40;

/// One scripted action against the observed set.
#[derive(Debug, Clone)]
enum Op {
    /// Send this value (through one of two sibling `Broadcast` clones,
    /// alternating by op index).
    Send(u64),
    /// Redact the `idx % minted`-th key minted so far (dropped if none).
    Redact(usize),
    /// Drain the observer to quiescence.
    Drain,
}

fn arb_ops() -> impl Strategy<Value = Vec<Op>> {
    vec(
        prop_oneof![
            4 => any::<u64>().prop_map(Op::Send),
            2 => any::<usize>().prop_map(Op::Redact),
            3 => Just(Op::Drain),
        ],
        0..=MAX_OPS,
    )
}

proptest! {
    /// §6.5 Exactly-once under interleaving: across an arbitrary
    /// send/redact/drain interleaving (sends through alternating sibling
    /// clones), no key is ever observed twice, and the observations cover
    /// the final live set.
    #[test]
    fn exactly_once_under_interleaving(ops in arb_ops()) {
        let known = Known::<u64>::seed();
        let broadcast = known.broadcast();
        let sibling = broadcast.clone();

        let mut obs = broadcast.messages();
        let mut minted: Vec<Key> = Vec::new();
        let mut observed: Vec<(Key, Version, u64)> = Vec::new();

        for (i, op) in ops.iter().enumerate() {
            match op {
                Op::Send(v) => {
                    let handle = if i % 2 == 0 { &broadcast } else { &sibling };
                    let pre = handle.latest();
                    handle.send(*v);
                    minted.push(minted_key(&handle.snapshot(), &pre));
                }
                Op::Redact(idx) => {
                    if !minted.is_empty() {
                        broadcast.redact(minted[idx % minted.len()]);
                    }
                }
                Op::Drain => {
                    observed.extend(drain(&mut obs).0);
                }
            }
        }

        // Close the set and take the final drain.
        let final_live = live_map(&block_on(async {
            drop(sibling);
            broadcast.reunite().await.expect("sole reuniter")
        }));
        let (final_items, ended) = drain(&mut obs);
        prop_assert!(ended, "all handles gone: the observer ends");
        observed.extend(final_items);

        let mut seen = BTreeSet::new();
        for (key, _, _) in &observed {
            prop_assert!(seen.insert(*key), "key {key:?} observed twice");
        }
        for (key, value) in &final_live {
            prop_assert!(
                observed.iter().any(|(k, _, m)| k == key && m == value),
                "a final live message was never observed",
            );
        }
    }

    /// §6.6 Checkpoint-resume: stop an observer at an arbitrary point and
    /// resume a fresh one from its `checkpoint()`; the union of observations
    /// covers every message that survived to the end (nothing lost). If the
    /// stop fell *mid-pass*, re-deliveries are permitted but only for
    /// messages the interrupted pass already delivered (at-least-once); if
    /// the observer had *completed* its pass, nothing from it is
    /// re-delivered (exactly-once across completed passes).
    #[test]
    fn checkpoint_resume_loses_nothing(
        phase_one in vec(any::<u64>(), 1..8),
        phase_two in vec(any::<u64>(), 0..8),
        taken in any::<usize>(),
        complete_pass in any::<bool>(),
    ) {
        let known = Known::<u64>::seed();
        {
            let mut batch = known.batch();
            for v in &phase_one {
                batch.send(*v);
            }
        }

        // Deliver a prefix of the first pass — or, when `complete_pass`,
        // drain to quiescence so the pass commits into the checkpoint.
        let mut obs = known.messages();
        let mut first_run: Vec<(Key, Version, u64)> = Vec::new();
        if complete_pass {
            let (items, _) = drain(&mut obs);
            first_run.extend(items);
        } else {
            for _ in 0..(taken % (phase_one.len() + 1)) {
                match step(&mut obs) {
                    Step::Item(item) => first_run.push(item),
                    other => panic!("the pass has more items, got {other:?}"),
                }
            }
        }
        let checkpoint = obs.checkpoint().clone();
        drop(obs);

        // More traffic after the stop.
        {
            let mut batch = known.batch();
            for v in &phase_two {
                batch.send(*v);
            }
        }

        // Resume from the persisted checkpoint and drain to the end.
        let mut resumed = known.messages_from(checkpoint);
        let final_live = live_map(&known);
        drop(known);
        let (second_run, ended) = drain(&mut resumed);
        prop_assert!(ended);

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

        // Re-delivery discipline: a key delivered by both runs must have
        // been part of the interrupted pass; after a *completed* pass,
        // there are no re-deliveries at all.
        let first_keys: BTreeSet<Key> = first_run.iter().map(|(k, _, _)| *k).collect();
        let second_keys: BTreeSet<Key> = second_run.iter().map(|(k, _, _)| *k).collect();
        let redelivered: Vec<&Key> = first_keys.intersection(&second_keys).collect();
        if complete_pass {
            prop_assert!(
                redelivered.is_empty(),
                "a completed pass's messages must not re-fire: {redelivered:?}",
            );
        }
        // (Mid-pass, `redelivered ⊆ first_keys` holds by construction; the
        // loss-freedom assertion above is the substantive check.)
    }
}
