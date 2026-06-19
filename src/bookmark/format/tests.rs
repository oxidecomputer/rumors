//! The frame is self-inverse and self-checking: it round-trips any payload,
//! rejects every single-byte corruption, and pins byte-for-byte.

use std::collections::BTreeMap;

use before::Clock;
use proptest::prelude::*;

use super::*;
use crate::Network;

/// A fixed, non-trivial record: one network mapped to a seed clock and two of
/// its forks.
///
/// Deterministic — `Network::from_bytes` and `Clock::seed`/`fork` draw no
/// randomness — so anything derived from it (a snapshot, a hash) is stable
/// across runs.
fn sample_record() -> BTreeMap<Network, Vec<Clock>> {
    let network = Network::from_bytes([0x5a; 16]);
    let mut clock = Clock::seed();
    let first = clock.fork();
    let second = clock.fork();
    BTreeMap::from([(network, vec![clock, first, second])])
}

/// Two records are equal when their canonical borsh encodings are: a [`Clock`]
/// is `!Clone` and exposes no value equality, so the bytes are the oracle.
fn borsh_eq(a: &BTreeMap<Network, Vec<Clock>>, b: &BTreeMap<Network, Vec<Clock>>) -> bool {
    borsh::to_vec(a).unwrap() == borsh::to_vec(b).unwrap()
}

proptest! {
    /// Framing is invertible: `unframe` recovers exactly the bytes `frame`
    /// wrapped, for any payload.
    #[test]
    fn framing_round_trips(payload: Vec<u8>) {
        let framed = frame(&payload);
        prop_assert_eq!(unframe(&framed).unwrap(), payload.as_slice());
    }

    /// A frame always carries the magic and version tag in its header, whatever
    /// the payload.
    #[test]
    fn frame_carries_the_tag(payload: Vec<u8>) {
        let framed = frame(&payload);
        let version = BOOKMARK_FORMAT_VERSION.to_be_bytes();
        prop_assert!(framed.starts_with(&BOOKMARK_MAGIC));
        prop_assert_eq!(&framed[VERSION_OFFSET..HASH_OFFSET], version.as_slice());
    }

    /// Flipping any one byte of the frame body (magic, version, hash, or
    /// payload) makes it fail to validate: nothing corrupt is ever accepted.
    #[test]
    fn any_single_byte_corruption_is_rejected(
        payload in prop::collection::vec(any::<u8>(), 1..64),
        index: prop::sample::Index,
    ) {
        let mut framed = frame(&payload);
        let i = index.index(framed.len());
        framed[i] ^= 0xff;
        prop_assert!(unframe(&framed).is_err());
    }

    /// A record survives a serialize/validate/deserialize round trip unchanged,
    /// for an arbitrary number of forked clocks under an arbitrary network id.
    #[test]
    fn record_round_trips(network: [u8; 16], extra_forks in 0usize..12) {
        let mut clock = Clock::seed();
        let mut clocks: Vec<Clock> = Vec::new();
        for _ in 0..extra_forks {
            clocks.push(clock.fork());
        }
        clocks.push(clock);
        let record = BTreeMap::from([(Network::from_bytes(network), clocks)]);

        let decoded = decode(&encode(&record)).expect("a freshly encoded record decodes");
        prop_assert!(borsh_eq(&decoded, &record));
    }
}

/// An empty record round-trips to an empty record, distinct from "absent".
#[test]
fn empty_record_round_trips() {
    let empty = BTreeMap::new();
    let decoded = decode(&encode(&empty)).expect("the empty record decodes");
    assert!(decoded.is_empty());
}

/// Foreign leading bytes are rejected as [`FormatError::BadMagic`], not misread.
#[test]
fn foreign_magic_is_rejected() {
    let mut framed = encode(&sample_record());
    framed[0] ^= 0xff;
    assert!(matches!(
        unframe(&framed),
        Err(FormatError::BadMagic { .. })
    ));
}

/// A frame tagged with an unknown format version is rejected, never decoded
/// under this build's assumptions.
#[test]
fn unknown_version_is_rejected() {
    let mut framed = encode(&sample_record());
    framed[VERSION_OFFSET..HASH_OFFSET].copy_from_slice(&0xbeef_u16.to_be_bytes());
    assert!(matches!(
        unframe(&framed),
        Err(FormatError::VersionMismatch { found: 0xbeef }),
    ));
}

/// A frame whose payload no longer matches its stored hash is rejected as
/// corrupt.
#[test]
fn payload_corruption_is_rejected() {
    let mut framed = encode(&sample_record());
    let last = framed.len() - 1;
    framed[last] ^= 0xff;
    assert!(matches!(unframe(&framed), Err(FormatError::HashMismatch)));
}

/// Anything shorter than the fixed header — including an empty buffer — is
/// [`FormatError::Truncated`], never mistaken for an absent bookmark.
#[test]
fn short_input_is_truncated() {
    assert!(matches!(
        unframe(&[]),
        Err(FormatError::Truncated { len: 0 }),
    ));
    let framed = encode(&sample_record());
    assert!(matches!(
        unframe(&framed[..HEADER_LEN - 1]),
        Err(FormatError::Truncated { .. }),
    ));
}

/// The encoded empty record pins byte-for-byte: a header (magic, version,
/// integrity hash) over the borsh encoding of an empty map. A change here is a
/// deliberate on-disk format change, like the wire-format snapshots.
#[test]
fn pins_the_empty_frame() {
    insta::assert_snapshot!("frame_empty", hex::encode(encode(&BTreeMap::new())));
}

/// The encoded non-trivial record pins byte-for-byte, so format drift cannot
/// hide in a populated payload (multiple clocks under a network id) the way it
/// could in an empty one.
#[test]
fn pins_a_non_trivial_frame() {
    insta::assert_snapshot!("frame_non_trivial", hex::encode(encode(&sample_record())));
}
