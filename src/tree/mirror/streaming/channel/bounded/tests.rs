//! Tests of the protocol channel's own contract: the capacity it derives from
//! `futures-channel`'s buffer, the full-on-arrival report, the non-waiting
//! send's two failures, and the zero capacity it rejects.

use std::cell::Cell;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use proptest::prelude::*;

use super::{SendError, TrySendError, channel};

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
