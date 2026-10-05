//! The identities a bookmark may reclaim, and the writes it must know first.
//!
//! Messages are addressed by version, and a replica infers a deletion whenever
//! its frontier dominates a version for which it holds no message. A version
//! issued twice is therefore not a duplicate but a bomb that destroys every
//! version which previously was issued in its future lightcone. Within one
//! incarnation of the peer, its own internal clock prevents reuse: a party's
//! own progress only grows, and no other party ticks in its own identity
//! region. A restarted peer discards that progress. This record lets the next
//! incarnation take that identity region back *safely*, once it is locally
//! certain that it supersedes every message previously issued under the
//! identity. Bookmarking therefore preserves representational space under
//! ungraceful restarts (i.e. without retirement): it is safe not to bookmark,
//! but it is wasteful.
//!
//! # The safety invariant
//!
//! For every live party `p` at frontier version `V`, `V / p` (the portion of
//! `V` owned by `p`) dominates every version the network knows inside `p`'s
//! region. When `p` ticks `V`, this strictly advances `V / p`, so each new
//! version lies above every known one there, never on or below one.
//!
//! Joining an identity region (another party) `R` into the live party preserves
//! the invariant exactly when:
//!
//! 1. no live peer owns `R`, or two parties would be able to register their own
//!    events there; and
//!
//! 2. the live frontier dominates every version the network knows in `R`, or
//!    the next tick could land on or below one already issued.
//!
//! # How the record establishes both
//!
//! A [`NetworkRecord`] holds the identities its owner recorded and one
//! frontier, `written`, shared by all of them.
//!
//! - An identity is a region the owner held when it recorded it and has not
//!   since given away. A donation to a bootstrapping peer is removed from
//!   every identity, durably, before it can leave the process. So an identity
//!   is owned by no live peer: condition 1.
//! - `written`, projected on an identity, dominates every version the
//!   network knows in that region: every own event is recorded before it is
//!   transmitted, and `written` never shrinks inside a recorded region. One
//!   frontier serves every identity, so an older, larger identity cannot
//!   bypass writes recorded under a newer, smaller one. So "the live frontier
//!   dominates `written / R`" gives condition 2.
//!
//! The reclaim rule is then one line: join an identity into the live party
//! exactly when the live frontier dominates `written` on it.
//!
//! Identities may overlap. A fork reserved for a bootstrap leaves the live
//! party while an identity recorded earlier still contains it, and a
//! checkpoint taken meanwhile records the smaller party beside the larger
//! one. The shared frontier keeps overlap harmless for condition 2. For
//! condition 1, a checkpoint reclaims only while no fork is reserved, and
//! reserving needs the same exclusive access to the replica that a checkpoint
//! runs under, so neither can begin during the other.
//!
//! The size constraint on a stored bookmark may drop older identities, which
//! only forfeits recovery; it must keep `written` whole over every identity it
//! retains for safety.

use std::collections::{VecDeque, hash_map::RandomState};

use before::{Party, Version};

use super::format;
use crate::Network;

/// One record per network, ordered by its most recent checkpoint.
///
/// Recovery rules belong to each network. This collection only chooses which
/// rights to retain and preserves network recency across stores.
#[derive(Default)]
pub(crate) struct Record {
    /// The limit applies to encoded contents, not the map's table allocation.
    pub(super) networks:
        schnellru::LruMap<Network, NetworkRecord, schnellru::Unlimited, RandomState>,
}

/// Maintain network recency and enforce the bookmark's stored-byte limit.
impl Record {
    /// Whether storage retains any record for this network.
    #[cfg(test)]
    pub(crate) fn contains_network(&self, network: Network) -> bool {
        self.networks.peek(&network).is_some()
    }

    /// Get a network's recovery record and mark it most recently used.
    pub(super) fn network(&mut self, network: Network) -> &mut NetworkRecord {
        self.networks
            .get_or_insert(network, NetworkRecord::default)
            .expect("the network map accepts every entry that can be allocated")
    }

    /// Remove a donation from a network's identities, and the network itself
    /// once no identity remains.
    pub(super) fn donate(&mut self, network: Network, donation: &Party) {
        if let Some(record) = self.networks.get(&network) {
            record.donate(donation);
            if record.identities.is_empty() {
                self.networks.remove(&network);
            }
        }
    }

    /// Encode within the byte limit, removing one oldest identity at a time.
    ///
    /// Eviction starts with the least recently used network and its oldest
    /// identity, so that a network's live party, which is the newest identity
    /// after a checkpoint, goes last: it is the identity the next restart
    /// needs first. Every network is compacted first, so that the frontier
    /// costs no bytes for a region no identity records.
    pub(super) fn encode_bounded(&mut self, limit: usize) -> Vec<u8> {
        for (_, network) in self.networks.iter_mut() {
            network.compact();
        }
        // Measure with the same serializer used for storage. Only the network
        // being pruned changes size; the frame and array headers are priced
        // separately because their widths depend on the remaining contents.
        let mut total: usize = self
            .networks
            .iter()
            .map(|(key, network)| format::encode_network(*key, network).len())
            .sum();
        while format::record_size(self.networks.len(), total) > limit {
            let Some((&key, _)) = self.networks.peek_oldest() else {
                break;
            };
            let network = self.networks.peek_mut(&key).unwrap();
            total -= format::encode_network(key, network).len();
            network.identities.pop_front();
            if network.identities.is_empty() {
                self.networks.pop_oldest();
            } else {
                network.compact();
                total += format::encode_network(key, network).len();
            }
        }
        format::encode(self)
    }
}

/// The identities one bookmark may reclaim in one network, and the writes it
/// must have caught up with before it can.
#[derive(Default)]
pub(crate) struct NetworkRecord {
    /// Every write any identity here recorded, as one frontier.
    ///
    /// Projected on an identity, it is the progress a restart must dominate
    /// before reclaiming that particular identity.
    pub(super) written: Version,
    /// Regions the owner held when it recorded them and has not since given
    /// away, oldest record first.
    ///
    /// After a checkpoint the live party is the newest.
    pub(super) identities: VecDeque<Party>,
}

/// Record, reclaim, and give away identities under the module's invariant.
impl NetworkRecord {
    /// Record the live party at its frontier.
    ///
    /// `written` grows by the frontier's own progress on the party, and the
    /// party becomes the newest identity, present once, so that recording it
    /// again refreshes its place rather than duplicating it.
    pub(crate) fn record(&mut self, party: &Party, frontier: &Version) {
        self.written |= &(frontier / party).to_version();
        self.identities.retain(|identity| identity != party);
        self.identities.push_back(party.dangerously_alias());
    }

    /// Join every identity the frontier has caught up with into the live party.
    ///
    /// The caller guarantees that no fork of the live party is reserved for a
    /// donation, because an identity recorded before the reservation still
    /// contains that fork and we would hand it back. A caught-up identity may
    /// overlap the live party, because a checkpoint taken while a fork was
    /// reserved recorded the smaller party beside an older, larger one. What
    /// such an identity contributes is the part outside the live party: that
    /// part is disjoint from the live party by construction, and the frontier
    /// has caught up with it because it has caught up with the whole identity.
    /// So the join cannot fail, the order of the identities does not matter,
    /// and an identity the live party already covers contributes nothing and
    /// drops out. Every identity that remains still awaits writes the frontier
    /// lacks, in its original order.
    pub(crate) fn reclaim(&mut self, party: &mut Party, frontier: &Version) {
        let mut waiting = VecDeque::new();
        for identity in std::mem::take(&mut self.identities) {
            let caught_up = &self.written / &identity <= *frontier;
            if !caught_up {
                waiting.push_back(identity);
                continue;
            }
            if let Some(missing) = identity.without(party) {
                party
                    .join(missing)
                    .expect("the part of an identity outside the live party is disjoint from it");
            }
        }
        self.identities = waiting;
    }

    /// Remove a donation from every identity, so that the record can no longer
    /// offer any part of it to a restart.
    ///
    /// `written` may still cover the donated region until the next encoding
    /// compacts it away. That costs only bytes, because nothing reads `written`
    /// outside an identity.
    pub(crate) fn donate(&mut self, donation: &Party) {
        self.identities = std::mem::take(&mut self.identities)
            .into_iter()
            .filter_map(|identity| identity.without(donation))
            .collect();
    }

    /// Forget `written` outside every identity, so that an encoding pays only
    /// for regions a restart could reclaim.
    ///
    /// Progress outside every identity bounds nothing reclaimable, so no
    /// reclaim decision changes; the record only shrinks.
    pub(crate) fn compact(&mut self) {
        self.written = self
            .identities
            .iter()
            .map(|identity| (&self.written / identity).to_version())
            .sum();
    }
}

pub(super) mod serde;

#[cfg(test)]
mod tests;
