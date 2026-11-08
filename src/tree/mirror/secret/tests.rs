//! The bootstrap's sealed secret: the items' framing, the seal's documented
//! suite and bindings, and the typed defect or transport error each failure
//! maps to.
//!
//! What HPKE itself guarantees (confidentiality, integrity, binding to the key
//! pair, and the public-key validation its deserialization makes) is the `hpke`
//! crate's to test, not this module's.

use hpke::{
    Deserializable, Kem as _, OpModeR, OpModeS, Serializable, aead::AesGcm256, kdf::HkdfSha256,
    kem::DhP256HkdfSha256, single_shot_open, single_shot_seal,
};
use proptest::prelude::*;

use super::{
    ENCAPPED_KEY_LEN, Joining, KEY_ITEM, KEY_ITEM_LEN, KEY_ITEM_PREFIX, PREFIX_LEN, PUBLIC_KEY_LEN,
    SEALED_ITEM, SEALED_ITEM_LEN, SEALED_ITEM_PREFIX, SEALED_LEN, SecretDefect, decode_key, seal,
    send,
};
use crate::{
    Error,
    error::{Phase, TransportOperation as Op},
    network::{NETWORK_BYTES, NetworkSecret},
    observe::SessionHandle,
    tags::{BOOTSTRAP_KEY_TAG, NETWORK_SECRET_TAG},
    tree::mirror::cbor::{self, MAJOR_BSTR},
};

// The suite the module documents, named from the `hpke` crate rather than
// imported from the module's aliases, so that the tests transcribing the
// seal through `hpke` directly disagree with any other suite the module
// would use.

/// The documented KEM: DHKEM(P-256, HKDF-SHA256).
type Kem = DhP256HkdfSha256;

/// The documented KDF: HKDF-SHA256.
type Kdf = HkdfSha256;

/// The documented AEAD: AES-256-GCM.
type Aead = AesGcm256;

/// A public key under the documented KEM.
type PublicKey = <Kem as hpke::Kem>::PublicKey;

/// An encapsulated key under the documented KEM.
type EncappedKey = <Kem as hpke::Kem>::EncappedKey;

/// The secret the fixed-input tests seal.
fn fixed_secret() -> NetworkSecret {
    NetworkSecret::from_bytes([0x5e; NETWORK_BYTES])
}

/// HPKE's `info` as the module documents it: the label `"rumors bootstrap
/// network secret"`, then protocol version 2 as a big-endian `u64`.
///
/// Both parts are spelled out as literals, so that a change to either in the
/// module is one these tests disagree with.
fn documented_info() -> Vec<u8> {
    [
        b"rumors bootstrap network secret".as_slice(),
        &2u64.to_be_bytes(),
    ]
    .concat()
}

/// Unwrap the typed defect of a rejected item, which belongs to the
/// identity-transfer phase.
fn defect<T>(result: Result<T, Error>) -> SecretDefect {
    match result {
        Err(Error::Protocol(error)) => {
            assert_eq!(error.context.phase, Phase::IdentityTransfer);
            *error
                .source
                .downcast::<SecretDefect>()
                .expect("secret item diagnostic")
        }
        Ok(_) => panic!("a rejected item must not decode"),
        Err(other) => panic!("a rejected item is a protocol violation, got {other:?}"),
    }
}

/// A fresh joiner, and `secret` sealed for it under its own network.
fn sealed_for(secret: &NetworkSecret) -> (Joining, [u8; SEALED_ITEM_LEN]) {
    let joining = Joining::new();
    let key = decode_key(&joining.item()).expect("a fresh key item decodes");
    let item = seal(secret, &key, secret.network());
    (joining, item)
}

/// The key item carrying `key`.
fn key_item(key: &[u8; PUBLIC_KEY_LEN]) -> [u8; KEY_ITEM_LEN] {
    [KEY_ITEM_PREFIX.as_slice(), key]
        .concat()
        .try_into()
        .expect("the key item's width")
}

/// Every copy of `item` with exactly one byte of its opening altered: each
/// position of the opening, changed by each nonzero difference.
fn with_one_prefix_byte_altered<const N: usize>(item: [u8; N]) -> impl Iterator<Item = [u8; N]> {
    (0..PREFIX_LEN).flat_map(move |at| {
        (1..=u8::MAX).map(move |difference| {
            let mut altered = item;
            altered[at] ^= difference;
            altered
        })
    })
}

/// Both items' hand-built openings are exactly what the shared CBOR writers
/// produce for their tags and byte-string lengths, so each item is one
/// well-formed CBOR item of its declared length.
#[test]
fn prefixes_match_the_writers() {
    for (prefix, tag, len, total) in [
        (
            KEY_ITEM_PREFIX,
            BOOTSTRAP_KEY_TAG,
            PUBLIC_KEY_LEN,
            KEY_ITEM_LEN,
        ),
        (
            SEALED_ITEM_PREFIX,
            NETWORK_SECRET_TAG,
            SEALED_LEN,
            SEALED_ITEM_LEN,
        ),
    ] {
        let mut expected = Vec::new();
        cbor::write_tag(&mut expected, tag);
        cbor::write_head(&mut expected, MAJOR_BSTR, len as u64);
        assert_eq!(prefix.as_slice(), expected.as_slice());
        assert_eq!(total, expected.len() + len);
    }
}

proptest! {
    /// A secret sent sealed is received whole by the joiner it was sealed
    /// for, through the wire path both sides run (`send`, then
    /// `Joining::receive`), and `send` writes exactly one sealed item.
    #[test]
    fn a_sealed_secret_round_trips(bytes in any::<[u8; NETWORK_BYTES]>()) {
        let secret = NetworkSecret::from_bytes(bytes);
        let joining = Joining::new();
        let key = decode_key(&joining.item()).expect("a fresh key item decodes");
        let mut wire = Vec::new();
        pollster::block_on(send(
            &secret,
            &key,
            secret.network(),
            &mut wire,
            &SessionHandle::default(),
        ))
        .expect("writing to memory succeeds");
        prop_assert_eq!(wire.len(), SEALED_ITEM_LEN);
        let opened = pollster::block_on(joining.receive(
            &mut wire.as_slice(),
            secret.network(),
            &SessionHandle::default(),
        ))
        .expect("the seal opens");
        prop_assert!(opened == secret);
    }
}

/// A key item whose opening has any one byte altered is a malformed key item,
/// and one carrying the all-zero string, which encodes no public key, is an
/// invalid key in the key item.
#[test]
fn malformed_and_invalid_key_items_are_typed_defects() {
    for altered in with_one_prefix_byte_altered(Joining::new().item()) {
        let rejected = defect(decode_key(&altered));
        assert!(
            matches!(rejected, SecretDefect::Malformed { item: KEY_ITEM }),
            "{rejected:?}"
        );
    }
    let rejected = defect(decode_key(&key_item(&[0; PUBLIC_KEY_LEN])));
    assert!(
        matches!(rejected, SecretDefect::InvalidKey { item: KEY_ITEM }),
        "{rejected:?}"
    );
}

/// A sealed item whose opening has any one byte altered is a malformed sealed
/// item, and one whose encapsulated key is the all-zero string is an invalid
/// key in the sealed item.
///
/// A sealed item that does not open, here by a flipped bit in its
/// authentication tag, is unauthentic.
#[test]
fn malformed_invalid_and_unauthentic_sealed_items_are_typed_defects() {
    let secret = fixed_secret();
    let (joining, item) = sealed_for(&secret);
    for altered in with_one_prefix_byte_altered(item) {
        let rejected = defect(joining.open(&altered, secret.network()));
        assert!(
            matches!(rejected, SecretDefect::Malformed { item: SEALED_ITEM }),
            "{rejected:?}"
        );
    }
    let mut zero_key = item;
    zero_key[PREFIX_LEN..PREFIX_LEN + ENCAPPED_KEY_LEN].fill(0);
    let rejected = defect(joining.open(&zero_key, secret.network()));
    assert!(
        matches!(rejected, SecretDefect::InvalidKey { item: SEALED_ITEM }),
        "{rejected:?}"
    );
    // The item ends with the authentication tag.
    let mut forged = item;
    forged[SEALED_ITEM_LEN - 1] ^= 0x01;
    let rejected = defect(joining.open(&forged, secret.network()));
    assert!(
        matches!(rejected, SecretDefect::Unauthentic),
        "{rejected:?}"
    );
}

/// The identifier check still runs after the seal opens: a provider that seals
/// one network's secret while declaring another fails as `WrongNetwork`, naming
/// both identifiers.
#[test]
fn a_secret_of_another_network_fails_the_identifier_check() {
    let sealed = NetworkSecret::from_bytes([1; NETWORK_BYTES]);
    let declared = NetworkSecret::from_bytes([2; NETWORK_BYTES]).network();
    let joining = Joining::new();
    let key = decode_key(&joining.item()).expect("a fresh key item decodes");
    let item = seal(&sealed, &key, declared);
    let SecretDefect::WrongNetwork {
        declared: named,
        received,
    } = defect(joining.open(&item, declared))
    else {
        panic!("expected a wrong-network rejection");
    };
    assert_eq!(named, declared);
    assert_eq!(received, sealed.network());
}

/// A stream that ends anywhere inside the sealed item is a transport truncation
/// of the read in the identity-transfer phase, never a decode or a hang.
///
/// The preamble exchange reads the key item, and its tests cover that item's
/// truncation.
#[test]
fn a_truncated_sealed_item_is_a_transport_truncation() {
    let secret = fixed_secret();
    let (_, sealed) = sealed_for(&secret);
    let truncation = |result: Result<(), Error>| {
        matches!(result, Err(Error::Transport(ref error))
            if error.source.kind() == std::io::ErrorKind::UnexpectedEof
                && error.operation == Op::Read
                && error.context.phase == Phase::IdentityTransfer)
    };
    for cut in 0..SEALED_ITEM_LEN {
        // Receiving consumes the joiner; the cut is detected before any key
        // is used, so a fresh one serves.
        let result = pollster::block_on(Joining::new().receive(
            &mut &sealed[..cut],
            secret.network(),
            &SessionHandle::default(),
        ));
        assert!(
            truncation(result.map(|_| ())),
            "a sealed item cut after {cut} bytes"
        );
    }
}

/// A write that fails while the provider sends the sealed item is a transport
/// error of the write in the identity-transfer phase, carrying the writer's
/// error.
#[test]
fn a_failed_send_is_a_transport_write_error() {
    let secret = fixed_secret();
    let joining = Joining::new();
    let key = decode_key(&joining.item()).expect("a fresh key item decodes");
    // The writer has room for all of the sealed item but its last byte, so
    // the write fails partway through, with `WriteZero`.
    let mut room = [0u8; SEALED_ITEM_LEN - 1];
    let result = pollster::block_on(send(
        &secret,
        &key,
        secret.network(),
        &mut std::io::Cursor::new(&mut room[..]),
        &SessionHandle::default(),
    ));
    assert!(
        matches!(result, Err(Error::Transport(ref error))
            if error.source.kind() == std::io::ErrorKind::WriteZero
                && error.operation == Op::Write
                && error.context.phase == Phase::IdentityTransfer),
        "{result:?}"
    );
}

/// The joiner opens by the documented suite and bindings: a sealed item built
/// by calling the `hpke` crate directly opens to the sealed secret.
///
/// The item is sealed under the documented suite, with the documented `info`
/// and the declared network as associated data.
#[test]
fn a_joiner_opens_a_seal_built_by_the_documented_bindings() {
    let secret = fixed_secret();
    let declared = secret.network();
    let joining = Joining::new();
    let joiner = PublicKey::from_bytes(&joining.public).expect("the joiner's key decodes");
    let (encapped, ciphertext) = single_shot_seal::<Aead, Kdf, Kem>(
        &OpModeS::Base,
        &joiner,
        &documented_info(),
        secret.as_bytes(),
        &declared.to_bytes(),
    )
    .expect("sealing succeeds");
    let item: [u8; SEALED_ITEM_LEN] = [
        SEALED_ITEM_PREFIX.as_slice(),
        &encapped.to_bytes(),
        &ciphertext,
    ]
    .concat()
    .try_into()
    .expect("the sealed item's width");
    let opened = joining
        .open(&item, declared)
        .expect("the documented seal opens");
    assert!(opened == secret);
}

/// The provider seals by the documented suite and bindings: its sealed item
/// opens, by calling the `hpke` crate directly, to the secret it sealed.
///
/// The item is opened under the documented suite, with the documented `info`
/// and the declared network as associated data.
#[test]
fn a_providers_seal_opens_by_the_documented_bindings() {
    let secret = fixed_secret();
    let declared = secret.network();
    let (private, public) = Kem::gen_keypair();
    let item = seal(
        &secret,
        &decode_key(&key_item(&public.to_bytes().into())).expect("the key item decodes"),
        declared,
    );
    let (encapped, ciphertext) = item[PREFIX_LEN..].split_at(ENCAPPED_KEY_LEN);
    let opened = single_shot_open::<Aead, Kdf, Kem>(
        &OpModeR::Base,
        &private,
        &EncappedKey::from_bytes(encapped).expect("the encapsulated key decodes"),
        &documented_info(),
        ciphertext,
        &declared.to_bytes(),
    )
    .expect("the provider's seal opens under the documented bindings");
    assert_eq!(opened.as_slice(), secret.as_bytes().as_slice());
}
