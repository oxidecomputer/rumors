//! The [`rumors::UnorderedMessages`] observer: delivery contract (exactly-once,
//! redaction honored, cursor resume and portability), checkpoint semantics,
//! termination, and non-interference with the actor handles.
//!
//! (The `Snapshot::range` differential proptest lives with the walk
//! machinery in `src/tree/tests.rs`.)
//!
//! The observer is pull-based, so "the listener is parked" is simply "the
//! caller has not asked": these tests drive observers step-by-step with
//! `now_or_never`, distinguishing an *item*, a *quiet* observer (pending:
//! no change to report, actors still live), and an *ended* one (no further
//! change possible, complete final state already yielded).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use futures::{FutureExt, StreamExt};
use proptest::collection::vec;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::{Peer, Rumors, Version, causally};

use crate::common::action::created_version;
use crate::common::observer::{
    Step, arb_ops, drain, interleave, live_map, redact_during_pass, step,
};
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// Genesis replay: a from-genesis observer on a populated set yields
/// exactly the live set, each message once, then goes quiet; after the
/// completed pass its checkpoint dominates every observed version.
#[test]
fn genesis_replay_observes_the_live_set_once() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    {
        rumors.send_all(0..8u64).unwrap();
    }

    let mut obs = rumors.unordered_messages();
    let (items, ended) = drain(&mut obs);
    assert!(
        !ended,
        "actors are live, so the observer goes quiet, not ended"
    );

    let observed: BTreeMap<Vec<u8>, u64> = items
        .iter()
        .map(|(v, m)| (v.as_bytes().to_vec(), *m))
        .collect();
    assert_eq!(observed.len(), items.len(), "no message is observed twice");
    assert_eq!(
        observed,
        live_map(&rumors),
        "exactly the live set is observed"
    );

    for (version, _) in &items {
        assert!(
            version <= obs.checkpoint(),
            "the post-pass checkpoint dominates every observed version"
        );
    }
}

/// Arbitrary start: `unordered_messages_since(v_mid)` observes exactly
/// the messages `v_mid` does not causally contain.
#[test]
fn checkpoint_start_observes_only_what_it_does_not_contain() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send_all([1, 2, 3]).unwrap();
    let v_mid = rumors.snapshot().latest().clone();
    rumors.send_all([4, 5, 6]).unwrap();

    let mut obs = rumors.unordered_messages_since(v_mid.clone());
    let (items, _) = drain(&mut obs);

    let observed: BTreeSet<u64> = items.iter().map(|(_, m)| *m).collect();
    assert_eq!(
        observed,
        BTreeSet::from([4, 5, 6]),
        "exactly the leaves above v_mid fire"
    );
    for (version, _) in &items {
        // The causal membership predicate itself: `since(&v_mid)` keeps
        // exactly the versions v_mid does not contain.
        assert!(
            causally::since(&v_mid).contains(version),
            "no observed version is contained in the starting checkpoint"
        );
    }
}

/// Live delivery: messages sent through a sibling `Rumors` clone
/// after subscription are observed, as are messages learned via gossip.
#[test]
fn live_sends_and_gossip_learned_messages_are_observed() {
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    let sibling = a.clone();

    let mut obs = a.unordered_messages();
    let (initial, _) = drain(&mut obs);
    assert!(initial.is_empty(), "nothing to observe yet");

    // A local send through a sibling clone.
    sibling.send(10).unwrap();
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 1, "the sibling's send is observed");
    assert_eq!(items[0].1, 10);

    // A message learned through gossip.
    b.send(20).unwrap();
    wire_gossip(&a, &b);
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 1, "the gossip-learned message is observed");
    assert_eq!(items[0].1, 20);
}

/// Redaction honored: an observed-then-redacted message fires nothing
/// further; one redacted before subscription never fires.
///
/// Further: one inserted and
/// redacted wholly between passes is never delivered; a from-now observer
/// does not see pre-subscription content.
#[test]
fn redactions_are_honored_silently() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();

    // Redacted before subscription: never fires.
    let pre = rumors.snapshot().latest().clone();
    rumors.send(1).unwrap();
    let version_1 = created_version(&rumors.snapshot(), &pre);
    rumors.redact(&version_1);
    let mut obs = rumors.unordered_messages();
    let (items, _) = drain(&mut obs);
    assert!(items.is_empty(), "a pre-subscription redaction never fires");

    // Observed, then redacted: nothing further fires.
    let pre = rumors.snapshot().latest().clone();
    rumors.send(2).unwrap();
    let version_2 = created_version(&rumors.snapshot(), &pre);
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 1, "the live message fires once");
    rumors.redact(&version_2);
    let (items, _) = drain(&mut obs);
    assert!(items.is_empty(), "a redaction fires no further observation");

    // Inserted and redacted wholly between passes: never delivered.
    let pre = rumors.snapshot().latest().clone();
    rumors.send(3).unwrap();
    let version_3 = created_version(&rumors.snapshot(), &pre);
    rumors.redact(&version_3);
    let (items, _) = drain(&mut obs);
    assert!(
        items.is_empty(),
        "content already redacted is never delivered"
    );

    // A from-now observer does not see pre-subscription content.
    rumors.send(4).unwrap();
    let mut from_now = rumors.unordered_messages_since(rumors.snapshot().latest().clone());
    let (items, _) = drain(&mut from_now);
    assert!(items.is_empty(), "a from-now observer starts quiet");
    rumors.send(5).unwrap();
    let (items, _) = drain(&mut from_now);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].1, 5, "only post-subscription content fires");
}

/// Termination: when the last handle on the set drops, the observer
/// yields the complete final state and then ends.
#[test]
fn observer_drains_the_final_state_then_ends() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send_all([1, 2]).unwrap();
    let expected = live_map(&rumors);

    let mut obs = rumors.unordered_messages();
    drop(rumors);

    let (items, ended) = drain(&mut obs);
    assert!(ended, "with every sender gone the observer ends");
    assert_eq!(
        items
            .into_iter()
            .map(|(v, m)| (v.as_bytes().to_vec(), m))
            .collect::<BTreeMap<_, _>>(),
        expected,
        "the complete final state is yielded before the end"
    );

    // Ended is terminal.
    assert_eq!(step(&mut obs), Step::Ended);
}

/// Pending while actors live: with any handle on the set alive, a
/// drained observer is quiet, not ended.
#[test]
fn observer_stays_quiet_while_actors_live() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send(1).unwrap();

    let mut obs = rumors.unordered_messages();
    let (_, ended) = drain(&mut obs);
    assert!(!ended, "a live handle keeps the observer open");

    let sibling = rumors.clone();
    drop(rumors);
    let (_, ended) = drain(&mut obs);
    assert!(!ended, "a surviving clone keeps the observer open");

    drop(sibling);
    let (items, ended) = drain(&mut obs);
    assert!(ended, "dropping the last handle ends the observer");
    assert!(items.is_empty());
}

/// Reunite non-interference: an outstanding observer is not an actor —
/// it neither blocks [`Rumors::try_into_peer`](rumors::Rumors::try_into_peer)
/// nor is ended by it, and it keeps observing across the round-trip.
#[test]
fn observer_does_not_block_reunite_and_survives_it() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();

    let mut obs = rumors.unordered_messages();
    let (_, ended) = drain(&mut obs);
    assert!(!ended);

    // Reuniting resolves immediately despite the outstanding observer.
    let peer = rumors
        .try_into_peer()
        .now_or_never()
        .expect("an observer does not count against quiescence")
        .expect("the sole reuniter reclaims the Peer");

    // The round-trip neither ended the observer nor closed the set: a send
    // through a fresh handle is still observed.
    let rumors = peer.into_rumors();
    rumors.send(42).unwrap();
    let (items, ended) = drain(&mut obs);
    assert!(!ended, "the reclaimed Peer keeps the set open");
    assert_eq!(
        items.len(),
        1,
        "the observer keeps observing across reunite"
    );
    assert_eq!(items[0].1, 42);
}

/// Non-blocking observer: an observer mid-pass — its most recent item
/// still lent out — holds no lock, so sends on the set proceed and the
/// observer sees their effects on its next passes.
#[test]
fn lent_borrows_do_not_block_senders() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send_all([1, 2]).unwrap();

    let mut obs = rumors.unordered_messages();
    let lent = block_on(obs.next()).expect("first item of the pass");
    let lent_value = *lent.1;

    // With the yielded item outstanding (the observer is mid-pass), a
    // send must not deadlock.
    rumors.send(3).unwrap();

    let (rest, _) = drain(&mut obs);
    assert!(
        rest.iter().any(|(_, m)| *m == 3),
        "the mid-pass send is observed by a later pass"
    );
    assert!(
        [1, 2].contains(&lent_value),
        "the lent item was a first-pass message"
    );
}

/// Checkpoint round-trip: a checkpoint earned by a completed pass, fed
/// to a fresh `unordered_messages_since` on an unchanged set, observes
/// nothing and earns an equal checkpoint.
#[test]
fn checkpoint_round_trips_on_an_unchanged_set() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send_all([1, 2, 3]).unwrap();

    let mut obs = rumors.unordered_messages();
    let (items, _) = drain(&mut obs);
    assert_eq!(items.len(), 3);
    let checkpoint = obs.checkpoint().clone();

    let mut resumed = rumors.unordered_messages_since(checkpoint.clone());
    let (items, _) = drain(&mut resumed);
    assert!(items.is_empty(), "nothing fires on an unchanged set");
    assert_eq!(
        resumed.checkpoint(),
        &checkpoint,
        "the resumed observer's completed pass earns an equal checkpoint"
    );
}

/// Replica portability: a checkpoint earned against replica A is a valid
/// `since` against replica B of the same universe — messages observed via A
/// are skipped, messages B holds that A never saw fire.
#[test]
fn checkpoint_is_portable_across_replicas() {
    let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    a.send(1).unwrap();
    b.send(2).unwrap();

    // Observe everything A has, completing the pass to earn the checkpoint.
    let mut obs_a = a.unordered_messages();
    let (items, _) = drain(&mut obs_a);
    assert_eq!(items.len(), 1);
    let checkpoint = obs_a.checkpoint().clone();

    // Converge the replicas, then resume against B.
    wire_gossip(&a, &b);
    let mut obs_b = b.unordered_messages_since(checkpoint);
    let (items, _) = drain(&mut obs_b);
    assert_eq!(items.len(), 1, "only the message A never observed fires");
    assert_eq!(items[0].1, 2, "A-observed messages are skipped at B");
}

/// The observer's non-blocking step yields the same owned items as the
/// `Stream` face, and distinguishes a *quiet* observer (nothing new,
/// actors live — where an awaited `next` would block) from an *ended*
/// one.
#[test]
fn try_next_distinguishes_quiet_from_ended() {
    use rumors::TryNext;

    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send_all([1, 2]).unwrap();

    let mut obs = rumors.unordered_messages();
    let mut seen = BTreeSet::new();
    while let TryNext::Message((_, m)) = obs.try_next() {
        seen.insert(*m);
    }
    assert_eq!(seen, BTreeSet::from([1, 2]), "the pending pass drains");
    assert!(
        matches!(obs.try_next(), TryNext::Quiet),
        "with a handle live, a drained observer is quiet, not ended"
    );

    rumors.send(3).unwrap();
    let TryNext::Message((_, m)) = obs.try_next() else {
        panic!("the new send is immediately available");
    };
    assert_eq!(*m, 3);

    drop(rumors);
    assert!(matches!(obs.try_next(), TryNext::Ended));
    assert!(
        matches!(obs.try_next(), TryNext::Ended),
        "ended is terminal"
    );
}

/// The `Stream` impl yields every message, owned, and terminates with
/// `None` once the set closes.
#[test]
fn stream_face_matches_and_terminates() {
    let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
    rumors.send_all([1, 2]).unwrap();
    let expected = live_map(&rumors);

    let mut obs = rumors.unordered_messages();
    let mut items = BTreeMap::new();
    while let Some(Some((v, m))) = obs.next().now_or_never() {
        items.insert(v.as_bytes().to_vec(), *m);
    }
    assert_eq!(items, expected, "the Stream face yields the live set");

    drop(rumors);
    assert_eq!(
        obs.next().now_or_never(),
        Some(None),
        "the Stream ends once the set closes"
    );
}

/// (negative control): folding *delivered* versions is not a sound
/// resume point.
///
/// Delivery is in path order (the hash of the version), not causal
/// order, so a stopped
/// pass can have delivered `m2` (later version) but not `m1` (earlier);
/// the fold then causally contains `m1`, and resuming from it skips `m1`
/// forever — loss, not re-delivery. `UnorderedMessages::checkpoint()` (the
/// last *completed* pass's frontier) re-delivers instead, which is why the
/// API exposes the pass checkpoint and not a per-item fold.
#[test]
fn folding_delivered_versions_can_lose_a_message() {
    // Search deterministic universes for the counterexample shape: the
    // *later*-created of two messages is delivered first. A leaf's path is
    // the hash of its version, and paths vs. causal versions disagree
    // about order roughly half the time, so varying the universe seed
    // (which varies the created versions) finds the shape quickly.
    let later_value = 1u64;
    let rumors = (0u64..256)
        .find_map(|seed| {
            let rumors = Peer::<u64>::seed_rng(&mut SmallRng::seed_from_u64(seed))
                .sync_window_floor()
                .into_rumors();
            rumors.send(0).unwrap();
            rumors.send(later_value).unwrap();
            let snapshot = rumors.snapshot();
            let first_yielded = snapshot.iter().next().expect("two live messages");
            let later_first = *first_yielded.1 == later_value;
            drop(snapshot);
            later_first.then_some(rumors)
        })
        .expect("some universe must collide into path-before-version order");

    // Deliver exactly one item — the later version — and stop mid-pass.
    let mut obs = rumors.unordered_messages();
    let Step::Item((delivered_version, delivered_value)) = step(&mut obs) else {
        panic!("the populated set delivers an item");
    };
    assert_eq!(delivered_value, later_value, "the later version came first");

    // The unsound resume: fold the delivered version into genesis.
    let fold = {
        let mut fold = Version::new();
        fold |= &delivered_version;
        fold
    };
    let mut resumed_from_fold = rumors.unordered_messages_since(fold);
    let (items, _) = drain(&mut resumed_from_fold);
    assert!(
        items.is_empty(),
        "the fold causally contains the never-delivered message: it is lost"
    );

    // The sound resume: the observer's pass checkpoint (genesis — no pass
    // completed) re-delivers both messages. At-least-once, never loss.
    let mut resumed_from_checkpoint = rumors.unordered_messages_since(obs.checkpoint().clone());
    let (items, _) = drain(&mut resumed_from_checkpoint);
    assert_eq!(
        items.len(),
        2,
        "the pass checkpoint re-delivers the interrupted pass instead of losing"
    );
}

proptest! {
    /// A captured pass survives local and remote redactions while it is partly read.
    /// A message redacted before capture produces no delivery.
    #[test]
    fn captured_pass_survives_redaction(
        values in vec(any::<u64>(), 2..10),
        taken in any::<prop::sample::Index>(),
        redactions in vec(any::<prop::sample::Index>(), 1..10),
        remote in any::<bool>(),
    ) {
        redact_during_pass(values, taken, redactions, remote, Rumors::unordered_messages)?;
    }

    /// Delivery remains duplicate-free and covers the final live set across
    /// sends, local and remote redactions, gossip, and partially drained passes.
    #[test]
    fn exactly_once_under_interleaving(ops in arb_ops()) {
        interleave(&ops, Rumors::unordered_messages)?;
    }

    /// Stopping and resuming loses no final live message; each run is duplicate-free.
    /// A checkpoint from a completed pass also prevents replay across runs.
    #[test]
    fn checkpoint_resume_loses_nothing(
        phase_one in vec(any::<u64>(), 1..8),
        phase_two in vec(any::<u64>(), 0..8),
        taken in any::<prop::sample::Index>(),
        complete_pass in any::<bool>(),
    ) {
        let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors();
        rumors.send_all(phase_one.iter().copied()).unwrap();

        // Deliver a prefix of the first pass — or, when `complete_pass`,
        // drain to quiescence so the pass commits into the checkpoint.
        let mut obs = rumors.unordered_messages();
        let mut first_run: Vec<(Version, u64)> = Vec::new();
        if complete_pass {
            let (items, _) = drain(&mut obs);
            first_run.extend(items);
        } else {
            for _ in 0..(taken.index(phase_one.len() + 1)) {
                match step(&mut obs) {
                    Step::Item(item) => first_run.push(item),
                    other => panic!("the pass has more items, got {other:?}"),
                }
            }
        }
        let checkpoint = obs.checkpoint().clone();
        drop(obs);

        // More traffic after the stop.
        rumors.send_all(phase_two.iter().copied()).unwrap();

        // Resume from the persisted checkpoint and drain to the end.
        let mut resumed = rumors.unordered_messages_since(checkpoint);
        let final_live = live_map(&rumors);
        drop(rumors);
        let (second_run, ended) = drain(&mut resumed);
        prop_assert!(ended);

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

        // Completed passes advance the checkpoint beyond their messages.
        let first_versions: BTreeSet<Vec<u8>> =
            first_run.iter().map(|(v, _)| v.as_bytes().to_vec()).collect();
        let second_versions: BTreeSet<Vec<u8>> =
            second_run.iter().map(|(v, _)| v.as_bytes().to_vec()).collect();
        prop_assert_eq!(first_versions.len(), first_run.len(), "first run repeated a version");
        prop_assert_eq!(second_versions.len(), second_run.len(), "resumed run repeated a version");
        if complete_pass {
            prop_assert!(first_versions.is_disjoint(&second_versions),
                "a completed pass's messages must not re-fire");
        }
    }
}
