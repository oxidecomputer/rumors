//! Focused entry points for deterministic resource measurements.
//!
//! These wrappers bypass public shortcuts when the meter needs to observe an
//! operation's full traversal. They do not define the production module
//! hierarchy or expose the stored representation.

use crate::error::Decode;
use crate::Version;

/// Strictly validate a version without materializing its tree.
pub fn validate(version: &Version) -> Result<(), Decode> {
    super::io::validate::whole(version.0.reader())
}

/// Copy a version's live bits for tests that inspect the representation.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn bits(version: &Version) -> crate::bits::BitsWriter {
    let len = version.stored_len();
    let mut out = crate::bits::BitsWriter::with_capacity(len);
    out.splice(&version.0, 0, len);
    out
}

/// Run the join traversal without public identity shortcuts.
#[cfg(feature = "meter")]
pub fn join(a: &Version, b: &Version) -> Version {
    super::lattice::Extreme::Higher.emit(a, b)
}

/// Run the meet traversal without public identity shortcuts.
#[cfg(feature = "meter")]
pub fn meet(a: &Version, b: &Version) -> Version {
    super::lattice::Extreme::Lower.emit(a, b)
}

/// Run the causal-comparison traversal used by public ordering operations.
#[cfg(feature = "meter")]
pub fn causal_cmp(a: &Version, b: &Version) -> Option<core::cmp::Ordering> {
    a.partial_cmp(b)
}

/// Run semantic equality rather than comparing canonical bytes.
#[cfg(feature = "meter")]
pub fn equal(a: &Version, b: &Version) -> bool {
    a.walk_eq(b)
}

/// Compute a version's rank through its streaming fold.
#[cfg(feature = "meter")]
pub fn rank(version: &Version) -> crate::Rank {
    version.rank()
}

/// Compute the minimum tick count through its streaming fold.
#[cfg(feature = "meter")]
pub fn min_ticks(version: &Version) -> crate::Ticks {
    version.min_ticks()
}

/// Materialize a version projected onto a party.
#[cfg(feature = "meter")]
pub fn project(version: &Version, party: &crate::Party) -> Version {
    version.project(party).to_version()
}

/// Read the pair-integral dense-digit counter.
#[cfg(feature = "meter")]
pub fn densified_digits() -> u64 {
    super::measure::integral::densified_digits()
}

/// Reset the pair-integral dense-digit counter.
#[cfg(feature = "meter")]
pub fn reset_densified_digits() {
    super::measure::integral::reset_densified_digits();
}
