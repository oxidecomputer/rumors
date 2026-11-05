//! Publication preserves concurrent changes, notifications, and payload ownership.

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::ops::Bound;
use std::rc::Rc;
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::Duration;

use before::Party;
use proptest::prelude::*;
use tokio::sync::watch;

use super::{
    Inner, MAX_BUDGETED_MESSAGE_SIZE, MAX_RUN_BUDGET_BYTES, MIN_SYNC_MEMORY_BUDGET,
    OPTIMISTIC_ATTEMPTS, Peer,
};
use crate::default;
use crate::message::Message;
use crate::testing::property::{LOCAL_DIVISOR, SESSION_DIVISOR, divided_proptest, network_seed};
use crate::testing::{MEMORY_CAPACITY, memory_connection};
use crate::tree::mirror::streaming::Local;
use crate::tree::mirror::streaming::materialized::Allowances;
use crate::tree::mirror::streaming::remote::{RunBudget, WRITE_BUFFER_BYTES};
use crate::tree::{Action, Tree};
use crate::{Joined, Version};

/// Add one event to a tree using its owner's current identity.
fn insert(tree: &mut Tree<u64>, party: &Party, value: u64) {
    tree.act(party, [Action::Insert(Message::new(value))]);
}

thread_local! {
    /// Interleave a local commit after candidate preparation, on this test's thread.
    static BEFORE_SWAP: RefCell<Option<Box<dyn FnMut()>>> = RefCell::new(None);
}

/// Give tests a deterministic conflict between preparing and swapping a root.
pub(super) fn before_swap() {
    BEFORE_SWAP.with_borrow_mut(|hook| {
        if let Some(hook) = hook {
            hook();
        }
    });
}

/// Detect synchronous lock deadlocks without blocking the test's own thread.
fn completes(test: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let worker = thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
        done.send(outcome).unwrap();
    });
    let outcome = finished
        .recv_timeout(Duration::from_secs(10))
        .expect("publication must complete without deadlocking");
    worker.join().unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// A local payload whose destruction may read or write the replica.
#[derive(serde::Serialize)]
struct OnDrop {
    /// A local callback, excluded from the payload's wire representation.
    #[serde(skip)]
    callback: Box<dyn Fn() + Send + Sync>,
}

/// Encode a message that runs a local callback when released.
fn on_drop(callback: impl Fn() + Send + Sync + 'static) -> Message {
    Message::new(OnDrop {
        callback: Box::new(callback),
    })
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// Settings remain readable across the Peer/Rumors split, with message targets
    /// reported after the saturation applied to the wire greeting, and the
    /// write batch range's end after saturation at the write buffer's
    /// capacity.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn synchronization_settings_report_effective_values(network in network_seed()) {
        let depth = 17;
        // Just above the minimum for the target it sets, so the session
        // runs on it as given.
        let budget = Allowances::of::<Local>(MAX_RUN_BUDGET_BYTES).minimum() + 1_234;
        let peer = network.peer::<u64>()
            .sync_memory_budget(budget)
            .target_message_size(usize::MAX)
            .write_batch_range(5..=usize::MAX)
            .payload_depth_limit(depth);
        let expected = super::SynchronizationSettings {
            sync_memory_budget: budget,
            target_message_size: MAX_RUN_BUDGET_BYTES,
            write_batch_range: (Bound::Included(5), Bound::Included(WRITE_BUFFER_BYTES)),
            payload_depth_limit: depth,
        };

        assert_eq!(peer.synchronization_settings(), expected);
        let peer_debug = format!("{peer:?}");
        assert!(peer_debug.contains(&format!("sync_memory_budget: {budget}")));
        assert!(peer_debug.contains(&format!("target_message_size: {MAX_RUN_BUDGET_BYTES}")));
        assert!(peer_debug.contains(&format!("write_batch_range: (Included(5), Included({WRITE_BUFFER_BYTES}))")));
        assert!(peer_debug.contains("payload_depth_limit: 17"));

        let rumors = peer.into_rumors();
        assert_eq!(rumors.synchronization_settings(), expected);
        let rumors_debug = format!("{rumors:?}");
        assert!(rumors_debug.contains(&format!("sync_memory_budget: {budget}")));
        assert!(rumors_debug.contains(&format!("target_message_size: {MAX_RUN_BUDGET_BYTES}")));
        assert!(rumors_debug.contains(&format!("write_batch_range: (Included(5), Included({WRITE_BUFFER_BYTES}))")));
        assert!(rumors_debug.contains("payload_depth_limit: 17"));
    }
}

/// One bound of a write batch range: included, excluded, or unbounded.
///
/// Its sizes lie around the conversion's edges (zero, one, the default
/// segment size, the write buffer's capacity, and the largest size) and
/// between them.
fn batch_bound() -> impl Strategy<Value = Bound<usize>> {
    let size = prop_oneof![
        Just(0usize),
        Just(1),
        0usize..=2 * WRITE_BUFFER_BYTES,
        Just(*crate::default::tls::WRITE_BATCH_RANGE.start()),
        (WRITE_BUFFER_BYTES - 1)..=(WRITE_BUFFER_BYTES + 1),
        Just(usize::MAX - 1),
        Just(usize::MAX),
    ];
    prop_oneof![
        size.clone().prop_map(Bound::Included),
        size.prop_map(Bound::Excluded),
        Just(Bound::Unbounded),
    ]
}

/// The write batch range a session applies for the bounds `start` and
/// `end`, step by step as `Peer::write_batch_range` documents it.
///
/// Each bound converts to an inclusive size, the end saturates at the
/// write buffer's capacity, and the start is lowered to the end, or to the
/// capacity under an unbounded end.
fn applied_range(start: Bound<usize>, end: Bound<usize>) -> (Bound<usize>, Bound<usize>) {
    let start = match start {
        Bound::Included(start) => start,
        Bound::Excluded(start) => start.saturating_add(1),
        Bound::Unbounded => 0,
    };
    let end = match end {
        Bound::Included(end) => Some(end),
        Bound::Excluded(end) => Some(end.saturating_sub(1)),
        Bound::Unbounded => None,
    }
    .map(|end| end.min(WRITE_BUFFER_BYTES));
    let start = start.min(end.unwrap_or(WRITE_BUFFER_BYTES));
    (
        Bound::Included(start),
        end.map_or(Bound::Unbounded, Bound::Included),
    )
}

/// A peer that never sets a write batch range applies
/// `default::tls::WRITE_BATCH_RANGE` exactly: neither of its ends moves
/// in conversion.
#[test]
fn the_default_write_batch_range_applies_as_given() {
    let default = crate::default::tls::WRITE_BATCH_RANGE;
    let peer = Peer::<u64>::seed();
    assert_eq!(
        peer.synchronization_settings().write_batch_range,
        (
            Bound::Included(*default.start()),
            Bound::Included(*default.end())
        )
    );
}

proptest! {
    /// The write batch range reads back as its documented conversion
    /// applies it, and selects the same batching when passed back in.
    ///
    /// It also follows the peer through `into_rumors`, cloning, reunion
    /// into a `Peer`, and bookmark attachment, as its docs promise.
    ///
    /// Each bound is drawn included, excluded, or unbounded, at sizes on
    /// both sides of every edge the conversion has: zero, the write
    /// buffer's capacity, and the largest size, so starts above ends,
    /// empty ranges, and saturating ends all occur.
    #[test]
    fn write_batch_range_follows_the_peer(
        network in network_seed(),
        start in batch_bound(),
        end in batch_bound(),
    ) {
        let expected = applied_range(start, end);
        let read = |settings: super::SynchronizationSettings| settings.write_batch_range;
        let peer = network.peer::<u64>().write_batch_range((start, end));
        prop_assert_eq!(read(peer.synchronization_settings()), expected);
        let again = network.peer::<u64>().write_batch_range(expected);
        prop_assert_eq!(read(again.synchronization_settings()), expected);
        let rumors = peer.into_rumors();
        let clone = rumors.clone();
        prop_assert_eq!(read(clone.synchronization_settings()), expected);
        drop(clone);
        let peer = pollster::block_on(rumors.try_into_peer())
            .expect("the last handle reunites into a peer");
        prop_assert_eq!(read(peer.synchronization_settings()), expected);
        let peer = pollster::block_on(peer.bookmark(crate::bookmark::NoBookmark))
            .unwrap_or_else(|_| panic!("attaching an inert bookmark succeeds"));
        prop_assert_eq!(read(peer.synchronization_settings()), expected);
    }
}

/// One call to a setter that the budget a session runs on reads.
#[derive(Clone, Copy, Debug)]
enum Setting {
    /// A call to `sync_memory_budget` with these bytes.
    Budget(usize),
    /// A call to `target_message_size` with these bytes.
    Target(usize),
}

/// Budgets on both sides of [`MIN_SYNC_MEMORY_BUDGET`], its neighbours
/// and the extremes included, and across the budgets that afford a target
/// above the default but not the wire's cap.
fn budgets() -> impl Strategy<Value = usize> {
    prop_oneof![
        0..=2 * MIN_SYNC_MEMORY_BUDGET,
        MIN_SYNC_MEMORY_BUDGET - 1..=MIN_SYNC_MEMORY_BUDGET + 1,
        MIN_SYNC_MEMORY_BUDGET..=Allowances::of::<Local>(MAX_RUN_BUDGET_BYTES).minimum(),
        Just(usize::MAX),
        any::<usize>(),
    ]
}

/// Targets on both sides of [`default::TARGET_MESSAGE_SIZE`] and of the
/// wire's cap ([`MAX_RUN_BUDGET_BYTES`]), zero and the largest value
/// included.
fn targets() -> impl Strategy<Value = usize> {
    prop_oneof![
        0..=2 * default::TARGET_MESSAGE_SIZE,
        Just(default::TARGET_MESSAGE_SIZE),
        MAX_RUN_BUDGET_BYTES - 1..=MAX_RUN_BUDGET_BYTES + 1,
        Just(usize::MAX),
        any::<usize>(),
    ]
}

/// Calls to either setter, from the families [`budgets`] and [`targets`].
fn settings() -> impl Strategy<Value = Setting> {
    prop_oneof![
        budgets().prop_map(Setting::Budget),
        targets().prop_map(Setting::Target),
    ]
}

proptest! {
    /// The target and the budget a peer's sessions run on depend only on
    /// the last budget and the last target set, never on the order or
    /// repetition of the setters' calls.
    ///
    /// Every sequence of calls, both setters interleaved and repeated,
    /// gives on a `Peer` what setting the two last values once gives in
    /// either order, and the same through `into_rumors` and on a
    /// `Bootstrap` builder. A setter the sequence never calls holds its
    /// default. Budgets lie on both sides of the minimum and across the
    /// range where the target is lowered, and targets on both sides of the
    /// default and of the wire's cap. What the two values are is
    /// `the_target_never_raises_the_budget`'s claim.
    #[test]
    fn effective_settings_ignore_the_setters_order(
        calls in prop::collection::vec(settings(), 0..8),
        network in network_seed(),
    ) {
        let budget = calls
            .iter()
            .rev()
            .find_map(|call| match call {
                Setting::Budget(bytes) => Some(*bytes),
                Setting::Target(_) => None,
            })
            .unwrap_or(default::SYNC_MEMORY_BUDGET);
        let target = calls
            .iter()
            .rev()
            .find_map(|call| match call {
                Setting::Target(bytes) => Some(*bytes),
                Setting::Budget(_) => None,
            })
            .unwrap_or(default::TARGET_MESSAGE_SIZE);

        let read = |settings: super::SynchronizationSettings| {
            (settings.sync_memory_budget, settings.target_message_size)
        };
        let budget_first = network.peer::<u64>()
            .sync_memory_budget(budget)
            .target_message_size(target);
        let expected = read(budget_first.synchronization_settings());
        let target_first = network.peer::<u64>()
            .target_message_size(target)
            .sync_memory_budget(budget);
        prop_assert_eq!(read(target_first.synchronization_settings()), expected);
        let sequenced = calls.iter().fold(network.peer::<u64>(), |peer, call| match *call {
            Setting::Budget(bytes) => peer.sync_memory_budget(bytes),
            Setting::Target(bytes) => peer.target_message_size(bytes),
        });
        prop_assert_eq!(read(sequenced.synchronization_settings()), expected);
        prop_assert_eq!(read(sequenced.into_rumors().synchronization_settings()), expected);

        let bootstrap = calls.iter().fold(Peer::<u64>::bootstrap(), |builder, call| match *call {
            Setting::Budget(bytes) => builder.sync_memory_budget(bytes),
            Setting::Target(bytes) => builder.target_message_size(bytes),
        });
        let limits = bootstrap.sync_memory_budget.session(bootstrap.run_budget);
        prop_assert_eq!((limits.budget.memory, limits.target.bytes()), expected);
    }

    /// A peer's message-size target never raises its memory budget.
    ///
    /// Its sessions run at the largest target, at most the one set, whose
    /// minimum fits the larger of the budget set and
    /// [`MIN_SYNC_MEMORY_BUDGET`], on the budget set raised to that
    /// target's minimum.
    ///
    /// For every budget and target set, the reported budget is the budget
    /// set raised to the reported target's minimum, never more than the
    /// larger of the budget set and [`MIN_SYNC_MEMORY_BUDGET`], and at or
    /// above that minimum exactly the budget set. The reported target never
    /// exceeds the target set, saturated at the wire's cap, nor falls
    /// below the smaller of that and the default; it is either that target
    /// or one below a target whose minimum exceeds what the budget affords.
    /// Setting the reported target back changes neither value.
    #[test]
    fn the_target_never_raises_the_budget(
        budget in budgets(),
        target in targets(),
        network in network_seed(),
    ) {
        let settings = network.peer::<u64>()
            .sync_memory_budget(budget)
            .target_message_size(target)
            .synchronization_settings();
        let (memory, lowered) = (settings.sync_memory_budget, settings.target_message_size);
        let minimum = |target: usize| Allowances::of::<Local>(target).minimum();
        let affords = budget.max(MIN_SYNC_MEMORY_BUDGET);
        let saturated = RunBudget::from_bytes(target).bytes();

        prop_assert_eq!(memory, budget.max(minimum(lowered)));
        prop_assert!(memory <= affords);
        if budget >= MIN_SYNC_MEMORY_BUDGET {
            prop_assert_eq!(memory, budget);
        }

        prop_assert!(lowered <= saturated);
        prop_assert!(lowered >= saturated.min(default::TARGET_MESSAGE_SIZE));
        prop_assert!(lowered == saturated || minimum(lowered + 1) > affords);

        let again = network.peer::<u64>()
            .sync_memory_budget(budget)
            .target_message_size(lowered)
            .synchronization_settings();
        prop_assert_eq!(again, settings);
    }
}

/// The minimum memory budget on the in-memory backend is the "about
/// 36.4 MiB" its documentation states, on the 64-bit targets the
/// documentation's figures are measured on.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_minimum_budget_is_the_documented_figure() {
    const MIB: usize = 1 << 20;
    assert!(
        (36 * MIB + 3 * MIB / 10..36 * MIB + MIB / 2).contains(&MIN_SYNC_MEMORY_BUDGET),
        "the minimum budget is {MIN_SYNC_MEMORY_BUDGET} B, not about 36.4 MiB",
    );
}

/// The largest budgeted message and the room for messages on their way, at
/// the default target, are the figures their documentation states.
///
/// Those are "about 512 KiB" and "about 1 MiB", on the 64-bit targets the
/// documentation's figures are measured on.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_message_room_is_the_documented_figure() {
    const KIB: usize = 1 << 10;
    const MIB: usize = 1 << 20;
    assert!(
        (508 * KIB..516 * KIB).contains(&MAX_BUDGETED_MESSAGE_SIZE),
        "the largest budgeted message is {MAX_BUDGETED_MESSAGE_SIZE} B, not about 512 KiB",
    );
    let pools =
        crate::tree::mirror::streaming::remote::wire_pools_bytes(default::TARGET_MESSAGE_SIZE);
    assert!(
        (MIB - MIB / 20..MIB + MIB / 20).contains(&pools),
        "the room for messages on their way is {pools} B, not about 1 MiB",
    );
}

/// The session's write buffer is the "about 63 KiB" the crate and peer
/// documentation state.
///
/// Its size follows the wire format's full-fan query frame, not the target's
/// pointer width, so the figure holds on every target.
#[test]
fn the_write_buffer_is_the_documented_figure() {
    const KIB: usize = 1 << 10;
    assert!(
        (62 * KIB + KIB / 2..63 * KIB + KIB / 2).contains(&WRITE_BUFFER_BYTES),
        "the write buffer is {WRITE_BUFFER_BYTES} B, not about 63 KiB",
    );
}

impl Drop for OnDrop {
    /// Run the payload's callback when its last message handle is released.
    fn drop(&mut self) {
        (self.callback)();
    }
}

proptest! {
    /// Exhausting optimistic swaps reaches the exclusive fallback, preserving
    /// every intervening commit and applying the identity callback once.
    #[test]
    fn conflicts_reach_exclusive_publication(
        conflicts in 0usize..=OPTIMISTIC_ATTEMPTS,
        remote_events in 0usize..8,
        redact in any::<bool>(),
        reject in any::<bool>(),
    ) {
        completes(move || {
            let mut party = Party::seed();
            let remote = party.fork();
            let prior = Tree::default();
            let mut incoming = prior.clone();
            for i in 0..remote_events { insert(&mut incoming, &remote, i as u64); }
            let sender = watch::Sender::new(Inner::new(party, prior.clone()));
            let receiver = Rc::new(RefCell::new(sender.subscribe()));
            let live = Rc::new(RefCell::new(prior.clone()));
            let attempts = Rc::new(Cell::new(0));
            BEFORE_SWAP.with_borrow_mut(|hook| {
                let sender = sender.clone();
                let live = live.clone();
                let attempts = attempts.clone();
                let receiver = receiver.clone();
                *hook = Some(Box::new(move || {
                    let attempt = attempts.get();
                    attempts.set(attempt + 1);
                    if attempt < conflicts {
                        Inner::commit(&sender, |party, tree| {
                            insert(tree, party, 100 + attempt as u64);
                            if redact {
                                let forgotten = tree.latest().clone();
                                tree.act(party, [Action::Forget(forgotten)]);
                            }
                            true
                        });
                        *live.borrow_mut() = Inner::snapshot(&sender);
                        receiver.borrow_mut().borrow_and_update();
                    }
                }));
            });
            let gate = sender.borrow().commit_gate.clone();
            let mut called = 0;
            let result = Inner::publish(&sender, &prior, &incoming, |_| {
                called += 1;
                assert_eq!(gate.try_read().is_err(), conflicts == OPTIMISTIC_ATTEMPTS,
                    "only exhausted retries should hold the gate exclusively");
                if reject { Err(()) } else { Ok(()) }
            });
            BEFORE_SWAP.with_borrow_mut(|hook| *hook = None);
            assert_eq!(called, 1);
            assert_eq!(attempts.get(), (conflicts + 1).min(OPTIMISTIC_ATTEMPTS));
            let mut expected = live.borrow().clone();
            if !reject { expected.join(incoming); }
            assert_eq!(result.is_err(), reject);
            Inner::snapshot(&sender).assert_memos_warm();
            assert_eq!(Inner::snapshot(&sender), expected);
            let changed = expected != *live.borrow();
            assert_eq!(receiver.borrow().has_changed().unwrap(), !reject && changed);
        });
    }

    /// Optimistic and exclusive publication both retain local changes made
    /// after the session snapshot, and notify exactly when state changes.
    #[test]
    fn publication_preserves_concurrent_changes(
        local_events in 0usize..8,
        remote_events in 0usize..8,
        redact_local in any::<bool>(),
        redact_remote in any::<bool>(),
        exclusive in any::<bool>(),
    ) {
        let mut local = Party::seed();
        let mut prior = Tree::default();
        insert(&mut prior, &local, 0);
        let original = prior.latest().clone();
        let remote = local.fork();
        let mut incoming = prior.clone();
        let mut live = prior.clone();
        for i in 0..local_events { insert(&mut live, &local, 100 + i as u64); }
        for i in 0..remote_events { insert(&mut incoming, &remote, 200 + i as u64); }
        if redact_local {
            live.act(&local, [Action::Forget(original.clone())]);
        }
        if redact_remote {
            incoming.act(&remote, [Action::Forget(original.clone())]);
        }
        let mut expected = live.clone();
        expected.join(incoming.clone());
        let changed = expected != live;
        let sender = watch::Sender::new(Inner::new(local, live));
        Inner::snapshot(&sender).assert_memos_warm();
        let mut receiver = sender.subscribe();
        let gate = sender.borrow().commit_gate.clone();
        let mut called = 0;
        let mut update = |_: &mut Inner<u64>| {
            called += 1;
            Ok::<_, Infallible>(())
        };
        if exclusive {
            Inner::publish_exclusive(&sender, &incoming, &gate, &mut update).unwrap();
        } else {
            Inner::publish(&sender, &prior, &incoming, &mut update).unwrap();
        }
        prop_assert_eq!(called, 1);
        prop_assert_eq!(&sender.borrow().tree, &expected);
        prop_assert_eq!(receiver.has_changed().unwrap(), changed);

        // Publishing the same state again must not cause an echo notification.
        receiver.borrow_and_update();
        Inner::publish(&sender, &expected, &expected, |_| Ok::<_, Infallible>(())).unwrap();
        prop_assert!(!receiver.has_changed().unwrap());
    }

    /// A stale snapshot cannot publish content or an identity update; the
    /// candidate stays available for rebasing, including across empty trees.
    #[test]
    fn stale_swaps_leave_state_and_callbacks_untouched(empty in any::<bool>()) {
        let party = Party::seed();
        let mut prior = Tree::default();
        if !empty { insert(&mut prior, &party, 0); }
        let mut live = prior.clone();
        insert(&mut live, &party, 1);
        if empty {
            let inserted = live.latest().clone();
            live.act(&party, [Action::Forget(inserted.clone())]);
        }
        let sender = watch::Sender::new(Inner::new(party, live.clone()));
        let receiver = sender.subscribe();
        let mut candidate = prior.clone();
        let mut called = false;
        let mut update = |_: &mut Inner<u64>| {
            called = true;
            Ok::<_, Infallible>(())
        };
        let gate = sender.borrow().commit_gate.clone();
        let result = {
            let _hold = gate.read().unwrap();
            Inner::try_swap(&sender, &prior, &mut candidate, &mut update)
        };
        prop_assert!(result.is_none());
        prop_assert!(!called);
        prop_assert_eq!(candidate, prior);
        prop_assert_eq!(&sender.borrow().tree, &live);
        prop_assert!(!receiver.has_changed().unwrap());
    }

    /// A rejected identity update publishes neither the candidate tree nor a
    /// notification, on either the optimistic or exclusive path.
    #[test]
    fn rejected_updates_do_not_publish(exclusive in any::<bool>(), events in 1usize..8) {
        let party = Party::seed();
        let prior = Tree::default();
        let mut incoming = prior.clone();
        for i in 0..events { insert(&mut incoming, &party, i as u64); }
        let sender = watch::Sender::new(Inner::new(party, prior.clone()));
        let receiver = sender.subscribe();
        let gate = sender.borrow().commit_gate.clone();
        let result = if exclusive {
            Inner::publish_exclusive(&sender, &incoming, &gate, &mut |_| Err::<(), _>("overlap"))
        } else {
            Inner::publish(&sender, &prior, &incoming, |_| Err::<(), _>("overlap"))
        };
        prop_assert_eq!(result, Err("overlap"));
        prop_assert_eq!(&sender.borrow().tree, &prior);
        prop_assert!(!receiver.has_changed().unwrap());
    }

    /// Competing gossip publishers and local commits all finish, and their
    /// combined result matches a join of the states actually committed.
    #[test]
    fn concurrent_publication_keeps_every_update(
        remote_events in prop::collection::vec(1usize..12, 2..5),
        local_events in 1usize..20,
        exclusive in any::<bool>(),
    ) {
        completes(move || {
            let mut local = Party::seed();
            let prior = Tree::default();
            let incoming: Vec<_> = remote_events.into_iter().map(|events| {
                let remote = local.fork();
                let mut tree = prior.clone();
                for i in 0..events { insert(&mut tree, &remote, i as u64); }
                tree
            }).collect();
            let mut expected = prior.clone();
            let sender = watch::Sender::new(Inner::new(local, prior.clone()));
            let start = Arc::new(Barrier::new(incoming.len() + 1));
            thread::scope(|scope| {
                for tree in &incoming {
                    let sender = &sender;
                    let prior = &prior;
                    let start = &start;
                    scope.spawn(move || {
                        start.wait();
                        if exclusive {
                            let gate = sender.borrow().commit_gate.clone();
                            Inner::publish_exclusive(sender, tree, &gate, &mut |_| Ok::<_, Infallible>(())).unwrap();
                        } else {
                            Inner::publish(sender, prior, tree, |_| Ok::<_, Infallible>(())).unwrap();
                        }
                    });
                }
                start.wait();
                for i in 0..local_events {
                    Inner::commit(&sender, |party, tree| {
                        insert(tree, party, i as u64);
                        if i + 1 == local_events {
                            // These stamps include any gossip already published;
                            // replaying the sends in isolation would mint others.
                            expected = tree.clone();
                        }
                        true
                    });
                    thread::yield_now();
                }
            });
            for tree in incoming { expected.join(tree); }
            Inner::snapshot(&sender).assert_memos_warm();
            assert_eq!(Inner::snapshot(&sender), expected);
        });
    }

    /// Releasing a redacted payload after either publication path permits a
    /// reentrant read and commit; its new message survives the outer commit.
    #[test]
    fn publication_releases_payloads_after_unlocking(exclusive in any::<bool>()) {
        completes(move || {
            let sender = watch::Sender::new(Inner::new(Party::seed(), Tree::<OnDrop>::default()));
            let prior = Inner::snapshot(&sender);
            let callback_sender = sender.clone();
            Inner::commit(&sender, |party, tree| {
                let message = on_drop(move || {
                    assert!(Inner::snapshot(&callback_sender).is_empty());
                    Inner::commit(&callback_sender, |party, tree| {
                        tree.act(party, [Action::Insert(on_drop(|| {}))]);
                        true
                    });
                });
                tree.act(party, [Action::Insert(message)]);
                true
            });
            // Only the published tree retains the payload. The incoming state
            // has observed and redacted it since this session's empty snapshot.
            let mut incoming = Inner::snapshot(&sender);
            {
                let inner = sender.borrow();
                let forgotten = incoming.latest().clone();
                incoming.act(&inner.party, [Action::Forget(forgotten)]);
            }
            if exclusive {
                let gate = sender.borrow().commit_gate.clone();
                Inner::publish_exclusive(&sender, &incoming, &gate, &mut |_| Ok::<_, Infallible>(())).unwrap();
            } else {
                Inner::publish(&sender, &prior, &incoming, |_| Ok::<_, Infallible>(())).unwrap();
            }
            assert_eq!(Inner::snapshot(&sender).len(), 1);
        });
    }
}

/// Excluding competing writers leaves snapshot readers free to proceed.
#[test]
fn exclusive_gate_allows_snapshots() {
    completes(|| {
        let sender = watch::Sender::new(Inner::new(Party::seed(), Tree::<u64>::default()));
        let gate = sender.borrow().commit_gate.clone();
        let _hold = gate.write().unwrap();
        let reader = thread::spawn(move || assert!(Inner::snapshot(&sender).is_empty()));
        reader.join().unwrap();
    });
}

/// An unwinding fallback leaves the candidate unpublished and releases its
/// exclusive gate, allowing a later commit to proceed despite lock poisoning.
#[test]
fn exclusive_publication_unwind_allows_later_commits() {
    completes(|| {
        let mut party = Party::seed();
        let remote = party.fork();
        let prior = Tree::default();
        let mut incoming = prior.clone();
        insert(&mut incoming, &remote, 0);
        let sender = watch::Sender::new(Inner::new(party, prior.clone()));
        BEFORE_SWAP.with_borrow_mut(|hook| {
            let sender = sender.clone();
            *hook = Some(Box::new(move || {
                Inner::commit(&sender, |party, tree| {
                    insert(tree, party, 1);
                    true
                });
            }));
        });
        let gate = sender.borrow().commit_gate.clone();
        let mut expected = None;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Inner::publish(
                &sender,
                &prior,
                &incoming,
                |inner| -> Result<(), Infallible> {
                    assert!(
                        gate.try_read().is_err(),
                        "the fallback holds the gate exclusively"
                    );
                    expected = Some(inner.tree.clone());
                    panic!("publication callback unwinds");
                },
            )
            .unwrap();
        }));
        BEFORE_SWAP.with_borrow_mut(|hook| *hook = None);
        assert!(outcome.is_err());
        assert_eq!(Inner::snapshot(&sender), expected.unwrap());
        Inner::commit(&sender, |party, tree| {
            insert(tree, party, 2);
            true
        });
        assert_eq!(Inner::snapshot(&sender).len(), OPTIMISTIC_ATTEMPTS + 1);
    });
}

proptest! {
    /// Installation and subsequent local commits publish warm metadata even
    /// when all leaves share a long compressed prefix.
    #[test]
    fn deep_installation_and_local_commit_prepare_memos(depth in 0usize..32, width in 1usize..64) {
        use crate::tree::typed::Path;
        let party = Party::seed();
        let mut version = crate::Version::new();
        let paths: Vec<_> = (0..=width).map(|index| {
            version.tick(&party);
            let mut path = [0; 32];
            path[depth] = u8::try_from(index).unwrap();
            (version.clone(), Path::from(path))
        }).collect();
        Path::with_leaf_paths(paths, || {
            let mut tree = Tree::default();
            for i in 0..width { insert(&mut tree, &party, i as u64); }
            let sender = watch::Sender::new(Inner::new(party, tree));
            Inner::snapshot(&sender).assert_memos_warm();
            Inner::commit(&sender, |party, tree| {
                insert(tree, party, width as u64);
                true
            });
            Inner::snapshot(&sender).assert_memos_warm();
        });
    }

    /// While local preparation is suspended, snapshots see the old tree and a
    /// concurrent fork waits to inherit the completed commit's history.
    #[test]
    fn local_preparation_orders_party_handoffs(events in 1usize..32) {
        completes(move || {
            let sender = watch::Sender::new(Inner::new(Party::seed(), Tree::default()));
            let (started, ready) = mpsc::channel();
            let (resume, proceed) = mpsc::channel();
            let writer = sender.clone();
            let worker = thread::spawn(move || {
                Inner::commit(&writer, |party, tree| {
                    for i in 0..events { insert(tree, party, i as u64); }
                    // Custody changes must wait until this candidate is published.
                    let gate = writer.borrow().commit_gate.clone();
                    assert!(gate.try_read().is_err());
                    started.send(()).unwrap();
                    proceed.recv().unwrap();
                    true
                });
            });
            ready.recv().unwrap();
            let owner = sender.clone();
            let handoff = thread::spawn(move || {
                let mut inherited = None;
                Inner::update_party(&owner, |inner| {
                    inherited = Some((inner.party.fork(), inner.tree.clone()));
                });
                inherited.unwrap()
            });
            // This read must complete while the writer and handoff remain pending.
            assert!(Inner::snapshot(&sender).is_empty());
            resume.send(()).unwrap();
            worker.join().unwrap();
            let (party, mut inherited) = handoff.join().unwrap();
            assert_eq!(inherited.len(), events);
            inherited.assert_memos_warm();
            let committed = inherited.latest().clone();
            insert(&mut inherited, &party, events as u64);
            assert!(inherited.latest() > committed);
            assert_eq!(inherited.len(), events + 1);
        });
    }
}

/// Join `provider`'s network over an in-memory connection, returning the
/// joined peer.
fn joined(provider: &Peer<u64>) -> Peer<u64> {
    pollster::block_on(async {
        let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        let (served, joined) = futures::join!(
            provider.gossip_once(&mut a_end.read, &mut a_end.write),
            Peer::<u64>::bootstrap().join(&mut b_end.read, &mut b_end.write),
        );
        served.expect("the provider serves the bootstrap");
        match joined {
            Joined::Joined { peer, .. } => peer,
            _ => panic!("the provider serves the bootstrap"),
        }
    })
}

/// Gossip `a` and `b` over an in-memory connection until both complete.
fn gossiped(a: &Peer<u64>, b: &Peer<u64>) {
    pollster::block_on(async {
        let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        let (a_out, b_out) = futures::join!(
            a.gossip_once(&mut a_end.read, &mut a_end.write),
            b.gossip_once(&mut b_end.read, &mut b_end.write),
        );
        a_out.expect("gossip completes on a");
        b_out.expect("gossip completes on b");
    });
}

/// The payloads a replica holds live, sorted.
fn live_values(peer: &Peer<u64>) -> Vec<u64> {
    let mut values: Vec<u64> = peer.snapshot().iter().map(|(_, value)| *value).collect();
    values.sort_unstable();
    values
}

/// Messages each side holds before the racing session: the ones both
/// hold, and the ones each originated since the join.
const RACED_MESSAGES: u64 = 24;

/// The chunks of `a`'s messages the racing threads redact, one per call of
/// the publication hook, and the messages one thread sends at each call.
const RACED_CHUNK: usize = 8;

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Redactions from two threads and sends from a third, racing a gossip
    /// session's publication, all survive it: no redacted message is live
    /// afterwards, and every sent message is.
    ///
    /// The racing commits land while the session's result waits to
    /// publish; the other side honors the redactions once the two gossip
    /// again.
    ///
    /// Each side holds messages both hold and messages only it holds. The
    /// publication hook runs before every optimistic swap of a session's
    /// result, on this thread, which publishes both sides' results. At
    /// each of its first `OPTIMISTIC_ATTEMPTS + 1` calls it runs two
    /// threads that each redact half of the next chunk of `a`'s messages,
    /// held before the session, and a third that sends, all on `a`, and
    /// joins them. Nothing moves `b`'s tree, so `b` publishes at its first
    /// attempt, and whichever side publishes first, each of `a`'s
    /// optimistic swaps finds its tree moved, and `a` publishes through the
    /// exclusive path. The hook's call count shows it: `a`'s two optimistic
    /// attempts and `b`'s one. A redacted message that `a` also supplies to
    /// `b` in the session is live on `b` until the next gossip honors the
    /// redaction.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn redactions_racing_publication_from_two_threads_are_honored(network in network_seed()) {
        let a = network.peer::<u64>();
        a.send_all(0..RACED_MESSAGES).unwrap();
        let b = joined(&a);
        a.send_all(1_000..1_000 + RACED_MESSAGES).unwrap();
        b.send_all(2_000..2_000 + RACED_MESSAGES).unwrap();
        let held: Vec<Version> = a.snapshot().versions().cloned().collect();
        let chunks: Vec<Vec<Version>> = held
            .iter()
            .step_by(2)
            .cloned()
            .collect::<Vec<_>>()
            .chunks(RACED_CHUNK)
            .take(OPTIMISTIC_ATTEMPTS + 1)
            .map(<[_]>::to_vec)
            .collect();
        prop_assert_eq!(chunks.len(), OPTIMISTIC_ATTEMPTS + 1);
        let redacted: Vec<Version> = chunks.iter().flatten().cloned().collect();
        let calls = Rc::new(Cell::new(0usize));
        let sent = Rc::new(RefCell::new(Vec::new()));
        BEFORE_SWAP.with_borrow_mut(|hook| {
            let racer = a.share();
            let calls = calls.clone();
            let sent = sent.clone();
            let mut chunks = chunks.into_iter();
            *hook = Some(Box::new(move || {
                let call = calls.get();
                calls.set(call + 1);
                let Some(chunk) = chunks.next() else { return };
                let (left, right) = chunk.split_at(chunk.len() / 2);
                let first = 3_000 + 100 * call as u64;
                let sends: Vec<u64> = (first..first + RACED_CHUNK as u64).collect();
                let racer = &racer;
                thread::scope(|scope| {
                    for part in [left, right] {
                        scope.spawn(move || {
                            for version in part {
                                racer.redact(version);
                            }
                        });
                    }
                    let sends = &sends;
                    scope.spawn(move || racer.send_all(sends.iter().copied()).unwrap());
                });
                sent.borrow_mut().extend(sends);
            }));
        });
        gossiped(&a, &b);
        BEFORE_SWAP.with_borrow_mut(|hook| *hook = None);
        prop_assert_eq!(calls.get(), OPTIMISTIC_ATTEMPTS + 1);
        let sent = sent.borrow().clone();
        let a_live = live_values(&a);
        for version in &redacted {
            prop_assert!(!a.snapshot().contains(version), "a redacted message is live on a");
        }
        for value in sent.iter().copied().chain(2_000..2_000 + RACED_MESSAGES) {
            prop_assert!(a_live.binary_search(&value).is_ok(), "message {} is missing on a", value);
        }

        gossiped(&a, &b);
        prop_assert_eq!(live_values(&a), live_values(&b));
        for version in &redacted {
            prop_assert!(!b.snapshot().contains(version), "a redacted message is live on b");
        }
    }
}
