//! Static assertions that every async public method on the `rumors`
//! handle types returns a `Send` future, and that the handle types
//! themselves are `Send + Sync`.
//!
//! The motivating use case is `tokio::spawn(...)` on a multi-threaded
//! runtime, which requires its argument to be `Send`. Each test compiles
//! iff the relevant future (or type) is `Send` — the body just drops the
//! future without awaiting — so a regression to a `!Send` return fails
//! this crate's compilation.

use futures::StreamExt;
use rumors::{
    Batch, Bookmark, CausalMessages, Changes, Error, Gossip, Joined, Led, Peer, Protocol, Retire,
    Rumors, Snapshot, TryTick, Unbookmarked, UnorderedMessages,
    link::{
        Link, MemoryAcceptor, MemoryConnector, SessionState,
        routed::{Config, Endpoint, Incoming, LinkInfo, StreamAcceptor, StreamConnector, Token},
    },
    testing::MemoryDial,
};
use serde::{Deserialize, Serialize};

/// Compile-time `Send`-bound check. Takes its argument by reference so the
/// future can be dropped (rather than awaited) afterwards.
fn require_send<T: Send + ?Sized>(_: &T) {}

/// Compile-time `Send + Sync` check for the handle types themselves.
fn require_send_sync<T: Send + Sync>() {}

/// Compile-time `Send`-only check, for the exclusively-driven observer.
fn require_send_type<T: Send>() {}

/// Compile-time `Debug` check for public wrapper outcomes.
fn require_debug<T: std::fmt::Debug>() {}

/// Compile-time `Clone + Debug` check for immutable public views.
fn require_clone_debug<T: Clone + std::fmt::Debug>() {}

/// Compile-time standard-error check for public failures.
fn require_error<T: std::error::Error>() {}

/// Compile-time `Hash + Eq` check for public value types.
fn require_hash_eq<T: std::hash::Hash + Eq>() {}

/// Compile-time total-order check for public value types.
fn require_ord<T: Ord>() {}

/// Compile-time equality check for public value types.
fn require_eq<T: Eq>() {}

/// A legal payload that deliberately provides neither `Clone` nor `Debug`.
#[derive(Deserialize, Eq, PartialEq, Serialize)]
struct Opaque(u64);

/// A legal bookmark whose handle deliberately provides no `Debug` impl.
struct SilentBookmark;

/// Provide inert storage for compile-time wrapper checks.
impl Bookmark for SilentBookmark {
    /// This storage never fails.
    type Error = std::convert::Infallible;
    /// No record is ever returned.
    type Reader = tokio::io::Empty;

    /// Report that no bookmark exists.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(None)
    }

    /// Accept and discard replacement records.
    async fn store(&self, _bytes: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The handle types are `Send + Sync` (and the exclusively-driven
/// `UnorderedMessages` observer is `Send`), so handles can be shared and moved
/// across tasks.
#[test]
fn handle_types_are_send_sync() {
    require_send_sync::<Peer<String>>();
    require_send_sync::<Rumors<String>>();
    require_send_sync::<Snapshot<String>>();
    // `UnorderedMessages` is an exclusively-driven observer: it must move
    // into a spawned task (`Send`), but `&UnorderedMessages` has no
    // concurrent use, so
    // `Sync` is not part of its contract (the materialized quiet-period
    // wait future is `Send`-only).
    require_send_type::<UnorderedMessages<String>>();
}

/// Public wrappers inherit only the bounds required by their stored fields.
#[test]
fn wrapper_traits_do_not_inspect_payloads_or_bookmark_handles() {
    require_error::<Error<SilentBookmark>>();
    require_clone_debug::<Snapshot<Opaque>>();
    require_debug::<Retire<Opaque, SilentBookmark>>();
    require_debug::<Unbookmarked<Opaque, SilentBookmark>>();
    require_debug::<Joined<Opaque, SilentBookmark>>();
    require_debug::<Batch<'static, Opaque>>();
    require_debug::<CausalMessages<Opaque>>();
    require_debug::<Changes<Opaque>>();
    require_debug::<UnorderedMessages<Opaque>>();
    require_debug::<Link<Opaque, Opaque, Opaque, Opaque>>();
    require_debug::<MemoryConnector>();
    require_debug::<MemoryAcceptor>();
    require_debug::<Endpoint<MemoryDial>>();
    require_debug::<Incoming<MemoryDial>>();
    require_debug::<StreamConnector<MemoryDial>>();
    require_debug::<StreamAcceptor<tokio::io::DuplexStream>>();
}

/// Small public values implement the standard traits needed as map keys and
/// comparable configuration.
#[test]
fn public_value_traits_are_complete() {
    require_hash_eq::<TryTick>();
    require_hash_eq::<Led>();
    require_hash_eq::<Gossip>();
    require_hash_eq::<Protocol>();
    require_ord::<Protocol>();
    require_ord::<Token>();
    require_eq::<SessionState>();
    require_eq::<Config>();
    require_eq::<LinkInfo<String>>();
}

/// `Rumors::gossip_once`'s future is `Send`: a session can be `tokio::spawn`ed.
#[test]
fn gossip_once_future_is_send() {
    let alice = Peer::<String>::seed().sync_window_floor();
    let (mut link, _peer) = rumors::link::memory();
    let rumors = alice.into_rumors();
    let fut = rumors.gossip_once(&mut link);
    require_send(&fut);
    drop(fut);
}

/// `Bootstrap::join`'s future is `Send`: joining can be `tokio::spawn`ed.
#[test]
fn bootstrap_future_is_send() {
    let (mut link, _peer) = rumors::link::memory();
    let fut = Peer::<String>::bootstrap().join(&mut link);
    require_send(&fut);
    drop(fut);
}

/// `Peer::retire`'s future is `Send`: leaving can be `tokio::spawn`ed.
#[test]
fn retire_future_is_send() {
    let alice = Peer::<String>::seed().sync_window_floor();
    let (mut link, _peer) = rumors::link::memory();
    let fut = alice.retire(&mut link);
    require_send(&fut);
    drop(fut);
}

/// `Rumors::try_into_peer`'s future is `Send`: the reunite wait can run in
/// a spawned task.
#[test]
fn try_into_peer_future_is_send() {
    let alice = Peer::<String>::seed().sync_window_floor();
    let rumors = alice.into_rumors();
    let fut = rumors.try_into_peer();
    require_send(&fut);
    drop(fut);
}

/// The observer's item future is `Send`, for spawned and
/// `select!`-driven consumers.
#[test]
fn observer_futures_are_send() {
    let alice = Peer::<String>::seed().sync_window_floor().into_rumors();
    let mut messages = alice.unordered_messages();
    // The `Stream` face's item future must be `Send`, for
    // `tokio::spawn`d `select!` consumers.
    let fut = messages.next();
    require_send(&fut);
    drop(fut);
}

/// Custom peer policies retain Send for the continuous driver and one-shot future.
#[test]
fn configured_gossip_is_send() {
    let rumors = Peer::<String>::seed()
        .gossip_when(|changes| changes)
        .session_deadline(std::future::pending)
        .into_rumors();
    let (mut link, _remote) = rumors::link::memory();
    let driver = rumors.gossip(&mut link);
    require_send(&driver);
    drop(driver);
    require_send(&rumors.gossip_once(&mut link));
}
