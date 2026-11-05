//! Tests for network secrets and for the key schedule that derives their
//! public identifiers.

use hmac::{Hmac, KeyInit, Mac};
use proptest::prelude::*;
use sha2::Sha256;

use super::{NETWORK_BYTES, NetworkSecret};

/// HMAC-SHA-256 of `message` under `key`, through the `hmac` crate's
/// one-shot interface.
fn hmac(key: &[u8], message: &[u8]) -> [u8; 32] {
    <Hmac<Sha256>>::new_from_slice(key)
        .expect("HMAC accepts a key of any length")
        .chain_update(message)
        .finalize()
        .into_bytes()
        .into()
}

/// The network identifier the key schedule derives from `secret`.
///
/// It is transcribed from RFC 5869 rather than built through the `hkdf`
/// crate or the module's label: the extract step is HMAC-SHA-256 keyed by
/// the default salt of 32 zero bytes over the secret, and the identifier
/// fits one expand block, HMAC-SHA-256 keyed by that pseudorandom key over
/// the info label and the block counter `0x01`.
fn transcribed_identifier(secret: &[u8; NETWORK_BYTES]) -> [u8; NETWORK_BYTES] {
    let prk = hmac(&[0; 32], secret);
    hmac(&prk, b"rumors network id\x01")[..NETWORK_BYTES]
        .try_into()
        .expect("an identifier is a prefix of one block")
}

proptest! {
    /// A network's identifier is what RFC 5869's HKDF-SHA-256 derives from
    /// its secret under the documented label.
    #[test]
    fn the_identifier_follows_rfc_5869(secret in any::<[u8; NETWORK_BYTES]>()) {
        prop_assert_eq!(
            NetworkSecret::from_bytes(secret).network().to_bytes(),
            transcribed_identifier(&secret)
        );
    }

    /// Two secrets give the same identifier exactly when they are the same
    /// secret.
    #[test]
    fn identifiers_tell_secrets_apart(
        a in any::<[u8; NETWORK_BYTES]>(),
        b in any::<[u8; NETWORK_BYTES]>(),
    ) {
        prop_assert_eq!(
            a == b,
            NetworkSecret::from_bytes(a).network() == NetworkSecret::from_bytes(b).network()
        );
    }

    /// A secret's `Debug` output names the type and nothing else: neither
    /// its hex, nor its bytes in Rust's array form, appears in it.
    #[test]
    fn secret_debug_reveals_nothing(bytes in any::<[u8; NETWORK_BYTES]>()) {
        let rendered = format!("{:?} {:#?}", NetworkSecret::from_bytes(bytes), NetworkSecret::from_bytes(bytes));
        prop_assert_eq!(rendered, "NetworkSecret(..) NetworkSecret(..)");
    }
}

/// Zeroizing a secret, which is what dropping one runs, clears every byte.
///
/// The drop itself cannot be observed: reading a value's memory after it
/// drops takes `unsafe`, which the crate forbids. So this exercises the
/// clearing the `Drop` impl calls, on a live value.
#[test]
fn zeroizing_clears_every_byte_of_the_secret() {
    use zeroize::Zeroize;
    let mut secret = NetworkSecret::from_bytes([0xa5; NETWORK_BYTES]);
    secret.zeroize();
    assert_eq!(secret.as_bytes(), &[0; NETWORK_BYTES]);
}
