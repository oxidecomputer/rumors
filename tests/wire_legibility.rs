//! Check that each direction of a session's connection is a CBOR sequence.
//!
//! A generic parser walks both directions' captured bytes, control items and
//! frames alike, using only `ciborium::Value`. It follows containers and tags, decoding tag 63's
//! byte string as an embedded sequence and tag 24's as exactly one embedded
//! item. Every byte at each level must belong to a complete CBOR item.
//!
//! Generated gossip, bootstrap, and retirement sessions supply the captures.
//! This complements wire snapshots: even an accepted snapshot change must still
//! produce bytes that a CBOR library can read without the Rumors codec.

use rumors_testkit::common;

use ciborium::value::Value;
use proptest::prelude::*;
use rumors::{Peer, Rumors};

use crate::common::gossip_snapshot::{capture_sides, corpora};
use crate::common::wire::{block_on, bootstrap_fork_async};
use rumors::testing::PipeCapture;

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

/// Assert one side's captured direction walks clean.
fn assert_legible(side: &str, capture: &PipeCapture) {
    walk_sequence(&capture.sent, &format!("{side} sent"))
        .unwrap_or_else(|e| panic!("illegible connection: {e}"));
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

proptest! {
    /// Each direction of an arbitrary V2 gossip session is a pure
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
                move |mut end| async move {
                    a.gossip_once(&mut end.read, &mut end.write).await.expect("gossip A");
                }
            },
            {
                let b = b.clone();
                move |mut end| async move {
                    b.gossip_once(&mut end.read, &mut end.write).await.expect("gossip B");
                }
            },
        );
        assert_legible("A", &a_capture);
        assert_legible("B", &b_capture);
    }

    /// Each direction of an arbitrary V2 bootstrap session is a
    /// pure CBOR sequence under the rumors-blind walk, the trailing party
    /// hand-off included.
    #[test]
    fn arbitrary_bootstrap_sessions_are_cbor_sequences(
        (shared, _, _) in corpora(),
    ) {
        let provider = loaded(None, &shared);
        let (provider_capture, newcomer_capture) = capture_sides(
            move |mut end| async move {
                provider.gossip_once(&mut end.read, &mut end.write).await.expect("provider gossip");
            },
            move |mut end| async move {
                let rumors::Joined::Joined { .. } =
                    Peer::<Vec<u8>>::bootstrap().join(&mut end.read, &mut end.write).await
                else {
                    panic!("provider must serve the bootstrap");
                };
            },
        );
        assert_legible("provider", &provider_capture);
        assert_legible("newcomer", &newcomer_capture);
    }

    /// Each direction of an arbitrary V2 retire session is a pure
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
                move |mut end| async move {
                    absorber.gossip_once(&mut end.read, &mut end.write).await.expect("absorber gossip");
                }
            },
            move |mut end| async move {
                match retiree.retire(&mut end.read, &mut end.write).await {
                    rumors::Retire::Retired { .. } => {}
                    other => panic!("the retiree must retire cleanly, got {other:?}"),
                }
            },
        );
        assert_legible("absorber", &absorber_capture);
        assert_legible("retiree", &retiree_capture);
    }
}
