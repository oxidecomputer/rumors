//! [`OwnVersion`]: the lazy projection view `&v / &p`.

use core::cmp::Ordering;

use crate::version::io::writer::VersionWriter;
use crate::{Party, Version};

#[cfg(test)]
mod tests;

/// The projection of a [`Version`] by a [`Party`]: `v / &p`.
///
/// The view compares directly against a [`Version`] or against another
/// `OwnVersion`, with `==`, `<=`, and the rest of [`PartialOrd`]; materializing
/// the projected [`Version`] is a separate, explicit call:
/// [`to_version`](Self::to_version) (or the [`From`] impl).
///
/// Materializing a projection can outgrow its operands by a multiplicative
/// factor of its operands' sizes, so is kept explicit.
///
/// # Complexity
///
/// The comparisons never materialize a projection; view construction
/// itself is `O(1)`:
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_cmp.html")))]
#[cfg_attr(
    not(doc),
    doc = "vs `Version`: `O(n)` in total input bytes; `O(|self| + |other|)`"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_pair_cmp.html")))]
#[cfg_attr(
    not(doc),
    doc = "vs `OwnVersion`: `O(n)` in total input bytes; `O(|self| + |other|)`"
)]
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// a.tick();
/// let mut b = a.fork();
/// b.tick();
/// let v = a.version();
/// // a's view of the shared history dominates nothing of b's own tick:
/// // the comparison is decided lazily, no projection is built.
/// assert!((v / a.party()) <= *v);
/// assert!((v / a.party()) != (b.version() / b.party()));
/// // The product-growth projection exists only on request:
/// let owned = (v / a.party()).to_version();
/// assert!(owned <= *v);
/// ```
#[derive(Debug, Clone, Copy)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct OwnVersion<'a> {
    /// The party whose owned region gates the version.
    pub(crate) party: &'a Party,
    /// The version being projected.
    pub(crate) version: &'a Version,
}

impl OwnVersion<'_> {
    /// Materializes the projected [`Version`].
    ///
    /// This is the one path to the projection as an object, and the one
    /// projection cost not linearly bounded by the operands: the result can
    /// grow as the product of their sizes. Prefer the view's own comparisons
    /// when the projection need only be compared.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_project.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n^2)` in total input bytes; the view is `O(1)`; materializing: `O(|self| + |result|)`, `|result| = O(|self|^2)`"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// a.tick();
    /// let owned: Version = a.own_version().to_version();
    /// assert_eq!(owned, *a.version());
    /// ```
    pub fn to_version(&self) -> Version {
        // Projecting through the whole interval is the identity. The clone
        // shares immutable storage, so this common case is constant-time.
        if self.party.is_seed() {
            self.version.clone()
        } else {
            VersionWriter::project(self)
        }
    }

    /// Compare this projection with a materialized version without building it.
    fn cmp_version(&self, other: &Version) -> Option<Ordering> {
        super::projection::Comparison::order(self.version, Some(self.party), other, None)
    }

    /// Test equality with a materialized version without building this projection.
    fn eq_version(&self, other: &Version) -> bool {
        super::projection::Comparison::equal(self.version, Some(self.party), other, None)
    }

    /// Compare two projected versions without materializing either one.
    fn cmp_own(&self, other: &OwnVersion<'_>) -> Option<Ordering> {
        super::projection::Comparison::order(
            self.version,
            Some(self.party),
            other.version,
            Some(other.party),
        )
    }

    /// Test two projected versions for equality without materializing either.
    fn eq_own(&self, other: &OwnVersion<'_>) -> bool {
        super::projection::Comparison::equal(
            self.version,
            Some(self.party),
            other.version,
            Some(other.party),
        )
    }
}

impl Version {
    /// Compare this version with a projection, applying the party mask to the
    /// second operand while reading both version streams.
    fn cmp_own(&self, other: &OwnVersion<'_>) -> Option<Ordering> {
        super::projection::Comparison::order(self, None, other.version, Some(other.party))
    }

    /// Test equality with a projection without materializing it.
    fn eq_own(&self, other: &OwnVersion<'_>) -> bool {
        super::projection::Comparison::equal(self, None, other.version, Some(other.party))
    }
}

/// Materializes the projection, as [`to_version`](OwnVersion::to_version).
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_project.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n^2)` in total input bytes; the view is `O(1)`; materializing: `O(|self| + |result|)`, `|result| = O(|self|^2)`"
)]
///
/// # Example
///
/// ```
/// use before::{Clock, Version};
/// let mut a = Clock::seed();
/// a.tick();
/// let owned = Version::from(a.own_version());
/// assert_eq!(owned, a.own_version().to_version());
/// ```
impl From<OwnVersion<'_>> for Version {
    fn from(view: OwnVersion<'_>) -> Version {
        view.to_version()
    }
}

// The view's causal comparison matrix, mirroring `Version`'s: every cell of
// `PartialEq`/`PartialOrd` between `OwnVersion` and `Version` (both directions)
// and between two `OwnVersion`s, over owned and borrowed operands. Every
// heterogeneous cell is the fused three-stream co-walk, every homogeneous cell
// the four-stream one; no cell materializes a projection. The macro takes the
// two comparison bodies per (lhs, rhs) pair and fans out the reference
// combinations (`&L vs &R` comes from std's blanket forwarding over `L:
// PartialEq<R>`).
macro_rules! view_cmp_impls {
    ($($lhs:ty, $rhs:ty, $eq:expr, $cmp:expr, ($($lt:lifetime),*));* $(;)?) => {
        $(
            impl<$($lt),*> PartialEq<$rhs> for $lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    $eq(self, o)
                }
            }
            impl<$($lt),*> PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    $cmp(self, o)
                }
            }
            impl<$($lt),*> PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    $eq(*self, o)
                }
            }
            impl<$($lt),*> PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    $cmp(*self, o)
                }
            }
            impl<$($lt),*> PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    $eq(self, *o)
                }
            }
            impl<$($lt),*> PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    $cmp(self, *o)
                }
            }
        )*
    };
}

view_cmp_impls! {
    OwnVersion<'a>, Version, OwnVersion::eq_version, OwnVersion::cmp_version, ('a);
    Version, OwnVersion<'a>, Version::eq_own, Version::cmp_own, ('a);
    OwnVersion<'a>, OwnVersion<'b>, OwnVersion::eq_own, OwnVersion::cmp_own, ('a, 'b);
}
