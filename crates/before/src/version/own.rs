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
/// These comparisons read the projected history through its [`Party`]; they
/// never materialize the projection. This preserves an input-linear comparison
/// cost even when the materialized projection would be much larger. View
/// construction itself is `O(1)`.
///
/// [`partial_cmp`](PartialOrd::partial_cmp) reads until it knows the complete
/// causal relation. A directional operator stops at the first counterexample,
/// and equality stops at the first difference:
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_cmp.html")))]
#[cfg_attr(
    not(doc),
    doc = "vs `Version`: full relation: `O(n)` in total input bytes; `O(|self| + |other|)`"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_order.html")))]
#[cfg_attr(
    not(doc),
    doc = "`OwnVersion <= Version`: `O(n)` in total input bytes; `O(|self| + |other|)`; stops when this direction is disproved"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_own_order.html")))]
#[cfg_attr(
    not(doc),
    doc = "`Version <= OwnVersion`: `O(n)` in total input bytes; `O(|self| + |other|)`; stops when this direction is disproved"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_eq.html")))]
#[cfg_attr(
    not(doc),
    doc = "`OwnVersion == Version`: `O(n)` in total input bytes; `O(|self| + |other|)`; stops at the first difference"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_pair_cmp.html")))]
#[cfg_attr(
    not(doc),
    doc = "vs `OwnVersion`: full relation: `O(n)` in total input bytes; `O(|self| + |other|)`"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_pair_order.html")))]
#[cfg_attr(
    not(doc),
    doc = "`OwnVersion <= OwnVersion`: `O(n)` in total input bytes; `O(|self| + |other|)`; stops when this direction is disproved"
)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/own_version_pair_eq.html")))]
#[cfg_attr(
    not(doc),
    doc = "`OwnVersion == OwnVersion`: `O(n)` in total input bytes; `O(|self| + |other|)`; stops at the first difference"
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
// method compares through the projected streams and never materializes a
// projection. The macro generates the operand and reference combinations;
// `&L == &R` comes from the standard library's forwarding implementation once
// `L: PartialEq<R>` exists.
macro_rules! view_cmp_impls {
    ($($lhs:ty, $rhs:ty, ($($lt:lifetime),*));* $(;)?) => {
        $(
            impl<$($lt),*> PartialEq<$rhs> for $lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    super::projection::Comparison::equal(self, o)
                }
            }
            impl<$($lt),*> PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    super::projection::Comparison::order(self, o)
                }
                fn lt(&self, o: &$rhs) -> bool { super::projection::Comparison::lt(self, o) }
                fn le(&self, o: &$rhs) -> bool { super::projection::Comparison::le(self, o) }
                fn gt(&self, o: &$rhs) -> bool { super::projection::Comparison::lt(o, self) }
                fn ge(&self, o: &$rhs) -> bool { super::projection::Comparison::le(o, self) }
            }
            impl<$($lt),*> PartialEq<$rhs> for &$lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    super::projection::Comparison::equal(*self, o)
                }
            }
            impl<$($lt),*> PartialOrd<$rhs> for &$lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    super::projection::Comparison::order(*self, o)
                }
                fn lt(&self, o: &$rhs) -> bool { super::projection::Comparison::lt(*self, o) }
                fn le(&self, o: &$rhs) -> bool { super::projection::Comparison::le(*self, o) }
                fn gt(&self, o: &$rhs) -> bool { super::projection::Comparison::lt(o, *self) }
                fn ge(&self, o: &$rhs) -> bool { super::projection::Comparison::le(o, *self) }
            }
            impl<$($lt),*> PartialEq<&$rhs> for $lhs {
                fn eq(&self, o: &&$rhs) -> bool {
                    super::projection::Comparison::equal(self, *o)
                }
            }
            impl<$($lt),*> PartialOrd<&$rhs> for $lhs {
                fn partial_cmp(&self, o: &&$rhs) -> Option<Ordering> {
                    super::projection::Comparison::order(self, *o)
                }
                fn lt(&self, o: &&$rhs) -> bool { super::projection::Comparison::lt(self, *o) }
                fn le(&self, o: &&$rhs) -> bool { super::projection::Comparison::le(self, *o) }
                fn gt(&self, o: &&$rhs) -> bool { super::projection::Comparison::lt(*o, self) }
                fn ge(&self, o: &&$rhs) -> bool { super::projection::Comparison::le(*o, self) }
            }
        )*
    };
}

view_cmp_impls! {
    OwnVersion<'a>, Version, ('a);
    Version, OwnVersion<'a>, ('a);
    OwnVersion<'a>, OwnVersion<'b>, ('a, 'b);
}
