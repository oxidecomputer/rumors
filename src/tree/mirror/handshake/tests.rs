use proptest::prelude::*;
use tokio::io::{duplex, split};

use super::{
    Error, Intent, Preamble, PreambleDefect, Staged, V2_PREAMBLE_LEN, V2_PREFIX, preamble,
};
use crate::observe::SessionHandle;
use crate::{Network, Protocol};

/// Construct a fully received V2 preamble with one caller-selected raw
/// byte in the intent item's place.
fn staged(network: Network, intent: u8) -> Staged {
    let encoded = Preamble {
        network,
        intent: Intent::Remain,
    }
    .encode();
    let mut staged = Staged::new();
    staged.buf[..encoded.len()].copy_from_slice(&encoded);
    staged.buf[V2_PREAMBLE_LEN - 1] = intent;
    staged.filled = V2_PREAMBLE_LEN;
    staged
}

/// The V2 preamble's pinned prefix literal is exactly what the head
/// writers render for the self-described tag, the four-item array, and
/// the text `"rumors"`: the validation constant cannot drift from the
/// encoder.
#[test]
fn prefix_matches_the_writers() {
    use crate::tree::mirror::cbor::{self, MAJOR_ARRAY, MAJOR_TEXT};
    let mut prefix = Vec::new();
    cbor::write_tag(&mut prefix, cbor::TAG_SELF_DESCRIBED);
    cbor::write_head(&mut prefix, MAJOR_ARRAY, 4);
    cbor::write_head(&mut prefix, MAJOR_TEXT, "rumors".len() as u64);
    prefix.extend_from_slice(b"rumors");
    assert_eq!(prefix, V2_PREFIX);
}

/// Both sides exchange the shared preamble over a one-byte transport without
/// deadlock, preserving each peer's network and intent exactly.
#[test]
fn fragmented_exchange_is_symmetric() {
    let left = Network::from_bytes([1; 16]);
    let right = Network::from_bytes([2; 16]);
    let (left_io, right_io) = duplex(1);
    let (left_read, left_write) = split(left_io);
    let (right_read, right_write) = split(right_io);
    let mut left_read = left_read;
    let mut left_write = left_write;
    let mut right_read = right_read;
    let mut right_write = right_write;
    let mut left_staged = Staged::new();
    let mut right_staged = Staged::new();

    let observe = SessionHandle::default();
    let (seen_by_left, seen_by_right) = pollster::block_on(async {
        tokio::join!(
            preamble(
                left,
                Intent::Remain,
                &mut left_staged,
                &mut left_read,
                &mut left_write,
                &observe,
            ),
            preamble(
                right,
                Intent::Retire,
                &mut right_staged,
                &mut right_read,
                &mut right_write,
                &observe,
            ),
        )
    });

    assert_eq!(
        seen_by_left.unwrap(),
        Preamble {
            network: right,
            intent: Intent::Retire,
        }
    );
    assert_eq!(
        seen_by_right.unwrap(),
        Preamble {
            network: left,
            intent: Intent::Remain,
        }
    );
}

/// Intent decoding is exhaustive over the raw byte in the intent item's
/// place.
///
/// The two defined values are accepted, other small uint items are the
/// typed intent rejection, and bytes that are no one-byte uint item at
/// all are the malformed-preamble class.
#[test]
fn intent_byte_space_is_exhaustive() {
    let network = Network::from_bytes([1; 16]);
    for byte in u8::MIN..=u8::MAX {
        match (byte, staged(network, byte).validate()) {
            (0, Ok(preamble)) => assert_eq!(preamble.intent, Intent::Remain),
            (1, Ok(preamble)) => assert_eq!(preamble.intent, Intent::Retire),
            (0 | 1, other) => panic!("defined intent {byte} was rejected: {other:?}"),
            (2..=0x17, Err(Error::IntentInvalid { byte: rejected })) => {
                assert_eq!(rejected, byte);
            }
            (
                0x18..,
                Err(Error::Malformed {
                    defect: PreambleDefect::Intent,
                }),
            ) => {}
            (_, other) => panic!("invalid intent {byte} produced the wrong result: {other:?}"),
        }
    }
}

/// A peer that closes the connection at any point inside the preamble
/// surfaces a typed truncation, never a hang and never a partial decode.
///
/// Every strict prefix of the fixed item is a structurally distinct
/// truncation, so the whole prefix space is swept, each cut resolving to
/// [`Error::Truncated`] carrying the exact byte counts of the cut.
#[test]
fn every_truncation_boundary_is_typed() {
    let network = Network::from_bytes([1; 16]);
    let full = Preamble {
        network,
        intent: Intent::Remain,
    }
    .encode();

    for cut in 0..full.len() {
        let mut staged = Staged::new();
        let mut reader = &full[..cut];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(preamble(
            network,
            Intent::Remain,
            &mut staged,
            &mut reader,
            &mut writer,
            &SessionHandle::default(),
        ));
        match result {
            Err(Error::Truncated { received, expected }) => {
                assert_eq!(received, cut, "the truncation reports the cut point");
                assert_eq!(
                    expected,
                    full.len(),
                    "the truncation reports the preamble's full width"
                );
            }
            other => {
                panic!("cut after {cut} bytes must be a typed truncation, got {other:?}")
            }
        }
    }
}

/// A wrong magic is diagnosed first, before any other field is judged.
///
/// The item here is wrong in every field and must still surface
/// [`Error::MagicMismatch`] carrying the leading remote bytes: the
/// diagnostic order promised by the module docs puts "not a rumors
/// stream" ahead of "wrong dialect".
#[test]
fn magic_mismatch_is_diagnosed_first() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0xFF);
    wrong.buf[..6].copy_from_slice(b"SROMUR");

    let result = wrong.validate();
    assert!(
        matches!(
            &result,
            Err(Error::MagicMismatch { remote_magic }) if remote_magic == b"SROMUR",
        ),
        "expected the magic's typed rejection, got {result:?}",
    );
}

/// A wrong wire version is diagnosed before the semantic fields.
///
/// With a correct opening but a foreign version, the item's (invalid)
/// intent must never be reached: the typed rejection is
/// [`Error::VersionMismatch`] carrying the remote's declared version, so a
/// dialect skew is reported as such rather than as a garbled body.
#[test]
fn version_mismatch_is_diagnosed_before_intent() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0xFF);
    wrong.buf[V2_PREFIX.len()] = 0x07;

    let result = wrong.validate();
    assert!(
        matches!(
            result,
            Err(Error::VersionMismatch {
                remote_version: 7,
                local_protocol: Protocol::V2,
            }),
        ),
        "expected the version's typed rejection",
    );
}

proptest! {
    /// Any complete V2 preamble whose fields are canonically spelled
    /// decodes exactly as the field-by-field oracle predicts.
    ///
    /// The prediction: a typed error naming the first invalid field in
    /// diagnostic order, or the valid preamble — never a panic.
    #[test]
    fn arbitrary_preamble_decodes_by_the_oracle(
        magic_valid in prop_oneof![Just(true), any::<bool>()],
        version in prop_oneof![Just(Protocol::V2 as u8), 0_u8..=0x17],
        network in any::<[u8; 16]>(),
        intent in prop_oneof![0_u8..=3, 0_u8..=0x17],
    ) {
        let mut bytes = Vec::with_capacity(V2_PREAMBLE_LEN);
        if magic_valid {
            bytes.extend_from_slice(&V2_PREFIX);
        } else {
            bytes.extend_from_slice(b"SROMURxxxxx");
        }
        bytes.push(version);
        bytes.push(0x50);
        bytes.extend_from_slice(&network);
        bytes.push(intent);

        let result = Preamble::decode(&bytes);
        let as_oracle = if !magic_valid {
            matches!(&result, Err(Error::MagicMismatch { remote_magic }) if remote_magic == b"SROMUR")
        } else if version != Protocol::V2 as u8 {
            matches!(
                &result,
                Err(Error::VersionMismatch { remote_version, .. })
                    if *remote_version == u64::from(version),
            )
        } else if intent > 1 {
            matches!(&result, Err(Error::IntentInvalid { byte }) if *byte == intent)
        } else if network == [0; 16] && intent == 1 {
            matches!(&result, Err(Error::BootstrapRetireConflict))
        } else {
            let expected = Preamble {
                network: Network::from_bytes(network),
                intent: if intent == 0 { Intent::Remain } else { Intent::Retire },
            };
            matches!(&result, Ok(preamble) if *preamble == expected)
        };
        prop_assert!(as_oracle, "decode disagreed with the oracle: {:?}", result);
    }

    /// Arbitrary bytes in the preamble's place decode to a typed error or
    /// a valid preamble, never a panic: the parser is total over its
    /// fixed-width input.
    #[test]
    fn arbitrary_bytes_never_panic(bytes in any::<[u8; V2_PREAMBLE_LEN]>()) {
        let _ = Preamble::decode(&bytes);
    }
}

/// The bootstrap placeholder composes only with remain intent; retirement
/// would promise both receiving and donating an identity in one session.
#[test]
fn bootstrap_intent_matrix_is_exhaustive() {
    assert_eq!(
        staged(Network::BOOTSTRAP, 0).validate().unwrap(),
        Preamble {
            network: Network::BOOTSTRAP,
            intent: Intent::Remain,
        }
    );
    assert!(matches!(
        staged(Network::BOOTSTRAP, 1).validate(),
        Err(Error::BootstrapRetireConflict)
    ));
}

// Defensive-variant exemption: `PreambleDefect::NetworkTruncated` and
// `PreambleDefect::TrailingBytes` deliberately have no construction tests.
// In the fixed 30-byte V2 preamble, a validated version and network head
// always leave exactly 17 bytes -- the 16 network bytes and the one-byte
// intent -- so neither arm is reachable from any input the dialect admits;
// both guard the decoder's width arithmetic. Every reachable defect
// (`Version`, `Network`, `Intent`) has a construction: `Intent` in
// `intent_byte_space_is_exhaustive`, the other two below.

/// A version item that is not an unsigned int is the typed version
/// defect: a negative-int head in the version item's place fails the
/// major-type filter, and the defect names the version field.
#[test]
fn version_item_wrong_major_is_the_version_defect() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0);
    // 0x38: a two-byte negative-int head; its argument byte (the network
    // head behind it) parses, so the head is well-formed but the wrong
    // major type.
    wrong.buf[V2_PREFIX.len()] = 0x38;
    let result = wrong.validate();
    assert!(
        matches!(
            result,
            Err(Error::Malformed {
                defect: PreambleDefect::Version,
            }),
        ),
        "expected the version defect, got {result:?}",
    );
}

/// A widened spelling of the correct version value is the typed version
/// defect.
///
/// The wire admits one spelling per value, so `0x18 0x02` (a two-byte
/// head for 2) is rejected as non-canonical before its value is compared
/// against the dialect.
#[test]
fn widened_version_spelling_is_the_version_defect() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0);
    wrong.buf[V2_PREFIX.len()..V2_PREFIX.len() + 2].copy_from_slice(&[0x18, 0x02]);
    let result = wrong.validate();
    assert!(
        matches!(
            result,
            Err(Error::Malformed {
                defect: PreambleDefect::Version,
            }),
        ),
        "expected the version defect, got {result:?}",
    );
}

/// A network item that is not a 16-byte byte string is the typed network
/// defect: a byte-string head declaring 17 bytes fails the length filter,
/// and the defect names the network field.
#[test]
fn network_item_wrong_length_is_the_network_defect() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0);
    wrong.buf[V2_PREFIX.len() + 1] = 0x51;
    let result = wrong.validate();
    assert!(
        matches!(
            result,
            Err(Error::Malformed {
                defect: PreambleDefect::Network,
            }),
        ),
        "expected the network defect, got {result:?}",
    );
}
