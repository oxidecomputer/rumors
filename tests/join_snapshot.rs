//! Golden byte-level snapshots of a single join session: a stateless
//! newcomer obtaining a fully-formed [`rumors::Peer`] from an established
//! provider that drives `gossip` concurrently.
//!
//! The companion to `gossip_snapshot.rs` for the *join* leg of the
//! protocol. Each test stages a provider, drives one join through the
//! recording connection in [`common::gossip_snapshot::capture_session`], and
//! pins every wire byte. Traffic is grouped by logical stream while preserving
//! its exact per-stream order. Drift in the preamble, reconciliation, or
//! trailing party hand-off shows up as a diff. Re-accept only after a
//! deliberate protocol change: a new protocol version, never a mutation of an
//! existing one. The re-accept procedure (`cargo insta review`) is in
//! `AGENTS.md`.
//!
//! Party convention: **A is the provider** — the established peer serving its
//! state through `gossip` — and **B is the joining newcomer**, running
//! [`Peer::join`]. The lone exception is [`mutual_join_bails`], where
//! both sides join and neither has state to give.
//!
//! As in `gossip_snapshot.rs` the payload is `u64` (a fixed 8 bytes, easy to
//! spot in the hex), except [`string_payload`], which pins how a
//! variable-length value is framed inside the served whole-tree transfer.

use rumors_testkit::common;

use rumors::{Peer, Rumors};

use crate::common::gossip_snapshot::{capture_session, observed, seeded};

use serde::Serialize;
use serde::de::DeserializeOwned;
/// Capture one successful join: `provider` serves its state via `gossip`
/// (party A) while a fresh newcomer runs [`Peer::join`] (party B) and is
/// expected to be served a successor.
fn capture_join<T>(provider: Rumors<T>) -> String
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    capture_session(
        move |mut end, hook| async move {
            let provider = observed(provider, hook).await;
            provider
                .gossip_once(&mut end.read, &mut end.write)
                .await
                .expect("provider gossip");
        },
        move |mut end, hook| async move {
            match Peer::<T>::joining()
                .observe(hook)
                .join(&mut end.read, &mut end.write)
                .await
            {
                rumors::Join::Connected {
                    peer,
                    connection: rumors::Connection::Reusable { .. },
                } => peer,
                _ => panic!("provider served the join"),
            };
        },
    )
}

/// Join from an *empty* provider: the minimal join session. After the
/// preamble the provider serves an empty tree and hands off a freshly-forked
/// party — the shortest path by which a newcomer joins a universe.
#[test]
fn empty_provider() {
    let provider: Rumors<u64> = seeded();
    insta::assert_snapshot!(capture_join(provider));
}

/// Join from a populated provider.
///
/// The provider holds three distinct messages (`1`, `2`, `3`); the newcomer
/// receives the whole tree in one descent, so this pins the content-bearing
/// whole-tree frame that the empty case never exercises.
#[test]
fn populated_provider() {
    let provider: Rumors<u64> = seeded();
    provider.send_all([1, 2, 3]).unwrap();
    insta::assert_snapshot!(capture_join(provider));
}

/// Join of a non-primitive, variable-length payload.
///
/// A `u64` payload CBOR-encodes as one compact integer; a `String` encodes as a
/// CBOR text string with its own length header, so this is the only join
/// scenario that pins how a variable-length value is framed inside a served
/// leaf.
#[test]
fn string_payload() {
    let provider: Rumors<String> = seeded();
    provider
        .send_all(["hello".to_string(), "world".to_string()])
        .unwrap();
    insta::assert_snapshot!(capture_join(provider));
}

/// Both sides declare joining: neither has state to give, so each reads
/// the other's join intent from the preamble and bails.
///
/// The capture pins the bytes of that mutual stand-down — the preamble exchange
/// and nothing after it.
#[test]
fn mutual_join_bounces() {
    let capture = capture_session(
        |mut end, hook| async move {
            let out = Peer::<u64>::joining()
                .observe(hook)
                .join(&mut end.read, &mut end.write)
                .await;
            assert!(
                matches!(out, rumors::Join::Bounced { .. }),
                "a mutually-joining peer must return its builder"
            );
        },
        |mut end, hook| async move {
            let out = Peer::<u64>::joining()
                .observe(hook)
                .join(&mut end.read, &mut end.write)
                .await;
            assert!(
                matches!(out, rumors::Join::Bounced { .. }),
                "a mutually-joining peer must return its builder"
            );
        },
    );
    insta::assert_snapshot!(capture);
}
