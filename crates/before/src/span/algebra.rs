//! Combines spans by containment or by their endpoints' causal order.
//!
//! Union (`+`) finds the least span containing both inputs; intersection (`*`)
//! finds their common versions, if any. Pointwise join (`|`) and meet (`&`)
//! instead apply the corresponding version operation to each endpoint. Each
//! operation is determined by how it combines the lower and upper endpoints:
//!
//! ```text
//!                lower       upper       total?
//!   + union       meet        join       yes (containment join)
//!   * intersect   join        meet       no  (containment meet)
//!   | join        join        join       yes (pointwise join)
//!   & meet        meet        meet       yes (pointwise meet)
//! ```
//!
//! Union preserves endpoint order because it can only lower the lower bound
//! and raise the upper bound. Pointwise join and meet preserve it because both
//! version operations are monotone. Intersection can instead raise the lower
//! bound beyond, or make it concurrent with, the upper bound. It returns
//! `None` unless the resulting lower endpoint is at most the upper endpoint.
//!
//! The three total operations also assign in place (`+=`, `|=`, `&=`), writing
//! the computed span back to the receiver. Intersection has no assignment
//! operator because assignment cannot return its possibly empty result.
//!
//! Every endpoint combine is a [`Version`] join or meet, whose result is no
//! larger than its operands together. The balanced folds therefore hold at most
//! the original input size across each level: `O(D log k)` total work and
//! `O(D)` live endpoint storage for `D` input bytes and `k` inputs.
//!
//! Union, pointwise join, and pointwise meet also accept a [`Version`],
//! treating it as a point span. When the version is on the left, an explicit
//! mixed-type implementation preserves the existing meaning of version-version
//! `|` and `&`; version-version `+` remains undefined. Intersection accepts
//! only spans because it can return [`None`]. Intersecting with a point
//! therefore requires an explicit [`Span::at`].

use std::borrow::Borrow;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Mul};

use crate::Version;

use super::Span;

impl<'a> Span<'a> {
    /// The *union* of `self` and `other`: the tightest [`Span`] covering both.
    ///
    /// The method spelling of `self + other`. A [`Version`] argument denotes
    /// its point span, so `span.union(version)` extends the span to cover the
    /// version.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// let head = a1.span(&a2);
    /// let tail = a2.span(&a3);
    /// // The union covers both operands…
    /// assert_eq!(head.union(&tail), a1.span(&a3));
    /// // …and is exactly the `+` operator.
    /// assert_eq!(head.union(&tail), &head + &tail);
    /// // A version is taken as its point span.
    /// assert_eq!(head.union(&a3), a1.span(&a3));
    /// ```
    pub fn union<'b>(&self, other: impl Into<Span<'b>>) -> Span<'static> {
        let other = other.into();
        if let (Some(a), Some(b)) = (self.shared_endpoint(), other.shared_endpoint()) {
            // The union of two points is their causal hull. `span` computes
            // its meet and join in one paired walk.
            return a.span(b);
        }
        let mut lo = self.lo().clone();
        let mut hi = self.hi().clone();
        lo &= other.lo();
        hi |= other.hi();
        // The lower bound only decreased and the upper bound only increased.
        Span::owned(lo, hi)
    }

    /// The tightest [`Span`] covering every input (including `self`).
    ///
    /// Each item may be a [`Span`] or a [`Version`], the latter of which
    /// denotes its point span.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let a1 = a.tick().clone();
    /// let a2 = a.tick().clone();
    /// let b1 = b.tick().clone();
    ///
    /// let spans = [a1.span(&a2), b1.span(&b1)];
    /// let span = spans[0].union_all(&spans[1..]);
    /// // The union covers every input span's endpoints.
    /// assert_eq!(span, &spans[0] + &spans[1]);
    /// // An empty iterator returns an owned copy of the receiver.
    /// assert_eq!(spans[0].union_all::<[Span; 0]>([]), spans[0]);
    /// ```
    pub fn union_all<'s, I>(&self, iter: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Into<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(iter.into_iter().map(Into::into), &UNION_OPS);
        Span::owned(lo, hi)
    }

    /// The *intersection* of `self` and `other`: the largest [`Span`] covered
    /// by both, or [`None`] when they share no overlap.
    ///
    /// The method spelling of `self * other`. This operation accepts only a
    /// [`Span`]; use [`Span::at`] explicitly to intersect with a version.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// let head = a1.span(&a2);
    /// let tail = a2.span(&a3);
    /// // Overlapping segments intersect at their shared segment…
    /// assert_eq!(head.intersect(&tail), Some(a2.span(&a2)));
    /// // …and disjoint segments have no intersection.
    /// assert_eq!(a1.span(&a1).intersect(&tail), None);
    /// ```
    pub fn intersect(&self, other: &Span<'_>) -> Option<Span<'static>> {
        if let (Some(a), Some(b)) = (self.shared_endpoint(), other.shared_endpoint()) {
            // Two points intersect exactly when they are equal.
            return (a == b).then(|| Span::owned(a.clone(), a.clone()));
        }
        let mut lo = self.lo().clone();
        let mut hi = self.hi().clone();
        lo |= other.lo();
        hi &= other.hi();
        if lo <= hi {
            Some(Span::owned(lo, hi))
        } else {
            None
        }
    }

    /// The largest [`Span`] every input (including `self`) covers, or `None` if
    /// the intersection is empty.
    ///
    /// Like [`intersect`](Span::intersect) and unlike the total operators' `_all`
    /// forms, items are true [`Span`]s only: a point item would silently empty
    /// the intersection unless every input contains it — spell [`Span::at`]
    /// to intersect with versions.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let a1 = a.tick().clone();
    /// let a2 = a.tick().clone();
    /// let a3 = a.tick().clone();
    ///
    /// let wide = a1.span(&a3);
    /// let tail = a2.span(&a3);
    /// // Chain segments intersect where they overlap…
    /// assert_eq!(wide.intersect_all([&tail]), Some(tail.clone()));
    /// // …and an empty intersection is None, never a panic.
    /// assert_eq!(a1.span(&a1).intersect_all([&tail]), None);
    /// ```
    pub fn intersect_all<'s, I>(&self, iter: I) -> Option<Span<'static>>
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(iter, &INTERSECT_OPS);
        if lo <= hi {
            Some(Span::owned(lo, hi))
        } else {
            None
        }
    }

    /// The *pointwise join* of `self` and `other`: the version lattice's join
    /// applied to each endpoint pair.
    ///
    /// The method spelling of `self | other`, mirroring [`Version::join`].
    /// A [`Version`] argument denotes its point span, so it joins into both
    /// endpoints.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// // A subtree's bounds, after every member also absorbs b1:
    /// let advanced = a1.span(&a2).join(&b1);
    /// assert_eq!(*advanced.lo(), &a1 | &b1);
    /// assert_eq!(*advanced.hi(), &a2 | &b1);
    /// // The version was taken as its point span.
    /// assert_eq!(advanced, a1.span(&a2).join(&b1.span(&b1)));
    /// ```
    pub fn join<'b>(&self, other: impl Into<Span<'b>>) -> Span<'static> {
        let other = other.into();
        if let (Some(a), Some(b)) = (self.shared_endpoint(), other.shared_endpoint()) {
            // Pointwise join keeps a point: both endpoints are the versions'
            // join and can share its immutable buffer.
            let joined = a.join(b);
            return Span::owned(joined.clone(), joined);
        }
        let mut lo = self.lo().clone();
        let mut hi = self.hi().clone();
        lo |= other.lo();
        hi |= other.hi();
        Span::owned(lo, hi)
    }

    /// The [`Span`] whose lower and upper bounds are, respectively, the joins
    /// of the lower and upper bounds of `self` and all the spans in `iter`,
    /// mirroring [`Version::join_all`].
    ///
    /// Each item may be a [`Span`] or a [`Version`], which denotes its point
    /// span.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let a1 = a.tick().clone();
    /// let b1 = b.tick().clone();
    ///
    /// // On points, the pointwise operator is the version join.
    /// let joined = a1.span(&a1).join_all([&b1.span(&b1)]);
    /// assert_eq!(joined.lo(), joined.hi());
    /// assert_eq!(joined.lo(), &(&a1 | &b1));
    /// ```
    pub fn join_all<'s, I>(&self, iter: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Into<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(iter.into_iter().map(Into::into), &JOIN_OPS);
        Span::owned(lo, hi)
    }

    /// The *pointwise meet* of `self` and `other`: the version lattice's meet
    /// applied to each endpoint pair.
    ///
    /// The method spelling of `self & other`, mirroring [`Version::meet`].
    /// A [`Version`] argument denotes its point span, clamping both endpoints
    /// to its past.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// // Clamping a segment to a point's past:
    /// let clamped = a2.span(&a3).meet(&a2);
    /// assert_eq!(clamped, a2.span(&a2));
    /// // The version was taken as its point span.
    /// assert_eq!(clamped, a2.span(&a3).meet(&a2.span(&a2)));
    /// ```
    pub fn meet<'b>(&self, other: impl Into<Span<'b>>) -> Span<'static> {
        let other = other.into();
        if let (Some(a), Some(b)) = (self.shared_endpoint(), other.shared_endpoint()) {
            // Pointwise meet likewise keeps a point and shares one buffer
            // between its endpoints.
            let met = a.meet(b);
            return Span::owned(met.clone(), met);
        }
        let mut lo = self.lo().clone();
        let mut hi = self.hi().clone();
        lo &= other.lo();
        hi &= other.hi();
        Span::owned(lo, hi)
    }

    /// The [`Span`] whose lower and upper bounds are, respectively, the meets
    /// of the lower and upper bounds of `self` and all the spans in `iter`,
    /// mirroring [`Version::meet_all`].
    ///
    /// Each item may be a [`Span`] or a [`Version`], which denotes its point
    /// span.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet_all.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count"
    )]
    ///
    /// Auxiliary space is `O(|self| + |iter|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let a1 = a.tick().clone();
    /// let b1 = b.tick().clone();
    ///
    /// // On points, the pointwise operator is the version meet.
    /// let met = a1.span(&a1).meet_all([&b1.span(&b1)]);
    /// assert_eq!(met.lo(), met.hi());
    /// assert_eq!(met.lo(), &(&a1 & &b1));
    /// ```
    pub fn meet_all<'s, I>(&self, iter: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Into<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(iter.into_iter().map(Into::into), &MEET_OPS);
        Span::owned(lo, hi)
    }

    /// Combine the receiver and input spans with a balanced fold of their
    /// lower and upper endpoints.
    ///
    /// Adjacent spans sharing both endpoint buffers are equivalent under all
    /// four idempotent operations, so only one from each run enters the fold.
    /// Inputs are borrowed until combined; intermediate endpoints are owned
    /// and updated in place. Two point spans can combine their single versions
    /// directly instead of repeating the operation for both endpoints.
    fn fold_endpoints<'s, I>(&self, iter: I, ops: &SpanFoldOps) -> (Version, Version)
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        // The dedup filter: one (lo, hi) buffer-identity pair of state.
        let mut last: Option<(Version, Version)> = None;
        let inputs = core::iter::once(FoldInput::Receiver(self))
            .chain(iter.into_iter().map(FoldInput::Item))
            .filter(move |input| {
                let s = input.span();
                let dup = last
                    .as_ref()
                    .is_some_and(|(lo, hi)| lo.ptr_eq(s.lo()) && hi.ptr_eq(s.hi()));
                if !dup {
                    last = Some((s.lo().clone(), s.hi().clone()));
                }
                !dup
            })
            .map(Group::Input);
        let group = crate::fold::balanced_reduce(inputs, |a, b| {
            // Combining two point spans needs one lattice operation rather
            // than separate work for equal lower and upper endpoints. Shared
            // storage identifies a point span in O(1).
            let (lo, hi) = if let (Some(va), Some(vb)) = (a.point(), b.point()) {
                (ops.points)(va, vb)
            } else {
                match (a, b) {
                    (Group::Input(a), Group::Input(b)) => {
                        let (a, b) = (a.span(), b.span());
                        ((ops.lo_refs)(a.lo(), b.lo()), (ops.hi_refs)(a.hi(), b.hi()))
                    }
                    (Group::Merged { mut lo, mut hi }, Group::Input(b)) => {
                        let b = b.span();
                        (ops.assign_lo)(&mut lo, b.lo());
                        (ops.assign_hi)(&mut hi, b.hi());
                        (lo, hi)
                    }
                    (
                        Group::Merged {
                            lo: mut a_lo,
                            hi: mut a_hi,
                        },
                        Group::Merged { lo: b_lo, hi: b_hi },
                    ) => {
                        (ops.assign_lo)(&mut a_lo, &b_lo);
                        (ops.assign_hi)(&mut a_hi, &b_hi);
                        (a_lo, a_hi)
                    }
                    // A lone input has weight zero, so the ordered closing
                    // reduction can place it only after a merged group. Keep
                    // the match total: the endpoint operations commute, so
                    // this order still produces the same value.
                    (Group::Input(a), Group::Merged { mut lo, mut hi }) => {
                        let a = a.span();
                        (ops.assign_lo)(&mut lo, a.lo());
                        (ops.assign_hi)(&mut hi, a.hi());
                        (lo, hi)
                    }
                }
            };
            Group::Merged { lo, hi }
        });
        match group.expect("the fold is seeded with the receiver: never empty") {
            // An empty iterator leaves only the receiver. Clone its endpoints
            // into the owned result; Version clones share storage.
            Group::Input(input) => {
                let s = input.span();
                (s.lo().clone(), s.hi().clone())
            }
            Group::Merged { lo, hi } => (lo, hi),
        }
    }
}

/// Endpoint operations for one kind of span fold.
///
/// Borrowed inputs produce a new endpoint; an owned intermediate result can
/// absorb the next input in place. Point spans have a separate combination
/// that avoids computing the same endpoint twice.
struct SpanFoldOps {
    /// Combine two borrowed lower endpoints into a fresh owned one.
    lo_refs: fn(&Version, &Version) -> Version,
    /// Combine two borrowed upper endpoints into a fresh owned one.
    hi_refs: fn(&Version, &Version) -> Version,
    /// Fold one borrowed lower endpoint into the owned lower endpoint.
    assign_lo: fn(&mut Version, &Version),
    /// Fold one borrowed upper endpoint into the owned upper endpoint.
    assign_hi: fn(&mut Version, &Version),
    /// Combine two point spans, taking advantage of their equal endpoints.
    points: fn(&Version, &Version) -> (Version, Version),
}

/// Enclose two point versions with their meet and join, computing both together.
fn union_points(a: &Version, b: &Version) -> (Version, Version) {
    a.hull(b)
}

/// Intersection's point-combine: two points share a version exactly when they
/// are equal.
///
/// Equal points retain that point. Unequal points produce reversed bounds:
/// their join is the lower endpoint and their meet is the upper endpoint.
/// Further intersections can only widen that reversal, so the fold's final
/// ordering check returns an empty intersection.
fn intersect_points(a: &Version, b: &Version) -> (Version, Version) {
    if a == b {
        return (a.clone(), a.clone());
    }
    (a.join(b), a.meet(b))
}

/// Combine two point spans under pointwise join.
///
/// Both output endpoints are the same join, so compute it once and share its
/// immutable storage.
fn join_points(a: &Version, b: &Version) -> (Version, Version) {
    let v = a.join(b);
    (v.clone(), v)
}

/// Combine two point spans under pointwise meet.
///
/// Both output endpoints are the same meet, so compute it once and share its
/// immutable storage.
fn meet_points(a: &Version, b: &Version) -> (Version, Version) {
    let v = a.meet(b);
    (v.clone(), v)
}

/// Endpoint operations for union: lower bounds meet and upper bounds join.
const UNION_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::meet,
    hi_refs: Version::join,
    assign_lo: |version, other| *version &= other,
    assign_hi: |version, other| *version |= other,
    points: union_points,
};

/// Endpoint operations for intersection: lower bounds join and upper bounds meet.
const INTERSECT_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::join,
    hi_refs: Version::meet,
    assign_lo: |version, other| *version |= other,
    assign_hi: |version, other| *version &= other,
    points: intersect_points,
};

/// Endpoint operations for pointwise join.
const JOIN_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::join,
    hi_refs: Version::join,
    assign_lo: |version, other| *version |= other,
    assign_hi: |version, other| *version |= other,
    points: join_points,
};

/// Endpoint operations for pointwise meet.
const MEET_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::meet,
    hi_refs: Version::meet,
    assign_lo: |version, other| *version &= other,
    assign_hi: |version, other| *version &= other,
    points: meet_points,
};

/// One input to a multi-span operation.
///
/// The receiver is borrowed directly. Iterator items retain the ownership form
/// supplied by the caller and are borrowed through [`Borrow`] only when read.
enum FoldInput<'r, 'i, T> {
    Receiver(&'r Span<'i>),
    Item(T),
}

impl<'i, 's, T: Borrow<Span<'s>>> FoldInput<'_, 'i, T> {
    /// The span this input contributes, borrowed.
    fn span<'x>(&'x self) -> &'x Span<'x>
    where
        'i: 'x,
        's: 'x,
    {
        match self {
            FoldInput::Receiver(s) => s,
            FoldInput::Item(t) => t.borrow(),
        }
    }
}

/// One group in the operators' balanced counter: an input exactly as the caller
/// supplied it, or the owned endpoints a combine produced.
enum Group<T> {
    /// An input the fold has not yet combined, still in the caller's
    /// form.
    Input(T),
    /// The owned running endpoints of one or more combines.
    Merged { lo: Version, hi: Version },
}

impl<'i, 's, T: Borrow<Span<'s>>> Group<FoldInput<'_, 'i, T>> {
    /// The shared endpoint when storage proves that this is a point span.
    fn point<'x>(&'x self) -> Option<&'x Version>
    where
        'i: 'x,
        's: 'x,
    {
        match self {
            Group::Input(input) => {
                let s = input.span();
                s.shared_endpoint()
            }
            Group::Merged { lo, hi } => lo.ptr_eq(hi).then_some(lo),
        }
    }
}

/// Implements a total span operator for owned and borrowed receivers.
///
/// The right operand may be any span-convertible value. The result owns its
/// endpoints, so the operand lifetimes remain independent.
macro_rules! span_total_binop_matrix {
    ($(#[$doc:meta])* $Op:ident::$op:ident, $method:ident) => {
        $(#[$doc])*
        impl<'a, 'b, T: Into<Span<'b>>> $Op<T> for Span<'a> {
            type Output = Span<'static>;
            fn $op(self, r: T) -> Span<'static> {
                Span::$method(&self, r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b, T: Into<Span<'b>>> $Op<T> for &Span<'a> {
            type Output = Span<'static>;
            fn $op(self, r: T) -> Span<'static> {
                Span::$method(self, r)
            }
        }
    };
}

/// Generates a total span operator with a version on the left.
///
/// Each implementation borrows the version through [`Span::at`], preserving
/// its point-span fast path. The left operand cannot be generic over
/// `Into<Span>`: that would also accept two versions and conflict with the
/// version lattice's `|` and `&` operators.
macro_rules! span_version_lhs_matrix {
    ($(#[$doc:meta])* $Op:ident::$op:ident, $method:ident) => {
        $(#[$doc])*
        impl<'b> $Op<Span<'b>> for Version {
            type Output = Span<'static>;
            fn $op(self, r: Span<'b>) -> Span<'static> {
                Span::$method(&Span::at(&self), r)
            }
        }
        $(#[$doc])*
        impl<'b> $Op<&Span<'b>> for Version {
            type Output = Span<'static>;
            fn $op(self, r: &Span<'b>) -> Span<'static> {
                Span::$method(&Span::at(&self), r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<Span<'b>> for &'a Version {
            type Output = Span<'static>;
            fn $op(self, r: Span<'b>) -> Span<'static> {
                Span::$method(&Span::at(self), r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<&Span<'b>> for &'a Version {
            type Output = Span<'static>;
            fn $op(self, r: &Span<'b>) -> Span<'static> {
                Span::$method(&Span::at(self), r)
            }
        }
    };
}

/// Implements a partial span operator for every owned/borrowed combination.
///
/// Unlike total operators, intersection accepts only actual spans. Implicitly
/// widening a version to a point would make disjointness too easy to overlook.
macro_rules! span_binop_matrix {
    ($(#[$doc:meta])* $Op:ident::$op:ident, $method:ident, $Out:ty) => {
        $(#[$doc])*
        impl<'a, 'b> $Op<Span<'b>> for Span<'a> {
            type Output = $Out;
            fn $op(self, r: Span<'b>) -> $Out {
                Span::$method(&self, &r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<&Span<'b>> for Span<'a> {
            type Output = $Out;
            fn $op(self, r: &Span<'b>) -> $Out {
                Span::$method(&self, r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<Span<'b>> for &Span<'a> {
            type Output = $Out;
            fn $op(self, r: Span<'b>) -> $Out {
                Span::$method(self, &r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<&Span<'b>> for &Span<'a> {
            type Output = $Out;
            fn $op(self, r: &Span<'b>) -> $Out {
                Span::$method(self, r)
            }
        }
    };
}

span_total_binop_matrix! {
    /// `a | b`: the *pointwise join*, the version lattice's `|` lifted to each
    /// endpoint pair.
    ///
    /// A [`Version`] right operand denotes its point span.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// // A subtree's bounds, after every member also absorbs b1:
    /// let advanced = &a1.span(&a2) | &b1;
    /// assert_eq!(*advanced.lo(), &a1 | &b1);
    /// assert_eq!(*advanced.hi(), &a2 | &b1);
    /// // The version was taken as its point span.
    /// assert_eq!(advanced, &a1.span(&a2) | &b1.span(&b1));
    /// ```
    BitOr::bitor, join
}

span_total_binop_matrix! {
    /// `a & b`: the *pointwise meet*, the version lattice's `&` lifted to each
    /// endpoint pair.
    ///
    /// A [`Version`] right operand denotes its point span, clamping both
    /// endpoints to its past.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// // Clamping a segment to a point's past:
    /// let clamped = &a2.span(&a3) & &a2;
    /// assert_eq!(clamped, a2.span(&a2));
    /// // Pointwise absorption: (a | b) & a == a.
    /// let (s, t) = (a1.span(&a2), a2.span(&a3));
    /// assert_eq!(&(&s | &t) & &s, s);
    /// ```
    BitAnd::bitand, meet
}

span_total_binop_matrix! {
    /// `a + b`: the *union*: the tightest span covering both operands.
    ///
    /// A [`Version`] right operand denotes its point span, so `span + &v`
    /// extends the span to cover `v`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// let ours = a1.span(&a2);
    /// let both = &ours + &b1;
    /// // The union covers both operands' whole segments…
    /// assert_eq!(*both.hi(), &a2 | &b1);
    /// // …and the version was taken as its point span, the same from either side.
    /// assert_eq!(both, &b1.span(&b1) + &ours);
    /// ```
    Add::add, union
}

span_version_lhs_matrix! {
    /// `v | s`: the *pointwise join* with a version on the left.
    ///
    /// The mirrored spelling of `s | &v`, with the version treated as its
    /// point span.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// // The same span from either side of the symbol.
    /// assert_eq!(&b1 | &a1.span(&a2), &a1.span(&a2) | &b1);
    /// ```
    BitOr::bitor, join
}

span_version_lhs_matrix! {
    /// `v & s`: the *pointwise meet* with a version on the left.
    ///
    /// The mirrored spelling of `s & &v`, with the version treated as its
    /// point span.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// // Clamping a segment to a point's past, spelled from the point.
    /// assert_eq!(&a2 & &a2.span(&a3), a2.span(&a2));
    /// assert_eq!(&a2 & &a2.span(&a3), &a2.span(&a3) & &a2);
    /// ```
    BitAnd::bitand, meet
}

span_version_lhs_matrix! {
    /// `v + s`: the *union* with a version on the left. It mirrors `s + &v`,
    /// treating the version as its point span, so the result covers `v` and
    /// all of `s`.
    ///
    /// No `Version + Version` exists: [`Sum`] for [`Version`] is the join fold,
    /// which a version-pair `+` would conceptually contradict. The smallest
    /// span containing two versions is [`span`](Version::span) (`v ^ w`).
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let mut bob = alice.fork();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let b1 = bob.tick().clone();
    ///
    /// // The same union from either side of the symbol.
    /// assert_eq!(&b1 + &a1.span(&a2), &a1.span(&a2) + &b1);
    /// ```
    Add::add, union
}

span_binop_matrix! {
    /// `a * b`: the *intersection*: the largest span covered by both operands,
    /// or [`None`] when they share no overlap.
    ///
    /// Alone among the span operators, `*` has no assigning form: `*=`
    /// returns nothing, so a disjoint pair would leave the [`None`] nowhere
    /// to land. Match on `a * b` instead and decide the miss explicitly.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// let head = a1.span(&a2);
    /// let tail = a2.span(&a3);
    /// let wide = a1.span(&a3);
    /// // Overlapping segments meet at their shared version…
    /// assert_eq!(&head * &tail, Some(a2.span(&a2)));
    /// // …a covered segment is absorbed…
    /// assert_eq!(&tail * &wide, Some(tail.clone()));
    /// // …and disjoint segments have no intersection.
    /// assert_eq!(&a1.span(&a1) * &tail, None);
    /// ```
    Mul::mul, intersect, Option<Span<'static>>
}

/// Implements an assigning span operator for any span-convertible right side.
///
/// `a ⊕= b` is exactly `a = a ⊕ b`, using the same kernel.
macro_rules! span_assign_matrix {
    ($(#[$doc:meta])* $Assign:ident::$assign:ident, $method:ident) => {
        $(#[$doc])*
        impl<'a, 'b, T: Into<Span<'b>>> $Assign<T> for Span<'a> {
            fn $assign(&mut self, r: T) {
                *self = Span::$method(self, r);
            }
        }
    };
}

span_assign_matrix! {
    /// `a |= b`: the *pointwise join* folded into the receiver.
    ///
    /// Exactly `a = a | b`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    BitOrAssign::bitor_assign, join
}

span_assign_matrix! {
    /// `a &= b`: the *pointwise meet* folded into the receiver.
    ///
    /// Exactly `a = a & b`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    BitAndAssign::bitand_assign, meet
}

span_assign_matrix! {
    /// `a += b`: the *union* folded into the receiver.
    ///
    /// Exactly `a = a + b`.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self| + |other|)`")]
    AddAssign::add_assign, union
}

/// Generates the union-fold collection impls for one item shape.
///
/// Summing or collecting an iterator of spans (or of versions, each treated as
/// its point span) yields their union (the fold of `+`) through the same
/// balanced n-ary fold as [`Span::union_all`].
///
/// The receiver is [`Option`] because union has no identity: the version
/// lattice has no top, so an empty iterator has no non-empty hull. `None`
/// means exactly "no spans came", never an empty union. Rust's coherence
/// rules prevent a blanket `impl<T: Into<Span>> Sum<T> for Option<Span>`:
/// its header has no local type (`Option` is foreign and `T` is uncovered).
/// The macro therefore emits the concrete owned and borrowed forms.
macro_rules! span_union_fold {
    ($(#[$doc:meta])* ($($lt:lifetime),*) $Item:ty) => {
        $(#[$doc])*
        impl<$($lt),*> Sum<$Item> for Option<Span<'static>> {
            fn sum<I: Iterator<Item = $Item>>(mut iter: I) -> Self {
                let first: Span<'_> = iter.next()?.into();
                Some(first.union_all(iter))
            }
        }

        $(#[$doc])*
        impl<$($lt),*> FromIterator<$Item> for Option<Span<'static>> {
            fn from_iter<I: IntoIterator<Item = $Item>>(iter: I) -> Self {
                iter.into_iter().sum()
            }
        }
    };
}

span_union_fold! {
    /// The union of every span in the iterator — the fold of `+`, run through
    /// the balanced n-ary fold of [`Span::union_all`] — or [`None`] on an
    /// empty iterator (union has no identity span).
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html")))]
    #[cfg_attr(not(doc), doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let a1 = a.tick().clone();
    /// let a2 = a.tick().clone();
    /// let a3 = a.tick().clone();
    ///
    /// let spans = [a1.span(&a2), a2.span(&a3)];
    /// // Sum and collect are the same union fold…
    /// let span: Option<Span> = spans.iter().sum();
    /// assert_eq!(span, Some(a1.span(&a3)));
    /// let span: Option<Span> = spans.into_iter().collect();
    /// assert_eq!(span, Some(a1.span(&a3)));
    /// // …and the empty iterator has no union.
    /// let empty: Option<Span> = std::iter::empty::<Span>().sum();
    /// assert_eq!(empty, None);
    /// ```
    ('a) Span<'a>
}

span_union_fold! {
    /// The union of every borrowed span in the iterator, or [`None`] when the
    /// iterator is empty.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html")))]
    #[cfg_attr(not(doc), doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count")]
    ('x, 'a) &'x Span<'a>
}

span_union_fold! {
    /// The tightest span covering every version in the iterator, each treated
    /// as its point span — the hull of the whole collection,
    /// mirroring [`Version::span_all`], or [`None`] on an empty iterator.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html")))]
    #[cfg_attr(not(doc), doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let a1 = a.tick().clone();
    /// let a2 = a.tick().clone();
    ///
    /// // Collecting versions yields their hull.
    /// let span: Option<Span> = [a1.clone(), a2.clone()].into_iter().collect();
    /// assert_eq!(span, Some(a1.span(&a2)));
    /// ```
    () Version
}

span_union_fold! {
    /// The tightest span covering every borrowed version in the iterator, or
    /// [`None`] when the iterator is empty.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html")))]
    #[cfg_attr(not(doc), doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count")]
    ('x) &'x Version
}

/// Generates the intersection-fold [`Product`] impl for one item shape:
/// multiplying out an iterator of spans yields their intersection — the fold
/// of `*` — through the same balanced n-ary fold as [`Span::intersect_all`].
///
/// [`None`] covers both an empty iterator (intersection has no identity: the
/// version lattice has no top, so no span is covered by every span) and a
/// nonempty family sharing no version — the two ways there is no product.
/// Items are true [`Span`]s rather than implicitly widened versions: a point
/// item would otherwise empty the product unless every input contained it.
macro_rules! span_intersect_fold {
    ($(#[$doc:meta])* ($($lt:lifetime),*) $Item:ty) => {
        $(#[$doc])*
        impl<$($lt),*> Product<$Item> for Option<Span<'static>> {
            fn product<I: Iterator<Item = $Item>>(mut iter: I) -> Self {
                let first = iter.next()?;
                first.borrow().intersect_all(iter)
            }
        }
    };
}

span_intersect_fold! {
    /// The intersection of every span in the iterator — the fold of `*`, run
    /// through the balanced n-ary fold of [`Span::intersect_all`] — or
    /// [`None`] on an empty iterator or an empty intersection.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect_all.html")))]
    #[cfg_attr(not(doc), doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let a1 = a.tick().clone();
    /// let a2 = a.tick().clone();
    /// let a3 = a.tick().clone();
    ///
    /// let spans = [a1.span(&a3), a2.span(&a3)];
    /// // The product is the shared segment…
    /// let shared: Option<Span> = spans.iter().product();
    /// assert_eq!(shared, Some(a2.span(&a3)));
    /// // …disjoint spans have none…
    /// let disjoint: Option<Span> = [a1.span(&a1), a2.span(&a3)].iter().product();
    /// assert_eq!(disjoint, None);
    /// // …and neither does the empty iterator.
    /// let empty: Option<Span> = std::iter::empty::<Span>().product();
    /// assert_eq!(empty, None);
    /// ```
    ('a) Span<'a>
}

span_intersect_fold! {
    /// The intersection of every borrowed span in the iterator, or [`None`] on
    /// an empty iterator or an empty intersection.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect_all.html")))]
    #[cfg_attr(not(doc), doc = "`O(n log n)` in total input bytes; `O((|self| + |iter|) log k)` time, `k` the operand count")]
    ('x, 'a) &'x Span<'a>
}
