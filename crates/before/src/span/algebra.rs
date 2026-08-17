//! The span algebra: four binary operators over owned and borrowed [`Span`]
//! operands, each with a receiver-seeded variadic form running one balanced
//! two-sided fold.
//!
//! The operators are the joins and meets of the two lattice structures spans
//! carry (the [`Span`] docs place them side by side). The
//! pointwise pair wears the version lattice's own symbols — `|` and `&` on
//! spans are exactly `|` and `&` on versions, lifted endpointwise — while the
//! containment pair's novel semantics get the novel symbols `+` and `*`.
//! Every operator folds through its `lo` and `hi` legs; the four operators
//! are exactly the four assignments of the two lattice directions to the two
//! legs:
//!
//! ```text
//!                lo leg      hi leg      total?
//!   + union       meet        join       yes (containment join)
//!   * intersect   join        meet       no  (containment meet)
//!   | join        join        join       yes (pointwise join)
//!   & meet        meet        meet       yes (pointwise meet)
//! ```
//!
//! Totality arguments, once: union's `lo` only descends below `a.lo` and its
//! `hi` only ascends above `a.hi`, so the output pair stays ordered; the
//! pointwise join's `hi` (a join of upper endpoints) bounds both lower
//! endpoints from above, so it bounds their join, and the pointwise meet
//! dually. Intersect is the one genuinely partial operator: its `lo` ascends while
//! its `hi` descends, and the pair crosses exactly when the spans share no
//! version.
//!
//! The three total operations also assign in place (`+=`, `|=`, `&=`), each
//! cell the value kernel written back to the receiver. Partiality is why `*`
//! has no `*=`: an assigning operator returns nothing, so a disjoint pair would
//! leave the miss nowhere to land.
//!
//! The total operators take any span-convertible operand (`impl Into<Span>`) on
//! the non-receiver side — a [`Version`] is taken as its coincident point span —
//! in the binary, assigning, variadic, and collection forms alike, and
//! mirrored cells accept a bare version on the *left* of a span (`v + s`,
//! `v | s`, `v & s`) with the same meaning. The one deliberate hole in the
//! symmetry is a version pair: `v | w` and `v & w` keep the version
//! lattice's own meaning, and `v + w` stays absent because [`Sum`] for
//! [`Version`] is the join fold, which a version-pair `+` would contradict
//! (the hull of two versions is `v ^ w`). The
//! partial operator alone stays span-only, in every form: a point operand
//! intersects to that point when the other operand contains it and to
//! [`None`] otherwise, which is too easy to reach silently through an
//! implicit conversion — an intersecting caller spells [`Span::at`]
//! (`Span::at(&v)`) so the vanishing case is a visible choice.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Mul};

use crate::codec;
use crate::Version;

use super::Span;

impl<'a> Span<'a> {
    /// The *union* of `self` and `other`: the tightest [`Span`] covering both.
    ///
    /// The method spelling of `self + other`. The argument is anything
    /// [`Into`] a [`Span`]; a [`Version`] is taken as its coincident point span,
    /// so the union with a version is the span extended to cover it.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html"))]
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
        union_core(self, &other.into())
    }

    /// The tightest [`Span`] covering every input (including `self`).
    ///
    /// Items are anything [`Into`] a [`Span`]; a [`Version`] is taken as its
    /// coincident point span.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
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
    /// // An empty iterator settles the receiver.
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
    /// The method spelling of `self * other`. Alone among the span operators this
    /// takes only a true [`Span`], never an implicit conversion: a point
    /// operand intersects to that point when the other operand contains it
    /// and to [`None`] otherwise, too easy a silent vanish to reach through
    /// a coercion — spell [`Span::at`] to intersect with a version.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect.html"))]
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
        intersect_core(self, other)
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect_all.html"))]
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
        match lo.partial_cmp(&hi) {
            Some(Ordering::Less | Ordering::Equal) => Some(Span::owned(lo, hi)),
            Some(Ordering::Greater) | None => None,
        }
    }

    /// The *pointwise join* of `self` and `other`: the version lattice's join
    /// applied to each endpoint pair.
    ///
    /// The method spelling of `self | other`, mirroring [`Version::join`].
    /// The argument is anything [`Into`] a [`Span`]; a [`Version`] is taken as
    /// its coincident point span, joining into both endpoints.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html"))]
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
        join_core(self, &other.into())
    }

    /// The [`Span`] whose lower and upper bounds are, respectively, the joins
    /// of the lower and upper bounds of `self` and all the spans in `iter`,
    /// mirroring [`Version::join_all`].
    ///
    /// Items are anything [`Into`] a [`Span`]; a [`Version`] is taken as its
    /// coincident point span.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join_all.html"))]
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
    /// The argument is anything [`Into`] a [`Span`]; a [`Version`] is taken as
    /// its coincident point span, clamping both endpoints to its past.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html"))]
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
        meet_core(self, &other.into())
    }

    /// The [`Span`] whose lower and upper bounds are, respectively, the meets
    /// of the lower and upper bounds of `self` and all the spans in `iter`,
    /// mirroring [`Version::meet_all`].
    ///
    /// Items are anything [`Into`] a [`Span`]; a [`Version`] is taken as its
    /// coincident point span.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet_all.html"))]
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

    /// The operators' shared balanced fold: `{self} ∪ iter` reduced through
    /// [`crate::fold::balanced_reduce`] with a two-sided accumulator, per-operator
    /// leg kernels, and the point-combine fast path.
    ///
    /// Adjacent clone-identical inputs collapse before the counter reads them
    /// (all four operators are idempotent — the binary operators' laws — so a run
    /// of one shared span is one input); the receiver enters as the first input,
    /// so the fold is never empty. Inputs enter untouched and are cloned only
    /// at their first combine, and every clone of a stored version is a
    /// refcount bump, never a byte copy.
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
                let dup = last.as_ref().is_some_and(|(lo, hi)| {
                    lo.view().ptr_eq(s.lo().view()) && hi.view().ptr_eq(s.hi().view())
                });
                if !dup {
                    last = Some((s.lo().clone(), s.hi().clone()));
                }
                !dup
            })
            .map(Group::Input);
        let group = crate::fold::balanced_reduce(inputs, |a, b| {
            // The point-combine: when both sides read one stream each (a
            // coincident input, or a merged group whose legs settled on one
            // shared buffer), the operator's fused kernel answers in one walk what
            // the per-leg folds would walk twice. Clone identity is the
            // certificate, so the check itself is O(1).
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
                        (ops.lo_view)(&mut lo, b.lo().view());
                        (ops.hi_view)(&mut hi, b.hi().view());
                        (lo, hi)
                    }
                    (
                        Group::Merged {
                            lo: mut a_lo,
                            hi: mut a_hi,
                        },
                        Group::Merged { lo: b_lo, hi: b_hi },
                    ) => {
                        (ops.lo_view)(&mut a_lo, b_lo.view());
                        (ops.hi_view)(&mut a_hi, b_hi.view());
                        (a_lo, a_hi)
                    }
                    // Unreachable through the counter's weight discipline (a
                    // weight-0 lone input never sits below a merged group in
                    // the closing drain), but the match stays total rather than
                    // asserting: every leg kernel is commutative, so folding
                    // the raw input into the owned group is value-identical.
                    (Group::Input(a), Group::Merged { mut lo, mut hi }) => {
                        let a = a.span();
                        (ops.lo_view)(&mut lo, a.lo().view());
                        (ops.hi_view)(&mut hi, a.hi().view());
                        (lo, hi)
                    }
                }
            };
            Group::Merged { lo, hi }
        });
        match group.expect("the fold is seeded with the receiver: never empty") {
            // The receiver alone (an empty iterator): settle its
            // endpoints owned, each an O(1) buffer-sharing clone.
            Group::Input(input) => {
                let s = input.span();
                (s.lo().clone(), s.hi().clone())
            }
            Group::Merged { lo, hi } => (lo, hi),
        }
    }
}

/// One n-ary span operator's kernels. Each operator folds through its `lo` and `hi`
/// legs; the module doc carries the four leg assignments and their totality
/// arguments.
struct SpanFoldOps {
    /// Combine two borrowed lower endpoints into a fresh owned one.
    lo_refs: fn(&Version, &Version) -> Version,
    /// Combine two borrowed upper endpoints into a fresh owned one.
    hi_refs: fn(&Version, &Version) -> Version,
    /// Fold one borrowed stream into the owned `lo` leg in place.
    lo_view: fn(&mut Version, &codec::Bits),
    /// Fold one borrowed stream into the owned `hi` leg in place.
    hi_view: fn(&mut Version, &codec::Bits),
    /// The fused point-combine: both sides read one stream each, so
    /// one walk answers both legs (each operator's kernel is named at its
    /// constant).
    points: fn(&Version, &Version) -> (Version, Version),
}

/// Union's point-combine: two points' union is their hull: one fused pair walk
/// through [`Version::span_refs`]'s ladder, fast paths and traffic accounting
/// included.
fn union_points(a: &Version, b: &Version) -> (Version, Version) {
    Version::span_refs(a, b)
}

/// Intersection's point-combine: two points share a version exactly when they
/// are equal.
///
/// One byte compare answers the only nonempty case; an unequal pair pays the
/// per-leg walks whose crossed output the operator's final validation rejects (or a
/// later combine absorbs).
fn intersect_points(a: &Version, b: &Version) -> (Version, Version) {
    if codec::canonical_eq(a.view(), b.view()) {
        return (a.clone(), a.clone());
    }
    (Version::join_refs(a, b), Version::meet_refs(a, b))
}

/// The pointwise join's point-combine: the legs read the same operand pair, so
/// one join walk feeds both, the result stored twice (clones share the
/// buffer, keeping the group point-like).
fn join_points(a: &Version, b: &Version) -> (Version, Version) {
    let v = Version::join_refs(a, b);
    (v.clone(), v)
}

/// The pointwise meet's point-combine: dually to [`join_points`], one meet
/// walk feeds both legs.
fn meet_points(a: &Version, b: &Version) -> (Version, Version) {
    let v = Version::meet_refs(a, b);
    (v.clone(), v)
}

/// Union: meets meet, joins join.
const UNION_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::meet_refs,
    hi_refs: Version::join_refs,
    lo_view: Version::meet_view,
    hi_view: Version::join_view,
    points: union_points,
};

/// Intersection: meets join, joins meet.
const INTERSECT_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::join_refs,
    hi_refs: Version::meet_refs,
    lo_view: Version::join_view,
    hi_view: Version::meet_view,
    points: intersect_points,
};

/// Pointwise join: both legs join.
const JOIN_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::join_refs,
    hi_refs: Version::join_refs,
    lo_view: Version::join_view,
    hi_view: Version::join_view,
    points: join_points,
};

/// Pointwise meet: both legs meet.
const MEET_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::meet_refs,
    hi_refs: Version::meet_refs,
    lo_view: Version::meet_view,
    hi_view: Version::meet_view,
    points: meet_points,
};

/// One input to the operators' shared fold: the receiver enters by reference beside
/// the caller's items, whatever ownership they carry (owned or borrowed through
/// [`Borrow`], never cloned on entry).
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
    /// The one stream a point-like group reads, when it is one: a coincident
    /// input span, or a merged group whose legs settled on one shared buffer.
    /// Clone identity is the certificate — `O(1)`, never a walk.
    fn point<'x>(&'x self) -> Option<&'x Version>
    where
        'i: 'x,
        's: 'x,
    {
        match self {
            Group::Input(input) => {
                let s = input.span();
                s.is_coincident().then(|| s.lo())
            }
            Group::Merged { lo, hi } => lo.view().ptr_eq(hi.view()).then_some(lo),
        }
    }
}

/// `a + b`'s kernel: the containment join over borrowed operands.
fn union_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
    if a.is_coincident() && b.is_coincident() {
        // Two points' union is their hull: one fused pair walk (Version::span's
        // ladder, fast paths and traffic accounting included) where the per-leg
        // folds below would walk the same operand pair twice.
        return a.lo().span(b.lo());
    }
    let mut lo = a.lo().clone(); // O(1): a stored version's clone shares its buffer
    let mut hi = a.hi().clone();
    lo.meet_view(b.lo().view());
    hi.join_view(b.hi().view());
    // Ordered by construction: `lo` only descended below `a`'s meet, `hi` only
    // ascended above `a`'s join, and `a`'s endpoints are ordered.
    Span::owned(lo, hi)
}

/// `a * b`'s kernel: the containment meet over borrowed operands, or [`None`]
/// where the segments share no version.
fn intersect_core(a: &Span<'_>, b: &Span<'_>) -> Option<Span<'static>> {
    if a.is_coincident() && b.is_coincident() {
        // Two points share a version exactly when they are equal: one byte
        // compare, no walk.
        return codec::canonical_eq(a.lo().view(), b.lo().view())
            .then(|| Span::owned(a.lo().clone(), a.lo().clone()));
    }
    let mut lo = a.lo().clone();
    let mut hi = a.hi().clone();
    lo.join_view(b.lo().view());
    hi.meet_view(b.hi().view());
    // The one genuinely partial operator: the joined meets must still sit under the
    // met joins, and the pair crosses exactly when the segments are disjoint.
    match lo.partial_cmp(&hi) {
        Some(Ordering::Less | Ordering::Equal) => Some(Span::owned(lo, hi)),
        Some(Ordering::Greater) | None => None,
    }
}

/// `a | b`'s kernel: the pointwise join over borrowed operands.
fn join_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
    if a.is_coincident() && b.is_coincident() {
        // On points the pointwise operation is exactly the version operator: one walk feeds
        // both endpoints, stored twice (the clones share one buffer, keeping
        // the result coincident).
        let v = Version::join_refs(a.lo(), b.lo());
        return Span::owned(v.clone(), v);
    }
    let mut lo = a.lo().clone();
    let mut hi = a.hi().clone();
    lo.join_view(b.lo().view());
    hi.join_view(b.hi().view());
    // Ordered by construction: `hi` bounds every operand endpoint from above,
    // the joined meets included.
    Span::owned(lo, hi)
}

/// `a & b`'s kernel: the pointwise meet over borrowed operands, dually to
/// [`join_core`] in every clause.
fn meet_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
    if a.is_coincident() && b.is_coincident() {
        let v = Version::meet_refs(a.lo(), b.lo());
        return Span::owned(v.clone(), v);
    }
    let mut lo = a.lo().clone();
    let mut hi = a.hi().clone();
    lo.meet_view(b.lo().view());
    hi.meet_view(b.hi().view());
    Span::owned(lo, hi)
}

/// Generates one *total* span operator's value matrix: two cells (owned and
/// borrowed receiver), each taking any span-convertible right operand and
/// making one call into the operator's borrowed-operand kernel.
///
/// The operand lifetimes are independent, and the output is always `'static`
/// (owned endpoints), so the operators compose freely in fold position. The
/// assign cells are `span_assign_matrix!`'s. The partial operator uses the
/// concrete `span_binop_matrix!` instead: it refuses the `Into` widening
/// (the module doc carries the argument), so it cannot share these cells.
macro_rules! span_total_binop_matrix {
    ($(#[$doc:meta])* $Op:ident::$op:ident, $core:ident) => {
        $(#[$doc])*
        impl<'a, 'b, T: Into<Span<'b>>> $Op<T> for Span<'a> {
            type Output = Span<'static>;
            fn $op(self, r: T) -> Span<'static> {
                $core(&self, &r.into())
            }
        }
        $(#[$doc])*
        impl<'a, 'b, T: Into<Span<'b>>> $Op<T> for &Span<'a> {
            type Output = Span<'static>;
            fn $op(self, r: T) -> Span<'static> {
                $core(self, &r.into())
            }
        }
    };
}

/// Generates one total span operator's mirrored cells: a bare version on the
/// left of a span, taken as its coincident point span.
///
/// Four cells (lhs over `{Version, &Version}` × rhs over `{Span, &Span}`),
/// each one call into the operator's borrowed-operand kernel through
/// [`Span::at`] (an `O(1)` borrow that keeps the coincidence certificate, so
/// the point fast paths still fire). These cells are concrete where the
/// span-receiver cells are generic: a `T: Into<Span>` *left* operand would
/// admit a version pair, and `v | w` and `v & w` must keep meaning what the
/// version lattice says, while `v + w` must stay absent — `Sum` for
/// [`Version`] is the join fold, which a version-pair `+` would contradict.
macro_rules! span_version_lhs_matrix {
    ($(#[$doc:meta])* $Op:ident::$op:ident, $core:ident) => {
        $(#[$doc])*
        impl<'b> $Op<Span<'b>> for Version {
            type Output = Span<'static>;
            fn $op(self, r: Span<'b>) -> Span<'static> {
                $core(&Span::at(&self), &r)
            }
        }
        $(#[$doc])*
        impl<'b> $Op<&Span<'b>> for Version {
            type Output = Span<'static>;
            fn $op(self, r: &Span<'b>) -> Span<'static> {
                $core(&Span::at(&self), r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<Span<'b>> for &'a Version {
            type Output = Span<'static>;
            fn $op(self, r: Span<'b>) -> Span<'static> {
                $core(&Span::at(self), &r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<&Span<'b>> for &'a Version {
            type Output = Span<'static>;
            fn $op(self, r: &Span<'b>) -> Span<'static> {
                $core(&Span::at(self), r)
            }
        }
    };
}

/// Generates the *partial* span operator's value matrix over owned and
/// borrowed operands: four cells (lhs × rhs over `{Span, &Span}`), every cell
/// one call into the operator's borrowed-operand kernel.
///
/// Only the intersection uses this concrete matrix: it deliberately takes
/// true spans, never an `Into` conversion (the module doc carries the
/// argument), where the total operators use `span_total_binop_matrix!`'s
/// widened cells.
macro_rules! span_binop_matrix {
    ($(#[$doc:meta])* $Op:ident::$op:ident, $core:ident, $Out:ty) => {
        $(#[$doc])*
        impl<'a, 'b> $Op<Span<'b>> for Span<'a> {
            type Output = $Out;
            fn $op(self, r: Span<'b>) -> $Out {
                $core(&self, &r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<&Span<'b>> for Span<'a> {
            type Output = $Out;
            fn $op(self, r: &Span<'b>) -> $Out {
                $core(&self, r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<Span<'b>> for &Span<'a> {
            type Output = $Out;
            fn $op(self, r: Span<'b>) -> $Out {
                $core(self, &r)
            }
        }
        $(#[$doc])*
        impl<'a, 'b> $Op<&Span<'b>> for &Span<'a> {
            type Output = $Out;
            fn $op(self, r: &Span<'b>) -> $Out {
                $core(self, r)
            }
        }
    };
}

span_total_binop_matrix! {
    /// `a | b`: the *pointwise join*, the version lattice's `|` lifted to each
    /// endpoint pair.
    ///
    /// The right operand is anything [`Into`] a [`Span`]; a [`Version`] is taken as
    /// as its coincident point span.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html"))]
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
    BitOr::bitor, join_core
}

span_total_binop_matrix! {
    /// `a & b`: the *pointwise meet*, the version lattice's `&` lifted to each
    /// endpoint pair.
    ///
    /// The right operand is anything [`Into`] a [`Span`]; a [`Version`] is taken as
    /// as its coincident point span, clamping both endpoints to its past.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html"))]
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
    BitAnd::bitand, meet_core
}

span_total_binop_matrix! {
    /// `a + b`: the *union*: the tightest span covering both operands.
    ///
    /// The right operand is anything [`Into`] a [`Span`]; a [`Version`] is taken as
    /// as its coincident point span, so `span + &v` extends the span to
    /// cover `v`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html"))]
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
    Add::add, union_core
}

span_version_lhs_matrix! {
    /// `v | s`: the *pointwise join* with a version on the left.
    ///
    /// The mirrored spelling of `s | &v`, the version taken as its coincident
    /// point span.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html"))]
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
    BitOr::bitor, join_core
}

span_version_lhs_matrix! {
    /// `v & s`: the *pointwise meet* with a version on the left.
    ///
    /// The mirrored spelling of `s & &v`, the version taken as its coincident
    /// point span.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html"))]
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
    BitAnd::bitand, meet_core
}

span_version_lhs_matrix! {
    /// `v + s`: the *union* with a version on the left — the mirrored
    /// spelling of `s + &v`, the version taken as its coincident point span,
    /// so the result covers `v` and all of `s`.
    ///
    /// No `Version + Version` exists: [`Sum`] for [`Version`] is the join fold,
    /// which a version-pair `+` would conceptually contradict. The smallest
    /// span containing [`Span`] two versions is [`span`](Version::span) (`v ^
    /// w`).
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html"))]
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
    Add::add, union_core
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect.html"))]
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
    Mul::mul, intersect_core, Option<Span<'static>>
}

/// Generates one total span operator's assign cell: any span-convertible
/// right operand, the value kernel written back to the receiver.
///
/// `a ⊕= b` is `a = a ⊕ b` exactly — the same kernel, fast paths and cost
/// included. The intersection has no assign cell; the module doc carries
/// the argument.
macro_rules! span_assign_matrix {
    ($(#[$doc:meta])* $Assign:ident::$assign:ident, $core:ident) => {
        $(#[$doc])*
        impl<'a, 'b, T: Into<Span<'b>>> $Assign<T> for Span<'a> {
            fn $assign(&mut self, r: T) {
                *self = $core(self, &r.into());
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_join.html"))]
    BitOrAssign::bitor_assign, join_core
}

span_assign_matrix! {
    /// `a &= b`: the *pointwise meet* folded into the receiver.
    ///
    /// Exactly `a = a & b`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_meet.html"))]
    BitAndAssign::bitand_assign, meet_core
}

span_assign_matrix! {
    /// `a += b`: the *union* folded into the receiver.
    ///
    /// Exactly `a = a + b`.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union.html"))]
    AddAssign::add_assign, union_core
}

/// Generates the union-fold collection impls for one item shape.
///
/// Summing or collecting an iterator of spans (or of versions, each taken
/// as its coincident point span) yields their union — the fold of `+` —
/// through the same balanced n-ary fold as [`Span::union_all`].
///
/// The receiver is [`Option`] because union has no identity: the version
/// lattice has no top, so an empty iterator has no non-empty hull. `None`
/// means exactly "no spans came", never an empty union. The item shapes
/// are enumerated concretely because coherence demands it: a blanket
/// `impl<T: Into<Span>> Sum<T> for Option<Span>` has no local type in its
/// header (`Option` is foreign and `T` is uncovered), so each span-convertible
/// item type carries its own cell.
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
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
    /// The union of every borrowed span in the iterator; see the owned impl.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
    ('x, 'a) &'x Span<'a>
}

span_union_fold! {
    /// The tightest span covering every version in the iterator, each taken
    /// as its coincident point span — the hull of the whole collection,
    /// mirroring [`Version::span_all`], or [`None`] on an empty iterator.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
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
    /// The tightest span covering every borrowed version in the iterator;
    /// see the owned impl.
    ///
    /// # Complexity
    ///
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
    ('x) &'x Version
}

/// Generates the intersection-fold [`Product`] impl for one item shape:
/// multiplying out an iterator of spans yields their intersection — the fold
/// of `*` — through the same balanced n-ary fold as [`Span::intersect_all`].
///
/// [`None`] covers both an empty iterator (intersection has no identity: the
/// version lattice has no top, so no span is covered by every span) and a
/// nonempty family sharing no version — the two ways there is no product.
/// Like every intersection form, items are true [`Span`]s only, never
/// versions: a point item would silently empty the product unless every
/// input contains it (the module doc carries the argument).
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
    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_intersect_all.html"))]
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
    /// The intersection of every borrowed span in the iterator; see the owned
    /// impl.
    ('x, 'a) &'x Span<'a>
}
