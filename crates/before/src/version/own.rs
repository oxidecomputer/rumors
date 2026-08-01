//! [`OwnVersion`]: the lazy projection view `&v / &p`.

use core::cmp::Ordering;

use crate::{Party, Version};

use super::skyline;

#[cfg(test)]
mod tests;

/// The part of a [`Version`] contributed within a [`Party`]'s id region —
/// "the version `p` owns of `v`" — as a borrowed, lazy view.
///
/// Both projection spellings construct it in `O(1)`, borrowing their
/// operands: the operator `&v / &p` and
/// [`Clock::own_version`](crate::Clock::own_version). The view compares
/// directly — against a [`Version`] or against another `OwnVersion`, with
/// `==`, `<=`, and the rest of [`PartialOrd`] — in one pass over the
/// operands' packed streams, and materializing the projected [`Version`]
/// is a separate, explicit call: [`to_version`](Self::to_version) (or the
/// [`From`] impl). Materialization is the one projection operation whose
/// output can outgrow its operands (its result is not bounded by a
/// constant factor of the inputs), which is why it does not happen
/// implicitly: every lazy comparison costs the operands' packed sizes,
/// never the projection's.
///
/// Equality is semantic — the projected histories agree — not
/// representational: `view == w` holds only if `w` is zero outside the
/// party's region. There is deliberately no `Hash`: the view holds
/// borrowed operands, not canonical bytes, and hashing would cost a
/// materialization it exists to avoid; materialize with
/// [`to_version`](Self::to_version) where a hashable value is needed.
///
/// # Complexity
///
/// Comparing a view against a [`Version`] `w` (either direction, `==` or
/// [`PartialOrd`]) is `O(|v| + |p| + |w|)` time and space in the packed
/// operand sizes; comparing two views is
/// `O(|v₁| + |p₁| + |v₂| + |p₂|)`. Both are one fused co-walk over the
/// operand streams, allocation-free but for the walk's transient
/// cursors. Construction and [`Clone`]/[`Copy`] are `O(1)`.
///
/// **Complexity**: construction `O(1)`; view vs version `O(|v| + |p| + |w|)`; view vs view `O(|v₁| + |p₁| + |v₂| + |p₂|)`.
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
    /// Materializes the projected [`Version`]: the explicit, eager form
    /// of this view.
    ///
    /// This is the one path to the projection as an object, and the one
    /// projection cost not bounded by the operands: the result re-codes
    /// the version's heights once per owned fragment, so its packed size
    /// can grow as the operands' product
    /// ([`encoded_bits`](Version::encoded_bits) on the result is the
    /// honest measure). Prefer the view's own comparisons wherever the
    /// projection is only being compared.
    ///
    /// # Complexity
    ///
    /// `O(|v| + |p| + |r|)` time and space, where `|r|` is the result's
    /// packed size (not bounded by a constant factor of the operands).
    ///
    /// **Complexity**: `O(|v| + |p| + |r|)`, `|r|` the result's packed size.
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
        Version::from_bits(skyline::query::project(self.version.view(), self.party))
    }
}

/// Materializes the projection, as [`to_version`](OwnVersion::to_version).
///
/// # Complexity
///
/// `O(|v| + |p| + |r|)` time and space, as
/// [`to_version`](OwnVersion::to_version) — `|r|` the result's packed
/// size, not bounded by a constant factor of the operands.
///
/// **Complexity**: `O(|v| + |p| + |r|)`, `|r|` the result's packed size.
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
        view.version.view(),
        Some(view.party.as_bits()),
        w.view(),
        None,
    )
}

/// The fused three-stream equality: `(v / p) == w`, no materialization.
fn view_eq_version(view: &OwnVersion<'_>, w: &Version) -> bool {
    skyline::masked::eq(
        view.version.view(),
        Some(view.party.as_bits()),
        w.view(),
        None,
    )
}

/// The fused four-stream comparison: `(v₁ / p₁) ⋚ (v₂ / p₂)`.
fn view_cmp_view(a: &OwnVersion<'_>, b: &OwnVersion<'_>) -> Option<Ordering> {
    skyline::masked::causal_cmp(
        a.version.view(),
        Some(a.party.as_bits()),
        b.version.view(),
        Some(b.party.as_bits()),
    )
}

/// The fused four-stream equality: the projected histories agree.
fn view_eq_view(a: &OwnVersion<'_>, b: &OwnVersion<'_>) -> bool {
    skyline::masked::eq(
        a.version.view(),
        Some(a.party.as_bits()),
        b.version.view(),
        Some(b.party.as_bits()),
    )
}

// The view's causal comparison matrix, mirroring `Version`'s: every cell
// of `PartialEq`/`PartialOrd` between `OwnVersion` and `Version` (both
// directions) and between two `OwnVersion`s, over owned and borrowed
// operands. Every heterogeneous cell is the fused three-stream co-walk,
// every homogeneous cell the four-stream one; no cell materializes a
// projection. The macro takes the two comparison bodies per (lhs, rhs)
// pair and fans out the reference combinations (`&L vs &R` comes from
// std's blanket forwarding over `L: PartialEq<R>`).
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
    Version, OwnVersion<'a>,
        (|w: &Version, v: &OwnVersion<'_>| view_eq_version(v, w)),
        (|w: &Version, v: &OwnVersion<'_>| view_cmp_version(v, w).map(Ordering::reverse)),
        ('a);
    OwnVersion<'a>, OwnVersion<'b>, view_eq_view, view_cmp_view, ('a, 'b);
}
