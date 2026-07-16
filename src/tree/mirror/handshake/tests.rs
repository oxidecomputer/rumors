use tokio::io::{duplex, split};

use super::{Error, Intent, PREAMBLE_LEN, Preamble, Staged, preamble};
use crate::Network;

/// Construct a fully received preamble with one caller-selected intent byte.
fn staged(network: Network, intent: u8) -> Staged {
    let mut staged = Staged::new();
    staged.buf[..6].copy_from_slice(&crate::PROTOCOL_MAGIC);
    staged.buf[6..8].copy_from_slice(&crate::PROTOCOL_VERSION.to_be_bytes());
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
                left,
                Intent::Remain,
                &mut left_staged,
                &mut left_read,
                &mut left_write,
            ),
            preamble(
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
        match (byte, staged(network, byte).validate()) {
            (0, Ok(preamble)) => assert_eq!(preamble.intent, Intent::Remain),
            (1, Ok(preamble)) => assert_eq!(preamble.intent, Intent::Retire),
            (0 | 1, other) => panic!("defined intent {byte} was rejected: {other:?}"),
            (byte, Err(Error::IntentInvalid { byte: rejected })) => assert_eq!(rejected, byte),
            (_, other) => panic!("invalid intent produced the wrong result: {other:?}"),
        }
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
