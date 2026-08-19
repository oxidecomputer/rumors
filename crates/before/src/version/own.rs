//! [`OwnVersion`]: the lazy projection view `&v / &p`.

use core::cmp::Ordering;

use crate::{Party, Version};

use super::skyline;

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
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_cmp.html"))]
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_pair_cmp.html"))]
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
    /// projection cost not linearly bounded by the operands: the size of its
    /// output can grow as the operands' product. Prefer the view's own
    /// comparisons wherever the projection is only being compared.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_project.html"))]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Version};
    /// let mut a = Clock::seed();
    /// a.tick();
    /// let owned: Version = a.own_version().to_version();
    /// assert_eq!(owned, *a.version()); // the seed owns its whole history
    /// ```
    pub fn to_version(&self) -> Version {
        // The whole-interval party is the projection identity — the
        // `seed_projection_is_identity` law in [`laws`](crate::laws) —
        // so the materialization is the version itself, handed back as
        // an `O(1)` buffer-sharing clone (the seed test is one byte
        // against the static seed stream).
        if self.party.is_seed() {
            return self.version.clone();
        }
        Version::from_bits(skyline::query::project(
            self.version.view().live(),
            self.party,
        ))
    }
}

/// Materializes the projection, as [`to_version`](OwnVersion::to_version).
///
/// # Complexity
///
#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_project.html"))]
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

/// The fused three-stream comparison: `(v / p) ⋚ w`, no materialization.
fn view_cmp_version(view: &OwnVersion<'_>, w: &Version) -> Option<Ordering> {
    skyline::masked::causal_cmp(
        view.version.view().live(),
        Some(view.party.as_bits()),
        w.view().live(),
        None,
    )
}

/// The fused three-stream equality: `(v / p) == w`, no materialization.
fn view_eq_version(view: &OwnVersion<'_>, w: &Version) -> bool {
    skyline::masked::eq(
        view.version.view().live(),
        Some(view.party.as_bits()),
        w.view().live(),
        None,
    )
}

/// The mirror three-stream comparison: `w ⋚ (v / p)`, the co-walk driven in
/// its own orientation — mask on the second side — rather than the first
/// orientation reversed.
///
/// The walk is total over every mask arrangement, and routing each matrix
/// cell through its natural arrangement is what keeps them all exercised
/// from the public surface; the two spellings agree by the antisymmetry of
/// the pointwise order, which the differential family beside [`OwnVersion`]'s
/// tests pins against the materialized projection.
fn version_cmp_view(w: &Version, view: &OwnVersion<'_>) -> Option<Ordering> {
    skyline::masked::causal_cmp(
        w.view().live(),
        None,
        view.version.view().live(),
        Some(view.party.as_bits()),
    )
}

/// The mirror three-stream equality: `w == (v / p)`, mask on the second side.
fn version_eq_view(w: &Version, view: &OwnVersion<'_>) -> bool {
    skyline::masked::eq(
        w.view().live(),
        None,
        view.version.view().live(),
        Some(view.party.as_bits()),
    )
}

/// The fused four-stream comparison: `(v₁ / p₁) ⋚ (v₂ / p₂)`.
fn view_cmp_view(a: &OwnVersion<'_>, b: &OwnVersion<'_>) -> Option<Ordering> {
    skyline::masked::causal_cmp(
        a.version.view().live(),
        Some(a.party.as_bits()),
        b.version.view().live(),
        Some(b.party.as_bits()),
    )
}

/// The fused four-stream equality: the projected histories agree.
fn view_eq_view(a: &OwnVersion<'_>, b: &OwnVersion<'_>) -> bool {
    skyline::masked::eq(
        a.version.view().live(),
        Some(a.party.as_bits()),
        b.version.view().live(),
        Some(b.party.as_bits()),
    )
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
    OwnVersion<'a>, Version, view_eq_version, view_cmp_version, ('a);
    Version, OwnVersion<'a>, version_eq_view, version_cmp_view, ('a);
    OwnVersion<'a>, OwnVersion<'b>, view_eq_view, view_cmp_view, ('a, 'b);
}
