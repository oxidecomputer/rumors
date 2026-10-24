//! Probes of operations that may stop once their answer is decided.

use before::testing::meter::registry::Shape;
use before::testing::meter::version;
use before::Version;

use crate::measured;

/// The magnitude in bits of the refuted comb's teeth, fixed whatever the
/// tooth count, so the deciding first interval's codes are the same at every
/// count and only the tail the walk must not read grows.
const REFUTED_TOOTH_BITS: usize = 1_024;

/// Measures the semantic equality walk of the comb `C(1024, teeth)` against
/// the empty version.
///
/// The comb's first leaf sits at `2^1024 − 1` and the empty version's one
/// plateau at zero, so the first elementary interval refutes equality.
pub fn eq_refuted_at_first_interval(teeth: usize) -> u64 {
    let comb = Shape::CliffComb.build2(REFUTED_TOOTH_BITS, teeth).version();
    let empty = Version::new();
    measured(
        || version::equal(&comb, &empty),
        |equal| assert!(!equal, "the comb differs from the empty version"),
    )
}
