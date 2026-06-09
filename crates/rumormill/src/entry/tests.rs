//! Wire-format tests for [`Entry`].
//!
//! The schema rides the rumors gossip protocol byte-for-byte, so its borsh
//! encoding is wire format: these tests pin both that every variant
//! round-trips and that the variant *order* (the enum discriminants) never
//! shifts under refactoring.

use rumors::borsh;

use super::*;

const ALICE: PeerId = [0xaa; 32];

fn samples() -> Vec<Entry> {
    vec![
        Entry::Chat {
            channel: "general".into(),
            author: ALICE,
            body: "hello".into(),
            sent_at: 1_000,
            ttl_ms: 300_000,
        },
        Entry::Presence {
            peer: ALICE,
            name: "alice".into(),
            at: 2_000,
        },
        Entry::Channel {
            name: "dogs".into(),
            created_by: ALICE,
            at: 3_000,
        },
        Entry::System {
            channel: "general".into(),
            body: "alice joined".into(),
            at: 4_000,
            ttl_ms: 15_000,
        },
    ]
}

/// Every variant survives a borsh round-trip unchanged.
#[test]
fn round_trip() {
    for entry in samples() {
        let bytes = borsh::to_vec(&entry).unwrap();
        let back: Entry = borsh::from_slice(&bytes).unwrap();
        assert_eq!(entry, back);
    }
}

/// The first encoded byte of each variant is its discriminant, in
/// declaration order. If this test fails, a refactor reordered or removed a
/// variant: that is a wire-format break, not a cleanup.
#[test]
fn variant_order_is_wire_format() {
    let discriminants: Vec<u8> = samples()
        .iter()
        .map(|e| borsh::to_vec(e).unwrap()[0])
        .collect();
    assert_eq!(discriminants, vec![0, 1, 2, 3]);
}

/// The full encoding of a representative entry, byte for byte. Catches any
/// silent change to field order, field types, or borsh configuration.
#[test]
fn chat_encoding_snapshot() {
    let entry = Entry::Chat {
        channel: "c".into(),
        author: [1; 32],
        body: "x".into(),
        sent_at: 2,
        ttl_ms: 3,
    };
    let bytes = borsh::to_vec(&entry).unwrap();
    let expected = {
        let mut v = vec![0u8]; // discriminant: Chat
        v.extend_from_slice(&1u32.to_le_bytes()); // channel length
        v.push(b'c');
        v.extend_from_slice(&[1; 32]); // author
        v.extend_from_slice(&1u32.to_le_bytes()); // body length
        v.push(b'x');
        v.extend_from_slice(&2u64.to_le_bytes()); // sent_at
        v.extend_from_slice(&3u64.to_le_bytes()); // ttl_ms
        v
    };
    assert_eq!(bytes, expected);
}

/// `expires_at` is `sent_at + ttl` for ephemeral entries, `None` for durable
/// and supersession-managed ones, and saturates rather than overflowing.
#[test]
fn expires_at_policy() {
    let [chat, presence, channel, system]: [Entry; 4] = samples().try_into().unwrap();
    assert_eq!(chat.expires_at(), Some(301_000));
    assert_eq!(system.expires_at(), Some(19_000));
    assert_eq!(presence.expires_at(), None);
    assert_eq!(channel.expires_at(), None);

    let forever = Entry::Chat {
        channel: String::new(),
        author: ALICE,
        body: String::new(),
        sent_at: u64::MAX,
        ttl_ms: u64::MAX,
    };
    assert_eq!(forever.expires_at(), Some(u64::MAX));
}
