//! Pins the payload-evolution contract the CBOR encoding was chosen for:
//! field and variant *names* are the wire contract, not positions.
//!
//! Two struct types with reordered fields, and two enum types with
//! reordered variants, exchange messages end to end — a `Peer` of one type
//! gossiping with a `Peer` of the other over an in-memory link, in both
//! directions — so the property is pinned through the crate's own encode
//! and decode paths, not against a serializer in isolation. A future
//! payload-encoding change that breaks name-keyed decoding (for example, a
//! positional struct encoding) fails these tests loudly.
//!
//! The evolution rules the crate documents ride the same mechanism and are
//! pinned beside it: unknown fields are skipped, and missing fields error
//! unless the field carries `#[serde(default)]`.
//!
//! The complement is pinned here too: payload bytes that do *not* decode
//! as the receiving type fail the session cleanly — a decode error at the
//! receiver's wire ingress, never a panic, moving nothing into its set.
//! Ingress decodes every payload through the receiving peer's own
//! deserializer, so no wire input can reach the typed-read downcast with
//! a foreign payload; these tests are the committed demonstration of that
//! claim.

use serde::{Deserialize, Serialize};

use rumors::Peer;

use serde::de::DeserializeOwned;
/// A struct payload in one field order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct WideV1 {
    id: u64,
    tag: String,
    data: Vec<u8>,
}

/// The same struct with its fields reordered: names unchanged, positions
/// scrambled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct WideV2 {
    data: Vec<u8>,
    id: u64,
    tag: String,
}

/// An enum payload in one variant order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum EventV1 {
    Ping(u64),
    Note { text: String, level: u8 },
    Stop,
}

/// The same enum with its variants (and one variant's fields) reordered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum EventV2 {
    Stop,
    Note { level: u8, text: String },
    Ping(u64),
}

/// Send `payload` from a fresh `Peer<A>` and receive it on a bootstrapped
/// `Peer<B>` over an in-memory link: the crate's whole encode/decode path,
/// across two payload *types*.
async fn exchanged<A, B>(payload: A) -> B
where
    A: Serialize + DeserializeOwned + Send + Sync + 'static,
    B: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    let sender = Peer::<A>::seed().into_rumors();
    sender.send(payload);

    let (mut near, mut far) = rumors::link::memory();
    let serve = sender.clone();
    let server = tokio::spawn(async move { serve.gossip(&mut far).await.unwrap() });
    let receiver = Peer::<B>::bootstrap()
        .join(&mut near)
        .await
        .expect("the bootstrap session succeeds")
        .expect("the sender is established")
        .into_rumors();
    server.await.expect("the serving task");

    let snapshot = receiver.snapshot();
    let (_, message) = snapshot.iter().next().expect("one live message");
    (*message).clone()
}

/// Struct fields decode by name: a payload encoded with one field order
/// decodes into the reordered type with every field intact, in both
/// directions.
#[tokio::test]
async fn reordered_struct_fields_decode_by_name() {
    let v1 = WideV1 {
        id: 7,
        tag: "meeting".to_string(),
        data: vec![1, 2, 3],
    };
    let v2: WideV2 = exchanged(v1.clone()).await;
    assert_eq!(v2.id, v1.id);
    assert_eq!(v2.tag, v1.tag);
    assert_eq!(v2.data, v1.data);

    let back: WideV1 = exchanged(v2).await;
    assert_eq!(back, v1);
}

/// Enum variants decode by name: a payload encoded against one variant
/// order decodes into the reordered enum — struct-variant fields included —
/// in both directions.
#[tokio::test]
async fn reordered_enum_variants_decode_by_name() {
    let note = EventV1::Note {
        text: "urgent".to_string(),
        level: 3,
    };
    let got: EventV2 = exchanged(note).await;
    assert_eq!(
        got,
        EventV2::Note {
            level: 3,
            text: "urgent".to_string()
        }
    );

    let ping: EventV1 = exchanged(EventV2::Ping(99)).await;
    assert_eq!(ping, EventV1::Ping(99));

    let stop: EventV1 = exchanged(EventV2::Stop).await;
    assert_eq!(stop, EventV1::Stop);
}

/// A field the decoder does not know is skipped: a sender speaking a wider
/// struct interoperates with a receiver speaking a narrower one.
#[tokio::test]
async fn unknown_fields_are_skipped() {
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Narrow {
        id: u64,
    }
    let wide = WideV1 {
        id: 42,
        tag: "extra".to_string(),
        data: vec![9],
    };
    let narrow: Narrow = exchanged(wide).await;
    assert_eq!(narrow, Narrow { id: 42 });
}

/// A payload that does not decode as the receiving type fails the session
/// cleanly: bootstrapping a `Peer<u64>` from a `String`-payload donor
/// returns an error — never a panic, and never a misconstructed peer.
#[tokio::test]
async fn undecodable_payload_fails_bootstrap_cleanly() {
    let sender = Peer::<String>::seed().into_rumors();
    sender.send("not a number".to_string());

    let (mut near, mut far) = rumors::link::memory();
    let serve = sender.clone();
    let server = tokio::spawn(async move { serve.gossip(&mut far).await });

    let joined = Peer::<u64>::bootstrap().join(&mut near).await;
    assert!(joined.is_err(), "a String payload must not decode as u64");

    // Drop the failed side's link so the donor sees the transport close
    // (a peer that errored out of a session hangs up); its session then
    // fails too — but only ever as an error, never a panic.
    drop(near);
    let _ = server.await.expect("the serving task must not panic");
}

/// An established cross-typed pair degrades cleanly at the first
/// undecodable value: the receiving session fails as an error, never a
/// panic, moving nothing into the receiver's set.
///
/// `u64` payloads that fit `u32` interoperate (the bootstrap here rides
/// one); the first value that does not fit is the one that must fail.
#[tokio::test]
async fn out_of_range_payload_fails_gossip_cleanly() {
    let sender = Peer::<u64>::seed().into_rumors();
    sender.send(7u64);

    let (mut near, mut far) = rumors::link::memory();
    let serve = sender.clone();
    let server = tokio::spawn(async move { serve.gossip(&mut far).await });
    let receiver = Peer::<u32>::bootstrap()
        .join(&mut near)
        .await
        .expect("in-range values decode across the pair")
        .expect("the sender is established")
        .into_rumors();
    server
        .await
        .expect("the serving task must not panic")
        .expect("the in-range session succeeds");

    // A value only `u64` can hold: the next session must fail at the
    // receiver's ingress. The sender's half fails too (its counterparty
    // aborted), but only ever as an error.
    sender.send(u64::MAX);
    let (mut near, mut far) = rumors::link::memory();
    let serve = sender.clone();
    let server = tokio::spawn(async move { serve.gossip(&mut far).await });
    let gossiped = receiver.gossip(&mut near).await;
    assert!(gossiped.is_err(), "u64::MAX must not decode as u32");
    // Hang up the failed side so the sender's half sees the transport
    // close rather than blocking on an unread stream.
    drop(near);
    let _ = server.await.expect("the serving task must not panic");

    let snapshot = receiver.snapshot();
    let values: Vec<u32> = snapshot.iter().map(|(_, m)| *m).collect();
    assert_eq!(values, vec![7u32], "a failed session moves nothing");
}

/// A missing field errors without `#[serde(default)]` and fills with it:
/// the documented boundary between tolerated and rejected evolution.
#[test]
fn missing_fields_error_absent_a_default() {
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Narrow {
        id: u64,
    }
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Wide {
        id: u64,
        tag: String,
    }
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct WideDefaulted {
        id: u64,
        #[serde(default)]
        tag: String,
    }

    let mut narrow = Vec::new();
    ciborium::ser::into_writer(&Narrow { id: 5 }, &mut narrow).unwrap();

    // Without a default, the absent field is an error, not a guess.
    assert!(ciborium::de::from_reader::<Wide, _>(narrow.as_slice()).is_err());

    // With one, the absent field fills in.
    let filled: WideDefaulted = ciborium::de::from_reader(narrow.as_slice()).unwrap();
    assert_eq!(
        filled,
        WideDefaulted {
            id: 5,
            tag: String::new()
        }
    );
}
