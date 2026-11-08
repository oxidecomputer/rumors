//! A network's secret, and the public identifier derived from it.
//!
//! Every network has a 128-bit secret, drawn when it is
//! [`seed`](crate::Peer::seed)ed and held by each of its peers. One key
//! schedule ([`NetworkKeys`]) derives three values from the secret: the
//! network's public identifier; the key of the hash that places every message
//! in the network's content tree (the tree's `PathKey`); and the key of the
//! hash that digests every subtree of that tree, the digests gossip lists and
//! compares (the tree's `HashKey`). These two keys ensure that a party who
//! hasn't joined the network canot predict where a future message will sit in
//! the tree or what any subtree's digest will be; the tree's `Hash` says why
//! the digests are keyed. A peer only sends the secret out encrypted, to an
//! authenticated peer, in the item a provider sends a bootstrapping peer (the
//! mirror's `secret` module). It is never printed: [`NetworkSecret`]'s `Debug`
//! redacts it, and it has no `Display`.
//!
//! Everything publicly accessible from the API speaks in terms of the
//! [`Network`] identifier instead, which reveals nothing about the secret or
//! the keys derived beside it. Preambles carry it, compatibility checks and
//! [`Mismatch::Network`](crate::error::Mismatch::Network) compare it, bookmarks
//! key their records by it.
//!
//! # Cryptographic primitives
//!
//! Every primitive the crate uses is aligned with a FIPS algorithm standard, so
//! that in principle, rumors might plausibly one day be deployed in a
//! FIPS-validated setting. Actually achieving this goal would mean swapping in
//! validated implementations and an approved random source, but not changing
//! the choice of algorithms. The choice of cryptography is intentionally
//! bog-standard:
//!
//! - HMAC-SHA-256 (FIPS 198-1) for leaf paths, and, truncated to 16 bytes (SP
//!   800-107), for the subtree digests gossip compares;
//! - HKDF with SHA-256 (RFC 5869, SP 800-56C) for the key schedule that derives
//!   both of those keys and the network identifier from the secret;
//! - HPKE (RFC 9180) for the sealed secret a provider sends a bootstrapping
//!   peer, with the suite DHKEM(P-256, HKDF-SHA256), HKDF-SHA256 and
//!   AES-256-GCM: ECDH over P-256 (SP 800-56A), HKDF with SHA-256 (SP 800-56C),
//!   and AES-256-GCM (FIPS 197, SP 800-38D);
//! - SHA3-256 (FIPS 202) for the bookmark's integrity checksum.
//!
//! Randomness (the secret, the bootstrap's key pairs) comes from the operating
//! system's generator, read through `getrandom` on every draw; a FIPS validated
//! deployment would supply an approved DRBG there.

use std::fmt;
#[cfg(any(test, feature = "test-internals"))]
use std::sync::{Arc, LazyLock};

use hkdf::Hkdf;
use rand::CryptoRng;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::tree::typed::{HMAC_KEY_LEN, HashKey, PathKey};

/// Bytes in one network identifier, and in one network secret.
pub(crate) const NETWORK_BYTES: usize = 16;

/// The key schedule's info label for the leaf-path key ([`NetworkKeys`]).
const PATH_KEY_LABEL: &[u8] = b"rumors path key";

/// The key schedule's info label for the tree hash key ([`NetworkKeys`]).
const HASH_KEY_LABEL: &[u8] = b"rumors tree hash key";

/// The key schedule's info label for the network identifier ([`NetworkKeys`]).
const NETWORK_ID_LABEL: &[u8] = b"rumors network id";

/// The identifier shared by every [`Rumors`](crate::Rumors) that descends from
/// the same [`seed`](crate::Peer::seed).
///
/// When two [`Rumors`](crate::Rumors) [`gossip`](crate::Rumors::gossip), they
/// must belong to the same causal universe. Two [`Peer`](crate::Peer)s from
/// *independent* [`seed`](crate::Peer::seed)s can end up with coincidentally
/// and transiently compatible causal state despite sharing no true causal
/// history. Such peers must never combine, and the protocol validates that
/// peers belong to the same [`Network`].
///
/// This type is opaque and [`Copy`]: callers can read it off a `Peer` with
/// [`network`](crate::Peer::network), compare two, and serialize one, for
/// example to key their own records by network. No operation takes one as
/// input, so a deserialized identifier can only be compared: a peer's network
/// is the one it was [`seed`](crate::Peer::seed)ed into or
/// [bootstrapped](crate::Peer::bootstrap) into.
//
// Internally, the identifier is expanded from the network's secret by its
// key schedule (`NetworkKeys`), so publishing it reveals nothing about the
// secret or the keys derived beside it. Each network's secret
// also keys where its messages sit in the content tree, which strengthens
// the separation of independently seeded networks: they place even
// identical versions at unrelated addresses, so nothing one network
// computes about its tree means anything in another.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Network([u8; NETWORK_BYTES]);

// The serde form is one byte string of the 16 raw bytes: the identifier is
// opaque, so no structure beyond its width belongs on the wire or on disk
// (the bookmark record keys its map by it).

impl Serialize for Network {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_bytes(deserializer, "a network identifier is exactly 16 bytes").map(Network)
    }
}

impl Network {
    /// The all-zero placeholder a bootstrapping peer sends in the handshake: it
    /// has no rumor set yet, hence no real network.
    ///
    /// It is the one value no network's identifier takes
    /// ([`NetworkSecret::from_rng`] redraws a secret that would derive it), so
    /// it unambiguously means "I am bootstrapping" on the wire and suppresses
    /// the network-match check.
    pub(crate) const BOOTSTRAP: Network = Network([0u8; NETWORK_BYTES]);

    /// Whether this is the [`BOOTSTRAP`](Self::BOOTSTRAP) placeholder rather
    /// than a real network's identifier.
    pub(crate) fn is_bootstrap(self) -> bool {
        self == Network::BOOTSTRAP
    }

    /// The raw 16 bytes, for placement into the preamble.
    pub(crate) fn to_bytes(self) -> [u8; NETWORK_BYTES] {
        self.0
    }

    /// Reconstructs an identifier from 16 bytes read off the preamble.
    pub(crate) fn from_bytes(bytes: [u8; NETWORK_BYTES]) -> Self {
        Network(bytes)
    }
}

impl fmt::Debug for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Network({})", hex::encode(self.0))
    }
}

/// The bare lowercase hex of the 16-byte identifier, with no surrounding
/// punctuation: the form for logs and operator-facing output.
impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

/// A network's 128-bit secret: the input from which its key schedule
/// ([`NetworkKeys`]) derives the keys of its leaf paths and tree digests, and
/// its public [`Network`] identifier.
///
/// Both are keyed because versions are predictable. A version is a
/// deterministic function of the network's causal history, so anyone who can
/// infer that history, and could compute the digests replicas compare, could
/// search offline for two version sets whose digests collide, then steer when
/// peers insert and gossip until those versions arise. Gossip settles a subtree
/// on digest equality alone, so that collision would silently delete the
/// messages beneath it fleet-wide. With 16-byte digests such a birthday search
/// costs about 2⁶⁴ evaluations. Every tree digest is a MAC under a key derived
/// from the secret, so nobody without the secret can evaluate one, leaving the
/// collisions the digest width argument (the tree's `Hash`) bounds.
///
/// Whoever holds the secret can compute every path and digest again, so a
/// leaked secret re-enables that search for the rest of the network's life. The
/// secret cannot be rotated: a new key moves every path, so no two replicas
/// under different keys could compare a single subtree.
///
/// So the secret lives only in its peers' memory and crosses the wire only
/// sealed for a bootstrapping peer (see the mirror's `secret` module). Nothing
/// stores it: a bookmark names each network by its public identifier, and a
/// restarted peer learns the secret again from the provider it joins through.
/// Nothing prints it either: `Debug` shows `NetworkSecret(..)`, there is no
/// `Display`, and errors and observations name a network only by its public
/// identifier. A decode error that rejects a supplied leaf prints the leaf's
/// keyed path and the scope it was expected under; those are outputs of the
/// path key, from which neither the secret nor any other key can be recovered.
///
/// A secret zeroizes its bytes when it drops, as best-effort hygiene: moves and
/// compiler temporaries can leave copies, and the threat model does not rely on
/// it. Long-lived holders share one allocation (`Arc`) rather than copies, so
/// one buffer holds the secret for as long as anything needs it.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct NetworkSecret([u8; NETWORK_BYTES]);

/// Clear the secret's bytes.
impl Zeroize for NetworkSecret {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

/// Clear the secret's bytes when it drops.
impl Drop for NetworkSecret {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// A dropped secret leaves its bytes zeroed.
impl ZeroizeOnDrop for NetworkSecret {}

impl NetworkSecret {
    /// The fixed secret of trees that tests build directly, outside any seeded
    /// peer.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) const FIXTURE: NetworkSecret = NetworkSecret([0x5e; NETWORK_BYTES]);

    /// Draws a fresh random secret from `rng`, which must be a cryptographic
    /// generator: the secret keys every leaf path and tree digest, so a
    /// predictable one re-enables the offline search that keying rules out.
    ///
    /// A secret whose identifier would be the bootstrap placeholder
    /// ([`Network::BOOTSTRAP`]) is drawn again, since its network would be
    /// indistinguishable from a bootstrapping peer. Drawing such a secret means
    /// finding a preimage of one HKDF output, probability `2^-128` per draw, so
    /// the redraw never runs in practice; it makes the placeholder unreachable
    /// by construction, and redrawing is the one response that keeps `seed`
    /// total.
    pub(crate) fn from_rng<R: CryptoRng + ?Sized>(rng: &mut R) -> Self {
        loop {
            let mut bytes = Zeroizing::new([0u8; NETWORK_BYTES]);
            rng.fill_bytes(&mut *bytes);
            let secret = NetworkSecret(*bytes);
            if !secret.network().is_bootstrap() {
                return secret;
            }
        }
    }

    /// The public identifier of this secret's network, as its key schedule
    /// derives it ([`NetworkKeys`]).
    pub(crate) fn network(&self) -> Network {
        KeySchedule::extract(self).network()
    }

    /// The raw secret bytes, to seal into the bootstrap's secret item.
    pub(crate) fn as_bytes(&self) -> &[u8; NETWORK_BYTES] {
        &self.0
    }

    /// Reconstructs a secret from 16 bytes received in a bootstrap's secret
    /// item or written by a test.
    pub(crate) fn from_bytes(bytes: [u8; NETWORK_BYTES]) -> Self {
        NetworkSecret(bytes)
    }
}

/// Redacts the secret: only the type's name is shown.
impl fmt::Debug for NetworkSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NetworkSecret(..)")
    }
}

/// A network's secret together with everything derived from it: the keyed hash
/// that places leaves ([`PathKey`]), the keyed hash that digests the content
/// tree ([`HashKey`]), and the public [`Network`] identifier.
///
/// The keys are derived once, when a peer draws or receives the secret, and
/// shared through `Arc` by everything that needs them: the peer, every clone of
/// its tree, and each session it runs. Constructing either keyed hash absorbs
/// its key into HMAC's padded key blocks, so deriving them once also spares
/// every path lookup and every tree digest that work.
///
/// # Key schedule
///
/// One HKDF-SHA-256 key schedule derives every value from the secret: the
/// extract-then-expand key derivation of RFC 5869, which NIST SP 800-56C
/// approves as its two-step key derivation. The extract step turns the secret
/// into one pseudorandom key, and the expand step derives each value from that
/// key under its own info label:
///
/// | Value                                | Info label               | Bytes |
/// |--------------------------------------|--------------------------|-------|
/// | the leaf-path key ([`PathKey`])      | `"rumors path key"`      | 32    |
/// | the tree hash key ([`HashKey`])      | `"rumors tree hash key"` | 32    |
/// | the network identifier ([`Network`]) | `"rumors network id"`    | 16    |
///
/// Each HMAC key is one SHA-256 output, the shortest key RFC 2104 recommends
/// for HMAC-SHA-256. A key's strength is still the secret's 128 bits, not its
/// own 256: whoever finds the secret derives every key from it. The identifier
/// is as wide as the secret.
///
/// The extract step takes no salt, which RFC 5869 defines as a salt of 32 zero
/// bytes. A salt strengthens extraction from input that is not uniformly random
/// (RFC 5869, section 3.1); the secret is a uniform draw, and the info labels,
/// not a salt, are what separate the values (section 3.2).
///
/// HKDF guarantees that values expanded under distinct info labels are
/// independent pseudorandom keys. So publishing the identifier reveals nothing
/// about the secret or either key, and each key reveals nothing about the
/// other. That is: whoever reads the identifier off a preamble is no closer to
/// computing a path or a compared digest than one who never saw it.
pub(crate) struct NetworkKeys {
    /// The secret every other field is derived from.
    secret: NetworkSecret,
    /// The keyed hash that places the network's leaves.
    path: PathKey,
    /// The keyed hash that digests the network's content tree, whose digests
    /// gossip lists and compares.
    hash: HashKey,
    /// The network's public identifier.
    network: Network,
}

impl NetworkKeys {
    /// Derives a network's keys and identifier from its secret, by the key
    /// schedule the type's docs give.
    pub(crate) fn new(secret: NetworkSecret) -> Self {
        let schedule = KeySchedule::extract(&secret);
        Self {
            path: PathKey::new(&schedule.expand::<HMAC_KEY_LEN>(PATH_KEY_LABEL)),
            hash: HashKey::new(&schedule.expand::<HMAC_KEY_LEN>(HASH_KEY_LABEL)),
            network: schedule.network(),
            secret,
        }
    }

    /// The keys of trees that tests build directly, outside any seeded peer:
    /// those of [`NetworkSecret::FIXTURE`], derived once and shared.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn fixture() -> Arc<Self> {
        /// The fixture keys, derived on first use.
        static FIXTURE: LazyLock<Arc<NetworkKeys>> =
            LazyLock::new(|| Arc::new(NetworkKeys::new(NetworkSecret::FIXTURE)));
        Arc::clone(&FIXTURE)
    }

    /// The network's secret, to seal for a bootstrapping peer.
    pub(crate) fn secret(&self) -> &NetworkSecret {
        &self.secret
    }

    /// The keyed hash that places the network's leaves.
    pub(crate) fn path(&self) -> &PathKey {
        &self.path
    }

    /// The keyed hash that digests the network's content tree, whose digests
    /// gossip lists and compares.
    pub(crate) fn hash(&self) -> &HashKey {
        &self.hash
    }

    /// The network's public identifier.
    pub(crate) fn network(&self) -> Network {
        self.network
    }
}

/// Keys are equal exactly when their secrets are, since every other field is a
/// function of the secret.
impl PartialEq for NetworkKeys {
    fn eq(&self, other: &Self) -> bool {
        self.secret == other.secret
    }
}

/// Key equality is secret equality, an equivalence relation.
impl Eq for NetworkKeys {}

/// Redacts the keys: every field but the identifier is a function of the
/// secret, so only the identifier is shown.
impl fmt::Debug for NetworkKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NetworkKeys").field(&self.network).finish()
    }
}

/// A network secret's key schedule after its extract step: HKDF-SHA-256's
/// pseudorandom key, from which [`expand`](Self::expand) derives each value
/// under its label ([`NetworkKeys`] gives the schedule).
///
/// The pseudorandom key is a function of the secret, so its HMAC state is
/// cleared when the schedule drops, through the `zeroize` support of the `hmac`
/// and `sha2` crates, as the same best-effort hygiene as [`NetworkSecret`]'s
/// own.
struct KeySchedule(Hkdf<Sha256>);

impl KeySchedule {
    /// Runs the extract step over `secret`, with no salt.
    fn extract(secret: &NetworkSecret) -> Self {
        Self(Hkdf::new(None, &secret.0))
    }

    /// Runs the expand step for `N` bytes under `label`.
    fn expand<const N: usize>(&self, label: &[u8]) -> Zeroizing<[u8; N]> {
        let mut value = Zeroizing::new([0; N]);
        self.0.expand(label, &mut *value).expect(
            "HKDF-SHA-256 expands up to 255 SHA-256 outputs, and no value is wider than one",
        );
        value
    }

    /// The network identifier, expanded under its label.
    fn network(&self) -> Network {
        Network(*self.expand(NETWORK_ID_LABEL))
    }
}

/// Deserializes exactly [`NETWORK_BYTES`] bytes, rejecting any other width with
/// `expected`.
fn deserialize_bytes<'de, D: Deserializer<'de>>(
    deserializer: D,
    expected: &'static str,
) -> Result<[u8; NETWORK_BYTES], D::Error> {
    let bytes = <Vec<u8>>::deserialize(deserializer)?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| serde::de::Error::custom(expected))
}

#[cfg(test)]
mod tests;
