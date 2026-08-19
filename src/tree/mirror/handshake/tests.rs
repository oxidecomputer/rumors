use proptest::prelude::*;
use tokio::io::{duplex, split};

use super::{Error, Intent, Preamble, Staged, V2_PREAMBLE_LEN, V2_PREFIX, preamble};
use crate::observe::SessionHandle;
use crate::{Network, Protocol};

/// Construct a fully received V2 preamble with one caller-selected raw
/// byte in the intent item's place.
fn staged(network: Network, intent: u8) -> Staged {
    let encoded = Preamble {
        network,
        intent: Intent::Remain,
    }
    .encode(Protocol::V2);
    let mut staged = Staged::new(Protocol::V2);
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
    cbor::write_tag(&mut prefix, 55799);
    cbor::write_head(&mut prefix, MAJOR_ARRAY, 4);
    cbor::write_head(&mut prefix, MAJOR_TEXT, "rumors".len() as u64);
    prefix.extend_from_slice(b"rumors");
    assert_eq!(prefix, V2_PREFIX);
}

/// Both sides exchange the shared preamble over a one-byte transport without
/// deadlock, preserving each peer's network and intent exactly.
#[test]
fn fragmented_exchange_is_symmetric() {
    for protocol in [Protocol::V1, Protocol::V2] {
        let left = Network::from_bytes([1; 16]);
        let right = Network::from_bytes([2; 16]);
        let (left_io, right_io) = duplex(1);
        let (left_read, left_write) = split(left_io);
        let (right_read, right_write) = split(right_io);
        let mut left_read = left_read;
        let mut left_write = left_write;
        let mut right_read = right_read;
        let mut right_write = right_write;
        let mut left_staged = Staged::new(protocol);
        let mut right_staged = Staged::new(protocol);

        let observe = SessionHandle::default();
        let (seen_by_left, seen_by_right) = pollster::block_on(async {
            tokio::join!(
                preamble(
                    protocol,
                    left,
                    Intent::Remain,
                    &mut left_staged,
                    &mut left_read,
                    &mut left_write,
                    &observe,
                ),
                preamble(
                    protocol,
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
            (0x18.., Err(Error::Malformed { .. })) => {}
            (_, other) => panic!("invalid intent {byte} produced the wrong result: {other:?}"),
        }
    }
}

/// A peer that closes the connection at any point inside the preamble
/// surfaces a typed error, never a hang and never a partial decode.
///
/// Every strict prefix of the fixed item is a structurally distinct
/// truncation, so the whole prefix space is swept in both dialects: zero
/// bytes is the clean-goodbye close, every longer prefix a mid-preamble
/// cut resolving to [`Error::Io`] with `UnexpectedEof` — except a V2
/// endpoint cut exactly where a whole legacy preamble ends, which is
/// diagnosed as the version mismatch it is.
#[test]
fn every_truncation_boundary_is_a_typed_eof() {
    for protocol in [Protocol::V1, Protocol::V2] {
        let network = Network::from_bytes([1; 16]);
        let full = Preamble {
            network,
            intent: Intent::Remain,
        }
        .encode(protocol);

        for cut in 0..full.len() {
            let mut staged = Staged::new(protocol);
            let mut reader = &full[..cut];
            let mut writer = tokio::io::sink();
            let result = pollster::block_on(preamble(
                protocol,
                network,
                Intent::Remain,
                &mut staged,
                &mut reader,
                &mut writer,
                &SessionHandle::default(),
            ));
            match result {
                Err(Error::Io(error)) => assert_eq!(
                    error.kind(),
                    std::io::ErrorKind::UnexpectedEof,
                    "cut after {cut} bytes must be an unexpected EOF",
                ),
                other => panic!("cut after {cut} bytes must be a typed I/O error, got {other:?}"),
            }
        }
    }
}

/// A V2 endpoint whose peer speaks the legacy dialect diagnoses the
/// version mismatch, not a bare cut or a foreign protocol.
///
/// The legacy 25-byte preamble ends five bytes short of the V2 item, and
/// its magic names the rumors protocol at version 1.
#[test]
fn legacy_peer_is_a_version_mismatch() {
    let network = Network::from_bytes([1; 16]);
    let legacy = Preamble {
        network,
        intent: Intent::Remain,
    }
    .encode(Protocol::V1);

    // The peer sent its whole legacy preamble and closed.
    let mut staged = Staged::new(Protocol::V2);
    let mut reader = legacy.as_slice();
    let mut writer = tokio::io::sink();
    let result = pollster::block_on(preamble(
        Protocol::V2,
        network,
        Intent::Remain,
        &mut staged,
        &mut reader,
        &mut writer,
        &SessionHandle::default(),
    ));
    assert!(
        matches!(
            result,
            Err(Error::VersionMismatch {
                local_protocol: Protocol::V2,
                remote_version: 1,
            })
        ),
        "expected the dialect diagnosis, got {result:?}",
    );

    // The peer's next five bytes (its greeting) arrived too: the full
    // 30-byte read then validates, and the magic check diagnoses the
    // dialect ahead of any structural complaint.
    let mut padded = legacy;
    padded.extend_from_slice(&[0; 5]);
    let mut staged = Staged::new(Protocol::V2);
    let mut reader = padded.as_slice();
    let mut writer = tokio::io::sink();
    let result = pollster::block_on(preamble(
        Protocol::V2,
        network,
        Intent::Remain,
        &mut staged,
        &mut reader,
        &mut writer,
        &SessionHandle::default(),
    ));
    assert!(matches!(
        result,
        Err(Error::VersionMismatch {
            local_protocol: Protocol::V2,
            remote_version: 1,
        })
    ));
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

        let result = Preamble::decode(&bytes, Protocol::V2);
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
        let _ = Preamble::decode(&bytes, Protocol::V2);
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
