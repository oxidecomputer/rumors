//! Coverage tables for the amplification board.
//!
//! [`BOARD_PRICED`] maps each rostered public method or grouped trait family to
//! the measurements that bound its cost. Several operations may share a
//! measurement when they use the same implementation.
//! [`BOARD_NOT_APPLICABLE`] records rows for which the board's resource model
//! does not apply. Tests require the tables to cover the method and family
//! rosters without overlap and require every named measurement to exist.
//!
//! Rejection rows place errors as late as possible in the input. Errors handled
//! by the same decoder, parser, or pairwise operation share that measurement;
//! errors bounded by a machine word or caller-owned I/O need no separate row.

/// Measurements that bound each priced method or trait family's resource use.
///
/// The coverage test requires every rostered method and trait family to appear
/// in exactly one coverage table and every named measurement to exist and be
/// used.
pub const BOARD_PRICED: &[(&str, &[&str])] = &[
    ("Party::tick", &["version_tick", "version_tick_adv_party"]),
    ("Party::ticks", &["version_ticks"]),
    ("Party::fork", &["party_fork"]),
    ("Party::forks", &["party_forks", "party_forks_full"]),
    ("Party::join", &["party_join", "party_join_overlap"]),
    ("Party::join_all", &["party_join_all"]),
    ("Party::is_disjoint", &["party_disjoint"]),
    ("Party::covers", &["party_covers"]),
    ("Party::without", &["party_without", "party_without_none"]),
    ("Party::encode", &["party_encode"]),
    ("Party::encode_to", &["party_encode"]),
    ("Party::shape", &["party_shape"]),
    (
        "Party::decode",
        &[
            "party_decode",
            "party_decode_truncated",
            "party_decode_trailing",
            "party_decode_noncanon",
        ],
    ),
    ("Version::tick", &["version_tick", "version_tick_adv_party"]),
    ("Version::ticks", &["version_ticks"]),
    ("Version::concurrent", &["version_concurrent"]),
    ("Version::min_ticks", &["version_min_ticks"]),
    ("Version::rank", &["version_rank"]),
    ("Version::encode_rank", &["ranked_encode_rank"]),
    ("Version::encode_rank_to", &["ranked_encode_rank"]),
    ("Version::distance", &["version_distance"]),
    ("Version::lag", &["version_lag"]),
    ("Version::join_all", &["version_join_all"]),
    ("Version::meet_all", &["version_meet_all"]),
    ("Version::join", &["version_join"]),
    ("Version::meet", &["version_meet"]),
    ("Version::span", &["version_span"]),
    ("Version::span_all", &["version_span_all"]),
    ("Version::encode", &["version_encode"]),
    ("Version::encode_to", &["version_encode"]),
    ("Version::shape", &["version_shape"]),
    (
        "Version::decode",
        &[
            "version_decode",
            "version_decode_truncated",
            "version_decode_trailing",
            "version_decode_noncanon",
        ],
    ),
    ("Clock::tick", &["clock_tick"]),
    ("Clock::ticks", &["version_ticks"]),
    ("Clock::fork", &["clock_fork"]),
    ("Clock::forks", &["clock_forks", "clock_forks_full"]),
    ("Clock::join", &["clock_join", "clock_join_overlap"]),
    ("Clock::join_all", &["version_join_all", "party_join_all"]),
    ("Clock::sync", &["clock_sync", "clock_sync_overlap"]),
    (
        "Clock::sync_all",
        &[
            "clock_sync_all",
            "version_join_all",
            "party_join_all",
        ],
    ),
    ("Clock::send", &["clock_tick"]),
    ("Clock::recv", &["clock_recv"]),
    ("Clock::recv_all", &["version_join_all"]),
    ("Clock::absorb", &["version_join"]),
    ("Clock::absorb_all", &["version_join_all"]),
    (
        "OwnVersion::to_version",
        &["own_version_to_version", "clock_own_version_to_version"],
    ),
    ("Clock::encode", &["clock_encode"]),
    ("Clock::encode_to", &["clock_encode"]),
    ("Clock::shape", &["clock_shape"]),
    (
        "Clock::decode",
        &[
            "clock_decode",
            "clock_decode_truncated",
            "clock_decode_trailing",
        ],
    ),
    ("Rank::checked_sub", &["rank_checked_sub"]),
    ("Rank::saturating_sub", &["rank_checked_sub"]),
    ("Rank::encode", &["rank_encode"]),
    ("Rank::encode_to", &["rank_encode"]),
    ("Rank::decode", &["rank_decode"]),
    ("Ranked::rank", &["version_rank"]),
    ("Ranked::encode", &["ranked_encode"]),
    ("Ranked::encode_to", &["ranked_encode"]),
    ("Ranked::encode_rank", &["ranked_encode_rank"]),
    ("Ranked::encode_rank_to", &["ranked_encode_rank"]),
    ("Ranked::decode", &["ranked_decode"]),
    ("causally::Floor::contains", &["causally_contains"]),
    ("causally::Ceiling::contains", &["causally_contains"]),
    (
        "causally::Query::contains",
        &[
            "causally_contains",
            "query_contains",
            "query_contains_many",
        ],
    ),
    (
        "causally::Query::coverage",
        &["query_coverage", "query_coverage_many"],
    ),
    ("causally::Query::into_owned", &["query_clone_many"]),
    (
        "causally & conjunction (atoms and queries, every admitted pairing)",
        &["version_join", "version_meet", "query_conjoin_many"],
    ),
    ("From into Query (atoms, spans, versions, borrowed queries)", &["query_clone_many"]),
    ("Span::place", &["span_place"]),
    ("Span::dominance", &["span_dominance"]),
    ("Span::precedence", &["span_precedence"]),
    // The span-shaped argument arm pays two causal comparisons instead of
    // the fused walk; the version_cmp rows price that pair.
    ("Span::contains", &["span_contains", "version_cmp"]),
    ("Span::encode", &["span_encode"]),
    ("Span::encode_to", &["span_encode"]),
    (
        "Span::decode",
        &[
            "span_decode",
            "span_decode_truncated",
            "span_decode_trailing",
            "span_decode_crossed",
        ],
    ),
    ("Span::union_all", &["version_span_all"]),
    (
        "Span::intersect_all",
        &["version_join_all", "version_meet_all"],
    ),
    ("Span::join_all", &["version_join_all"]),
    ("Span::meet_all", &["version_meet_all"]),
    (
        "shape::combine",
        &["shape_combine_pair", "shape_combine_many"],
    ),
    (
        "Version | Version (BitOr/BitOrAssign, owned and borrowed)",
        &["version_join", "version_join_assign"],
    ),
    (
        "Version & Version (BitAnd/BitAndAssign, owned and borrowed)",
        &["version_meet", "version_meet_assign"],
    ),
    (
        "Version ^ Version (BitXor, owned and borrowed — the pair hull)",
        &["version_span"],
    ),
    (
        "OwnVersion vs Version comparisons (PartialEq/PartialOrd, both directions, owned and borrowed)",
        &["own_version_cmp"],
    ),
    (
        "OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)",
        &["own_version_pair_cmp"],
    ),
    (
        "From<OwnVersion> for Version (explicit materialization)",
        &["own_version_to_version"],
    ),
    (
        "Version PartialOrd (the comparison matrix, owned and borrowed)",
        &["version_cmp"],
    ),
    (
        "Version Sum / FromIterator (owned and borrowed)",
        &["version_join_all"],
    ),
    (
        "Span Sum / FromIterator (spans or versions, owned and borrowed — the union fold)",
        &["version_span_all"],
    ),
    (
        "Span Product (spans only, owned and borrowed — the intersection fold)",
        &["version_join_all", "version_meet_all"],
    ),
    (
        "Version Eq / Hash (canonical byte compare)",
        &["version_eq", "version_hash"],
    ),
    ("Party Eq / Hash (canonical byte compare)", &["party_hash"]),
    ("Clock Eq / Hash (canonical byte compare)", &["clock_hash"]),
    (
        "Clock | Version and Version | Clock (heterogeneous joins, |=)",
        &["clock_recv", "clock_hash"],
    ),
    (
        "serde / borsh impls (feature-gated, strict-decode pinned)",
        &[
            #[cfg(feature = "serde")]
            "party_serde_deserialize",
            #[cfg(feature = "serde")]
            "version_serde_deserialize",
            #[cfg(feature = "borsh")]
            "party_borsh_deserialize",
            #[cfg(feature = "borsh")]
            "version_borsh_deserialize",
            "version_encode",
            "version_decode",
            "party_encode",
            "party_decode",
            "clock_encode",
            "clock_decode",
            "rank_encode",
            "rank_decode",
            "ranked_encode",
            "ranked_decode",
            "span_encode",
            "span_decode",
        ],
    ),
    (
        "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display / FromStr",
        &[
            "rank_add",
            "rank_checked_sub",
            "rank_cmp",
            "rank_sum",
            "rank_display",
            "rank_display_precision",
            "rank_parse",
        ],
    ),
    (
        "Ranked comparisons and the Ranked / Rank From conversions (the total order)",
        &["ranked_cmp"],
    ),
    (
        "From<Party> for [Party; N] (consuming balanced split)",
        &["party_split_array"],
    ),
    (
        "From<Clock> for [Clock; N] (consuming balanced split)",
        &["clock_split_array"],
    ),
    (
        "iter::Party / iter::Clock (fork iterators and partial-drop conservation)",
        &[
            "party_forks",
            "party_forks_full",
            "clock_forks",
            "clock_forks_full",
        ],
    ),
];

/// The board's not-applicable table: every `before::surface` row with no board
/// row of its own, and the mechanism-based reason why.
///
/// The machine-readable excused half of the board's coverage tiling, covering
/// method and family rows alike; [`BOARD_PRICED`] is the priced half, and the
/// tiling test beside them holds the two disjoint and jointly total over the
/// public surface.
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
        "stores no constraints at all; the membership and coverage rows price \
         the sweeps",
    ),
    (
        "causally::after",
        "stores its bound version and performs no walk; the membership and \
         coverage rows price the sweeps",
    ),
    (
        "causally::before",
        "stores its bound version and performs no walk; the membership and \
         coverage rows price the sweeps",
    ),
    (
        "causally::since",
        "stores its bound version as one hole and performs no walk; the \
         membership and coverage rows price the sweeps",
    ),
    (
        "causally::until",
        "stores its bound version as one hole and performs no walk; the \
         membership and coverage rows price the sweeps",
    ),
    (
        "causally::strictly_after",
        "stores its bound version as floor and hole (one buffer-sharing clone) \
         and performs no walk; the membership and coverage rows price the sweeps",
    ),
    (
        "causally::strictly_before",
        "stores its bound version as ceiling and hole (one buffer-sharing clone) \
         and performs no walk; the membership and coverage rows price the sweeps",
    ),
    (
        "causally::delta",
        "assembles its bounds through the cross-side merge, which performs no \
         comparison; the query_coverage row prices the verdicts it feeds",
    ),
    (
        "causally::toward",
        "assembles its bounds through the cross-side merge, which performs no \
         comparison; the query_coverage row prices the verdicts it feeds",
    ),
    (
        "causally::Floor::or_concurrent",
        "moves its bound version into one hole and performs no walk; the \
         membership and coverage rows price the sweeps",
    ),
    (
        "causally::Ceiling::or_concurrent",
        "moves its bound version into one hole and performs no walk; the \
         membership and coverage rows price the sweeps",
    ),
    (
        "causally ! complement (atom negation into the polar hole)",
        "O(1) hole mint over the atom's bound: no comparison, no walk",
    ),
    (
        "Span::new",
        "stores two borrows plus one validating causal comparison, the \
         identical comparison the causally_contains row prices",
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
        "Span::lo",
        "a borrow of a stored endpoint: no walk, no allocation",
    ),
    (
        "Span::hi",
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
        "OwnSpan::lo",
        "O(1) view construction (two borrows); the comparison and \
         materialization costs are celled at the OwnVersion rows",
    ),
    (
        "OwnSpan::hi",
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
        "OwnSpan::precedence",
        "at most two of the masked co-walks the OwnVersion comparison rows \
         cell, one when the end refutes",
    ),
    (
        "OwnSpan::contains",
        "at most two of the masked co-walks the OwnVersion comparison rows \
         cell, one when the start refutes",
    ),
    (
        "OwnSpan::to_span",
        "two of the materializations the OwnVersion to_version row cells, one \
         per endpoint",
    ),
    (
        "Span + impl Into<Span> / Version + Span (Add/AddAssign, owned and borrowed — the containment join)",
        "the celled version meet/join, one per endpoint pair; a point-like \
         operand pair fuses to the celled version_span walk; assigning is \
         the value kernel written back",
    ),
    (
        "Span * Span (Mul, owned and borrowed — the containment meet; partial, so no MulAssign and no Into widening)",
        "the celled version join/meet, one per endpoint pair, plus one \
         validating causal comparison",
    ),
    (
        "Span::union",
        "the method spelling of the containment join (`+`): the celled \
         version meet/join, one per endpoint pair",
    ),
    (
        "Span::intersect",
        "the method spelling of the containment meet (`*`): the celled \
         version join/meet, one per endpoint pair, plus one validating \
         causal comparison",
    ),
    (
        "Span | impl Into<Span> / Version | Span (BitOr/BitOrAssign, owned and borrowed — the pointwise join)",
        "the celled version join, one per endpoint pair; a point-like operand \
         pair pays one join, shared across both legs; assigning is the value \
         kernel written back",
    ),
    (
        "Span & impl Into<Span> / Version & Span (BitAnd/BitAndAssign, owned and borrowed — the pointwise meet)",
        "the celled version meet, one per endpoint pair; a point-like operand \
         pair pays one meet, shared across both legs; assigning is the value \
         kernel written back",
    ),
    (
        "Span::join",
        "the method spelling of the pointwise join (`|`): the celled version \
         join, one per endpoint pair",
    ),
    (
        "Span::meet",
        "the method spelling of the pointwise meet (`&`): the celled version \
         meet, one per endpoint pair",
    ),
    (
        "&Span / &Party (Div — the lazy span projection view)",
        "O(1) view construction (two borrows); the verdict and materialization \
         costs sit on the OwnSpan entries above",
    ),
    (
        "Span::project",
        "the named spelling of the span projection (`/`): O(1) view \
         construction (two borrows); the verdict and materialization costs \
         sit on the OwnSpan entries above",
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
        "Version::project",
        "the named spelling of the projection (`/`): O(1) view construction \
         (two borrows); the materialization and fused comparison costs are \
         celled at the OwnVersion rows",
    ),
    (
        "Ticks ZERO / From / TryFrom / Display / Add / Sum / Ord / Eq / Hash",
        "an opaque count carrier: word-to-width-scale arithmetic with no \
         encoded-input axis; the operations denominated in it are celled at \
         their own rows (version_ticks, version_min_ticks)",
    ),
    (
        "Ticks::limbs",
        "a borrowing view of the stored count: word-scale construction, one \
         word per step, no encoded-input axis",
    ),
    (
        "shape iterators (Plateaus / Regions / Overlay / Cells / Limbs: Iterator, FusedIterator, ExactSizeIterator)",
        "iterator traits add no work beyond each item; the shape methods are \
         measured directly, and Ticks::limbs has its own disposition",
    ),
    (
        "shape item types (Plateau / Rise / Region / Cell: Clone, Eq, Debug)",
        "value carriers: word-scale fields plus one count held at its own \
         width",
    ),
    (
        "unbounded depth (beyond the differential grids)",
        "a coverage disposition, not an operation: depth safety is pinned by \
         deep_tree_stack_safety, and every board family already scales depth",
    ),
    (
        "meter instrumentation plumbing",
        "the measurement apparatus itself, feature-gated out of production \
         builds; no encoded-input computation of its own",
    ),
    (
        "error verdict types (Decode / Crossed)",
        "verdict carriers with no encoded-input computation; the operations \
         that produce them are measured directly",
    ),
];

#[cfg(test)]
mod tests;
