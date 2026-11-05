//! The sealed exchange by which a provider hands a bootstrapping peer its
//! network's secret.
//!
//! A peer that joins a network must come away holding the network's secret
//! (`NetworkSecret`), and the secret must never cross the wire in the
//! clear: the transport is assumed authenticated, not confidential, and an
//! observer of the transport is in the crate's model. So the provider seals
//! the secret to a key pair the joiner draws for this one session, with
//! HPKE (RFC 9180), the IETF's standard public-key encryption:
//!
//! 1. The joiner draws an HPKE key pair and sends its public key in the
//!    *key item*, `BOOTSTRAP_KEY_TAG(bstr(public key))`, in the same write as
//!    its preamble, so the exchange adds no round trip. It sends the item
//!    before it knows whether its counterparty provides; the preamble
//!    exchange reads the key item of any counterparty whose preamble
//!    declares bootstrapping, and one that is bootstrapping too discards
//!    it.
//! 2. The provider, once the preambles show its counterparty is
//!    bootstrapping, reads and validates that key, and seals the secret to it
//!    with HPKE's single-shot encryption in base mode. It sends the *sealed
//!    item*, `NETWORK_SECRET_TAG(bstr(encapsulated key ‖ ciphertext ‖
//!    tag))`, right after the preambles and ahead of its greeting.
//! 3. The joiner opens the seal with its private key, and accepts the secret
//!    only if its identifier is the network the provider's preamble
//!    declared.
//!
//! # Ciphersuite and bindings
//!
//! The ciphersuite is one RFC 9180 registers: the KEM DHKEM(P-256,
//! HKDF-SHA256), the KDF HKDF-SHA256, and the AEAD AES-256-GCM. Public keys
//! and the encapsulated key are uncompressed SEC1 points, as RFC 9180
//! encodes them.
//!
//! HPKE's `info` input holds the domain label `"rumors bootstrap network
//! secret"` and the protocol version, and its associated data holds the
//! network identifier the provider's preamble declared. A seal therefore
//! opens only in a bootstrap of this protocol version that declares the
//! same network.
//!
//! # What HPKE guarantees here
//!
//! - **Confidentiality.** Base mode is IND-CCA2 secure (RFC 9180, section
//!   9.1): without the joiner's private key, an observer of both items
//!   learns nothing about the secret.
//! - **Integrity.** A sealed item that was altered in any way fails to open,
//!   so the secret's protection does not rest on the transport's integrity
//!   either.
//! - **Binding.** The KEM binds the encapsulated key and the joiner's public
//!   key into the key schedule, and the key schedule binds the suite and the
//!   `info`, so a seal opens only for the key pair it was sealed to, under
//!   these bindings.
//!
//! Base mode does not authenticate the sender: anyone holding the joiner's
//! public key could seal something to it. The transport's authentication
//! identifies the provider, which is the only peer the joiner reads a
//! sealed item from.
//!
//! Every primitive here is aligned with a FIPS algorithm standard, as all
//! of the crate's are (the `network` module lists them): SP 800-56A ECDH
//! over P-256 inside the KEM, SP 800-56C HKDF-SHA-256, and SP 800-38D
//! AES-256-GCM. A validated deployment would swap in validated
//! implementations and an approved random source, without changing the
//! protocol.
//!
//! Each item is one tagged byte string whose length the reader checks, so a
//! later dialect can extend an item's contents by changing its declared
//! length and layout here alone.

use hpke::{
    Deserializable, Kem as _, OpModeR, OpModeS, Serializable,
    aead::{AeadTag, AesGcm256},
    hybrid_array::typenum::Unsigned,
    kdf::HkdfSha256,
    kem::DhP256HkdfSha256,
    single_shot_open_inout_detached, single_shot_seal_inout_detached,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zeroize::Zeroizing;

use crate::{
    Error, Network, Protocol,
    error::{Phase, TransportOperation as Op},
    network::{NETWORK_BYTES, NetworkSecret},
    observe::SessionHandle,
    tags::{BOOTSTRAP_KEY_TAG, NETWORK_SECRET_TAG},
    tree::mirror::cbor::{self, MAJOR_BSTR},
};

/// The HPKE KEM: DHKEM(P-256, HKDF-SHA256).
type Kem = DhP256HkdfSha256;

/// The HPKE KDF: HKDF-SHA256.
type Kdf = HkdfSha256;

/// The HPKE AEAD: AES-256-GCM.
type Aead = AesGcm256;

/// A joiner's HPKE public key.
type PublicKey = <Kem as hpke::Kem>::PublicKey;

/// The KEM's encapsulated key, which the sealed item carries.
type EncappedKey = <Kem as hpke::Kem>::EncappedKey;

/// Bytes in a serialized public key: an uncompressed SEC1 P-256 point.
const PUBLIC_KEY_LEN: usize = <PublicKey as Serializable>::OutputSize::USIZE;

/// Bytes in a serialized encapsulated key, which is the provider's
/// ephemeral public key.
const ENCAPPED_KEY_LEN: usize = <EncappedKey as Serializable>::OutputSize::USIZE;

/// Bytes in the AEAD's authentication tag.
const TAG_LEN: usize = <AeadTag<Aead> as Serializable>::OutputSize::USIZE;

/// Bytes in the opening of either item: a tag head with a two-byte argument,
/// then the head of a byte string between 24 and 255 bytes long.
const PREFIX_LEN: usize = 3 + 2;

/// The opening of an item: `tag`'s head, then the head of a `len`-byte byte
/// string.
const fn item_prefix(tag: u64, len: usize) -> [u8; PREFIX_LEN] {
    assert!(
        tag > u8::MAX as u64 && tag <= u16::MAX as u64,
        "a two-byte tag argument"
    );
    assert!(
        len >= 24 && len <= u8::MAX as usize,
        "a one-byte length argument"
    );
    let tag = tag.to_be_bytes();
    let len = len.to_be_bytes();
    [
        // The tag major type with additional information 25: a two-byte
        // argument follows.
        (cbor::MAJOR_TAG << 5) | 25,
        tag[6],
        tag[7],
        // The byte-string major type with additional information 24: a
        // one-byte length follows.
        (MAJOR_BSTR << 5) | 24,
        len[len.len() - 1],
    ]
}

/// Bytes of the sealed item's byte string: the encapsulated key, the sealed
/// secret, and its authentication tag.
const SEALED_LEN: usize = ENCAPPED_KEY_LEN + NETWORK_BYTES + TAG_LEN;

/// The key item's opening.
const KEY_ITEM_PREFIX: [u8; PREFIX_LEN] = item_prefix(BOOTSTRAP_KEY_TAG, PUBLIC_KEY_LEN);

/// The sealed item's opening.
const SEALED_ITEM_PREFIX: [u8; PREFIX_LEN] = item_prefix(NETWORK_SECRET_TAG, SEALED_LEN);

/// Bytes in the complete key item.
pub(crate) const KEY_ITEM_LEN: usize = PREFIX_LEN + PUBLIC_KEY_LEN;

/// Bytes in the complete sealed item.
pub(crate) const SEALED_ITEM_LEN: usize = PREFIX_LEN + SEALED_LEN;

/// The domain label opening HPKE's `info`, ahead of the protocol version.
const INFO_LABEL: &[u8] = b"rumors bootstrap network secret";

/// Bytes of HPKE's `info`: the domain label, then the protocol version.
const INFO_LEN: usize = INFO_LABEL.len() + size_of::<u64>();

/// HPKE's `info` for a seal: the domain label, then the protocol version as
/// a big-endian `u64`. The label has a fixed length, so the version's
/// position is fixed too.
fn info() -> [u8; INFO_LEN] {
    let mut info = [0; INFO_LEN];
    let (label, version) = info.split_at_mut(INFO_LABEL.len());
    label.copy_from_slice(INFO_LABEL);
    version.copy_from_slice(&Protocol::V2.wire_version().to_be_bytes());
    info
}

/// A defect in a received key item or sealed item.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SecretDefect {
    /// The item does not open with its tag wrapping a byte string of its
    /// length.
    #[error("the {item} is not its tagged byte string")]
    Malformed {
        /// Which item.
        item: &'static str,
    },

    /// A public key in the item is not the uncompressed encoding of a point
    /// of the curve other than the identity.
    #[error("the {item} carries an invalid public key")]
    InvalidKey {
        /// Which item.
        item: &'static str,
    },

    /// The sealed secret did not open: the item was altered, sealed to
    /// another key, or sealed for another declared network or protocol
    /// version.
    #[error("the sealed network secret does not open")]
    Unauthentic,

    /// The secret belongs to a network other than the one the provider's
    /// preamble declared.
    #[error("the network secret's identifier is {received:?}, not the declared {declared:?}")]
    WrongNetwork {
        /// The network the provider's preamble declared.
        declared: Network,
        /// The public identifier of the secret received.
        received: Network,
    },
}

/// The name of the key item, in defects.
const KEY_ITEM: &str = "bootstrap key item";

/// The name of the sealed item, in defects.
const SEALED_ITEM: &str = "sealed network secret item";

/// A bootstrapping peer's half of the exchange: the HPKE key pair it draws
/// for this one session.
///
/// The private key zeroizes when it drops, through `p256`'s own support.
pub(crate) struct Joining {
    /// The private key, which opens the provider's seal.
    private: <Kem as hpke::Kem>::PrivateKey,
    /// The public key, as the key item carries it.
    public: [u8; PUBLIC_KEY_LEN],
}

impl Joining {
    /// Draw this bootstrap's key pair.
    ///
    /// # Panics
    ///
    /// Panics if the operating system's random source fails.
    pub(crate) fn new() -> Self {
        let (private, public_key) = Kem::gen_keypair();
        let mut public = [0; PUBLIC_KEY_LEN];
        public_key.write_exact(&mut public);
        Self { private, public }
    }

    /// The key item announcing this side's public key, to follow its
    /// preamble.
    pub(crate) fn item(&self) -> [u8; KEY_ITEM_LEN] {
        let mut item = [0; KEY_ITEM_LEN];
        item[..PREFIX_LEN].copy_from_slice(&KEY_ITEM_PREFIX);
        item[PREFIX_LEN..].copy_from_slice(&self.public);
        item
    }

    /// Receive and open the sealed secret of the network `declared`, which
    /// the provider's preamble named.
    pub(crate) async fn receive<R>(
        self,
        reader: &mut R,
        declared: Network,
        observe: &SessionHandle,
    ) -> Result<NetworkSecret, Error>
    where
        R: AsyncRead + Unpin + ?Sized,
    {
        let mut item = [0; SEALED_ITEM_LEN];
        read_item(reader, &mut item).await?;
        let secret = self.open(&item, declared)?;
        observe.control_received(&item);
        Ok(secret)
    }

    /// Open a complete sealed item against the declared network.
    fn open(
        &self,
        item: &[u8; SEALED_ITEM_LEN],
        declared: Network,
    ) -> Result<NetworkSecret, Error> {
        let violation = |defect| Error::violation(Phase::IdentityTransfer, defect);
        let (prefix, sealed) = item.split_at(PREFIX_LEN);
        if prefix != SEALED_ITEM_PREFIX {
            return Err(violation(SecretDefect::Malformed { item: SEALED_ITEM }));
        }
        let (encapped, sealed) = sealed.split_at(ENCAPPED_KEY_LEN);
        let (ciphertext, tag) = sealed.split_at(NETWORK_BYTES);
        let encapped = EncappedKey::from_bytes(encapped)
            .map_err(|_| violation(SecretDefect::InvalidKey { item: SEALED_ITEM }))?;
        let tag = AeadTag::<Aead>::from_bytes(tag).expect("the tag's width");
        let mut plaintext = Zeroizing::new([0u8; NETWORK_BYTES]);
        plaintext.copy_from_slice(ciphertext);
        single_shot_open_inout_detached::<Aead, Kdf, Kem>(
            &OpModeR::Base,
            &self.private,
            &encapped,
            &info(),
            (&mut plaintext[..]).into(),
            &declared.to_bytes(),
            &tag,
        )
        .map_err(|_| violation(SecretDefect::Unauthentic))?;
        let secret = NetworkSecret::from_bytes(*plaintext);
        let received = secret.network();
        if received != declared {
            return Err(violation(SecretDefect::WrongNetwork { declared, received }));
        }
        Ok(secret)
    }
}

/// A bootstrapping counterparty's validated public key, from its key item.
pub(crate) struct JoinerKey(PublicKey);

/// Validate a bootstrapping counterparty's key item, which the preamble
/// exchange read after its preamble: by a provider, to seal the secret for
/// it, and by a bootstrapping peer, before discarding it.
///
/// The `hpke` crate's deserialization makes the public-key validation RFC
/// 9180 requires (section 7.1.4, by SP 800-56A): the key must be the
/// uncompressed encoding, its coordinates reduced field elements, and its
/// point on the curve and not the identity.
pub(crate) fn decode_key(item: &[u8; KEY_ITEM_LEN]) -> Result<JoinerKey, Error> {
    let violation = |defect| Error::violation(Phase::IdentityTransfer, defect);
    let (prefix, key) = item.split_at(PREFIX_LEN);
    if prefix != KEY_ITEM_PREFIX {
        return Err(violation(SecretDefect::Malformed { item: KEY_ITEM }));
    }
    PublicKey::from_bytes(key)
        .map(JoinerKey)
        .map_err(|_| violation(SecretDefect::InvalidKey { item: KEY_ITEM }))
}

/// Seal `secret` to the bootstrapping counterparty whose key is `joiner`,
/// bound to the network identifier `declared` this side's preamble sent.
///
/// # Panics
///
/// Panics if the operating system's random source fails.
fn seal(secret: &NetworkSecret, joiner: &JoinerKey, declared: Network) -> [u8; SEALED_ITEM_LEN] {
    let mut sealed = Zeroizing::new(*secret.as_bytes());
    let (encapped, tag) = single_shot_seal_inout_detached::<Aead, Kdf, Kem>(
        &OpModeS::Base,
        &joiner.0,
        &info(),
        (&mut sealed[..]).into(),
        &declared.to_bytes(),
    )
    .expect("encapsulating to a validated key and sealing one message cannot fail");
    let mut item = [0; SEALED_ITEM_LEN];
    let (prefix, rest) = item.split_at_mut(PREFIX_LEN);
    prefix.copy_from_slice(&SEALED_ITEM_PREFIX);
    let (encapped_at, rest) = rest.split_at_mut(ENCAPPED_KEY_LEN);
    encapped.write_exact(encapped_at);
    let (ciphertext_at, tag_at) = rest.split_at_mut(NETWORK_BYTES);
    ciphertext_at.copy_from_slice(&*sealed);
    tag.write_exact(tag_at);
    item
}

/// Write the network's secret, sealed, to a bootstrapping counterparty.
///
/// The item is not flushed here: the greeting follows it on the same write
/// half before this side waits for anything from the joiner, and that
/// greeting's flush delivers both. The joiner, which waits for this item
/// before sending its greeting, therefore never waits on bytes held in a
/// buffer.
///
/// # Panics
///
/// Panics if the operating system's random source fails.
pub(crate) async fn send<W>(
    secret: &NetworkSecret,
    joiner: &JoinerKey,
    declared: Network,
    writer: &mut W,
    observe: &SessionHandle,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + ?Sized,
{
    let item = seal(secret, joiner, declared);
    writer
        .write_all(&item)
        .await
        .map_err(|source| Error::transport(Phase::IdentityTransfer, Op::Write, source))?;
    observe.control_sent(&item);
    Ok(())
}

/// Read exactly one fixed-width item.
async fn read_item<R>(reader: &mut R, item: &mut [u8]) -> Result<(), Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    reader
        .read_exact(item)
        .await
        .map(|_| ())
        .map_err(|source| Error::transport(Phase::IdentityTransfer, Op::Read, source))
}

#[cfg(test)]
mod tests;
