//! Serde adapters for the ordered network records and their tagged atoms.
//!
//! CBOR writes each network as `[network, frontier, identities]`. The outer
//! sequence is newest-first; identities are oldest-first. Required tags keep
//! the atom kinds distinct, and `before` validates their canonical encodings.

use std::fmt;

use ::serde::de::{Error as _, SeqAccess, Visitor};
use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use ciborium::tag::Required;

use super::{NetworkRecord, Record};
use crate::bookmark::serde::Complete;
use crate::{
    Network,
    tags::{PARTY_TAG, VERSION_TAG},
};

/// Borrow a network and its recovery record for tuple serialization.
pub(in crate::bookmark) struct Entry<'a>(
    /// The network owning these recovery rights.
    pub Network,
    /// Its shared frontier and retained identities.
    pub &'a NetworkRecord,
);

/// Write the fixed network tuple without copying its identities or frontier.
impl Serialize for Entry<'_> {
    /// Delegate tuple structure and atom encodings to Serde.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (
            &self.0,
            Required::<_, VERSION_TAG>(&self.1.written),
            Identities(&self.1.identities),
        )
            .serialize(serializer)
    }
}

/// Borrow identities while adding the required CBOR tag to each one.
struct Identities<'a>(
    /// Recovery candidates in checkpoint order.
    &'a std::collections::VecDeque<before::Party>,
);

/// A decoded network tuple, including its closing array boundary.
struct DecodedEntry(
    /// The network owning these recovery rights.
    Network,
    /// Its validated frontier and identities.
    NetworkRecord,
);

/// Require exactly three fields, including for indefinite CBOR arrays.
impl<'de> Deserialize<'de> for DecodedEntry {
    /// Use Serde for the fields and explicitly consume the tuple's end.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let Complete((key, Required(written), identities)): Complete<(
            Network,
            Required<before::Version, VERSION_TAG>,
            Vec<Required<before::Party, PARTY_TAG>>,
        )> = Complete::deserialize(deserializer)?;
        let identities = identities
            .into_iter()
            .map(|Required(party)| party)
            .collect();
        Ok(DecodedEntry(
            key,
            NetworkRecord {
                written,
                identities,
            },
        ))
    }
}

/// Preserve the identities' oldest-first order and exact sequence length.
impl Serialize for Identities<'_> {
    /// Write tagged identities using their existing canonical Serde encoding.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(Required::<_, PARTY_TAG>))
    }
}

/// Write networks in LRU order without an intermediate collection.
impl Serialize for Record {
    /// Keep the newest network first, as required by the bookmark format.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(
            self.networks
                .iter()
                .map(|(key, record)| Entry(*key, record)),
        )
    }
}

/// Read unique networks and restore their stored recency.
impl<'de> Deserialize<'de> for Record {
    /// Let Serde validate tuple shapes, required tags, and atom contents.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Construct the map one network at a time, rejecting duplicate keys.
        struct Networks;

        /// Read the stored sequence without materializing a generic CBOR tree.
        impl<'de> Visitor<'de> for Networks {
            /// The ordered, validated recovery record.
            type Value = Record;

            /// Name the expected value in decoding diagnostics.
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a sequence of unique network records")
            }

            /// Decode one tuple at a time and preserve both recency orders.
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Record, A::Error> {
                let mut record = Record::default();
                let mut order = Vec::new();
                while let Some(DecodedEntry(key, entry)) = seq.next_element()? {
                    if record.networks.peek(&key).is_some() {
                        return Err(A::Error::custom("duplicate bookmark network"));
                    }
                    assert!(
                        record.networks.insert(key, entry),
                        "the network map accepts every entry that can be allocated"
                    );
                    order.push(key);
                }
                // Insertion promotes each entry. Visiting keys in reverse
                // restores the stored order without copying recovery records.
                for key in order.into_iter().rev() {
                    record.networks.get(&key);
                }
                Ok(record)
            }
        }

        deserializer.deserialize_seq(Networks)
    }
}
