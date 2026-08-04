//! The coverage rosters: the not-applicable half of the board tiling and
//! the red-triage buffer — committed data the complexity-claims suite
//! enforces.
//!
//! Rows price delegations at their shared mechanism, so several surface
//! rows legitimately cite one row: `Clock::send` is `Clock::tick` by
//! definition; `clock | version` (either operand order, `|=` included)
//! folds through the same join-assign the `recv` row measures;
//! `Party::tick` is `Version::tick`'s mirror (the `tick_adv_party`
//! row); the operator matrix (`|`, `&`, and their assign forms, over
//! every borrow shape) routes through the same `join_view`/`meet_view`
//! emitters and cmp walk the `join`/`meet`/`cmp` rows measure;
//! `Version::concurrent` is one `partial_cmp` and keeps its own row as
//! the documented entry point; the serde/borsh wrappers serialize as
//! the canonical encoding and deserialize through the strict decoder
//! (the `encode`/`decode` rows); `Party::ticks` and `Clock::ticks` run
//! the same fused kernel as `version_ticks` through their own
//! spellings. Derived surfaces with no roster row of their own ride
//! the same cells: `Clone` copies stored bits or value content
//! wholesale with no walk in the contract, `Debug` delegates to
//! `Display`, and the byte-compare `Eq`s are the `eq`/`hash` rows'
//! wholesale compares.
//!
//! Two coverage notes that are dispositions of *error paths*, not
//! operations (so they live here rather than in the table): the
//! rejection rows price the fallible surface (the board module doc's
//! rejection section; the `defect` module carries the placed defects),
//! and **the rejection surface's bounded-or-delegated remainder** is:
//! `Clock::join_all`'s overlap hand-back runs the identical up-front
//! indexed test against self that `party_join_all_overlap` prices,
//! inline; clock non-canonicality — packed or text — is the component
//! validators on the same streams the version and party non-canonical
//! rows drive; [`Parse::Anonymous`](crate::error::Parse) is the one-token
//! `"0"`; [`Decode::Io`](crate::error::Decode) is the caller's reader —
//! a failing reader is a truncation carrying an error, priced by the
//! truncated rows — and `encode_to`'s error the caller's writer, at
//! most the encode row's work before it propagates; the `TryFrom`
//! literal rejections have word-scale or type-bounded operands;
//! `Version::meet_all`'s `None` is the empty iterator;
//! `Rank::checked_sub`'s `None` is measured on the `rank_pair_ops`
//! row, which attempts both directions; other decode non-canonicality
//! genres (a negative running height, nonzero padding) ride the same
//! single validator pass at the same full-parse cost as the committed
//! maximally-deferred tails; serde/borsh deserialize errors are the
//! strict decoder through the wrappers (the decode rejection rows).
//! `Debug` for all three types delegates to `Display`.

/// The board's not-applicable table: every `before::surface` row with
/// no board row of its own, and the mechanism-based reason why.
///
/// The machine-readable half of the board's coverage tiling,
/// covering method and family rows alike.
///
/// The tiling test in the complexity-claims suite
/// (`board_coverage_tiles_the_public_surface`) holds this table and the
/// claims roster's board citations disjoint and jointly total over the
/// public surface: an operation is priced by named rows or excused
/// here, never both, never neither.
pub const BOARD_NOT_APPLICABLE: &[(&str, &str)] = &[
    (
        "Party::seed",
        "word-scale constructor: no input axis to measure against",
    ),
    (
        "Party::is_seed",
        "word-scale predicate: one comparison against the two-bit seed form",
    ),
    (
        "Party::forks",
        "iterates the measured fork (the party_fork row) on shrinking operands; a \
         mid-run drop rejoins in O(log n) measured joins",
    ),
    (
        "Party::dangerously_alias",
        "one refcount bump: the alias shares the stored canonical buffer",
    ),
    (
        "Party::encoded_bits",
        "a stored-length read: no walk, no allocation",
    ),
    ("Party::as_bytes", "a borrow of the stored canonical bytes"),
    (
        "Version::new",
        "word-scale constructor: the canonical two-bit empty stream",
    ),
    (
        "Version::is_empty",
        "an O(1) bit test against the canonical empty stream",
    ),
    (
        "Version::encoded_bits",
        "a stored-length read: no walk, no allocation",
    ),
    (
        "Version::as_bytes",
        "a borrow of the stored canonical bytes",
    ),
    (
        "Clock::seed",
        "word-scale constructor: the seed party over the empty version",
    ),
    (
        "Clock::forks",
        "iterates the measured fork on shrinking operands plus one version clone \
         (a refcount bump on the shared stored buffer) per child",
    ),
    (
        "Clock::from_parts",
        "two moves of the stored parts: no walk, no allocation",
    ),
    (
        "Clock::into_parts",
        "two moves of the stored parts: no walk, no allocation",
    ),
    (
        "Clock::party",
        "a borrow of a stored part: no walk, no allocation",
    ),
    (
        "Clock::version",
        "a borrow of a stored part: no walk, no allocation",
    ),
    (
        "Clock::own_version",
        "O(1) view construction (two borrows); the materialization and fused \
         comparison costs are celled at the OwnVersion rows",
    ),
    (
        "Clock::encoded_bits",
        "a stored-length read per part: no walk, no allocation",
    ),
    (
        "Clock::dangerously_alias",
        "one refcount bump per part: both stored buffers are shared",
    ),
    (
        "Version::ranked",
        "an O(1) borrowing view construction: no walk, no allocation",
    ),
    (
        "Ranked::version",
        "a borrow of the viewed version: no walk, no allocation",
    ),
    (
        "Ranked::into_owned",
        "at most one clone of the borrowed version (a refcount bump): no walk, no byte copy",
    ),
    (
        "causally::all",
        "stores two borrows; the comparison cost is on the membership predicates \
         (the causally_contains row)",
    ),
    (
        "causally::since",
        "stores two borrows; the comparison cost is on the membership predicates \
         (the causally_contains row)",
    ),
    (
        "causally::not_before",
        "stores two borrows; the comparison cost is on the membership predicates \
         (the causally_contains row)",
    ),
    (
        "causally::known_at",
        "stores two borrows; the comparison cost is on the membership predicates \
         (the causally_contains row)",
    ),
    (
        "causally::before",
        "stores two borrows; the comparison cost is on the membership predicates \
         (the causally_contains row)",
    ),
    (
        "causally::delta",
        "stores two borrows plus at most one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "causally::delta_before",
        "stores two borrows plus at most one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "causally::Range::since",
        "stores two borrows plus at most one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "causally::Range::not_before",
        "stores two borrows plus at most one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "causally::Range::known_at",
        "stores two borrows plus at most one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "causally::Range::before",
        "stores two borrows plus at most one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "Span::new",
        "stores two borrows plus one validating causal comparison, the \
         identical comparison the causally_contains row prices",
    ),
    (
        "Span::new_unchecked",
        "stores two borrows and performs no comparison at all: the trusted \
         door's debug assertion sits outside the cost contract",
    ),
    (
        "Span::at",
        "one refcount-bump buffer-sharing clone at most (a lent version is \
         stored as two borrows): no walk, no comparison",
    ),
    (
        "From<Version> for Span (the coincident constructor, owned and borrowed)",
        "the trait spellings of Span::at: one refcount-bump clone at most, \
         no walk, no comparison",
    ),
    (
        "Span::meet",
        "a borrow of a stored endpoint: no walk, no allocation",
    ),
    (
        "Span::join",
        "a borrow of a stored endpoint: no walk, no allocation",
    ),
    (
        "Span::into_parts",
        "at most one clone per borrowed endpoint (a refcount bump): no walk, no byte copy",
    ),
    (
        "Span::reborrow",
        "stores two fresh borrows of the stored endpoints: no walk, no \
         allocation, no comparison",
    ),
    (
        "Span::into_owned",
        "at most one clone per borrowed endpoint (a refcount bump): no walk, no byte copy",
    ),
    (
        "OwnSpan::meet",
        "O(1) view construction (two borrows); the comparison and \
         materialization costs are celled at the OwnVersion rows",
    ),
    (
        "OwnSpan::join",
        "O(1) view construction (two borrows); the comparison and \
         materialization costs are celled at the OwnVersion rows",
    ),
    (
        "OwnSpan::place",
        "two of the masked co-walks the OwnVersion comparison rows cell; the \
         nine-state transcription adds no walk of its own",
    ),
    (
        "OwnSpan::dominance",
        "at most two of the masked co-walks the OwnVersion comparison rows \
         cell, one when the start refutes",
    ),
    (
        "OwnSpan::to_span",
        "two of the materializations the OwnVersion to_version row cells, one \
         per endpoint",
    ),
    (
        "Span | Span (BitOr, owned and borrowed — the containment join)",
        "the celled version meet/join, one per endpoint pair; a point-like \
         operand pair fuses to the celled version_span walk",
    ),
    (
        "Span & Span (BitAnd, owned and borrowed — the containment meet)",
        "the celled version join/meet, one per endpoint pair, plus one \
         validating causal comparison",
    ),
    (
        "Span + Span (Add, owned and borrowed — the pointwise join)",
        "the celled version join, one per endpoint pair; a point-like operand \
         pair pays one join, shared across both legs",
    ),
    (
        "Span * Span (Mul, owned and borrowed — the pointwise meet)",
        "the celled version meet, one per endpoint pair; a point-like operand \
         pair pays one meet, shared across both legs",
    ),
    (
        "&Span / &Party (Div — the lazy span projection view)",
        "O(1) view construction (two borrows); the verdict and materialization \
         costs sit on the OwnSpan entries above",
    ),
    (
        "From<OwnSpan> for Span (explicit materialization)",
        "delegation to OwnSpan::to_span: two of the materializations the \
         OwnVersion to_version row cells",
    ),
    (
        "&Version / &Party (Div — the lazy projection view)",
        "O(1) view construction (two borrows); the materialization and fused \
         comparison costs are celled at the OwnVersion rows",
    ),
    (
        "From<Party> for [Party; N] (consuming balanced split)",
        "the forks machinery consuming its operand: the measured fork on \
         shrinking operands plus N moves",
    ),
    (
        "From<Clock> for [Clock; N] (consuming balanced split)",
        "the clock forks machinery consuming its operand: the measured fork on \
         shrinking operands plus one version refcount-bump clone per child",
    ),
    (
        "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        "iterate the measured fork on shrinking operands (one version clone per \
         clock child); a mid-run drop rejoins in O(log n) measured joins",
    ),
    (
        "Ticks ZERO / From / FromStr / Display / Add / Sum / Ord / Eq / Hash",
        "an opaque count carrier: word-to-width-scale arithmetic with no \
         packed-input axis; the operations denominated in it are celled at \
         their own rows (version_ticks, version_min_ticks)",
    ),
    (
        "unbounded depth (beyond the differential grids)",
        "a coverage disposition, not an operation: depth safety is pinned by \
         deep_tree_stack_safety, and every board family already scales depth",
    ),
    (
        "meter / error / iter plumbing",
        "instrumentation and data plumbing with no packed-input computation; \
         the meters are the measurement apparatus itself, feature-gated out of \
         production builds",
    ),
];

/// One in-flight triage entry in the red buffer: a board cell reading
/// red whose triage — a cure, or an owner-declared model at the cell —
/// someone owns but has not landed yet.
///
/// `exponent` is a scaling-class finding (some counter's growth exceeds
/// its ceiling — flat or declared); `constant` a proportionality finding
/// at exponent ~1 (a per-byte constant, a segments count, or a
/// declared-model band). The tags are the render's `mech[...]` column as
/// committed data: the class-binding seal in
/// `testing::complexity_claims` forbids any linear rustdoc claim from
/// citing an operation with a standing exponent-mechanism entry, and
/// requires every counter-superlinear claim to keep one.
pub struct ExpectedRed {
    /// The board row's operation name.
    pub op: &'static str,
    /// The input family.
    pub family: &'static str,
    /// Whether the cell reads red on an exponent mechanism at either
    /// acceptance scale.
    pub exponent: bool,
    /// Whether the cell reads red on a constant mechanism at either
    /// acceptance scale.
    pub constant: bool,
    /// The live task that owns this entry's triage.
    ///
    /// An entry with no owner is normalization of deviance; the
    /// acceptance assertion
    /// (`expected_red_buffer_is_an_empty_triage_buffer` in the
    /// complexity-claims suite) refuses it, and refuses any entry at
    /// all at acceptance.
    pub task: &'static str,
}

/// The red-triage buffer: board cells currently red whose triage is in
/// flight — **empty at acceptance, and empty on the settled tree**.
///
/// Red means untriaged, nothing else. Every dashboard contradiction
/// resolves to exactly one of: a cure, or an owner-declared model with
/// a dated rationale committed at the declaration site (the `ceilings`
/// module's declared-models section — the cell then reads green because the
/// behavior is intended and modeled). This buffer exists only so a
/// freshly-found red can be committed while its triage is worked; every
/// entry carries the live task that owns it, and the acceptance
/// assertion (`expected_red_buffer_is_an_empty_triage_buffer`) holds
/// the buffer EMPTY, so a red that persists across commits is a process
/// failure, not a status. The acceptance protocol diffs each rendered
/// red set against this list: on the settled tree both are empty and
/// the boards render all-green at both scales.
pub const BOARD_EXPECTED_REDS: &[ExpectedRed] = &[];
