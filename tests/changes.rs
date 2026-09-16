//! The [`rumors::Changes`] observer: the content-free change signal.
//!
//! Pins the contract stated on the type: an immediate first yield, exactly
//! one coalesced tick per observed frontier advance (however many commits
//! that was), ticks for every kind of commit — send, redact, and a join
//! learned by gossip — and a clean end once the set closes.

mod common;

use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::{Context, Poll, Wake, Waker};

use futures::{FutureExt, Stream, StreamExt};
use proptest::{collection::vec, prelude::*};
use rumors::{Changes, Peer, Rumors, TryTick, Version};

use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// A fresh observer yields immediately — even on an empty set — because a
/// new subscriber has seen nothing, so whatever the set holds is news.
#[test]
fn first_poll_yields_immediately() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let mut changes = rumors.changes();
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    // And with nothing further committed, the stream is quiet.
    assert_eq!(changes.next().now_or_never(), None);
}

/// Gossip reports new causal history even when no live message moves.
/// Sending and redacting before gossip leaves this history-only change.
#[test]
fn gossip_frontier_only_advance_ticks_the_observer() {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    // Register the wait before the update, as an idle gossip driver would.
    let mut b_changes = b.changes();
    assert_eq!(b_changes.next().now_or_never(), Some(Some(())));
    assert_eq!(b_changes.next().now_or_never(), None);

    // A sends and redacts before any session runs: A's tree is empty again,
    // and the redaction's only trace is A's advanced causal frontier.
    let version = a.send(7).unwrap();
    a.redact(&version);

    // Both sets stay empty, but B must learn and report A's history.
    let b_before = b.snapshot().latest().clone();
    wire_gossip(&a, &b);
    assert_ne!(*b.snapshot().latest(), b_before, "B's frontier advanced");
    assert_eq!(
        b.snapshot().latest(),
        a.snapshot().latest(),
        "B absorbed the redaction frontier"
    );

    // The documented contract owes the parked observer a tick for it.
    assert_eq!(
        b_changes.next().now_or_never(),
        Some(Some(())),
        "a frontier-only gossip advance must tick the observer"
    );
}

/// The stream ends once the set closes: with the `Peer` and every `Rumors`
/// gone no further change is possible, and a tick still owed (committed
/// after the last poll) is delivered before the end.
#[test]
fn set_closure_ends_the_stream() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let mut changes = rumors.changes();
    assert_eq!(changes.next().now_or_never(), Some(Some(())));

    rumors.send(1).unwrap();
    drop(rumors);

    // The final commit is still reported, then the stream ends.
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    assert_eq!(changes.next().now_or_never(), Some(None));
}

/// Holding a `Changes` does not count against the quiescence that lets
/// [`Rumors::try_into_peer`] reclaim the `Peer`.
#[test]
fn observer_does_not_block_peer_reclaim() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let _changes = rumors.changes();
    assert!(block_on(rumors.try_into_peer()).is_some());
}

/// A mutation or a poll of either observer interface.
#[derive(Clone, Debug)]
enum Op {
    /// Send a batch at either replica, including an empty batch.
    Send(bool, Vec<u64>),
    /// Redact a live version, or an absent version when the replica is empty.
    Redact(bool, prop::sample::Index),
    /// Exchange both replicas' state.
    Gossip,
    /// Read once, selecting either `Stream` or `try_next`.
    Read(bool),
}

/// Interleave local and remote changes with periods in which notifications coalesce.
fn arb_ops() -> impl Strategy<Value = Vec<Op>> {
    vec(
        prop_oneof![
            3 => (any::<bool>(), vec(any::<u64>(), 0..4))
                .prop_map(|(remote, values)| Op::Send(remote, values)),
            2 => (any::<bool>(), any::<prop::sample::Index>())
                .prop_map(|(remote, index)| Op::Redact(remote, index)),
            2 => Just(Op::Gossip),
            3 => any::<bool>().prop_map(Op::Read),
        ],
        0..40,
    )
}

/// Express one stream poll in the same terms as the non-blocking interface.
fn read(changes: &mut Changes<u64>, stream: bool) -> TryTick {
    if stream {
        match changes.next().now_or_never() {
            None => TryTick::Quiet,
            Some(None) => TryTick::Ended,
            Some(Some(())) => TryTick::Tick,
        }
    } else {
        changes.try_next()
    }
}

/// Count notifications of a registered task, independently of yielded items.
#[derive(Default)]
struct WakeCount(AtomicUsize);

/// Record both consuming and borrowed wakeups.
impl Wake for WakeCount {
    /// Record a wake while releasing the caller's handle.
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Record a wake without releasing the caller's handle.
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

proptest! {
    /// Either interface reports exactly the history advances since the last read,
    /// coalesces unread changes, and reports the final state before ending.
    #[test]
    fn changes_follow_history_under_interleaving(ops in arb_ops()) {
        let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork(&a);
        let mut changes = a.changes();
        let mut reported = None;
        for op in ops {
            match op {
                Op::Send(remote, values) => {
                    let peer = if remote { &b } else { &a };
                    peer.send_all(values).unwrap();
                }
                Op::Redact(remote, index) => {
                    let peer = if remote { &b } else { &a };
                    let snapshot = peer.snapshot();
                    let version = if snapshot.is_empty() {
                        Version::new()
                    } else {
                        snapshot.iter().nth(index.index(snapshot.len())).unwrap().0.clone()
                    };
                    peer.redact(&version);
                }
                Op::Gossip => wire_gossip(&a, &b),
                Op::Read(stream) => {
                    let latest = a.snapshot().latest().clone();
                    let expected = if reported.as_ref() == Some(&latest) {
                        TryTick::Quiet
                    } else {
                        TryTick::Tick
                    };
                    prop_assert_eq!(read(&mut changes, stream), expected);
                    reported = Some(latest);
                }
            }
        }
        let latest = a.snapshot().latest().clone();
        drop(a);
        if reported.as_ref() != Some(&latest) {
            prop_assert_eq!(read(&mut changes, true), TryTick::Tick);
        }
        prop_assert_eq!(read(&mut changes, false), TryTick::Ended);
        prop_assert_eq!(read(&mut changes, true), TryTick::Ended);
    }

    /// Empty batches and absent or repeated redactions do not wake a parked task.
    #[test]
    fn noop_commits_do_not_wake_observers(ops in vec(0u8..3, 1..40)) {
        let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let removed = a.send(7).unwrap();
        a.redact(&removed);
        let latest = a.snapshot().latest().clone();
        let mut changes = a.changes();
        let wakes = Arc::new(WakeCount::default());
        let waker = Waker::from(wakes.clone());
        let mut cx = Context::from_waker(&waker);
        prop_assert_eq!(Pin::new(&mut changes).poll_next(&mut cx), Poll::Ready(Some(())));
        prop_assert_eq!(Pin::new(&mut changes).poll_next(&mut cx), Poll::Pending);
        let parked = wakes.0.load(Ordering::Relaxed);

        for op in ops {
            match op {
                0 => a.send_all(std::iter::empty::<u64>()).unwrap(),
                1 => a.redact(&Version::new()),
                _ => a.redact(&removed),
            }
            // An unnecessary watch notification may wake the task even though
            // the observer suppresses the resulting unchanged version.
            prop_assert_eq!(wakes.0.load(Ordering::Relaxed), parked);
            prop_assert_eq!(a.snapshot().latest().clone(), latest.clone());
            prop_assert_eq!(Pin::new(&mut changes).poll_next(&mut cx), Poll::Pending);
        }
        // A real change proves that the waker was registered and can fire.
        a.send(8).unwrap();
        prop_assert!(wakes.0.load(Ordering::Relaxed) > parked);
        prop_assert_eq!(Pin::new(&mut changes).poll_next(&mut cx), Poll::Ready(Some(())));
    }
}
