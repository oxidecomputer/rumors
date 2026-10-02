//! Tests for the oracle-to-production conversion boundary.

use std::sync::Arc;

use num_bigint::BigUint;

use crate::testing::oracles::tree;

use super::{from_oracle_party, from_oracle_version};

/// An empty oracle party is rejected instead of becoming an anonymous
/// production party.
#[test]
#[should_panic(expected = "production Party type represents nonempty ownership")]
fn empty_party_is_rejected_at_the_conversion_boundary() {
    let _ = from_oracle_party(&tree::Party::Leaf(false));
}

/// A reducible oracle party is rejected instead of aliasing a different
/// canonical production party.
#[test]
#[should_panic(expected = "oracle Party must be normal before conversion")]
fn non_normal_party_is_rejected_at_the_conversion_boundary() {
    let empty = Arc::new(tree::Party::Leaf(false));
    let _ = from_oracle_party(&tree::Party::Node(empty.clone(), empty));
}

/// Equal sibling leaves are rejected instead of producing a non-canonical
/// production version.
#[test]
#[should_panic(expected = "oracle Version must be normal before conversion")]
fn non_normal_version_is_rejected_at_the_conversion_boundary() {
    let one = Arc::new(tree::Version::Leaf(BigUint::from(1u8)));
    from_oracle_version(&tree::Version::Node(BigUint::ZERO, one.clone(), one));
}
