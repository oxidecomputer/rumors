//! Calibration of the independent encoder and the flat-tree operations
//! against the recursive oracle and production's encoders.
//!
//! The near-valid generators trust this module to spell canonical trees
//! exactly as production does, and the decoder properties trust its order
//! and rank to judge decoded values. No public doc states the version or
//! party bit layout, so production's encoders are the authority the
//! canonical spellings are checked against. Each property compares one of
//! these against an answer computed elsewhere: the recursive oracle's lattice
//! operations and rank, or production's `encode` of the same value.

use num_bigint::BigInt;
use proptest::prelude::*;

use super::{encode_rank, Party, Version};
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, arb_shape, shape_party, shape_version,
};

/// Splits every leaf into two children with its value: a non-normal spelling
/// of the same tree, which normalizing must undo.
fn split_leaves<T: Clone>(leaves: &[(u64, T)]) -> Vec<(u64, T)> {
    leaves
        .iter()
        .flat_map(|(depth, value)| [(depth + 1, value.clone()), (depth + 1, value.clone())])
        .collect()
}

proptest! {
    /// A flat version agrees with the recursive oracle on every operation the
    /// generators and decoder properties use.
    ///
    /// For two arbitrary normal-form oracle versions: the flat form of a
    /// normal tree is normal, and normalizing a spelling with every leaf
    /// split in two recovers it; the pointwise maximum and minimum equal the
    /// flat forms of the oracle's join and meet; the causal order equals the
    /// oracle's; the rank equals the oracle's exactly, and its encoding
    /// equals production's encoding of that rank; and the version's encoding
    /// equals production's encoding of the same version.
    #[test]
    fn flat_versions_agree_with_the_oracle_and_production(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
    ) {
        let (flat_a, flat_b) = (Version::from_oracle(&a), Version::from_oracle(&b));
        prop_assert_eq!(&flat_a.normalized(), &flat_a);
        prop_assert_eq!(
            &Version::from_leaves(split_leaves(flat_a.leaves())).normalized(),
            &flat_a
        );
        prop_assert_eq!(
            flat_a.zip(&flat_b, |x: &BigInt, y: &BigInt| x.max(y).clone()),
            Version::from_oracle(&(a.clone() | b.clone()))
        );
        prop_assert_eq!(
            flat_a.zip(&flat_b, |x: &BigInt, y: &BigInt| x.min(y).clone()),
            Version::from_oracle(&(a.clone() & b.clone()))
        );
        prop_assert_eq!(flat_a.le(&flat_b), a <= b);
        let rank = a.rank();
        let (num, exp) = rank.raw_parts();
        prop_assert_eq!(flat_a.rank(), (num.clone(), exp));
        prop_assert_eq!(encode_rank(num, exp), rank.encode());
        prop_assert_eq!(flat_a.encode(), from_oracle_version(&a).encode());
    }

    /// A flat party's encoding equals production's, and its normalization
    /// undoes a split of every leaf.
    #[test]
    fn flat_parties_agree_with_production(party in arb_oracle_party_nonempty()) {
        let flat = Party::from_oracle(&party);
        prop_assert_eq!(&flat.normalized(), &flat);
        prop_assert_eq!(
            &Party::from_leaves(split_leaves(flat.leaves())).normalized(),
            &flat
        );
        prop_assert_eq!(flat.encode(), from_oracle_party(&party).encode());
    }

    /// The flat encoding of every committed shape, spines up to 399 levels and
    /// bushy trees of up to 400 leaves, equals production's encoding of the
    /// same value.
    ///
    /// The arbitrary-tree properties above stop at the committed generators'
    /// depth; these spines carry the comparison to the deepest spines the
    /// near-valid generators draw.
    #[test]
    fn flat_encoding_matches_production_on_deep_shapes(
        shape in arb_shape(),
        scale in 1usize..400,
    ) {
        let version = shape_version(shape, scale);
        prop_assert_eq!(
            Version::from_oracle(&to_oracle_version(&version)).encode(),
            version.encode()
        );
        let party = shape_party(shape, scale);
        prop_assert_eq!(
            Party::from_oracle(&to_oracle_party(&party)).encode(),
            party.encode()
        );
    }
}
