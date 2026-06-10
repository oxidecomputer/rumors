//! Static assertions that every async public method on [`rumors::Known`]
//! returns a `Send` future.
//!
//! The motivating use case is `tokio::spawn(local.method(...))` on a
//! multi-threaded runtime, which requires its argument to be `Send`. Each
//! test compiles iff the relevant future is `Send`; the runtime body just
//! drops the future without awaiting. If any of the async methods
//! regresses to a `!Send` return, this crate fails to compile.

use rumors::Known;

/// Compile-time `Send`-bound check. Takes its argument by reference so the
/// future can be dropped (rather than awaited) afterwards.
fn require_send<T: Send + ?Sized>(_: &T) {}

#[test]
fn message_future_is_send() {
    let mut alice = Known::<String>::seed();
    let fut = alice.message(["hello".to_string()]);
    require_send(&fut);
    drop(fut);
}

#[test]
fn join_future_is_send() {
    let mut alice = Known::<String>::seed();
    let bob = Known::<String>::seed();
    let bob_fork = bob.rumors();
    let fut = alice.join(bob_fork);
    require_send(&fut);
    drop(fut);
}

#[test]
fn gossip_future_is_send() {
    let mut alice = Known::<String>::seed();
    let (_, b) = tokio::io::duplex(64);
    let (mut r, mut w) = tokio::io::split(b);
    let fut = alice.gossip(&mut r, &mut w);
    require_send(&fut);
    drop(fut);
}

/// The async API accepts non-`'static` callbacks: the closure can borrow
/// local state from the calling scope and the borrow remains valid for the
/// duration of the await. This test compiles iff `OnMessage` /
/// `OnMessageFut` are bound `+ Send + 'a` rather than `+ Send + 'static`,
/// and additionally exercises the borrow at runtime by collecting messages
/// into a borrowed `&mut Vec`.
#[tokio::test]
async fn callback_can_borrow_local_state() {
    let mut alice = Known::<String>::seed();
    let mut observed: Vec<String> = Vec::new();
    alice
        .message_then(
            ["one".to_string(), "two".to_string(), "three".to_string()],
            |_, _, m| {
                observed.push(m.as_ref().clone());
                std::future::ready(())
            },
        )
        .await;
    // `observed` is reclaimed once the future completes and releases the
    // borrow; the test would not compile under a `'static` callback bound.
    observed.sort();
    assert_eq!(observed, vec!["one", "three", "two"]);
}

/// The sync API similarly accepts non-`'static` callbacks. Without the
/// borrow relaxation this would force callers into `Arc<Mutex<_>>` for
/// every observation log.
#[test]
fn sync_callback_can_borrow_local_state() {
    use rumors::sync::Known as SyncLocal;
    let mut alice = SyncLocal::<String>::seed();
    let mut observed: Vec<String> = Vec::new();
    alice.message_then(
        ["one".to_string(), "two".to_string(), "three".to_string()],
        |_, _, m| observed.push(m.as_ref().clone()),
    );
    observed.sort();
    assert_eq!(observed, vec!["one", "three", "two"]);
}
