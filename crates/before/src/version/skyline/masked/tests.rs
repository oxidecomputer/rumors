//! Contract pins for the masked comparison co-walk.
//!
//! The verdict mass lives elsewhere: the projection laws in [`crate::laws`]
//! pin every entry point against the materialized form over generated,
//! organic, and fuzzed populations (the module doc's testing section). What
//! lives here is the Panics contract's negative space — the silent sweep over
//! the canonicality violations the walk does not structurally notice.

use crate::codec::{self, Base, BitsMut};
use crate::error::Decode;
use crate::version::skyline::validate_bits;

use super::{causal_cmp, eq};

/// A canonicality violation the walk does not structurally notice sweeps
/// silently.
///
/// A collapsible-sibling-pair stream — which the validator rejects as
/// [`Decode::NotCanonical`] — flows through [`causal_cmp`] and [`eq`]
/// without panicking. The verdict is unspecified by contract, so the pin
/// asserts only that the calls return; what it protects is the Panics
/// sections' split between the structurally-noticed violations (truncation,
/// malformation), which panic, and the rest, which do not.
#[test]
fn collapsible_sibling_pair_sweeps_without_panicking() {
    // (5, 5): internal root, first leaf absolute gamma(5), then the zero
    // right-sibling delta — the collapsible pair.
    let mut bad = BitsMut::new();
    bad.push(false); // root: internal
    bad.push(true); // left leaf
    codec::encode_int(&mut bad, &Base::from(5u64));
    bad.push(true); // right leaf
    codec::encode_int(&mut bad, &Base::from(0u64)); // zigzag(0): equal sibling
    assert!(
        matches!(
            validate_bits(crate::codec::built_view(&bad)),
            Err(Decode::NotCanonical)
        ),
        "the witness must sit outside the contract's canonical-operand precondition"
    );
    // The canonical spelling of the same step function: the single leaf 5.
    let mut good = BitsMut::new();
    good.push(true);
    codec::encode_int(&mut good, &Base::from(5u64));
    validate_bits(crate::codec::built_view(&good)).expect("the peer operand is canonical");
    // Both entry points, both operand positions: each call must return. The
    // verdicts are unspecified and deliberately unpinned.
    let _ = causal_cmp(
        crate::codec::built_view(&bad),
        None,
        crate::codec::built_view(&good),
        None,
    );
    let _ = causal_cmp(
        crate::codec::built_view(&good),
        None,
        crate::codec::built_view(&bad),
        None,
    );
    let _ = eq(
        crate::codec::built_view(&bad),
        None,
        crate::codec::built_view(&good),
        None,
    );
    let _ = eq(
        crate::codec::built_view(&good),
        None,
        crate::codec::built_view(&bad),
        None,
    );
}
