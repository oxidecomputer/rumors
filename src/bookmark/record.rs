//! One write frontier per network, with recency kept separately from progress.
//!
//! Identities may overlap while a bootstrap holds a fork outside the live party.
//! Every identity uses the network's shared frontier, so an older, larger claim
//! cannot authorize recovery without the writes recorded by a smaller claim.
//! Check only the frontier inside the candidate's region: unrelated remote
//! progress does not constrain recovery. Eviction can remove an identity, but
//! must preserve the full frontier over every region still eligible for recovery.

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

    /// Remove transferred rights, including a network left with no rights.
    pub(super) fn slice(&mut self, network: Network, party: &Party) {
        if let Some(record) = self.networks.get(&network) {
            record.slice(party);
            if record.identities.is_empty() {
                self.networks.remove(&network);
            }
        }
    }

    /// Encode within the byte limit, removing one oldest identity at a time.
    pub(super) fn bounded_bytes(&mut self, limit: usize) -> Vec<u8> {
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

/// Retained recovery rights and the writes required to exercise them.
#[derive(Default)]
pub(crate) struct NetworkRecord {
    /// Write progress shared by every identity; consult only its owned region.
    pub(super) written: Version,
    /// Recovery candidates in checkpoint order, oldest first.
    pub(super) identities: VecDeque<Party>,
}

/// Maintain recovery requirements independently of identity recency.
impl NetworkRecord {
    /// Record the live party's writes without acquiring any stored identity.
    pub(crate) fn record(&mut self, party: &Party, version: &Version) {
        // Every overlapping claim must require these writes, regardless of
        // which identity originally recorded them.
        self.written |= &(version / party).to_version();
        self.identities.retain(|candidate| candidate != party);
        self.identities.push_back(party.dangerously_alias());
    }

    /// Acquire caught-up identities when no bootstrap guard holds a fork.
    pub(crate) fn reclaim(&mut self, party: &mut Party, version: &Version) {
        for candidate in std::mem::take(&mut self.identities) {
            // Catch-up and disjoint ownership are separate requirements.
            // Projection checks the first; Party::join checks the second.
            let caught_up = &self.written / &candidate <= *version;
            if !caught_up {
                self.identities.push_back(candidate);
                continue;
            }
            if let Err(candidate) = party.join(candidate) {
                self.identities.push_back(candidate);
            }
        }
        // An alias covered by the final live party adds no recovery rights.
        // Larger claims may include rights whose writes are still missing.
        // Keep them available for later recovery.
        self.identities.retain(|candidate| !party.covers(candidate));
    }

    /// Remove a donation from every claim before its ownership can be transferred.
    pub(crate) fn slice(&mut self, party: &Party) {
        self.identities = std::mem::take(&mut self.identities)
            .into_iter()
            .filter_map(|candidate| candidate.without(party))
            .collect();
        self.compact();
    }

    /// Forget write progress outside the regions still available for recovery.
    pub(crate) fn compact(&mut self) {
        self.written = self
            .identities
            .iter()
            .map(|party| (&self.written / party).to_version())
            .sum();
    }
}

pub(super) mod serde;

#[cfg(test)]
mod tests;
