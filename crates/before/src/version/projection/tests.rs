//! Contract pins for the masked comparison co-walk.
//!
//! The verdict mass lives elsewhere: the projection laws in [`crate::testing::laws`]
//! pin every entry point against the materialized form over generated,
//! organic, and fuzzed populations (the module doc's testing section). What
//! lives here is the Panics contract's negative space — the silent sweep over
//! the canonicality violations the walk does not structurally notice.

use num_bigint::BigUint;

use crate::bits::BitsWriter;
use crate::error::Decode;
use crate::version::io::validate::whole;
use crate::Version;

use super::Comparison;

/// A canonicality violation the walk does not structurally notice sweeps
/// silently.
///
/// A collapsible-sibling-pair stream — which the validator rejects as
/// [`Decode::NotCanonical`] — flows through both comparison questions
/// without panicking. The verdict is unspecified by contract, so the pin
/// asserts only that the calls return; what it protects is the Panics
/// sections' split between the structurally-noticed violations (truncation,
/// malformation), which panic, and the rest, which do not.
#[test]
fn collapsible_sibling_pair_sweeps_without_panicking() {
    // (5, 5): internal root, first leaf absolute gamma(5), then the zero
    // right-sibling delta — the collapsible pair.
    let mut bad = BitsWriter::new();
    bad.push(false); // root: internal
    bad.push(true); // left leaf
    bad.write_gamma(&BigUint::from(5u64));
    bad.push(true); // right leaf
    bad.write_gamma(&BigUint::ZERO); // zigzag(0): equal sibling
    assert!(
        matches!(whole(bad.reader()), Err(Decode::NotCanonical)),
        "the witness must sit outside the contract's canonical-operand precondition"
    );
    // The canonical spelling of the same step function: the single leaf 5.
    let mut good = BitsWriter::new();
    good.push(true);
    good.write_gamma(&BigUint::from(5u64));
    whole(good.reader()).expect("the peer operand is canonical");
    let bad = Version::from_test_bits(bad);
    let good = Version::from_test_bits(good);
    // Both entry points, both operand positions: each call must return. The
    // verdicts are unspecified and deliberately unpinned.
    let _ = Comparison::order(&bad, None, &good, None);
    let _ = Comparison::order(&good, None, &bad, None);
    let _ = Comparison::equal(&bad, None, &good, None);
    let _ = Comparison::equal(&good, None, &bad, None);
}
