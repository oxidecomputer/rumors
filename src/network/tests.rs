//! Tests for network secrets and for the key schedule that derives their
//! path keys and public identifiers.

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

/// The key schedule's two outputs for `secret`: the path key and the
/// network identifier, in that order.
///
/// They are transcribed from RFC 5869 rather than built through the `hkdf`
/// crate or the module's labels: the extract step is HMAC-SHA-256 keyed by
/// the default salt of 32 zero bytes over the secret, and each output fits
/// one expand block, HMAC-SHA-256 keyed by that pseudorandom key over the
/// info label and the block counter `0x01`.
fn transcribed_schedule(secret: &[u8; NETWORK_BYTES]) -> ([u8; 32], [u8; NETWORK_BYTES]) {
    let prk = hmac(&[0; 32], secret);
    let id = hmac(&prk, b"rumors network id\x01");
    (
        hmac(&prk, b"rumors path key\x01"),
        id[..NETWORK_BYTES]
            .try_into()
            .expect("an identifier is a prefix of one block"),
    )
}

proptest! {
    /// A network's path key and identifier are what RFC 5869's HKDF-SHA-256
    /// derives from its secret under the documented labels, and the two
    /// outputs differ.
    ///
    /// The path key is checked through what it computes: a leaf path is
    /// HMAC-SHA-256 under the path key.
    #[test]
    fn the_key_schedule_follows_rfc_5869(
        secret in any::<[u8; NETWORK_BYTES]>(),
        message in any::<Vec<u8>>(),
    ) {
        let (path_key, id) = transcribed_schedule(&secret);
        prop_assert_ne!(&path_key[..NETWORK_BYTES], &id[..]);

        let secret = NetworkSecret::from_bytes(secret);
        prop_assert_eq!(secret.network().to_bytes(), id);
        let path: [u8; 32] = secret.path_key().hash(&message).into();
        prop_assert_eq!(path, hmac(&path_key, &message));
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
