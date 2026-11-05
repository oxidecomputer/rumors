//! A network's secret, and the public identifier derived from it.
//!
//! Every network has a 128-bit secret, drawn when it is
//! [`seed`](crate::Peer::seed)ed and held by each of its peers. It leaves a
//! peer only sealed, in the item a provider sends a bootstrapping peer (the
//! mirror's `secret` module), and it is never printed: [`NetworkSecret`]'s
//! `Debug` redacts it, and it has no `Display`.
//!
//! Everything public speaks the [`Network`] identifier instead, which the
//! secret's key schedule derives and which reveals nothing about the secret.
//! Preambles carry it, compatibility checks and
//! [`Mismatch::Network`](crate::error::Mismatch::Network) compare it,
//! bookmarks key their records by it, and the API reports it.
//!
//! # Cryptographic primitives
//!
//! Every primitive the crate uses is aligned with a FIPS algorithm standard,
//! so that a fully FIPS deployment means swapping in validated
//! implementations and an approved random source, never changing the
//! protocol:
//!
//! - SHA-256 (FIPS 180-4) for leaf paths and comparison digests;
//! - HKDF with SHA-256 (RFC 5869, SP 800-56C) for the key schedule that
//!   derives the network identifier from the secret;
//! - HPKE (RFC 9180) for the sealed secret a provider sends a bootstrapping
//!   peer, with the suite DHKEM(P-256, HKDF-SHA256), HKDF-SHA256 and
//!   AES-256-GCM: ECDH over P-256 (SP 800-56A), HKDF with SHA-256
//!   (SP 800-56C), and AES-256-GCM (FIPS 197, SP 800-38D);
//! - SHA3-256 (FIPS 202) for the bookmark's integrity checksum.
//!
//! Randomness (the secret, the bootstrap's key pairs) comes from the operating
//! system's generator through the `rand` and `getrandom` crates; a validated
//! deployment would supply an approved DRBG there, a change of source, not of
//! protocol.

use std::fmt;

use hkdf::Hkdf;
use rand::Rng;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Bytes in one network identifier, and in one network secret.
pub(crate) const NETWORK_BYTES: usize = 16;

/// The key schedule's info label for the network identifier
/// ([`KeySchedule`]).
const NETWORK_ID_LABEL: &[u8] = b"rumors network id";

/// The identifier shared by every [`Rumors`](crate::Rumors) that descends
/// from the same [`seed`](crate::Peer::seed).
///
/// When two [`Rumors`](crate::Rumors) [`gossip`](crate::Rumors::gossip), they
/// must belong to the same causal universe. Two
/// [`Peer`](crate::Peer)s from *independent* [`seed`](crate::Peer::seed)s can
/// end up with coincidentally and transiently compatible causal state despite
/// sharing no true causal history. Such peers must never combine, and the
/// protocol validates that peers belong to the same [`Network`].
///
/// This type is opaque and [`Copy`]: callers can read it off a `Peer` with
/// [`network`](crate::Peer::network) and compare two for equality, but cannot
/// create one except through [`seed`](crate::Peer::seed).
//
// Internally, the identifier is expanded from the network's secret by its
// key schedule (`KeySchedule`), so publishing it reveals nothing about the
// secret.
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
    /// ([`NetworkSecret::from_rng`] redraws a secret that would derive it),
    /// so it unambiguously means "I am bootstrapping" on the wire and
    /// suppresses the network-match check.
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

/// A network's 128-bit secret: the preimage of its public [`Network`]
/// identifier.
///
/// The secret lives only in its peers' memory and crosses the wire only
/// sealed for a bootstrapping peer (see the mirror's `secret` module).
/// Nothing stores it: a bookmark names each network by its public
/// identifier, and a restarted peer learns the secret again from the
/// provider it joins through. Nothing prints it either: `Debug` shows
/// `NetworkSecret(..)`, there is no `Display`, and errors and observations
/// carry only the public identifier.
///
/// A secret zeroizes its bytes when it drops, as best-effort hygiene: moves
/// and compiler temporaries can leave copies, and the threat model does not
/// rely on it. Long-lived holders share one allocation (`Arc`) rather than
/// copies, so one buffer holds the secret for as long as anything needs it.
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
    /// Draws a fresh random secret from `rng`.
    ///
    /// A secret whose identifier would be the bootstrap placeholder
    /// ([`Network::BOOTSTRAP`]) is drawn again, since its network would be
    /// indistinguishable from a bootstrapping peer. Drawing such a secret
    /// means finding a preimage of one HKDF output, probability `2^-128`
    /// per draw, so the redraw never runs in practice; it makes the
    /// placeholder unreachable by construction, and redrawing is the one
    /// response that keeps `seed` total.
    pub(crate) fn from_rng<R: Rng + ?Sized>(rng: &mut R) -> Self {
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
    /// derives it ([`KeySchedule`]).
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

/// A network secret's key schedule after its extract step: HKDF-SHA-256's
/// pseudorandom key, from which [`network`](Self::network) expands the
/// network's public identifier.
///
/// HKDF is the extract-then-expand key derivation of RFC 5869, which NIST
/// SP 800-56C approves as its two-step key derivation. The extract step
/// turns the secret into one pseudorandom key, and the expand step derives
/// the identifier from that key under its own info label, `"rumors network
/// id"`, as 16 bytes: as wide as the secret.
///
/// The extract step takes no salt, which RFC 5869 defines as a salt of 32
/// zero bytes. A salt strengthens extraction from input that is not
/// uniformly random (RFC 5869, section 3.1); the secret is a uniform draw,
/// and an info label, not a salt, is what separates one expanded value from
/// another (section 3.2).
///
/// HKDF's expand output is pseudorandom and reveals nothing about the key it
/// was expanded from. That is: whoever reads the identifier off a preamble is
/// no closer to the secret than one who never saw it.
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

/// Deserializes exactly [`NETWORK_BYTES`] bytes, rejecting any other width
/// with `expected`.
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
