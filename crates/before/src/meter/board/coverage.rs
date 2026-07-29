//! The coverage rosters: the not-applicable half of the board tiling and
//! the red-triage buffer — committed data the complexity-claims suite
//! enforces.

/// The board's not-applicable table: every `before::surface` row with
/// no board row of its own, and the mechanism-based reason why.
///
/// The machine-readable half of the module doc's coverage tiling,
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
        "one byte copy of the stored canonical bits",
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
         (a byte copy) per child",
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
    ("Clock::dangerously_alias", "one byte copy per part"),
    (
        "Ranked::version",
        "a borrow of a stored part: no walk, no allocation",
    ),
    (
        "Ranked::rank",
        "a borrow of a stored part: no walk, no allocation",
    ),
    (
        "Ranked::into_parts",
        "two moves of the stored parts: no walk, no allocation",
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
/// a dated rationale committed at the declaration site (the
/// declared-models section — the cell then reads green because the
/// behavior is intended and modeled). This buffer exists only so a
/// freshly-found red can be committed while its triage is worked; every
/// entry carries the live task that owns it, and the acceptance
/// assertion (`expected_red_buffer_is_an_empty_triage_buffer`) holds
/// the buffer EMPTY, so a red that persists across commits is a process
/// failure, not a status. The acceptance protocol diffs each rendered
/// red set against this list: on the settled tree both are empty and
/// the boards render all-green at both scales.
pub const BOARD_EXPECTED_REDS: &[ExpectedRed] = &[];
