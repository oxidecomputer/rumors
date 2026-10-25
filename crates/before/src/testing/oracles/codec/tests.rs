//! Calibration of the independent encoder against the recursive oracle and
//! production's encoders.
//!
//! The near-valid generators and the decoder properties trust this module to
//! spell canonical trees exactly as production does, and to bring any
//! spelling to its normal form. No public doc states the version or party bit
//! layout, so production's encoders are the authority the canonical
//! spellings are checked against. Each property compares the encoder against
//! an answer computed elsewhere: the recursive oracle's normal form, or
//! production's `encode` of the same value.

use proptest::prelude::*;

use super::{encode_rank, Party, Version};
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::generators::near_valid::SPINE_LEVELS;
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
    /// A flat version spells and normalizes as the recursive oracle and
    /// production do.
    ///
    /// For an arbitrary normal-form oracle version: its flat form is normal,
    /// and normalizing a spelling with every leaf split in two recovers it;
    /// the encoding of the oracle's rank equals production's encoding of that
    /// rank; and the version's encoding equals production's encoding of the
    /// same version.
    #[test]
    fn flat_versions_agree_with_the_oracle_and_production(a in arb_oracle_version()) {
        let flat = Version::from_oracle(&a);
        prop_assert_eq!(&flat.normalized(), &flat);
        prop_assert_eq!(
            &Version::from_leaves(split_leaves(flat.leaves())).normalized(),
            &flat
        );
        let rank = a.rank();
        let (num, exp) = rank.raw_parts();
        prop_assert_eq!(encode_rank(num, exp), rank.encode());
        prop_assert_eq!(flat.encode(), from_oracle_version(&a).encode());
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

    /// The flat encoding of every committed shape at every scale the
    /// near-valid spines reach, below the end of [`SPINE_LEVELS`], equals
    /// production's encoding of the same value.
    ///
    /// The arbitrary-tree properties above stop at the committed generators'
    /// depth; these spines carry the comparison to the deepest spines the
    /// near-valid generators draw, and the bushy shapes to as many leaves.
    #[test]
    fn flat_encoding_matches_production_on_deep_shapes(
        shape in arb_shape(),
        scale in 1..SPINE_LEVELS.end,
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
