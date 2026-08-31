//! The rumors-blind wire legibility property.
//!
//! Every directed stream of a V2 session — the control stream and every
//! data stream, in both directions — parses as an RFC 8742 CBOR sequence
//! under a generic walk that knows nothing of rumors: standard tag
//! unwrapping only (55799 self-described CBOR passes through as a tag;
//! 63 wraps an embedded CBOR sequence; 24 wraps exactly one embedded
//! item; unknown tags pass through), with zero bytes outside CBOR items,
//! recursively down through every embedded byte string. The claim is a
//! family over sessions, so it is stated as a proptest: randomized peer
//! contents drive real gossip, bootstrap, and retire sessions through
//! the recording link, and every captured stream must walk clean.
//!
//! The walker deliberately uses only `ciborium::Value` — no rumors codec
//! type appears — so this suite is the committed, tamper-evident form of
//! the legibility promise: a wire change that smuggles a non-CBOR byte
//! anywhere onto a V2 stream fails here, whatever the snapshots say.

mod common;

use ciborium::value::Value;
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Peer, Rumors};

use crate::common::gossip_snapshot::{CapturedLink, capture_sides};
use crate::common::wire::{block_on, bootstrap_fork_async};

/// Tag number for an embedded CBOR sequence in a byte string (RFC 9277).
const TAG_CBOR_SEQUENCE: u64 = 63;

/// Tag number for an embedded CBOR data item in a byte string (RFC 8949).
const TAG_EMBEDDED_ITEM: u64 = 24;

/// Walk `bytes` as a CBOR sequence of at least zero items, recursing into
/// embedded byte strings, and report the first residue or parse failure.
fn walk_sequence(bytes: &[u8], context: &str) -> Result<(), String> {
    let mut input = bytes;
    let mut item = 0;
    while !input.is_empty() {
        let value: Value = ciborium::de::from_reader(&mut input)
            .map_err(|e| format!("{context}: item {item} does not parse: {e}"))?;
        walk_value(&value, &format!("{context}: item {item}"))?;
        item += 1;
    }
    Ok(())
}

/// Walk one parsed value, recursing into containers and embedded strings.
fn walk_value(value: &Value, context: &str) -> Result<(), String> {
    match value {
        Value::Tag(TAG_CBOR_SEQUENCE, inner) => match &**inner {
            Value::Bytes(bytes) => walk_sequence(bytes, &format!("{context}: tag 63")),
            _ => Err(format!("{context}: tag 63 does not wrap a byte string")),
        },
        Value::Tag(TAG_EMBEDDED_ITEM, inner) => match &**inner {
            Value::Bytes(bytes) => {
                let mut input = bytes.as_slice();
                let value: Value = ciborium::de::from_reader(&mut input)
                    .map_err(|e| format!("{context}: tag 24 content does not parse: {e}"))?;
                if !input.is_empty() {
                    return Err(format!(
                        "{context}: {} residue bytes behind tag 24's one item",
                        input.len()
                    ));
                }
                walk_value(&value, &format!("{context}: tag 24"))
            }
            _ => Err(format!("{context}: tag 24 does not wrap a byte string")),
        },
        Value::Tag(_, inner) => walk_value(inner, context),
        Value::Array(items) => items.iter().try_for_each(|item| walk_value(item, context)),
        Value::Map(entries) => entries.iter().try_for_each(|(key, value)| {
            walk_value(key, context)?;
            walk_value(value, context)
        }),
        _ => Ok(()),
    }
}

/// Assert every directed stream of one side's capture walks clean.
fn assert_legible(side: &str, capture: &CapturedLink) {
    walk_sequence(&capture.control, &format!("{side} control"))
        .unwrap_or_else(|e| panic!("illegible control stream: {e}"));
    for (index, stream) in capture.streams.iter().enumerate() {
        walk_sequence(stream, &format!("{side} data stream {index}"))
            .unwrap_or_else(|e| panic!("illegible data stream: {e}"));
    }
}

/// A peer loaded with `payloads`, forked from `parent` when one is given
/// (same universe) or freshly seeded otherwise.
fn loaded(parent: Option<&Rumors<Vec<u8>>>, payloads: &[Vec<u8>]) -> Rumors<Vec<u8>> {
    let peer = match parent {
        Some(parent) => block_on(bootstrap_fork_async(parent)),
        None => Peer::seed().sync_window_floor().into_rumors(),
    };
    for payload in payloads {
        peer.send(payload.clone()).unwrap();
    }
    peer
}

/// Arbitrary payload corpora: variable-length byte payloads on both
/// sides, small enough to keep hundreds of full sessions cheap and
/// varied enough to drive matches, queries, empty queries, and batched
/// supply runs.
#[allow(clippy::type_complexity)]
fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    let payload = vec(any::<u8>(), 0..48);
    (
        vec(payload.clone(), 0..12),
        vec(payload.clone(), 0..12),
        vec(payload, 0..12),
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 24,
        ..ProptestConfig::default()
    })]

    /// Every directed stream of an arbitrary V2 gossip session is a pure
    /// CBOR sequence under the rumors-blind walk.
    #[test]
    fn arbitrary_gossip_sessions_are_cbor_sequences(
        (shared, only_a, only_b) in corpora(),
    ) {
        let a = loaded(None, &shared);
        let b = loaded(Some(&a), &only_b);
        for payload in &only_a {
            a.send(payload.clone()).unwrap();
        }
        let (a_capture, b_capture) = capture_sides(
            {
                let a = a.clone();
                move |mut link| async move {
                    a.gossip(&mut link).await.expect("gossip A");
                }
            },
            {
                let b = b.clone();
                move |mut link| async move {
                    b.gossip(&mut link).await.expect("gossip B");
                }
            },
        );
        assert_legible("A", &a_capture);
        assert_legible("B", &b_capture);
    }

    /// Every directed stream of an arbitrary V2 bootstrap session is a
    /// pure CBOR sequence under the rumors-blind walk, the trailing party
    /// hand-off included.
    #[test]
    fn arbitrary_bootstrap_sessions_are_cbor_sequences(
        (shared, _, _) in corpora(),
    ) {
        let provider = loaded(None, &shared);
        let (provider_capture, newcomer_capture) = capture_sides(
            move |mut link| async move {
                provider.gossip(&mut link).await.expect("provider gossip");
            },
            move |mut link| async move {
                Peer::<Vec<u8>>::bootstrap()
                    .join(&mut link)
                    .await
                    .expect("bootstrap handshake")
                    .expect("provider served the bootstrap");
            },
        );
        assert_legible("provider", &provider_capture);
        assert_legible("newcomer", &newcomer_capture);
    }

    /// Every directed stream of an arbitrary V2 retire session is a pure
    /// CBOR sequence under the rumors-blind walk, the trailing party
    /// hand-off included.
    #[test]
    fn arbitrary_retire_sessions_are_cbor_sequences(
        (shared, only_absorber, only_retiree) in corpora(),
    ) {
        let absorber = loaded(None, &shared);
        let retiree = loaded(Some(&absorber), &only_retiree);
        for payload in &only_absorber {
            absorber.send(payload.clone()).unwrap();
        }
        let retiree = block_on(retiree.try_into_peer())
            .expect("the retiree handle is unique");
        let (absorber_capture, retiree_capture) = capture_sides(
            {
                let absorber = absorber.clone();
                move |mut link| async move {
                    absorber.gossip(&mut link).await.expect("absorber gossip");
                }
            },
            move |mut link| async move {
                match retiree.retire(&mut link).await {
                    rumors::Retire::Retired => {}
                    other => panic!("the retiree must retire cleanly, got {other:?}"),
                }
            },
        );
        assert_legible("absorber", &absorber_capture);
        assert_legible("retiree", &retiree_capture);
    }
}
