//! The coverage rosters: both halves of the board tiling — committed data the
//! tiling test beside them enforces.
//!
//! Rows price delegations at their shared mechanism, so several surface rows
//! legitimately cite one row: `Clock::send` is `Clock::tick` by definition;
//! `clock | version` (either operand order, `|=` included) folds through the
//! same join-assign the `recv` row measures; `Party::tick` is `Version::tick`'s
//! mirror (the `tick_adv_party` row); the operator matrix (`|`, `&`, and their
//! assign forms, over every borrow shape) routes through the same
//! `join_view`/`meet_view` emitters and cmp walk the `join`/`meet`/`cmp` rows
//! measure; `Version::concurrent` is one `partial_cmp` and keeps its own row as
//! the documented entry point; the serde/borsh wrappers serialize as the
//! canonical encoding and deserialize through the strict decoder (the
//! `encode`/`decode` rows); `Party::ticks` and `Clock::ticks` run the same
//! fused kernel as `version_ticks` through their own spellings. Derived
//! surfaces with no roster row of their own ride the same cells: `Clone` copies
//! stored bits or value content wholesale with no walk in the contract, `Debug`
//! delegates to `Display`, and the byte-compare `Eq`s are the `eq`/`hash` rows'
//! wholesale compares.
//!
//! Two coverage notes that are dispositions of *error paths*, not operations
//! (so they live here rather than in the table): the rejection rows price the
//! fallible surface (the board module doc's rejection section; the `defect`
//! module carries the placed defects), and **the rejection surface's
//! bounded-or-delegated remainder** is: `Clock::join_all`'s overlap hand-back
//! runs the identical up-front indexed test against self that
//! `party_join_all_overlap` prices, inline; clock non-canonicality — packed or
//! text — is the component validators on the same streams the version and party
//! non-canonical rows drive; [`Parse::Anonymous`](crate::error::Parse) is the
//! one-token `"0"`; [`Decode::Io`](crate::error::Decode) is the caller's reader
//! — a failing reader is a truncation carrying an error, priced by the
//! truncated rows — and `encode_to`'s error the caller's writer, at most the
//! encode row's work before it propagates; the `TryFrom` literal rejections
//! have word-scale or type-bounded operands; `Rank::checked_sub`'s `None` is
//! measured on the `rank_pair_ops` row, which attempts both directions; other
//! decode non-canonicality genres (a negative running height, malformed padding)
//! ride
//! the same single validator pass at the same full-parse cost as the committed
//! maximally-deferred tails; serde/borsh deserialize errors are the strict
//! decoder through the wrappers (the decode rejection rows). `Debug` for all
//! three types delegates to `Display`.

/// The board's priced table: every `before::surface` row measured by the board,
/// with the board rows that price it.
///
/// Rows price delegations at their shared mechanism (the module doc maps the
/// delegations), so several surface rows legitimately cite one row.
///
/// The tiling test beside this table
/// (`board_coverage_tiles_the_public_surface`) holds it and
/// [`BOARD_NOT_APPLICABLE`] disjoint and jointly total over the public surface,
/// every cited row live on the board's operation axis, and every board row
/// cited: an operation is priced by named rows or excused, never both, never
/// neither, and the board carries no orphan row.
pub const BOARD_PRICED: &[(&str, &[&str])] = &[
    ("Party::tick", &["version_tick", "version_tick_adv_party"]),
    ("Party::ticks", &["version_ticks"]),
    ("Party::fork", &["party_fork"]),
    ("Party::join", &["party_join", "party_join_overlap"]),
    (
        "Party::join_all",
        &["party_join_all", "party_join_all_overlap"],
    ),
    ("Party::is_disjoint", &["party_disjoint"]),
    ("Party::covers", &["party_covers"]),
    ("Party::without", &["party_without", "party_without_none"]),
    ("Party::encode", &["party_encode"]),
    ("Party::encode_to", &["party_encode"]),
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
    ("Clock::join", &["clock_join", "clock_join_overlap"]),
    ("Clock::join_all", &["version_join_all", "party_join_all"]),
    ("Clock::sync", &["clock_sync", "clock_sync_overlap"]),
    ("Clock::sync_all", &["version_join_all", "party_join_all"]),
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
    (
        "Clock::decode",
        &[
            "clock_decode",
            "clock_decode_truncated",
            "clock_decode_trailing",
        ],
    ),
    ("Rank::checked_sub", &["rank_pair_ops"]),
    ("Rank::saturating_sub", &["rank_pair_ops"]),
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
        &["causally_contains", "query_contains"],
    ),
    ("causally::Query::coverage", &["query_coverage"]),
    ("Span::place", &["span_place"]),
    ("Span::dominance", &["span_dominance"]),
    ("Span::precedence", &["span_precedence"]),
    ("Span::contains", &["span_contains"]),
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
        "Span Sum / FromIterator (owned and borrowed — the union fold)",
        &["version_span_all"],
    ),
    (
        "Span Product (owned and borrowed — the intersection fold)",
        &["version_join_all", "version_meet_all"],
    ),
    (
        "Version Eq / Hash (canonical byte compare)",
        &["version_eq", "version_hash"],
    ),
    ("Party Eq / Hash (canonical byte compare)", &["party_hash"]),
    (
        "Clock | Version and Version | Clock (heterogeneous joins, |=)",
        &["clock_recv", "clock_hash"],
    ),
    (
        "Party Display / FromStr / TryFrom literals",
        &[
            "party_display",
            "party_from_str",
            "party_parse_trailing",
            "party_parse_noncanon",
        ],
    ),
    (
        "Version Display / FromStr / TryFrom literals",
        &[
            "version_display",
            "version_from_str",
            "version_parse_trailing",
            "version_parse_noncanon",
        ],
    ),
    (
        "Clock Display / FromStr / TryFrom",
        &["clock_display", "clock_from_str", "clock_parse_trailing"],
    ),
    (
        "serde / borsh impls (feature-gated, strict-decode pinned)",
        &[
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
        "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display",
        &["rank_pair_ops", "rank_sum"],
    ),
    (
        "Ranked comparisons and the Ranked / Rank From conversions (the total order)",
        &["ranked_cmp"],
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
        "causally::Query::into_owned",
        "one refcount bump per borrowed bound: no walk, no byte copy",
    ),
    (
        "causally & conjunction (atoms and queries, every admitted pairing)",
        "the celled version join/meet on same-side bounds, plus the hole \
         re-admission: one masked comparison (the version_cmp row's walk) \
         against the merged bound per hole and one per cross-side hole pair \
         — bilinear in the operands' hole counts, never in the bound sizes",
    ),
    (
        "causally ! complement (atom negation into the polar hole)",
        "O(1) hole mint over the atom's bound: no comparison, no walk",
    ),
    (
        "From into Query (atoms, spans, versions, borrowed queries)",
        "O(1) constructions through the cross-side merge, which performs no \
         comparison; the membership rows price the walks the queries feed",
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
        "Span + Span (Add, owned and borrowed — the containment join)",
        "the celled version meet/join, one per endpoint pair; a point-like \
         operand pair fuses to the celled version_span walk",
    ),
    (
        "Span * Span (Mul, owned and borrowed — the containment meet)",
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
        "Span | Span (BitOr, owned and borrowed — the pointwise join)",
        "the celled version join, one per endpoint pair; a point-like operand \
         pair pays one join, shared across both legs",
    ),
    (
        "Span & Span (BitAnd, owned and borrowed — the pointwise meet)",
        "the celled version meet, one per endpoint pair; a point-like operand \
         pair pays one meet, shared across both legs",
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

#[cfg(test)]
mod tests;
