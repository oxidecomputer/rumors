//! The replicated message schema.
//!
//! Everything rumormill knows rides one `Rumors<Entry>`: chat lines, channel
//! creations, presence heartbeats, and ephemeral system notices all live in
//! the same replicated set, so a peer that bootstraps from a single contact
//! learns the whole room — including who else there is to gossip with.
//!
//! Wall-clock fields (`sent_at`, `at`) drive TTL expiry, staleness sweeps,
//! and cosmetic timestamps only. Display ordering is purely causal, by the
//! [`Version`](rumors::Version) each entry was observed at.

use rumors::borsh::{BorshDeserialize, BorshSerialize};

/// Milliseconds since the Unix epoch.
pub type Millis = u64;

/// An iroh `EndpointId` (ed25519 public key) as raw bytes, the demo's peer
/// identity. Kept as bytes so the schema does not depend on iroh.
pub type PeerId = [u8; 32];

/// One replicated fact.
///
/// Variant order is wire format: append new variants at the end, never
/// reorder (pinned by a byte-snapshot test).
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(crate = "rumors::borsh")]
pub enum Entry {
    /// A chat line. Ephemeral: every holder redacts it at `sent_at + ttl_ms`.
    Chat {
        /// The channel this line belongs to.
        channel: String,
        /// The author's peer id (asserted, not authenticated; see crate docs).
        author: PeerId,
        /// The message text.
        body: String,
        /// Wall-clock send time, for expiry and display.
        sent_at: Millis,
        /// Lifetime after `sent_at`; every holder redacts past it.
        ttl_ms: u64,
    },
    /// A liveness heartbeat. The publisher redacts its own previous one each
    /// beat; any peer redacts one older than the staleness threshold.
    Presence {
        /// Who is alive.
        peer: PeerId,
        /// Their display name.
        name: String,
        /// Wall-clock beat time, for the staleness sweep.
        at: Millis,
    },
    /// A channel exists. Durable: the one entry kind that never expires.
    /// Keyed by name; concurrent same-name creations merge harmlessly.
    Channel {
        /// The channel name.
        name: String,
        /// Who created it.
        created_by: PeerId,
        /// Wall-clock creation time, for display.
        at: Millis,
    },
    /// An ephemeral system notice ("x joined", "x left"). Short TTL, so the
    /// redaction churn is easy to watch.
    System {
        /// The channel the notice is shown in.
        channel: String,
        /// The notice text.
        body: String,
        /// Wall-clock time, for expiry and display.
        at: Millis,
        /// Lifetime after `at`; every holder redacts past it.
        ttl_ms: u64,
    },
}

impl Entry {
    /// The wall-clock deadline after which every holder redacts this entry,
    /// or `None` for durable entries ([`Entry::Channel`]) and entries whose
    /// lifecycle is supersession rather than expiry ([`Entry::Presence`]).
    pub fn expires_at(&self) -> Option<Millis> {
        match self {
            Entry::Chat {
                sent_at, ttl_ms, ..
            } => Some(sent_at.saturating_add(*ttl_ms)),
            Entry::System { at, ttl_ms, .. } => Some(at.saturating_add(*ttl_ms)),
            Entry::Presence { .. } | Entry::Channel { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
