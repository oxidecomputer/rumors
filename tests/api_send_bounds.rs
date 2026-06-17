//! Static assertions that every async public method on the `rumors`
//! handle types returns a `Send` future, and that the handle types
//! themselves are `Send + Sync`.
//!
//! The motivating use case is `tokio::spawn(...)` on a multi-threaded
//! runtime, which requires its argument to be `Send`. Each test compiles
//! iff the relevant future (or type) is `Send`; the runtime body just
//! drops the future without awaiting. If any of the async methods
//! regresses to a `!Send` return, this crate fails to compile.

use futures::StreamExt;
use rumors::{Peer, Rumors, Snapshot, UnorderedMessages};

/// Compile-time `Send`-bound check. Takes its argument by reference so the
/// future can be dropped (rather than awaited) afterwards.
fn require_send<T: Send + ?Sized>(_: &T) {}

/// Compile-time `Send + Sync` check for the handle types themselves.
fn require_send_sync<T: Send + Sync>() {}

/// Compile-time `Send`-only check, for the exclusively-driven observer.
fn require_send_type<T: Send>() {}

/// The handle types are `Send + Sync` (and the exclusively-driven
/// `Messages` observer is `Send`), so handles can be shared and moved
/// across tasks.
#[test]
fn handle_types_are_send_sync() {
    require_send_sync::<Peer<String>>();
    require_send_sync::<Rumors<String>>();
    require_send_sync::<Snapshot<String>>();
    // `Messages` is an exclusively-driven observer: it must move into a
    // spawned task (`Send`), but `&Messages` has no concurrent use, so
    // `Sync` is not part of its contract (the materialized quiet-period
    // wait future is `Send`-only).
    require_send_type::<UnorderedMessages<String>>();
}

/// `Rumors::gossip`'s future is `Send`: a session can be `tokio::spawn`ed.
#[test]
fn gossip_future_is_send() {
    let alice = Peer::<String>::seed();
    let (_, b) = tokio::io::duplex(64);
    let (mut r, mut w) = tokio::io::split(b);
    let rumors = alice.into_rumors();
    let fut = rumors.gossip(&mut r, &mut w);
    require_send(&fut);
    drop(fut);
}

/// `Peer::bootstrap`'s future is `Send`: joining can be `tokio::spawn`ed.
#[test]
fn bootstrap_future_is_send() {
    let (_, b) = tokio::io::duplex(64);
    let (mut r, mut w) = tokio::io::split(b);
    let fut = Peer::<String>::bootstrap(&mut r, &mut w);
    require_send(&fut);
    drop(fut);
}

/// `Peer::retire`'s future is `Send`: leaving can be `tokio::spawn`ed.
#[test]
fn retire_future_is_send() {
    let alice = Peer::<String>::seed();
    let (_, b) = tokio::io::duplex(64);
    let (mut r, mut w) = tokio::io::split(b);
    let fut = alice.retire(&mut r, &mut w);
    require_send(&fut);
    drop(fut);
}

/// `Rumors::try_into_peer`'s future is `Send`: the reunite wait can run in
/// a spawned task.
#[test]
fn try_into_peer_future_is_send() {
    let alice = Peer::<String>::seed();
    let rumors = alice.into_rumors();
    let fut = rumors.try_into_peer();
    require_send(&fut);
    drop(fut);
}

/// Both observer faces — `borrow_next`'s future and the `Stream`'s item
/// future — are `Send`, for spawned and `select!`-driven consumers.
#[test]
fn observer_futures_are_send() {
    let alice = Peer::<String>::seed().into_rumors();
    let mut messages = alice.unordered_messages();
    {
        let fut = messages.borrow_next();
        require_send(&fut);
        drop(fut);
    }
    // The `Stream` face's item future must be `Send` too, for
    // `tokio::spawn`d `select!` consumers.
    let fut = messages.next();
    require_send(&fut);
    drop(fut);
}
