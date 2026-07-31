//! Causal spans: ordered pairs of concrete [`Version`]s, the chain
//! segments between them, and the algebra that combines them.
//!
//! A *span* is two concrete versions `lo <= hi` and the chain segment
//! between them — a genuinely different object from a
//! [`causally::Range`](crate::causally::Range): a range's bounds are
//! down-set cut-points with inclusivity kinds, so its verdicts fold
//! range semantics; a span is the ordered pair itself, and every
//! verdict about it is a raw order fact against the endpoints, with no
//! inclusivity to fold. [`Span::place`] answers the placement question
//! at the finest resolution the partial order admits — the nine
//! [`Placement`] regions — and [`Span::dominance_of`] coarsens it to
//! the three-way [`Dominance`] verdict a filter over version-bounded
//! regions consumes. Every nonempty collection of versions has a
//! tightest containing span — its lattice hull — derived by
//! [`Version::span`] and [`Version::span_all`], the total construction
//! doors living beside the join/meet family they compose.
//!
//! # The span algebra
//!
//! Spans carry two lattice structures, and the operator vocabulary
//! splits along them:
//!
//! - **The containment order** (set-like symbols): `a | b` is the
//!   *union* — the tightest span covering both, endpoints
//!   `[lo_a ∧ lo_b, hi_a ∨ hi_b]`, total by construction — and
//!   `a & b` is the *intersection* — the chain segment common to
//!   both, endpoints `[lo_a ∨ lo_b, hi_a ∧ hi_b]`, [`None`] when the
//!   spans share no version.
//! - **The pointwise order** (arithmetic symbols): `a + b` and
//!   `a * b` lift the version lattice itself to spans — `a + b` is
//!   the span of possible joins (`{v_a ∨ v_b}` for `v_a` in `a`,
//!   `v_b` in `b`, endpoints `[lo_a ∨ lo_b, hi_a ∨ hi_b]`) and
//!   `a * b` the span of possible meets
//!   (`[lo_a ∧ lo_b, hi_a ∧ hi_b]`), both total. On coincident
//!   spans they restrict to the version operators exactly
//!   (`[a, a] + [b, b] == [a ∨ b, a ∨ b]`), where `|` on two points
//!   yields their hull instead — that contrast is the reason the
//!   pointwise pair wears arithmetic symbols: join is already the
//!   crate's summation monoid (`Version: Sum` sums by join), and
//!   meet distributes over it.
//!
//! Each operator has a receiver-seeded n-ary door in the
//! [`Version::span_all`] idiom — [`Span::union_all`],
//! [`Span::intersect_all`], [`Span::sum_all`], [`Span::product_all`]
//! — one balanced fold each, total in arity because the receiver is
//! the guaranteed first input (so `intersect_all`'s [`None`] means
//! exactly one thing: an empty intersection). Deliberately absent:
//! assign forms (`|=` and the rest) — `&`'s partial verdict cannot
//! assign, an asymmetric assign set would mislead, and endpoints are
//! `O(1)` handles, so `a = &a | &b` costs the same.
//!
//! A span also projects onto a party: `&span / &party` is
//! [`OwnSpan`], the lazy quotient view — both endpoints projected,
//! placement answered without materializing, in
//! [`OwnVersion`]'s idiom.
//!
//! # The span wire form
//!
//! A [`Span`] has a canonical byte encoding: the meet's
//! [`Version::encode`] bytes, then the join's. Each component is
//! byte-aligned, independently canonical, and self-delimiting, so the
//! two concatenate with no length prefix —
//! [`Clock::encode`](crate::Clock::encode)'s framing rule. Decoding is
//! **one forward pass**: [`Span::decode`] parses the first version,
//! then parses the second while validating, in the same walk, that it
//! dominates the first — so a span parsed from the wire is valid by
//! construction, with no separate validation step for a loader to
//! forget. A composite whose well-formed components no encode ever
//! pairs — a crossed or concurrent pair — is rejected as
//! [`Decode::NotCanonical`], the genre for well-formed structure that
//! is the canonical spelling of no value (the
//! [`Ranked`](crate::Ranked) composite key's rank-mismatch precedent).
//!
//! Deliberately absent from the format: a discriminated short form for
//! the coincident span. `lo == hi` encodes both endpoints in full — a
//! flag would tax every span's wire size and every decode's parse for
//! the one case that is already the cheapest to carry, and canonicality
//! stays trivial (one spelling per span, byte equality exactly span
//! equality) with no flag-consistency rule to enforce.

use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::ops::{Add, BitAnd, BitOr, Div, Mul};

use crate::codec;
use crate::error::{Crossed, Decode};
use crate::version::skyline;
use crate::version::skyline::place;
use crate::{OwnVersion, Party, Version};

#[cfg(test)]
mod tests;

/// A causal span: an ordered pair of versions `lo <= hi` and the
/// chain segment between them.
///
/// A genuinely different object from [`Range`](crate::causally::Range):
/// a range's bounds are
/// down-set cut-points with inclusivity kinds, so its verdicts fold
/// range semantics; a span is two *concrete versions*, and every
/// verdict about it is a raw order fact against the endpoints, with no
/// inclusivity to fold. [`place`](Self::place) answers the placement
/// question at full resolution — the nine [`Placement`] regions — and
/// [`dominance_of`](Self::dominance_of) coarsens it to the three-way
/// [`Dominance`] verdict.
///
/// Construction's doors: [`new`](Self::new) validates the
/// pair, rejecting a reversed or incomparable one with [`Crossed`];
/// [`new_unchecked`](Self::new_unchecked) trusts a caller who already holds
/// `lo <= hi` structurally and skips the validating comparison;
/// [`at`](Self::at) (or its `From<Version>` and `From<&Version>`
/// spellings) builds the
/// coincident span at one version, total because a point needs no
/// ordering; and
/// the derived constructors on [`Version`] — [`span`](Version::span)
/// and [`span_all`](Version::span_all), beside the join/meet family
/// they compose — *derive* the span as a collection's lattice hull,
/// total where the validating door must reject and the trusted one
/// must trust. Every named door takes its version operands as
/// anything [`Into`] a [`Cow`] of [`Version`], so owned and borrowed
/// endpoints mix freely and no per-ownership door variants exist. An existing span
/// pays no door twice: [`reborrow`](Self::reborrow) hands out a
/// shorter-lived span over the same endpoints, and
/// [`into_owned`](Self::into_owned) settles the borrows so the span
/// outlives them — both carry `lo <= hi` through from the source, so
/// neither opens an unvalidated construction path.
///
/// # Complexity
///
/// Constructors and accessors are `O(1)` ([`new`](Self::new) pays its
/// one validating comparison; endpoints are borrows or `O(1)`
/// buffer-sharing clones). Each binary operator (`|`, `&`, `+`, `*`)
/// runs its legs once over the operands' packed endpoints, and a
/// point-like operand pair fuses to a single walk.
///
/// **Complexity**: operators `O(a + b)` in the operands' packed sizes; constructors and accessors `O(1)`, plus `new`'s one validating comparison.
///
/// ```
/// use before::{Clock, causally::{Dominance, Endpoint, Span, Placement}};
///
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let a3 = alice.tick().clone();
/// let b1 = bob.tick().clone(); // concurrent to alice's whole line
///
/// let span = Span::new(&a1, &a3).unwrap();
/// assert_eq!(span.place(&a2), Placement::Between);
/// assert_eq!(span.place(&b1), Placement::Concurrent(Endpoint::Both));
/// // A reversed or incomparable pair is not a span.
/// assert!(Span::new(&a3, &a1).is_err());
/// assert!(Span::new(&a1, &b1).is_err());
/// // The dominance coarsening: a3 dominates the whole span, a2
/// // only its start, and b1 not even that.
/// assert_eq!(span.dominance_of(&a3), Dominance::After);
/// assert_eq!(span.dominance_of(&a2), Dominance::Between);
/// assert_eq!(span.dominance_of(&b1), Dominance::Before);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span<'a> {
    lo: Cow<'a, Version>,
    hi: Cow<'a, Version>,
}

impl<'a> Span<'a> {
    /// The validating door: the span `[lo, hi]`, checking that the
    /// pair is ordered.
    ///
    /// Each endpoint is anything [`Into`] a [`Cow`] of [`Version`] —
    /// owned and borrowed versions mix freely, borrows lent and owned
    /// values moved, so no door variant per ownership pattern exists
    /// to choose between.
    ///
    /// # Errors
    ///
    /// [`Crossed`] unless `lo <= hi` — a reversed pair is rejected, and
    /// so is an incomparable one, where neither version bounds the
    /// other and no chain segment exists between them.
    pub fn new(
        lo: impl Into<Cow<'a, Version>>,
        hi: impl Into<Cow<'a, Version>>,
    ) -> Result<Self, Crossed> {
        let (lo, hi) = (lo.into(), hi.into());
        match lo.as_ref().partial_cmp(hi.as_ref()) {
            Some(Ordering::Less | Ordering::Equal) => Ok(Self { lo, hi }),
            Some(Ordering::Greater) | None => Err(Crossed),
        }
    }

    /// The trusted door — [`new`](Self::new) without the validating
    /// comparison: the span `[lo, hi]` from a caller who already
    /// holds `lo <= hi`.
    ///
    /// For callers whose pairs are ordered *structurally* — a floor and
    /// ceiling maintained as meet and join of one underlying set, say —
    /// where re-validating per construction would pay a causal
    /// comparison for a fact already invariant. Everyone else should
    /// use [`new`](Self::new).
    ///
    /// The caller **must** guarantee `lo <= hi`. On a pair that
    /// violates it, every verdict of [`place`](Self::place) and
    /// [`dominance_of`](Self::dominance_of) is unspecified and
    /// meaningless.
    ///
    /// # Panics
    ///
    /// Debug builds assert `lo <= hi` and panic when it fails; release
    /// builds construct the span unchecked.
    pub fn new_unchecked(lo: impl Into<Cow<'a, Version>>, hi: impl Into<Cow<'a, Version>>) -> Self {
        let (lo, hi) = (lo.into(), hi.into());
        debug_assert!(
            lo.as_ref() <= hi.as_ref(),
            "Span::new_unchecked requires lo <= hi: the caller's structural guarantee failed"
        );
        Self { lo, hi }
    }

    /// The coincident span `[version, version]`: the span at one
    /// point.
    ///
    /// The named door for the single-version span — the shape whose
    /// placement collapses to pairwise comparison and on which the
    /// pointwise operators restrict to the version operators exactly
    /// (the [module docs](self) place the point identity beside the
    /// operator vocabulary). The version rides as [`new`](Self::new)'s
    /// endpoints do — anything [`Into`] a [`Cow`] of [`Version`], a
    /// borrow lent to both endpoints or an owned value moved into one
    /// and buffer-share-cloned into the other — and the `From` impls
    /// are the same door as trait conversions. Total: a point needs
    /// no ordering, so nothing is validated and nothing can be
    /// rejected. Either way the endpoints read one shared buffer, so
    /// the built span is certified coincident by clone identity and
    /// takes every coincident fast path; the
    /// `at_is_the_coincident_hull` law in [`laws`](crate::laws) pins
    /// every door to the pair hull `version.span(&version)`.
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let v = alice.tick().clone();
    /// let point = Span::at(v.clone()); // owned; `Span::at(&v)` lends
    /// // Both endpoints are the version, and the span is exactly
    /// // the singleton hull.
    /// assert_eq!((point.meet(), point.join()), (&v, &v));
    /// assert_eq!(point, v.span(&v));
    /// assert_eq!(Span::from(v.clone()), point);
    /// assert_eq!(Span::from(&v), point);
    /// ```
    ///
    /// # Complexity
    ///
    /// At most one refcount-bump clone of the version's stored
    /// buffer; no walk, no comparison.
    ///
    /// **Complexity**: `O(1)`.
    pub fn at(version: impl Into<Cow<'a, Version>>) -> Span<'a> {
        let lo = version.into();
        // A borrowed endpoint is lent twice; an owned one moves in
        // and its buffer-sharing clone fills the second slot — either
        // way the pair reads one shared buffer, the O(1) coincidence
        // certificate every fast path reads.
        let hi = lo.clone();
        Span { lo, hi }
    }

    /// The crate-internal owned door: a span from endpoints the caller
    /// derived as one collection's meet and join.
    ///
    /// [`Version::span`] and [`Version::span_all`] construct through
    /// here — their endpoints are minted owned, so the span borrows
    /// nothing — and so does the borsh deserializer, whose pair the
    /// fused admission parse validated on the wire. The caller must
    /// guarantee `lo <= hi`; a
    /// meet/join pair over one nonempty collection always does, and the
    /// hull laws (`span_is_the_pair_hull`, `span_all_is_the_family_hull`)
    /// pin every deriving caller's endpoints to the committed lattice
    /// folds on every law consumer — every combine arm of the hull fold
    /// included, since the family law's drivers sweep arity past the
    /// merged–merged combine the small arities never build. There is
    /// deliberately no
    /// re-validating assertion here: recomputing the comparison per
    /// construction would spend the fused hull walk's entire saving on
    /// re-checking an invariant the differential laws already pin.
    pub(crate) fn owned(lo: Version, hi: Version) -> Span<'static> {
        Span {
            lo: Cow::Owned(lo),
            hi: Cow::Owned(hi),
        }
    }

    /// A span borrowing this span's endpoints: the same `[lo, hi]`
    /// with a fresh, shorter lifetime.
    ///
    /// The lending door for a span held long-term: a stored
    /// `Span<'static>` answers each caller with a view of itself
    /// instead of cloning an endpoint or re-entering a construction
    /// door. No validation runs and none is needed — the source span
    /// already carries `lo <= hi`, and the reborrowed span reads the
    /// same endpoints, so the ordering rides through by construction
    /// (the `span_is_the_pair_hull` law in [`laws`](crate::laws) pins
    /// the endpoints byte-equal to the source's).
    ///
    /// ```
    /// use before::{Clock, causally::Span};
    ///
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    ///
    /// // A stored span lends a view of itself without being consumed.
    /// let stored: Span<'static> = a1.span(&a2); // owned endpoints
    /// let view: Span<'_> = stored.reborrow();
    /// assert_eq!(view, stored); // the same endpoints, byte for byte
    /// assert_eq!(view.dominance_of(&a2), stored.dominance_of(&a2));
    /// ```
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    pub fn reborrow(&self) -> Span<'_> {
        Span {
            lo: Cow::Borrowed(self.meet()),
            hi: Cow::Borrowed(self.join()),
        }
    }

    /// Places `probe` against this span at full resolution: the
    /// nine-way [`Placement`] verdict, the finest partition of the
    /// question the partial order admits.
    ///
    /// The name deliberately echoes
    /// [`placement_of`](crate::causally::Range::placement_of) — the same
    /// question,
    /// asked of a different object. The verdict is a pure transcription
    /// of the two causal comparisons against the endpoints; the
    /// `span_place_matches_relations` law in [`laws`](crate::laws)
    /// pins it on every law consumer, and
    /// `degenerate_span_place_is_partial_cmp` pins the coincident
    /// span `[v, v]` to pairwise comparison itself.
    ///
    /// One fused comparison pass answers both relations: the probe and
    /// the endpoint streams are walked simultaneously, each decoded
    /// once — against two probe decodes when the comparisons are
    /// composed. An endpoint whose relation is decided (a detected
    /// concurrency) stops being scanned, and once both endpoints have
    /// refuted the verdict returns at the deciding interval; no earlier
    /// exit exists at full resolution, because distinguishing
    /// `Concurrent(Start)` from `Concurrent(Both)` needs the other
    /// endpoint's relation to complete. When only the dominance
    /// question is asked, [`dominance_of`](Self::dominance_of) bails
    /// earlier.
    pub fn place(&self, probe: &Version) -> Placement {
        // The coincident span collapses placement to pairwise
        // comparison — the `degenerate_span_place_is_partial_cmp` law
        // in [`laws`](crate::laws) — and clone identity certifies
        // `lo == hi` in `O(1)`: a coincident span built by the hull
        // doors or the wire decode stores one buffer twice, so the
        // fused three-stream walk would read that buffer twice where
        // one pair sweep answers. Coincident endpoints in distinct
        // buffers still take the fused walk below.
        if self.lo.view().ptr_eq(self.hi.view()) {
            return match probe.partial_cmp(self.meet()) {
                Some(Ordering::Less) => Placement::Before,
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::Both),
            };
        }
        place::span(probe.view(), self.lo.view(), self.hi.view())
    }

    /// How much of this span `probe` dominates: the three-way
    /// [`Dominance`] verdict, [`place`](Self::place) coarsened to the
    /// dominance question.
    ///
    /// The coarsening table, pinned by the
    /// `span_dominance_coarsens_place` law in
    /// [`laws`](crate::laws):
    ///
    /// - [`After`](Dominance::After) ⟸ `At(End)`, `At(Both)`, `After`.
    /// - [`Between`](Dominance::Between) ⟸ `At(Start)`, `Between`,
    ///   `Concurrent(End)`.
    /// - [`Before`](Dominance::Before) ⟸ `Before`,
    ///   `Concurrent(Start)`, `Concurrent(Both)`.
    ///
    /// The coarser question buys the placement family's earliest exit:
    /// the verdict reads only the endpoint-at-or-below-probe
    /// directions, so the moment `lo <= probe` is refuted the answer is
    /// [`Before`](Dominance::Before) regardless of `hi` and the fused
    /// walk returns at the refuting interval — where
    /// [`place`](Self::place) must sweep on — and the moment
    /// `hi <= probe` is refuted the `hi` stream stops being scanned
    /// while `lo` decides [`Between`](Dominance::Between) against
    /// [`Before`](Dominance::Before). On sweeps with no refutation the
    /// cost is [`place`](Self::place)'s exactly.
    pub fn dominance_of(&self, probe: &Version) -> Dominance {
        // The coincident span collapses the dominance question to one
        // containment — `degenerate_span_place_is_partial_cmp` composed
        // with `span_dominance_coarsens_place` (both in
        // [`laws`](crate::laws)): on `lo == hi` the `After` bucket is
        // exactly `hi <= probe` and everything else is `Before`
        // (`Between` needs the endpoints to differ). Clone identity
        // certifies the coincidence in `O(1)` — the hull doors and the
        // wire decode store a coincident span's one buffer twice — so
        // one single-bound placement (each stream decoded once) answers
        // where the fused walk would read the shared buffer twice.
        // This is the compressed-subtree classification fast path: a
        // node whose version bounds coincide is classified against one
        // stream, not two.
        if self.lo.view().ptr_eq(self.hi.view()) {
            // `hi <= probe` is exactly membership in the probe's causal
            // past (`causally::known_at(probe).contains(hi)`).
            return if matches!(
                self.join().partial_cmp(probe),
                Some(Ordering::Less | Ordering::Equal)
            ) {
                Dominance::After
            } else {
                Dominance::Before
            };
        }
        place::dominance(probe.view(), self.lo.view(), self.hi.view())
    }

    /// The span's start endpoint, read as the *meet*.
    ///
    /// The lattice reading is honest for any valid span, not only a
    /// derived hull: `lo <= hi` makes the start the meet of the two
    /// endpoints — and of everything the span covers. On a hull from
    /// [`Version::span`] or [`Version::span_all`] it is definitionally
    /// the collection's [`meet_all`](Version::meet_all); the hull laws
    /// in [`laws`](crate::laws) pin the accessor spelling.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    pub fn meet(&self) -> &Version {
        &self.lo
    }

    /// The span's end endpoint, read as the *join* — dually to
    /// [`meet`](Self::meet).
    ///
    /// `lo <= hi` makes the end the join of the two endpoints and of
    /// everything the span covers; on a derived hull it is
    /// definitionally the collection's
    /// [`join_all`](Version::join_all), accessor spelling pinned by
    /// the hull laws in [`laws`](crate::laws).
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    pub fn join(&self) -> &Version {
        &self.hi
    }

    /// Destructures this span into its owned `(meet, join)` endpoints.
    ///
    /// The order is [`meet`](Self::meet) then [`join`](Self::join).
    /// Owned endpoints move out; borrowed endpoints settle by cloning,
    /// which shares the stored buffer.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    pub fn into_parts(self) -> (Version, Version) {
        (self.lo.into_owned(), self.hi.into_owned())
    }

    /// Settles this span onto owned endpoints, erasing the borrow
    /// lifetime.
    ///
    /// [`into_parts`](Self::into_parts) that keeps the span a span:
    /// the endpoints and the ordering they already carry are
    /// preserved exactly (the `span_is_the_pair_hull` law in
    /// [`laws`](crate::laws) pins the settled endpoints byte-equal),
    /// so no construction door is re-entered. An inherent method in
    /// [`Cow`]'s own vocabulary, following
    /// [`Ranked::into_owned`](crate::Ranked::into_owned)
    /// (a `ToOwned` impl cannot exist here, because std's blanket
    /// `impl<T: Clone> ToOwned for T` already claims every `Clone`
    /// type, `Span` included).
    ///
    /// ```
    /// use before::{Clock, causally::Span};
    ///
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let owned: Span<'static> = {
    ///     let borrowed = Span::new(&a1, &a2).unwrap();
    ///     borrowed.into_owned() // outlives the borrows
    /// };
    /// assert_eq!((owned.meet(), owned.join()), (&a1, &a2));
    /// ```
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(1)`.
    pub fn into_owned(self) -> Span<'static> {
        Span {
            lo: Cow::Owned(self.lo.into_owned()),
            hi: Cow::Owned(self.hi.into_owned()),
        }
    }

    /// Encodes this [`Span`] as canonical bytes: the meet's
    /// [`Version::encode`] bytes, then the join's.
    ///
    /// Each endpoint is byte-aligned, independently canonical, and
    /// self-delimiting, so the two concatenate with no length prefix
    /// (the [module docs](self) carry the wire form). Byte equality on
    /// these composites is exactly span equality, and no span's
    /// encoding is a byte prefix of another's — pinned directly,
    /// riding the components' committed prefix-freedom.
    ///
    /// # Complexity
    ///
    /// One copy of each endpoint's stored bytes.
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::{causally::Span, Clock};
    /// let mut clock = Clock::seed();
    /// let older = clock.tick().clone();
    /// let newer = clock.tick().clone();
    /// let span = Span::new(&older, &newer).unwrap();
    /// // The framing: the meet's bytes, then the join's.
    /// assert_eq!(span.encode(), [older.encode(), newer.encode()].concat());
    /// assert_eq!(Span::decode(&span.encode()[..]).unwrap(), span);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = self.lo.encode();
        bytes.extend_from_slice(self.hi.as_bytes());
        bytes
    }

    /// Encodes this [`Span`]'s canonical bytes to an arbitrary writer:
    /// exactly [`encode`](Self::encode)'s bytes, one endpoint's write
    /// after the other's.
    ///
    /// # Errors
    ///
    /// Whatever the writer itself reports; the encoding side is
    /// infallible.
    ///
    /// # Complexity
    ///
    /// One write of each endpoint's stored bytes, plus whatever the
    /// writer itself costs.
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::{causally::Span, Clock};
    /// let mut clock = Clock::seed();
    /// let v = clock.tick().clone();
    /// let span = Span::new(&v, &v).unwrap();
    /// let mut buf = Vec::new();
    /// span.encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, span.encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.lo.encode_to(writer)?;
        self.hi.encode_to(writer)
    }

    /// Decodes a [`Span`] from a reader of canonical bytes, strictly
    /// rejecting everything else — validity included.
    ///
    /// The one forward pass that parses the second version also proves
    /// it dominates the first, so every span this returns is valid by
    /// construction and no separate validation step exists to forget.
    /// Total over arbitrary input: every byte string either decodes to
    /// the one span that encodes to it, or is rejected. The fused
    /// second parse maintains the pair comparison's running difference
    /// where parse-then-validate would run the standalone validator's
    /// accumulator *and* the comparison's — the resource pins in
    /// `tests/meter.rs`'s span rows hold the decode to exactly the
    /// first component's parse plus one comparison sweep.
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`]: the bytes end mid-component — inside
    ///   either version's tree, or with the second missing entirely.
    /// - [`Decode::TrailingBits`]: live bits past a component's
    ///   complete tree, or nonzero padding.
    /// - [`Decode::NotCanonical`]: a non-canonical component, or a
    ///   pair that no [`Span`] encodes — crossed or concurrent — the
    ///   canonical spelling of no value (the [module docs](self) place
    ///   the genre choice beside the wire form).
    /// - [`Decode::Io`]: the reader itself fails.
    ///
    /// On an input defective several ways at once, the components'
    /// structural genres win: the pair rejection is pronounced only
    /// over a composite whose components parse whole and whose padding
    /// is exact — a defect the component decoders would report rejects
    /// here with the same genre.
    ///
    /// # Complexity
    ///
    /// `O(n)` time and space in the bytes read, accepted or rejected:
    /// one strict parse of the first component, then one fused
    /// parse-and-compare pass over the second against the first.
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::{causally::Span, error::Decode, Clock};
    /// let mut clock = Clock::seed();
    /// let older = clock.tick().clone();
    /// let newer = clock.tick().clone();
    /// let bytes = Span::new(&older, &newer).unwrap().encode();
    /// let span = Span::decode(&bytes[..]).unwrap();
    /// assert_eq!(span.meet(), &older);
    /// assert_eq!(span.join(), &newer);
    /// // A reversed pair is the canonical spelling of no span.
    /// let crossed = [newer.encode(), older.encode()].concat();
    /// assert!(matches!(
    ///     Span::decode(&crossed[..]),
    ///     Err(Decode::NotCanonical)
    /// ));
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Span<'static>, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        // Both components are validated against the borrowed buffer
        // first; the endpoints then adopt slices of that one buffer, so
        // the whole span shares a single allocation. The meet is the
        // byte-aligned self-delimiting prefix: parse its tree to find
        // the split and check its padding. The join's admission walk
        // parses its stream while deciding, in the same pass, whether
        // it dominates — or equals — the meet: never a parse and then a
        // second comparison walk. The pair verdict is pronounced last,
        // after the padding check, so a composite defective several
        // ways rejects by its structural genre first, exactly as
        // decoding the components would.
        let (lo_end, lo_bytes, hi_end, admission) = {
            let bits = codec::bytes_as_bits(&buf);
            let lo_end = skyline::validate_prefix(bits)?;
            let lo_bytes = lo_end.div_ceil(8);
            codec::require_zero_padding(&bits[..8 * lo_bytes], lo_end)?;
            let tail = &bits[8 * lo_bytes..];
            let mut cursor = codec::DsiCursor::new(tail);
            let admission = skyline::validate_dominating_from(&bits[..lo_end], &mut cursor)?;
            let hi_end = codec::BitCursor::position(&cursor);
            codec::require_zero_padding(tail, hi_end)?;
            if admission == skyline::Admission::Refuted {
                return Err(Decode::NotCanonical);
            }
            (lo_end, lo_bytes, hi_end, admission)
        };
        let buf = bytes::Bytes::from(buf);
        let lo = Version::from_frozen(codec::Bits::from_canonical(buf.slice(..lo_bytes), lo_end));
        let hi = match admission {
            // The coincident span stores one buffer twice: the admission
            // walk proved the second stream byte-equal to the first, so
            // the join is the meet's clone — an `O(1)` refcount bump the
            // ptr_eq fast paths then recognize.
            skyline::Admission::Equal => lo.clone(),
            skyline::Admission::Dominates => {
                Version::from_frozen(codec::Bits::from_canonical(buf.slice(lo_bytes..), hi_end))
            }
            skyline::Admission::Refuted => unreachable!("refuted admissions rejected above"),
        };
        Ok(Span {
            lo: Cow::Owned(lo),
            hi: Cow::Owned(hi),
        })
    }
}

/// Lends this version to a [`Cow`]-accepting door: the borrowed lift.
///
/// The [`Span`] constructors take each version operand as anything
/// [`Into`] a [`Cow`] of [`Version`]; this lift and its consuming
/// dual are what let owned and borrowed versions mix freely there.
///
/// # Complexity
///
/// **Complexity**: `O(1)`.
impl<'a> From<&'a Version> for Cow<'a, Version> {
    fn from(version: &'a Version) -> Cow<'a, Version> {
        Cow::Borrowed(version)
    }
}

/// Moves this version into a [`Cow`]-accepting door, dually to the
/// lending lift.
///
/// # Complexity
///
/// **Complexity**: `O(1)`.
impl From<Version> for Cow<'_, Version> {
    fn from(version: Version) -> Self {
        Cow::Owned(version)
    }
}

/// The coincident span `[version, version]`, as [`Span::at`] — the
/// consuming trait spelling of the same door.
///
/// # Complexity
///
/// One refcount-bump clone of the version's stored buffer; no walk,
/// no comparison.
///
/// **Complexity**: `O(1)`.
///
/// ```
/// use before::{Clock, Span};
/// let mut alice = Clock::seed();
/// let v = alice.tick().clone();
/// assert_eq!(Span::from(v.clone()), Span::at(&v));
/// ```
impl From<Version> for Span<'static> {
    fn from(version: Version) -> Span<'static> {
        Span::at(version)
    }
}

/// The coincident span at a borrowed version: [`Span::at`]'s lending
/// trait spelling, both endpoints borrowed from the one version.
///
/// # Complexity
///
/// Stores two copies of the one borrow; no clone at all.
///
/// **Complexity**: `O(1)`.
///
/// ```
/// use before::{Clock, Span};
/// let mut alice = Clock::seed();
/// let v = alice.tick().clone();
/// let point = Span::from(&v); // borrows; `v` stays usable
/// assert_eq!(point, v.span(&v));
/// ```
impl<'a> From<&'a Version> for Span<'a> {
    fn from(version: &'a Version) -> Span<'a> {
        Span::at(version)
    }
}

/// A [`Span`] endpoint, as a verdict payload: *which* endpoint an
/// at- or beside-the-span verdict speaks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    /// The span's lower endpoint, `lo`.
    Start,
    /// The span's upper endpoint, `hi`.
    End,
    /// Both endpoints at once. What that means is the carrying
    /// verdict's: see [`Placement::At`] and [`Placement::Concurrent`].
    Both,
}

/// Where a version sits relative to a [`Span`]: the placement
/// family's finest verdict.
///
/// In a partial order, a point sits in exactly one of nine regions
/// relative to a span — below, within, above, at either or both
/// endpoints, or beside it in one of three ways distinguished by which
/// endpoint still bounds it. The enum's shape is the proof of that
/// count: three bare variants for the regions on the chain through the
/// span, and two endpoint-qualified variants times three
/// [`Endpoint`] payloads for the rest — `3 + 2×3 = 9` — while the seven
/// combinations `lo <= hi` forbids (below `lo` yet not strictly below
/// `hi`; equal to `lo` yet above or beside `hi`; concurrent to `lo` yet
/// at or above `hi`) have no spelling at all.
/// Each variant's doc states the raw relations it reports and the
/// relations its payload forces.
///
/// **Vocabulary kinship, divergent semantics**: `Before`, `Between`,
/// and `After` deliberately echo
/// [`Bounded`](crate::causally::Bounded)'s words — the same
/// question, asked of a different object. `Bounded`'s region verdicts
/// fold range semantics (a version concurrent to the start bound is
/// `Bounded::Between`, because start bounds keep concurrent versions);
/// `Placement`'s variants are raw strict-order facts against two
/// concrete versions (a version concurrent to `lo` is
/// `Concurrent(Start)`, never `Between`). [`Dominance`] reuses the
/// words a third way, coarser than both: each of its verdicts is a
/// bucket of these nine regions (its variant docs carry the exact
/// tables). On a two-bounded range,
/// [`bounded`](crate::causally::Range::bounded) is exactly a coarsening
/// of this verdict,
/// pinned by the `bounded_coarsens_span_place` law in
/// [`laws`](crate::laws).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    /// Strictly below the whole span: `p < lo`, hence `p < hi`.
    Before,
    /// Exactly at an endpoint:
    ///
    /// - `At(Start)`: `p == lo < hi`.
    /// - `At(End)`: `lo < p == hi`.
    /// - `At(Both)`: `p == lo == hi`. Equality to one endpoint of a
    ///   coincident span is equality to both, so on `lo == hi`
    ///   every at-endpoint verdict is `At(Both)` — the coincident
    ///   span is the common single-version case, not a corner.
    At(Endpoint),
    /// Strictly inside: `lo < p < hi`.
    Between,
    /// Beside the span: incomparable to the endpoint(s) the payload
    /// names, with the opposite relation forced by `lo <= hi`:
    ///
    /// - `Concurrent(Start)`: `p ∥ lo`, forcing `p < hi` (at or above
    ///   `hi` would put `p` above `lo`).
    /// - `Concurrent(End)`: `p ∥ hi`, forcing `p > lo` (at or below
    ///   `lo` would put `p` below `hi`).
    /// - `Concurrent(Both)`: `p ∥ lo` and `p ∥ hi`.
    Concurrent(Endpoint),
    /// Strictly above the whole span: `p > hi`, hence `p > lo`.
    After,
}

/// How much of a [`Span`] a probe dominates:
/// [`Placement`] coarsened to the dominance question, "is the probe
/// causally at or after the span's content?"
///
/// The three verdicts a filter over version-bounded regions consumes: a
/// probe dominating the whole span has seen everything the span
/// covers; one dominating only the start has seen some of it; one
/// dominating not even the start has seen none of it.
///
/// **Vocabulary kinship, divergent semantics**: `After`, `Between`,
/// and `Before` echo [`Placement`]'s and
/// [`Bounded`](crate::causally::Bounded)'s words at a
/// third, coarser resolution, and the names understate what each
/// bucket folds in: the variants are position-named by the probe's
/// relation to the span's *endpoints as dominance thresholds*, and
/// each deliberately buckets concurrent and at-endpoint placements
/// with the regions (a probe concurrent to the start is
/// `Dominance::Before`, where `Placement` says `Concurrent(Start)`
/// and a range's `bounded` would keep it `Between`). Each variant's
/// doc states its exact [`Placement`] bucket; the
/// `span_dominance_coarsens_place` law in [`laws`](crate::laws) pins
/// the tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dominance {
    /// The probe dominates the whole span: `hi <= p`, and with it
    /// every version the span covers.
    ///
    /// The bucket of [`Placement`]s with `hi <= p` — `At(End)`,
    /// `At(Both)`, and `After`: at the end counts as dominance, so
    /// this is "at or beyond the whole span", not only strictly
    /// after it.
    After,
    /// The probe dominates the start but not the whole: `lo <= p`,
    /// while `hi` is above or beside the probe.
    ///
    /// The bucket of [`Placement`]s with `lo <= p` but not `hi <= p`
    /// — `At(Start)`, `Between`, and `Concurrent(End)`: at the start
    /// counts as dominance, and a probe concurrent to the end still
    /// dominates the start, so "between" here means between the
    /// endpoints *as dominance thresholds*, incomparability to the
    /// end included.
    Between,
    /// The probe does not dominate even the start: `lo` is above or
    /// beside the probe — and with it `hi`.
    ///
    /// The bucket of [`Placement`]s without `lo <= p` — `Before`,
    /// `Concurrent(Start)`, and `Concurrent(Both)`: a probe
    /// *concurrent* to the start dominates none of the span, so it
    /// lands here beside the strictly-below region, not in
    /// [`Between`](Self::Between).
    Before,
}
// ───────────────────────── the span algebra ─────────────────────────
//
// Four binary operators over owned and borrowed `Span` operands, the
// joins and meets of the two lattice structures spans carry (the
// module docs place them side by side), each with a receiver-seeded
// n-ary door running one balanced two-sided fold. Every door folds the
// inputs' meets into its `lo` leg and their joins into its `hi` leg;
// the four doors are exactly the four assignments of the two lattice
// directions to the two legs:
//
//              lo leg      hi leg      total?
//   union       meet        join       yes (containment join)
//   intersect   join        meet       no  (containment meet)
//   sum         join        join       yes (pointwise join)
//   product     meet        meet       yes (pointwise meet)
//
// Totality arguments, once: union's `lo` only descends below `a.lo`
// and its `hi` only ascends above `a.hi`, so the output pair stays
// ordered; sum's `hi` (a join of upper endpoints) bounds both lower
// endpoints from above, so it bounds their join, and product dually.
// Intersect is the one genuinely partial door: its `lo` ascends while
// its `hi` descends, and the pair crosses exactly when the spans share
// no version.

impl<'a> Span<'a> {
    /// Whether both endpoints read one shared stored buffer: the
    /// coincident span's `O(1)` certificate.
    ///
    /// The hull doors, the wire decode, and the algebra's point
    /// combines all store a coincident span's one stream twice
    /// (clones share the buffer), so clone identity certifies
    /// `lo == hi` without a walk. Coincident endpoints in distinct
    /// buffers are still equal — they just take the general walks.
    fn is_coincident(&self) -> bool {
        self.lo.view().ptr_eq(self.hi.view())
    }

    /// The union of `{self} ∪ others`: the tightest span covering
    /// every input — the containment lattice's join, endpoints the
    /// meet of the meets and the join of the joins.
    ///
    /// The receiver is the guaranteed first input, which keeps the
    /// door total in arity exactly as the receiver keeps
    /// [`Version::span_all`] total: the containment lattice has no
    /// least span to seed an empty fold with. An empty iterator
    /// settles the receiver onto owned endpoints. The items may be
    /// owned spans or references — anything that [borrows](Borrow) as
    /// a [`Span`].
    ///
    /// The `span_folds_match_the_sequential_operators` and
    /// `span_folds_are_rotation_invariant` laws in
    /// [`laws`](crate::laws) pin the door at every arity to the
    /// binary `|` folded over the family in any grouping, and
    /// `span_union_of_points_is_span_all` pins the all-coincident
    /// case to [`Version::span_all`] exactly.
    ///
    /// # Complexity
    ///
    /// One balanced fold, the accumulator carrying both endpoints
    /// through a single pass — the iterator is never buffered, and
    /// adjacent clone-identical inputs collapse before the fold reads
    /// them (union is idempotent). A combine of two point-like sides
    /// (coincident endpoints, certified by clone identity) derives
    /// its pair hull in one fused walk — [`Version::span`]'s ladder,
    /// fast paths and traffic accounting included; every other
    /// combine folds per endpoint, because its two legs read
    /// *different* operand pairs, so no shared decode exists to fuse.
    /// `D` is the inputs' total packed size and `k` their number.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
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
    /// let hull = spans[0].union_all(&spans[1..]);
    /// // The union covers every input span's endpoints.
    /// assert_eq!(hull, &spans[0] | &spans[1]);
    /// // An empty iterator settles the receiver.
    /// assert_eq!(spans[0].union_all::<[Span; 0]>([]), spans[0]);
    /// ```
    pub fn union_all<'s, I>(&self, others: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(others, &UNION_OPS);
        Span::owned(lo, hi)
    }

    /// The intersection of `{self} ∪ others`: the chain segment every
    /// input covers — the containment lattice's meet, endpoints the
    /// join of the meets and the meet of the joins — or [`None`] when
    /// the inputs share no version.
    ///
    /// The receiver seeds the fold, so [`None`] means exactly one
    /// thing: an *empty intersection*, never an empty input (an empty
    /// iterator settles the receiver, [`Some`] always). The items may
    /// be owned spans or references.
    ///
    /// The verdict is pronounced once, at the end: the `lo` leg only
    /// ascends and the `hi` leg only descends, so an empty
    /// intermediate intersection forces an empty final one and
    /// deferring the check loses no answer — it trades the binary
    /// `&`'s early exit for one comparison total. A caller that wants
    /// the early exit folds the binary operator instead.
    ///
    /// The `span_folds_match_the_sequential_operators` and
    /// `span_folds_are_rotation_invariant` laws in
    /// [`laws`](crate::laws) pin the door at every arity to the
    /// binary `&` folded through [`Option`] over the family.
    ///
    /// # Complexity
    ///
    /// One balanced fold as [`union_all`](Self::union_all) (with the
    /// legs swapped: joins of meets, meets of joins), plus one final
    /// validating comparison. `D` is the inputs' total packed size
    /// and `k` their number.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
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
    pub fn intersect_all<'s, I>(&self, others: I) -> Option<Span<'static>>
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(others, &INTERSECT_OPS);
        match lo.partial_cmp(&hi) {
            Some(Ordering::Less | Ordering::Equal) => Some(Span::owned(lo, hi)),
            Some(Ordering::Greater) | None => None,
        }
    }

    /// The pointwise join of `{self} ∪ others`: the span of possible
    /// joins — where each input's value could be anywhere within it,
    /// the join of one value from each input lies here, and both
    /// endpoints are attained.
    ///
    /// Endpoints are the join of the meets and the join of the joins;
    /// on coincident inputs the door restricts to
    /// [`Version::join_all`] exactly (the point identity in the
    /// [module docs](self)). The receiver seeds the fold; an empty
    /// iterator settles the receiver. The items may be owned spans or
    /// references.
    ///
    /// The `span_folds_match_the_sequential_operators` and
    /// `span_folds_are_rotation_invariant` laws in
    /// [`laws`](crate::laws) pin the door at every arity to the
    /// binary `+` folded over the family.
    ///
    /// # Complexity
    ///
    /// One balanced fold as [`union_all`](Self::union_all), with a
    /// point-combine that pays one walk for both legs (the legs read
    /// the same operand pair, and the shared result is stored twice).
    /// `D` is the inputs' total packed size and `k` their number.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let a1 = a.tick().clone();
    /// let b1 = b.tick().clone();
    ///
    /// // On points, the pointwise door is the version join.
    /// let sum = a1.span(&a1).sum_all([&b1.span(&b1)]);
    /// assert_eq!(sum.meet(), sum.join());
    /// assert_eq!(sum.meet(), &(&a1 | &b1));
    /// ```
    pub fn sum_all<'s, I>(&self, others: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(others, &SUM_OPS);
        Span::owned(lo, hi)
    }

    /// The pointwise meet of `{self} ∪ others`: the span of possible
    /// meets — dually to [`sum_all`](Self::sum_all) in every clause.
    ///
    /// Endpoints are the meet of the meets and the meet of the joins;
    /// on coincident inputs the door restricts to
    /// [`Version::meet_all`] exactly. The receiver seeds the fold and
    /// keeps the door total — the version-level
    /// [`meet_all`](Version::meet_all) returns [`Option`] because the
    /// lattice has no top to seed with, and the receiver is exactly
    /// that seed here. The items may be owned spans or references.
    ///
    /// The `span_folds_match_the_sequential_operators` and
    /// `span_folds_are_rotation_invariant` laws in
    /// [`laws`](crate::laws) pin the door at every arity to the
    /// binary `*` folded over the family.
    ///
    /// # Complexity
    ///
    /// One balanced fold as [`sum_all`](Self::sum_all), dual legs.
    /// `D` is the inputs' total packed size and `k` their number.
    ///
    /// **Complexity**: `O(D log k)` time, `O(D)` space.
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut a = Clock::seed();
    /// let mut b = a.fork();
    /// let a1 = a.tick().clone();
    /// let b1 = b.tick().clone();
    ///
    /// // On points, the pointwise door is the version meet.
    /// let product = a1.span(&a1).product_all([&b1.span(&b1)]);
    /// assert_eq!(product.meet(), product.join());
    /// assert_eq!(product.meet(), &(&a1 & &b1));
    /// ```
    pub fn product_all<'s, I>(&self, others: I) -> Span<'static>
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        let (lo, hi) = self.fold_endpoints(others, &PRODUCT_OPS);
        Span::owned(lo, hi)
    }

    /// The doors' shared balanced fold: `{self} ∪ others` reduced
    /// through [`crate::fold::balanced_reduce`] with a two-sided
    /// accumulator, per-door leg kernels, and the point-combine fast
    /// path.
    ///
    /// Adjacent clone-identical inputs collapse before the counter
    /// reads them (all four doors are idempotent — the binary
    /// operators' laws — so a run of one shared span is one input);
    /// the receiver rides as the first input, so the fold is never
    /// empty. Inputs enter untouched and are cloned only at their
    /// first combine, and every clone of a stored version is a
    /// refcount bump, never a byte copy.
    fn fold_endpoints<'s, I>(&self, others: I, ops: &SpanFoldOps) -> (Version, Version)
    where
        I: IntoIterator,
        I::Item: Borrow<Span<'s>>,
    {
        // The dedup filter: one (lo, hi) buffer-identity pair of state.
        let mut last: Option<(Version, Version)> = None;
        let inputs = core::iter::once(FoldInput::Receiver(self))
            .chain(others.into_iter().map(FoldInput::Item))
            .filter(move |input| {
                let s = input.span();
                let dup = last.as_ref().is_some_and(|(lo, hi)| {
                    lo.view().ptr_eq(s.meet().view()) && hi.view().ptr_eq(s.join().view())
                });
                if !dup {
                    last = Some((s.meet().clone(), s.join().clone()));
                }
                !dup
            })
            .map(Group::Input);
        let group = crate::fold::balanced_reduce(inputs, |a, b| {
            // The point-combine: when both sides read one stream each
            // (a coincident input, or a merged group whose legs
            // settled on one shared buffer), the door's fused kernel
            // answers in one walk what the per-leg folds would walk
            // twice. Clone identity is the certificate, so the check
            // itself is O(1).
            let (lo, hi) = if let (Some(va), Some(vb)) = (a.point(), b.point()) {
                (ops.points)(va, vb)
            } else {
                match (a, b) {
                    (Group::Input(a), Group::Input(b)) => {
                        let (a, b) = (a.span(), b.span());
                        (
                            (ops.lo_refs)(a.meet(), b.meet()),
                            (ops.hi_refs)(a.join(), b.join()),
                        )
                    }
                    (Group::Merged { mut lo, mut hi }, Group::Input(b)) => {
                        let b = b.span();
                        (ops.lo_view)(&mut lo, b.meet().view());
                        (ops.hi_view)(&mut hi, b.join().view());
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
                    // Unreachable through the counter's weight
                    // discipline (a weight-0 lone input never sits
                    // below a merged group in the closing drain), but
                    // the match stays total rather than asserting:
                    // every leg kernel is commutative, so folding the
                    // raw input into the owned group is
                    // value-identical.
                    (Group::Input(a), Group::Merged { mut lo, mut hi }) => {
                        let a = a.span();
                        (ops.lo_view)(&mut lo, a.meet().view());
                        (ops.hi_view)(&mut hi, a.join().view());
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
                (s.meet().clone(), s.join().clone())
            }
            Group::Merged { lo, hi } => (lo, hi),
        }
    }
}

/// One n-ary span door's kernels. Every door folds the inputs' meets
/// into its `lo` leg and their joins into its `hi` leg; the section
/// comment above carries the four leg assignments and their totality
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
    /// one walk answers both legs (each door's kernel is named at its
    /// constant).
    points: fn(&Version, &Version) -> (Version, Version),
}

/// Union's point-combine: two points' union is their hull — one fused
/// pair walk through [`Version::span_refs`]'s ladder, fast paths and
/// traffic accounting included.
fn union_points(a: &Version, b: &Version) -> (Version, Version) {
    Version::span_refs(a, b)
}

/// Intersection's point-combine: two points share a version exactly
/// when they are equal.
///
/// One byte compare answers the only nonempty case; an unequal pair
/// pays the per-leg walks whose crossed output the door's final
/// validation rejects (or a later combine absorbs).
fn intersect_points(a: &Version, b: &Version) -> (Version, Version) {
    if codec::canonical_eq(a.view(), b.view()) {
        return (a.clone(), a.clone());
    }
    (Version::join_refs(a, b), Version::meet_refs(a, b))
}

/// Sum's point-combine: the legs read the same operand pair, so one
/// join walk feeds both, the result stored twice (clones share the
/// buffer, keeping the group point-like).
fn sum_points(a: &Version, b: &Version) -> (Version, Version) {
    let v = Version::join_refs(a, b);
    (v.clone(), v)
}

/// Product's point-combine: dually to [`sum_points`], one meet walk
/// feeds both legs.
fn product_points(a: &Version, b: &Version) -> (Version, Version) {
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
const SUM_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::join_refs,
    hi_refs: Version::join_refs,
    lo_view: Version::join_view,
    hi_view: Version::join_view,
    points: sum_points,
};

/// Pointwise meet: both legs meet.
const PRODUCT_OPS: SpanFoldOps = SpanFoldOps {
    lo_refs: Version::meet_refs,
    hi_refs: Version::meet_refs,
    lo_view: Version::meet_view,
    hi_view: Version::meet_view,
    points: product_points,
};

/// One input to the doors' shared fold: the receiver rides by
/// reference beside the caller's items, whatever ownership they carry
/// (owned or borrowed through [`Borrow`], never cloned on entry).
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

/// One group in the doors' balanced counter: an input exactly as the
/// caller supplied it, or the owned endpoints a combine produced.
enum Group<T> {
    /// An input the fold has not yet combined, still in the caller's
    /// form.
    Input(T),
    /// The owned running endpoints of one or more combines.
    Merged { lo: Version, hi: Version },
}

impl<'i, 's, T: Borrow<Span<'s>>> Group<FoldInput<'_, 'i, T>> {
    /// The one stream a point-like group reads, when it is one: a
    /// coincident input span, or a merged group whose legs settled on
    /// one shared buffer. Clone identity is the certificate — `O(1)`,
    /// never a walk.
    fn point<'x>(&'x self) -> Option<&'x Version>
    where
        'i: 'x,
        's: 'x,
    {
        match self {
            Group::Input(input) => {
                let s = input.span();
                s.is_coincident().then(|| s.meet())
            }
            Group::Merged { lo, hi } => lo.view().ptr_eq(hi.view()).then_some(lo),
        }
    }
}

/// `a | b`'s kernel: the containment join over borrowed operands.
fn union_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
    if a.is_coincident() && b.is_coincident() {
        // Two points' union is their hull: one fused pair walk
        // (Version::span's ladder, fast paths and traffic accounting
        // included) where the per-leg folds below would walk the same
        // operand pair twice.
        return a.meet().span(b.meet());
    }
    let mut lo = a.meet().clone(); // O(1): a stored version's clone shares its buffer
    let mut hi = a.join().clone();
    lo.meet_view(b.meet().view());
    hi.join_view(b.join().view());
    // Ordered by construction: `lo` only descended below `a`'s meet,
    // `hi` only ascended above `a`'s join, and `a`'s endpoints are
    // ordered.
    Span::owned(lo, hi)
}

/// `a & b`'s kernel: the containment meet over borrowed operands, or
/// [`None`] where the segments share no version.
fn intersect_core(a: &Span<'_>, b: &Span<'_>) -> Option<Span<'static>> {
    if a.is_coincident() && b.is_coincident() {
        // Two points share a version exactly when they are equal: one
        // byte compare, no walk.
        return codec::canonical_eq(a.meet().view(), b.meet().view())
            .then(|| Span::owned(a.meet().clone(), a.meet().clone()));
    }
    let mut lo = a.meet().clone();
    let mut hi = a.join().clone();
    lo.join_view(b.meet().view());
    hi.meet_view(b.join().view());
    // The one genuinely partial door: the joined meets must still sit
    // under the met joins, and the pair crosses exactly when the
    // segments are disjoint.
    match lo.partial_cmp(&hi) {
        Some(Ordering::Less | Ordering::Equal) => Some(Span::owned(lo, hi)),
        Some(Ordering::Greater) | None => None,
    }
}

/// `a + b`'s kernel: the pointwise join over borrowed operands.
fn sum_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
    if a.is_coincident() && b.is_coincident() {
        // On points the pointwise door is the version operator: one
        // walk feeds both endpoints, stored twice (the clones share
        // one buffer, keeping the result coincident).
        let v = Version::join_refs(a.meet(), b.meet());
        return Span::owned(v.clone(), v);
    }
    let mut lo = a.meet().clone();
    let mut hi = a.join().clone();
    lo.join_view(b.meet().view());
    hi.join_view(b.join().view());
    // Ordered by construction: `hi` bounds every operand endpoint
    // from above, the joined meets included.
    Span::owned(lo, hi)
}

/// `a * b`'s kernel: the pointwise meet over borrowed operands,
/// dually to [`sum_core`] in every clause.
fn product_core(a: &Span<'_>, b: &Span<'_>) -> Span<'static> {
    if a.is_coincident() && b.is_coincident() {
        let v = Version::meet_refs(a.meet(), b.meet());
        return Span::owned(v.clone(), v);
    }
    let mut lo = a.meet().clone();
    let mut hi = a.join().clone();
    lo.meet_view(b.meet().view());
    hi.meet_view(b.join().view());
    Span::owned(lo, hi)
}

/// Generates one span operator's full matrix over owned and borrowed
/// operands: four value cells (lhs × rhs over `{Span, &Span}`), every
/// cell one call into the operator's borrowed-operand kernel.
///
/// The operand lifetimes are independent, and the output is always
/// `'static` (owned endpoints), so the operators compose freely in
/// fold position. No assign cells exist for any span operator — the
/// module docs carry the argument.
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

span_binop_matrix! {
    /// `a | b` — the *union*: the tightest span covering both
    /// operands, the containment lattice's join.
    ///
    /// Endpoints are `[meet ∧ meet, join ∨ join]`, so the operator is
    /// total: the output's ends only move outward. Commutative,
    /// associative, and idempotent, with each operand placing within
    /// the result — the `span_union_is_the_containment_join` law in
    /// [`laws`](crate::laws) pins all of it, across every owned and
    /// borrowed cell.
    ///
    /// On two coincident spans this is exactly the pair hull
    /// ([`Version::span`]), one fused walk; otherwise the two legs
    /// read different operand pairs and fold separately.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(a + b)` in the operands' packed sizes.
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
    /// let theirs = b1.span(&b1);
    /// let both = &ours | &theirs;
    /// // The union covers both operands' whole segments…
    /// assert_eq!(*both.join(), &a2 | &b1);
    /// // …and is the same span from either side.
    /// assert_eq!(both, &theirs | &ours);
    /// ```
    BitOr::bitor, union_core, Span<'static>
}

span_binop_matrix! {
    /// `a & b` — the *intersection*: the chain segment both operands
    /// cover, the containment lattice's meet — [`None`] when they
    /// share no version.
    ///
    /// Endpoints are `[meet ∨ meet, join ∧ join]` when that pair is
    /// ordered; a crossed or incomparable pair means the segments are
    /// disjoint, and the verdict is [`None`] — a legitimate answer,
    /// not a construction defect, which is why the type is [`Option`]
    /// and not [`Crossed`]. Commutative and idempotent, absorbing
    /// with the union (`a & (a | b)` is `Some(a)`), and a version
    /// placing within both operands places within the intersection —
    /// the `span_intersect_is_the_shared_segment` law in
    /// [`laws`](crate::laws) pins all of it.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(a + b)` in the operands' packed sizes.
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
    /// assert_eq!(&head & &tail, Some(a2.span(&a2)));
    /// // …a covered segment is absorbed…
    /// assert_eq!(&tail & &wide, Some(tail.clone()));
    /// // …and disjoint segments have no intersection.
    /// assert_eq!(&a1.span(&a1) & &tail, None);
    /// ```
    BitAnd::bitand, intersect_core, Option<Span<'static>>
}

span_binop_matrix! {
    /// `a + b` — the *pointwise join*: the span of possible joins,
    /// the version lattice's `|` lifted to spans.
    ///
    /// Endpoints are `[meet ∨ meet, join ∨ join]`, total: the upper
    /// endpoint bounds every operand endpoint from above. Where each
    /// operand's value is only known to lie within its span, the join
    /// of the two values lies within the sum — and on coincident
    /// spans the operator restricts to the version join exactly
    /// (`[a, a] + [b, b] == [a ∨ b, a ∨ b]`), which is what earns it
    /// the arithmetic symbol: join is the crate's summation monoid
    /// ([`Version`]'s [`Sum`](core::iter::Sum) sums by join). The
    /// `span_sum_is_the_pointwise_join` law in [`laws`](crate::laws)
    /// pins the endpoints, the point identity, commutativity, and
    /// idempotence.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(a + b)` in the operands' packed sizes.
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
    /// let advanced = &a1.span(&a2) + &b1.span(&b1);
    /// assert_eq!(*advanced.meet(), &a1 | &b1);
    /// assert_eq!(*advanced.join(), &a2 | &b1);
    /// ```
    Add::add, sum_core, Span<'static>
}

span_binop_matrix! {
    /// `a * b` — the *pointwise meet*: the span of possible meets,
    /// the version lattice's `&` lifted to spans — dually to `+` in
    /// every clause.
    ///
    /// Endpoints are `[meet ∧ meet, join ∧ join]`, total: the lower
    /// endpoint bounds every operand endpoint from below. On
    /// coincident spans the operator restricts to the version meet
    /// exactly, and meet distributes over join — the semiring reading
    /// behind the symbol pair. The `span_product_is_the_pointwise_meet`
    /// law in [`laws`](crate::laws) pins the endpoints, the point
    /// identity, commutativity, idempotence, and the pointwise
    /// absorption with `+`.
    ///
    /// # Complexity
    ///
    /// **Complexity**: `O(a + b)` in the operands' packed sizes.
    ///
    /// ```
    /// use before::{Clock, Span};
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let a2 = alice.tick().clone();
    /// let a3 = alice.tick().clone();
    ///
    /// // Clamping a segment to a point's past:
    /// let clamped = &a2.span(&a3) * &a2.span(&a2);
    /// assert_eq!(clamped, a2.span(&a2));
    /// // Pointwise absorption: (a + b) * a == a.
    /// let (s, t) = (a1.span(&a2), a2.span(&a3));
    /// assert_eq!(&(&s + &t) * &s, s);
    /// ```
    Mul::mul, product_core, Span<'static>
}

// ───────────────────────── the quotient view ─────────────────────────

/// The part of a [`Span`] contributed within a [`Party`]'s id region —
/// both endpoints projected, as a borrowed, lazy view.
///
/// `&span / &party` constructs it in `O(1)`, borrowing both operands.
/// The view answers the placement family directly —
/// [`place`](Self::place) and [`dominance_of`](Self::dominance_of),
/// against the *projected* endpoints — without materializing a
/// projection, and hands its endpoints out as
/// [`OwnVersion`] views ([`meet`](Self::meet)/[`join`](Self::join))
/// that compare the same way. Materializing the projected span is a
/// separate, explicit call — [`to_span`](Self::to_span) (or the
/// [`From`] impl) — because projection is the one operation whose
/// output can outgrow its operands ([`OwnVersion::to_version`]
/// carries the argument).
///
/// The projection is monotone (it is a homomorphism of both lattice
/// directions — the projection laws in [`laws`](crate::laws) — and
/// `a <= b` is `a | b == b`), so the projected endpoints stay ordered
/// and every view of a valid span is a valid span. The
/// `own_span_matches_the_projected_span` law pins every verdict and
/// the materialization to the eagerly-projected span.
///
/// Deliberately absent: the span algebra on views. A lazy `|` over
/// quotients would have to hold every operand's party across an
/// unbounded fold — parties are linear (`!Clone`) — and no masked
/// *emit* kernel exists to combine under a mask lazily
/// ([`OwnVersion`] draws the same line: comparisons lazy,
/// combination through materialization). Materialize with
/// [`to_span`](Self::to_span), then combine.
///
/// # Complexity
///
/// Construction and [`Clone`]/[`Copy`] are `O(1)`. Placement verdicts
/// cost two masked co-walks (`O(|v| + |p| + |probe|)` each, the
/// [`OwnVersion`] comparison kernel); [`dominance_of`](Self::dominance_of)
/// stops after one when the first relation already decides.
///
/// **Complexity**: construction `O(1)`; placement `O(|lo| + |hi| + |p| + |probe|)`; materialization as [`OwnVersion::to_version`], per endpoint.
///
/// ```
/// use before::{causally::Dominance, Clock};
/// let mut alice = Clock::seed();
/// let mut bob = alice.fork();
/// let a1 = alice.tick().clone();
/// let b1 = bob.tick().clone();
/// let both = &a1 | &b1;
/// let span = a1.span(&both);
///
/// // Alice's view of the span drops bob's contribution: a1 already
/// // dominates everything alice owns of it — no projection is built.
/// assert_eq!((&span / alice.party()).dominance_of(&a1), Dominance::After);
/// // Against the unprojected span, a1 dominates only the start.
/// assert_eq!(span.dominance_of(&a1), Dominance::Between);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct OwnSpan<'a> {
    /// The party whose owned region gates both endpoints.
    party: &'a Party,
    /// The span being projected.
    span: &'a Span<'a>,
}

impl<'a> OwnSpan<'a> {
    /// The view's start endpoint, projected: `span.meet() / party`,
    /// as the lazy [`OwnVersion`] view.
    ///
    /// # Complexity
    ///
    /// The view borrows; its operations carry the costs.
    ///
    /// **Complexity**: `O(1)`.
    pub fn meet(&self) -> OwnVersion<'a> {
        self.span.meet() / self.party
    }

    /// The view's end endpoint, projected: `span.join() / party`,
    /// dually to [`meet`](Self::meet).
    ///
    /// # Complexity
    ///
    /// The view borrows; its operations carry the costs.
    ///
    /// **Complexity**: `O(1)`.
    pub fn join(&self) -> OwnVersion<'a> {
        self.span.join() / self.party
    }

    /// Places `probe` against the projected span: the nine-way
    /// [`Placement`] verdict [`Span::place`] gives, against the
    /// endpoints' projections.
    ///
    /// The verdict is the pure transcription of the two masked causal
    /// comparisons (`probe` against each projected endpoint — the
    /// [`OwnVersion`] comparison kernel), composed rather than fused:
    /// the `own_span_matches_the_projected_span` law in
    /// [`laws`](crate::laws) pins it to the eagerly-projected span's
    /// [`place`](Span::place) on every consumer. A fused
    /// probe-mask-endpoints kernel would save one probe decode per
    /// verdict; no consumer's profile has asked for it, so the
    /// composed form stands.
    ///
    /// # Complexity
    ///
    /// **Complexity**: two masked co-walks, `O(|lo| + |hi| + |p| + |probe|)`.
    pub fn place(&self, probe: &Version) -> Placement {
        let (lo, hi) = (self.meet(), self.join());
        match probe.partial_cmp(&lo) {
            Some(Ordering::Less) => Placement::Before,
            Some(Ordering::Equal) => match probe.partial_cmp(&hi) {
                Some(Ordering::Equal) => Placement::At(Endpoint::Both),
                _ => Placement::At(Endpoint::Start),
            },
            Some(Ordering::Greater) => match probe.partial_cmp(&hi) {
                Some(Ordering::Less) => Placement::Between,
                Some(Ordering::Equal) => Placement::At(Endpoint::End),
                Some(Ordering::Greater) => Placement::After,
                None => Placement::Concurrent(Endpoint::End),
            },
            None => match probe.partial_cmp(&hi) {
                None => Placement::Concurrent(Endpoint::Both),
                _ => Placement::Concurrent(Endpoint::Start),
            },
        }
    }

    /// How much of the projected span `probe` dominates:
    /// [`place`](Self::place) coarsened to the three-way
    /// [`Dominance`] verdict, [`Span::dominance_of`]'s buckets
    /// exactly.
    ///
    /// The coarse question buys the early exit the fine one cannot
    /// have: the moment the projected start refutes `lo <= probe` the
    /// answer is [`Before`](Dominance::Before) and the end is never
    /// walked.
    ///
    /// # Complexity
    ///
    /// **Complexity**: at most two masked co-walks, one when the start refutes.
    pub fn dominance_of(&self, probe: &Version) -> Dominance {
        if !matches!(
            probe.partial_cmp(&self.meet()),
            Some(Ordering::Greater | Ordering::Equal)
        ) {
            return Dominance::Before;
        }
        if matches!(
            probe.partial_cmp(&self.join()),
            Some(Ordering::Greater | Ordering::Equal)
        ) {
            Dominance::After
        } else {
            Dominance::Between
        }
    }

    /// Materializes the projected span: the explicit, eager form of
    /// this view.
    ///
    /// One [`OwnVersion::to_version`] per endpoint; the projection is
    /// monotone (the type's docs carry the argument), so the
    /// projected pair is ordered and the construction revalidates
    /// nothing.
    ///
    /// # Complexity
    ///
    /// **Complexity**: as [`OwnVersion::to_version`], per endpoint — the results' packed sizes are not bounded by a constant factor of the operands.
    ///
    /// ```
    /// use before::Clock;
    /// let mut alice = Clock::seed();
    /// let a1 = alice.tick().clone();
    /// let span = a1.span(&a1);
    /// // The seed owns its whole history: the projection is the span.
    /// assert_eq!((&span / alice.party()).to_span(), span);
    /// ```
    pub fn to_span(&self) -> Span<'static> {
        Span::owned(self.meet().to_version(), self.join().to_version())
    }
}

/// `&span / &party` — the part of the [`Span`] within the [`Party`]'s
/// id region, as the lazy [`OwnSpan`] view. Both operands are
/// borrowed, not consumed.
///
/// # Complexity
///
/// The view borrows its operands; every cost lives on the view's
/// operations ([`OwnSpan`]'s doc carries them).
///
/// **Complexity**: `O(1)`.
///
/// ```
/// use before::{causally::Placement, Clock};
/// let mut alice = Clock::seed();
/// let a1 = alice.tick().clone();
/// let a2 = alice.tick().clone();
/// let span = a1.span(&a2);
/// let view = &span / alice.party();
/// // The seed owns everything: the view places like the span.
/// assert_eq!(view.place(&a1), span.place(&a1));
/// ```
impl<'a> Div<&'a Party> for &'a Span<'a> {
    type Output = OwnSpan<'a>;
    fn div(self, party: &'a Party) -> OwnSpan<'a> {
        OwnSpan { party, span: self }
    }
}

/// Materializes the projection, as [`to_span`](OwnSpan::to_span).
///
/// # Complexity
///
/// **Complexity**: as [`OwnSpan::to_span`].
///
/// ```
/// use before::{Clock, Span};
/// let mut alice = Clock::seed();
/// let a1 = alice.tick().clone();
/// let span = a1.span(&a1);
/// assert_eq!(Span::from(&span / alice.party()), (&span / alice.party()).to_span());
/// ```
impl From<OwnSpan<'_>> for Span<'static> {
    fn from(view: OwnSpan<'_>) -> Span<'static> {
        view.to_span()
    }
}
