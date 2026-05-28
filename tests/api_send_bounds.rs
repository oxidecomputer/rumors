//! Static assertions that every async public method on [`rumors::Local`]
//! returns a `Send` future.
//!
//! The motivating use case is `tokio::spawn(local.method(...))` on a
//! multi-threaded runtime, which requires its argument to be `Send`. Each
//! test compiles iff the relevant future is `Send`; the runtime body just
//! drops the future without awaiting. If any of the async methods
//! regresses to a `!Send` return, this crate fails to compile.

use rumors::{Local, ignore};

/// Compile-time `Send`-bound check. Takes its argument by reference so the
/// future can be dropped (rather than awaited) afterwards.
fn require_send<T: Send + ?Sized>(_: &T) {}

#[test]
fn message_future_is_send() {
    let mut alice = Local::<String, _>::for_party("send-bound-message", 0).unwrap();
    let fut = alice.message(["hello".to_string()], ignore);
    require_send(&fut);
    drop(fut);
}

#[test]
fn process_future_is_send() {
    let mut alice = Local::<String, _>::for_party("send-bound-process-a", 0).unwrap();
    let bob = Local::<String, _>::for_party("send-bound-process-b", 0).unwrap();
    let bob_fork = bob.fork();
    let fut = alice.process(bob_fork, ignore);
    require_send(&fut);
    drop(fut);
}

#[test]
fn gossip_future_is_send() {
    let alice = Local::<String, _>::for_party("send-bound-gossip", 0).unwrap();
    let (_, b) = tokio::io::duplex(64);
    let (mut r, mut w) = tokio::io::split(b);
    let fut = alice.gossip(&mut r, &mut w, ignore);
    require_send(&fut);
    drop(fut);
}
