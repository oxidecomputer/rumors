//! Tests of the protocol channel's own contract, and of what the send hook
//! reports about each send.
//!
//! The contract covers the capacity the channel derives from
//! `futures-channel`'s buffer, the full-on-arrival report, the non-waiting
//! send's two failures, and the zero capacity it rejects.

use std::cell::{Cell, RefCell};
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use proptest::prelude::*;

use super::super::QueueKind;
use super::{EdgeSend, SendError, TrySendError, channel, observe_edge_sends};

proptest! {
    /// A send waits exactly when `capacity` items are queued, and only then
    /// does `send_with` call `on_full`, once.
    ///
    /// `on_full` runs once however often the waiting send is polled. The
    /// waiting send completes once the receiver takes an item, and items
    /// arrive in the order sent.
    #[test]
    fn send_with_reports_a_full_arrival_once(capacity in 1..=4usize, queued in 0..=4usize) {
        let queued = queued.min(capacity);
        let (mut sender, mut receiver) = channel(capacity);
        for item in 0..queued {
            pollster::block_on(sender.send(item)).expect("there is room");
        }

        let full = Cell::new(0);
        let mut cx = Context::from_waker(Waker::noop());
        let mut send = pin!(sender.send_with(queued, || full.set(full.get() + 1)));
        if queued == capacity {
            prop_assert!(send.as_mut().poll(&mut cx).is_pending());
            prop_assert!(send.as_mut().poll(&mut cx).is_pending());
            prop_assert_eq!(full.get(), 1);
            prop_assert_eq!(pollster::block_on(receiver.recv()), Some(0));
            prop_assert_eq!(send.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
        } else {
            prop_assert_eq!(send.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
        }
        prop_assert_eq!(full.get(), usize::from(queued == capacity));
    }

    /// `try_send` accepts exactly `capacity` items while nothing is
    /// received, then returns the next item as `Full`; once the receiver
    /// takes one item, it accepts one more.
    #[test]
    fn try_send_is_full_exactly_at_capacity(capacity in 1..=4usize) {
        let (mut sender, mut receiver) = channel(capacity);
        for item in 0..capacity {
            prop_assert_eq!(sender.try_send(item), Ok(()));
        }
        prop_assert_eq!(sender.try_send(capacity), Err(TrySendError::Full(capacity)));
        prop_assert_eq!(pollster::block_on(receiver.recv()), Some(0));
        prop_assert_eq!(sender.try_send(capacity), Ok(()));
        prop_assert_eq!(sender.try_send(capacity + 1), Err(TrySendError::Full(capacity + 1)));
    }
}

/// `send_with` on a channel whose receiver is gone returns the item without
/// calling `on_full`, even when the channel was full when the receiver went.
#[test]
fn send_with_on_a_closed_channel_returns_the_item() {
    let (mut sender, receiver) = channel(1);
    pollster::block_on(sender.send(1u8)).expect("there is room");
    drop(receiver);
    let full = Cell::new(false);
    let sent = pollster::block_on(sender.send_with(2, || full.set(true)));
    assert_eq!(sent, Err(SendError(2)));
    assert!(!full.get());
}

/// `try_send` on a channel whose receiver is gone returns the item as
/// `Closed`, even when the channel was full when the receiver went.
#[test]
fn try_send_on_a_closed_full_channel_returns_closed() {
    let (mut sender, receiver) = channel(1);
    assert_eq!(sender.try_send(1u8), Ok(()));
    assert_eq!(sender.try_send(2), Err(TrySendError::Full(2)));
    drop(receiver);
    assert_eq!(sender.try_send(3), Err(TrySendError::Closed(3)));
}

/// A capacity of zero panics, as it does for Tokio's bounded channel: an edge
/// with no slot could never make progress.
#[test]
#[should_panic(expected = "a protocol channel needs at least one slot")]
fn zero_capacity_panics() {
    let _ = channel::<u8>(0);
}

// On targets without native thread-local storage (illumos among the gate's),
// `thread_local!` expands through the OS-keyed path, and clippy's
// `missing_const_for_thread_local` misreads that expansion, reporting the
// `const` block below as missing. The allow is scoped to that target so the
// lint keeps guarding everywhere else.
std::thread_local! {
    /// The sends reported on this test thread while a case records them.
    #[cfg_attr(target_os = "illumos", allow(clippy::missing_const_for_thread_local))]
    static REPORTED: RefCell<Option<Vec<EdgeSend>>> = const { RefCell::new(None) };
}

/// Keep one reported send if this thread is recording.
///
/// Every test thread's instrumented edges report through the process's one
/// hook, so only the recording thread keeps what it sees.
fn keep(send: EdgeSend) {
    REPORTED.with(|reported| {
        if let Some(reported) = reported.borrow_mut().as_mut() {
            reported.push(send);
        }
    });
}

/// One step of a [`sends_park_exactly_when_they_fill_the_edge`] case.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Send one item, letting the receiver take one first if the send finds
    /// the channel full.
    Send,
    /// Send one item without waiting, if the channel has room.
    TrySend,
    /// Take one item, if any is queued.
    Receive,
}

proptest! {
    /// Under any interleaving of sends and receives, each completed send
    /// reports that it waited exactly when it found `capacity` items
    /// queued, and that it parked its sender exactly when it left
    /// `capacity` items queued.
    ///
    /// So a send parks exactly when it fills the edge, and a send into an
    /// edge with room to spare after it never parks: on an edge of two
    /// slots, a send that finds it empty does not park. The parked reading
    /// comes from the channel's own state; the queued counts come from
    /// this test's account of what it sent and took, kept apart from the
    /// channel. A send refused as full reports nothing.
    #[test]
    fn sends_park_exactly_when_they_fill_the_edge(
        capacity in 1..=4usize,
        steps in prop::collection::vec(
            prop_oneof![Just(Step::Send), Just(Step::TrySend), Just(Step::Receive)],
            0..48,
        ),
    ) {
        prop_assert!(observe_edge_sends(keep), "this suite installs the process's send hook");
        let kind = QueueKind::PipeOutgoing;
        let (sender, mut receiver) = channel(capacity);
        let mut sender = sender.edge(kind);
        let mut cx = Context::from_waker(Waker::noop());
        let mut queued = 0;
        let mut expected = Vec::new();
        REPORTED.with(|reported| reported.replace(Some(Vec::new())));
        for (item, step) in steps.into_iter().enumerate() {
            match step {
                Step::Send => {
                    let waited = queued == capacity;
                    let mut send = pin!(sender.send(item));
                    if waited {
                        prop_assert!(send.as_mut().poll(&mut cx).is_pending());
                        prop_assert!(matches!(receiver.poll_recv(&mut cx), Poll::Ready(Some(_))));
                        queued -= 1;
                    }
                    prop_assert_eq!(send.as_mut().poll(&mut cx), Poll::Ready(Ok(())));
                    queued += 1;
                    expected.push(EdgeSend { kind, waited, parked: queued == capacity });
                }
                Step::TrySend => match sender.try_send(item) {
                    Ok(()) => {
                        prop_assert!(queued < capacity);
                        queued += 1;
                        expected.push(EdgeSend { kind, waited: false, parked: queued == capacity });
                    }
                    Err(TrySendError::Full(_)) => prop_assert_eq!(queued, capacity),
                    Err(TrySendError::Closed(_)) => unreachable!("the receiver is present"),
                },
                Step::Receive => {
                    if queued > 0 {
                        prop_assert!(matches!(receiver.poll_recv(&mut cx), Poll::Ready(Some(_))));
                        queued -= 1;
                    }
                }
            }
        }
        let reported = REPORTED.with(|reported| reported.take()).unwrap_or_default();
        prop_assert_eq!(reported, expected);
    }
}
