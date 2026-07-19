use proptest::prelude::*;
use tokio::io::{duplex, split};

use super::{Error, Intent, PREAMBLE_LEN, Preamble, Staged, preamble};
use crate::{Network, Protocol};

/// Construct a fully received preamble with one caller-selected intent byte.
fn staged(network: Network, intent: u8) -> Staged {
    let mut staged = Staged::new();
    staged.buf[..6].copy_from_slice(&crate::PROTOCOL_MAGIC);
    staged.buf[6..8].copy_from_slice(&(Protocol::V2 as u16).to_be_bytes());
    staged.buf[8..24].copy_from_slice(&network.to_bytes());
    staged.buf[24] = intent;
    staged.filled = PREAMBLE_LEN;
    staged
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

    let (seen_by_left, seen_by_right) = pollster::block_on(async {
        tokio::join!(
            preamble(
                Protocol::V2,
                left,
                Intent::Remain,
                &mut left_staged,
                &mut left_read,
                &mut left_write,
            ),
            preamble(
                Protocol::V2,
                right,
                Intent::Retire,
                &mut right_staged,
                &mut right_read,
                &mut right_write,
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

/// Intent decoding is exhaustive: exactly the two defined bytes are accepted
/// for an established network and every other byte retains its typed value.
#[test]
fn intent_byte_space_is_exhaustive() {
    let network = Network::from_bytes([1; 16]);
    for byte in u8::MIN..=u8::MAX {
        match (byte, staged(network, byte).validate(Protocol::V2)) {
            (0, Ok(preamble)) => assert_eq!(preamble.intent, Intent::Remain),
            (1, Ok(preamble)) => assert_eq!(preamble.intent, Intent::Retire),
            (0 | 1, other) => panic!("defined intent {byte} was rejected: {other:?}"),
            (byte, Err(Error::IntentInvalid { byte: rejected })) => assert_eq!(rejected, byte),
            (_, other) => panic!("invalid intent produced the wrong result: {other:?}"),
        }
    }
}

/// A peer that closes the connection at any point inside the preamble
/// surfaces a typed I/O error, never a hang and never a partial decode.
///
/// Every strict prefix of the 25-byte frame is a structurally distinct
/// truncation (the boundaries between magic, version, network, and intent
/// included), so the whole prefix space is swept: zero bytes is the
/// clean-goodbye close, every longer prefix a mid-preamble cut, and both
/// must resolve to [`Error::Io`] with `UnexpectedEof`.
#[test]
fn every_truncation_boundary_is_a_typed_eof() {
    let network = Network::from_bytes([1; 16]);
    let full = Preamble {
        network,
        intent: Intent::Remain,
    }
    .encode(Protocol::V2);

    for cut in 0..full.len() {
        let mut staged = Staged::new();
        let mut reader = &full[..cut];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(preamble(
            Protocol::V2,
            network,
            Intent::Remain,
            &mut staged,
            &mut reader,
            &mut writer,
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

/// A wrong magic is diagnosed first, before any other field is judged.
///
/// The frame here is wrong in every field — magic, version, and intent —
/// and must still surface [`Error::MagicMismatch`] carrying the exact
/// remote bytes: the diagnostic order promised by the module docs puts
/// "not a rumors stream" ahead of "wrong dialect".
#[test]
fn magic_mismatch_is_diagnosed_first() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0xFF);
    wrong.buf[..6].copy_from_slice(b"SROMUR");
    wrong.buf[6..8].copy_from_slice(&0xFFFF_u16.to_be_bytes());

    let result = wrong.validate(Protocol::V2);
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
/// With a correct magic but a foreign version, the frame's (invalid)
/// intent byte must never be reached: the typed rejection is
/// [`Error::VersionMismatch`] carrying the remote's declared version, so a
/// dialect skew is reported as such rather than as a garbled body.
#[test]
fn version_mismatch_is_diagnosed_before_intent() {
    let mut wrong = staged(Network::from_bytes([1; 16]), 0xFF);
    wrong.buf[6..8].copy_from_slice(&7_u16.to_be_bytes());

    let result = wrong.validate(Protocol::V2);
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
    /// Any complete 25-byte preamble decodes exactly as the field-by-field
    /// oracle predicts: a typed error naming the first invalid field in
    /// diagnostic order, or the valid preamble — never a panic.
    ///
    /// The strategy weights the magic and version toward their valid values
    /// so the deeper fields' arms are actually reached; the oracle
    /// recomputes the documented diagnosis order (magic, then version, then
    /// intent, then the network/intent combination) independently of the
    /// decoder.
    #[test]
    fn arbitrary_preamble_decodes_by_the_oracle(
        magic in prop_oneof![Just(crate::PROTOCOL_MAGIC), any::<[u8; 6]>()],
        version in prop_oneof![Just(Protocol::V2 as u16), any::<u16>()],
        network in any::<[u8; 16]>(),
        intent in prop_oneof![0_u8..=3, any::<u8>()],
    ) {
        let mut bytes = [0u8; PREAMBLE_LEN];
        bytes[..6].copy_from_slice(&magic);
        bytes[6..8].copy_from_slice(&version.to_be_bytes());
        bytes[8..24].copy_from_slice(&network);
        bytes[24] = intent;

        let result = Preamble::decode(&bytes, Protocol::V2);
        let as_oracle = if magic != crate::PROTOCOL_MAGIC {
            matches!(
                &result,
                Err(Error::MagicMismatch { remote_magic }) if *remote_magic == magic,
            )
        } else if version != Protocol::V2 as u16 {
            matches!(
                &result,
                Err(Error::VersionMismatch { remote_version, .. }) if *remote_version == version,
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
}

/// The bootstrap placeholder composes only with remain intent; retirement
/// would promise both receiving and donating an identity in one session.
#[test]
fn bootstrap_intent_matrix_is_exhaustive() {
    assert_eq!(
        staged(Network::BOOTSTRAP, 0)
            .validate(Protocol::V2)
            .unwrap(),
        Preamble {
            network: Network::BOOTSTRAP,
            intent: Intent::Remain,
        }
    );
    assert!(matches!(
        staged(Network::BOOTSTRAP, 1).validate(Protocol::V2),
        Err(Error::BootstrapRetireConflict)
    ));
}
