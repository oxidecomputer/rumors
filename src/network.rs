//! The per-universe network identifier.

use std::fmt;

use borsh::{BorshDeserialize, BorshSerialize};
use rand::RngCore;

/// The random, unique identifier of a causally connected universe of
/// [`Peer`](crate::Peer)s.
///
/// It exists to catch a failure mode that party disjointness alone cannot: two
/// `Peer`s from *independent* [`seed`](crate::Peer::seed)s can end up with
/// *coincidentally* disjoint parties despite sharing no causal history. Such
/// peers must never combine.
///
/// Opaque and [`Copy`]: callers can read it off a `Peer` with
/// [`network`](crate::Peer::network) and compare two for equality, but cannot
/// mint one except through [`seed`](crate::Peer::seed).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshDeserialize, BorshSerialize)]
pub struct Network([u8; 16]);

impl Network {
    /// The all-zero placeholder a bootstrapping peer sends in the handshake: it
    /// has no rumor set yet, hence no real network. It is the one value
    /// [`from_rng`](Self::from_rng) never mints, so it unambiguously means "I
    /// am bootstrapping" on the wire and suppresses the network-match check.
    pub(crate) const BOOTSTRAP: Network = Network([0u8; 16]);

    /// Mint a fresh random identifier by drawing 16 bytes from `rng`.
    ///
    /// Re-draws in the (cryptographically impossible, `2^-128`) event of the
    /// all-zero value, keeping [`BOOTSTRAP`](Self::BOOTSTRAP) reserved as the
    /// unambiguous bootstrap sentinel.
    pub(crate) fn from_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        loop {
            let mut bytes = [0u8; 16];
            rng.fill_bytes(&mut bytes);
            let network = Network(bytes);
            if !network.is_bootstrap() {
                return network;
            }
        }
    }

    /// Whether this is the [`BOOTSTRAP`](Self::BOOTSTRAP) placeholder rather
    /// than a real, randomly-minted universe id.
    pub(crate) fn is_bootstrap(self) -> bool {
        self == Network::BOOTSTRAP
    }

    /// The raw 16 bytes, for placement into the greeting frame.
    pub(crate) fn to_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Reconstruct a network from 16 bytes read off the greeting frame.
    pub(crate) fn from_bytes(bytes: [u8; 16]) -> Self {
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
