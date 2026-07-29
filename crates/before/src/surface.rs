//! The public operation surface, as data: one row per public operation,
//! with each differential leg's disposition.
//!
//! This module is the machine-readable roster the surface-coverage suite (the
//! crate's test-only differential architecture) enforces totality over:
//! its tests hold
//! [`METHOD_SURFACE`](crate::surface::METHOD_SURFACE) equal, name for
//! name, to the inherent `pub fn`
//! surface extracted from the public-API source files, and hold every
//! cited test name resolvable to an executable binding. The rows live
//! here, outside the test-only tree, so external instrument crates can
//! bind their own coverage to the same roster — a coverage table keyed by
//! these row names is total over the public surface exactly as far as the
//! coverage suite's totality pins reach, with no second hand-maintained
//! enumeration to drift. Public under the `meter` feature (the instrument
//! crates' feature) and never part of a production build.
//!
//! Leg vocabulary, exclusion families, and the adequacy tripwires are the
//! coverage suite's business and are documented there; a row's
//! dispositions are carried here verbatim as the suite's committed record.

/// One leg's disposition: how (or whether) two of the three
/// implementations are compared for one operation.
#[derive(Debug)]
pub enum Leg {
    /// A direct differential on this leg, by test name.
    Bound(&'static str),
    /// An algebraic-law pin on production alone, by test name.
    Law(&'static str),
    /// Transitively bound (definitional reduction, or composition of the
    /// other two legs); the named test anchors the reduction.
    Trans(&'static str),
    /// Not bound, with the reason.
    Excluded(&'static str),
}

impl Leg {
    /// The test name this disposition cites, if any.
    pub fn cited(&self) -> Option<&'static str> {
        match self {
            Leg::Bound(t) | Leg::Law(t) | Leg::Trans(t) => Some(t),
            Leg::Excluded(_) => None,
        }
    }

    /// The exclusion reason, if this leg is excluded.
    pub fn exclusion_reason(&self) -> Option<&'static str> {
        match self {
            Leg::Excluded(reason) => Some(reason),
            _ => None,
        }
    }
}

/// One row of the roster: a public operation and its three leg
/// dispositions.
pub struct SurfaceRow {
    /// The operation, named as the coverage suite's extractor names it
    /// (`Type::fn`, `module::fn`) for [`METHOD_SURFACE`], or as a family
    /// description for [`FAMILY_SURFACE`].
    pub op: &'static str,
    /// The production ↔ recursive-oracle leg.
    pub prod_tree: Leg,
    /// The production ↔ function-space leg.
    pub prod_fs: Leg,
    /// The recursive-oracle ↔ function-space leg.
    pub tree_fs: Leg,
}

/// Shorthand for a codec/text method row: representation is exactly what
/// both references quotient away, so all three legs are excluded and
/// correctness lives in the production-side pins.
const fn codec_row(op: &'static str) -> SurfaceRow {
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(
            "no wire format exists in the paper or the oracle; correctness is \
             production-side pins (decode_encode_arbitrary, as_bytes_matches_encode, \
             decode_never_panics, snapshot goldens, exhaustive codec checks)",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    }
}

/// Shorthand for a `causally` row: every predicate is a definitional
/// combination of `partial_cmp` verdicts, which are bound on all three
/// legs; binding the combinator adds sampling of a totally-derived form.
const fn causally_row(op: &'static str) -> SurfaceRow {
    const REASON: &str = "definitional combinator over the bound causal order \
         (partial_cmp); unit-tested in causally/tests.rs";
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(REASON),
        prod_fs: Leg::Excluded(REASON),
        tree_fs: Leg::Excluded(REASON),
    }
}

/// The n-ary hand-back exclusion, shared by the `join_all`/`forks`
/// family of rows; the reason carries the half-binding rationale.
const HANDBACK: &str = "hand-back value identity and order against the fixed accumulator \
     are not functions of the geometry, and a verdict-only binding would read as \
     coverage while the hand-back contract stayed unbound — ratified by owner, \
     2026-07-26";

/// The roster over the mechanically-extracted inherent `pub fn` surface.
/// The surface-coverage suite's `roster_is_total_over_the_public_fn_surface` holds
/// this equal, name for name, to its extractor's listing.
pub const METHOD_SURFACE: &[SurfaceRow] = &[
    // ───────────────────────────── Party ─────────────────────────────
    SurfaceRow {
        op: "Party::seed",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Party::is_seed",
        prod_tree: Leg::Bound("is_seed_matches_the_oracle"),
        prod_fs: Leg::Excluded(
            "API-convenience predicate; the function-space boundary excludes Rust API \
             mechanics — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded(
            "definitional: in normal form the full region is exactly the oracle seed \
             leaf, which the prod↔tree leg compares against",
        ),
    },
    SurfaceRow {
        op: "Party::tick",
        prod_tree: Leg::Trans("tick_matches_oracle"),
        prod_fs: Leg::Trans("event_dominates_local_and_advances"),
        tree_fs: Leg::Trans("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Party::ticks",
        prod_tree: Leg::Trans("party_ticks_matches_version_ticks"),
        prod_fs: Leg::Trans("party_ticks_matches_version_ticks"),
        tree_fs: Leg::Trans("party_ticks_matches_version_ticks"),
    },
    SurfaceRow {
        op: "Party::fork",
        prod_tree: Leg::Bound("d_fork_join_roundtrip"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("fork_partitions"),
    },
    SurfaceRow {
        op: "Party::forks",
        prod_tree: Leg::Law("forks_matches_from_array"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(
            "no oracle n-ary split; the balanced-split shape is law-pinned on \
             production (forks_partial_drop_folds_back, party_join_all_reunites_a_fork)",
        ),
    },
    SurfaceRow {
        op: "Party::join",
        prod_tree: Leg::Bound("sum_arbitrary"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("sum_of_disjoint_is_union"),
    },
    SurfaceRow {
        op: "Party::join_all",
        prod_tree: Leg::Bound("join_all_matches_the_recursive_oracle"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(HANDBACK),
    },
    SurfaceRow {
        op: "Party::is_disjoint",
        prod_tree: Leg::Bound("disjoint_arbitrary"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Party::covers",
        prod_tree: Leg::Bound("covers_arbitrary"),
        prod_fs: Leg::Trans("covers_realizes_containment"),
        tree_fs: Leg::Bound("covers_realizes_containment"),
    },
    SurfaceRow {
        op: "Party::without",
        prod_tree: Leg::Bound("without_arbitrary"),
        prod_fs: Leg::Trans("without_realizes_region_difference"),
        tree_fs: Leg::Bound("without_realizes_region_difference"),
    },
    SurfaceRow {
        op: "Party::dangerously_alias",
        prod_tree: Leg::Excluded(
            "aliasing violates production linearity by design; the Clone oracle has no \
             counterpart — pinned on production by the alias_is_byte_identical_overlap law",
        ),
        prod_fs: Leg::Excluded(
            "linearity mechanics of the Rust API — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded("linearity mechanics of the Rust API"),
    },
    codec_row("Party::encode"),
    codec_row("Party::encode_to"),
    codec_row("Party::encoded_bits"),
    codec_row("Party::decode"),
    codec_row("Party::as_bytes"),
    // ───────────────────────────── Version ─────────────────────────────
    SurfaceRow {
        op: "Version::new",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version::is_empty",
        prod_tree: Leg::Trans("is_empty_iff_new"),
        prod_fs: Leg::Trans("is_empty_iff_new"),
        tree_fs: Leg::Excluded(
            "definitional: emptiness is equality with the empty version, which the \
             other legs compare",
        ),
    },
    SurfaceRow {
        op: "Version::tick",
        prod_tree: Leg::Bound("tick_arbitrary"),
        prod_fs: Leg::Bound("event_dominates_local_and_advances"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version::ticks",
        prod_tree: Leg::Bound("ticks_matches_oracle"),
        prod_fs: Leg::Trans("ticks_agrees_with_iterated_ticks"),
        tree_fs: Leg::Trans("ticks_matches_oracle"),
    },
    SurfaceRow {
        op: "Version::concurrent",
        prod_tree: Leg::Bound("clock_observers_match_oracle"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version::min_ticks",
        prod_tree: Leg::Bound("min_ticks_matches_oracle"),
        prod_fs: Leg::Trans("min_ticks_realizes_base_sum"),
        tree_fs: Leg::Bound("min_ticks_realizes_base_sum"),
    },
    SurfaceRow {
        op: "Version::rank",
        prod_tree: Leg::Bound("rank_matches_oracle"),
        prod_fs: Leg::Bound("rank_realizes_riemann_sum"),
        tree_fs: Leg::Bound("rank_realizes_riemann_sum"),
    },
    SurfaceRow {
        op: "Version::distance",
        prod_tree: Leg::Bound("distance_and_lag_realize_both_oracles"),
        prod_fs: Leg::Bound("distance_and_lag_realize_both_oracles"),
        tree_fs: Leg::Bound("distance_and_lag_realize_both_oracles"),
    },
    SurfaceRow {
        op: "Version::lag",
        prod_tree: Leg::Bound("distance_and_lag_realize_both_oracles"),
        prod_fs: Leg::Bound("distance_and_lag_realize_both_oracles"),
        tree_fs: Leg::Bound("distance_and_lag_realize_both_oracles"),
    },
    SurfaceRow {
        op: "Version::join_all",
        prod_tree: Leg::Trans("join_all_equals_the_sequential_fold"),
        prod_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner, 2026-07-26",
        ),
    },
    SurfaceRow {
        op: "Version::meet_all",
        prod_tree: Leg::Bound("meet_all_matches_oracle"),
        prod_fs: Leg::Excluded(
            "n-ary pointwise-min realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded(
            "n-ary pointwise-min realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner, 2026-07-26",
        ),
    },
    codec_row("Version::encode"),
    codec_row("Version::encode_to"),
    codec_row("Version::decode"),
    codec_row("Version::encoded_bits"),
    codec_row("Version::as_bytes"),
    // ───────────────────────────── Clock ─────────────────────────────
    SurfaceRow {
        op: "Clock::seed",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::tick",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::ticks",
        prod_tree: Leg::Trans("clock_ticks_matches_version_ticks"),
        prod_fs: Leg::Trans("clock_ticks_matches_version_ticks"),
        tree_fs: Leg::Trans("clock_ticks_matches_version_ticks"),
    },
    SurfaceRow {
        op: "Clock::fork",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::join",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::join_all",
        prod_tree: Leg::Bound("join_all_matches_the_recursive_oracle"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(HANDBACK),
    },
    SurfaceRow {
        op: "Clock::forks",
        prod_tree: Leg::Trans("join_all_agrees_with_oracle_on_forked_and_aliased_populations"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(
            "no oracle n-ary split; composition of the party split and a version clone",
        ),
    },
    SurfaceRow {
        op: "Clock::sync",
        prod_tree: Leg::Bound("sync"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::send",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::recv",
        prod_tree: Leg::Bound("master_differential"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::from_parts",
        prod_tree: Leg::Trans("master_differential"),
        prod_fs: Leg::Trans("replay_matches_across_references"),
        tree_fs: Leg::Trans("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::into_parts",
        prod_tree: Leg::Trans("master_differential"),
        prod_fs: Leg::Trans("replay_matches_across_references"),
        tree_fs: Leg::Trans("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::party",
        prod_tree: Leg::Trans("master_differential"),
        prod_fs: Leg::Trans("replay_matches_across_references"),
        tree_fs: Leg::Trans("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::version",
        prod_tree: Leg::Trans("master_differential"),
        prod_fs: Leg::Trans("replay_matches_across_references"),
        tree_fs: Leg::Trans("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Clock::own_version",
        prod_tree: Leg::Bound("own_version_matches_oracle"),
        prod_fs: Leg::Trans("quotient_realizes_region_mask"),
        tree_fs: Leg::Bound("quotient_realizes_region_mask"),
    },
    SurfaceRow {
        op: "OwnVersion::to_version",
        prod_tree: Leg::Bound("div_matches_oracle"),
        prod_fs: Leg::Trans("quotient_realizes_region_mask"),
        tree_fs: Leg::Bound("quotient_realizes_region_mask"),
    },
    codec_row("Clock::encode"),
    codec_row("Clock::encode_to"),
    codec_row("Clock::decode"),
    codec_row("Clock::encoded_bits"),
    SurfaceRow {
        op: "Clock::dangerously_alias",
        prod_tree: Leg::Excluded(
            "linearity mechanics; an O(1) two-field composition over the party alias, \
             which the alias_is_byte_identical_overlap law pins",
        ),
        prod_fs: Leg::Excluded(
            "linearity mechanics of the Rust API — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded("linearity mechanics of the Rust API"),
    },
    // ───────────────────────────── Rank / Ranked ─────────────────────────────
    SurfaceRow {
        op: "Rank::checked_sub",
        prod_tree: Leg::Law("rank_checked_sub_iff_dominated"),
        prod_fs: Leg::Excluded(
            "Rank is not a paper object; the rank quantity itself is bound on all \
             three legs at Version::rank, and Rank's order/arithmetic to the in-test \
             alignment oracle",
        ),
        tree_fs: Leg::Excluded(
            "Rank is not a paper object; see the Version::rank row and the alignment \
             oracle",
        ),
    },
    SurfaceRow {
        op: "Ranked::version",
        prod_tree: Leg::Law("ranked_sort_respects_causality"),
        prod_fs: Leg::Excluded("accessor over the byte-tiebroken total order; law-pinned"),
        tree_fs: Leg::Excluded("accessor over the byte-tiebroken total order; law-pinned"),
    },
    SurfaceRow {
        op: "Ranked::rank",
        prod_tree: Leg::Law("ranked_tiebreaks_equal_ranks_by_bytes"),
        prod_fs: Leg::Excluded("accessor over the byte-tiebroken total order; law-pinned"),
        tree_fs: Leg::Excluded("accessor over the byte-tiebroken total order; law-pinned"),
    },
    SurfaceRow {
        op: "Ranked::into_parts",
        prod_tree: Leg::Law("ranked_tiebreaks_equal_ranks_by_bytes"),
        prod_fs: Leg::Excluded("accessor over the byte-tiebroken total order; law-pinned"),
        tree_fs: Leg::Excluded("accessor over the byte-tiebroken total order; law-pinned"),
    },
    // ───────────────────────────── causally ─────────────────────────────
    causally_row("causally::all"),
    causally_row("causally::since"),
    causally_row("causally::not_before"),
    causally_row("causally::known_at"),
    causally_row("causally::before"),
    causally_row("causally::delta"),
    causally_row("causally::delta_before"),
    causally_row("causally::Range::since"),
    causally_row("causally::Range::not_before"),
    causally_row("causally::Range::known_at"),
    causally_row("causally::Range::before"),
    causally_row("causally::Range::contains"),
    causally_row("causally::Range::placement_of"),
];

/// The roster over the operator/trait surface the `pub fn` scan cannot
/// reach. Totality here is by review of this file: a new operator impl is
/// a deliberate API event that must add a family row.
pub const FAMILY_SURFACE: &[SurfaceRow] = &[
    SurfaceRow {
        op: "Version | Version (BitOr/BitOrAssign, owned and borrowed)",
        prod_tree: Leg::Bound("merge_arbitrary"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version & Version (BitAnd/BitAndAssign, owned and borrowed)",
        prod_tree: Leg::Bound("meet_arbitrary"),
        prod_fs: Leg::Trans("meet_realizes_pointwise_min"),
        tree_fs: Leg::Bound("meet_realizes_pointwise_min"),
    },
    SurfaceRow {
        op: "&Version / &Party (Div — the lazy projection view)",
        prod_tree: Leg::Bound("div_matches_oracle"),
        prod_fs: Leg::Trans("quotient_realizes_region_mask"),
        tree_fs: Leg::Bound("quotient_realizes_region_mask"),
    },
    SurfaceRow {
        op: "OwnVersion vs Version comparisons (PartialEq/PartialOrd, both directions, owned and borrowed)",
        prod_tree: Leg::Bound("view_cmp_matches_oracle_composed"),
        prod_fs: Leg::Trans("own_version_cmp_matches_materialized"),
        tree_fs: Leg::Trans("quotient_realizes_region_mask"),
    },
    SurfaceRow {
        op: "OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)",
        prod_tree: Leg::Bound("view_pair_cmp_matches_oracle_composed"),
        prod_fs: Leg::Trans("own_version_pair_cmp_matches_materialized"),
        tree_fs: Leg::Trans("quotient_realizes_region_mask"),
    },
    SurfaceRow {
        op: "From<OwnVersion> for Version (explicit materialization)",
        prod_tree: Leg::Trans("from_impl_is_to_version"),
        prod_fs: Leg::Trans("from_impl_is_to_version"),
        tree_fs: Leg::Trans("from_impl_is_to_version"),
    },
    SurfaceRow {
        op: "Version PartialOrd (the comparison matrix, owned and borrowed)",
        prod_tree: Leg::Bound("compare_matrix_matches_oracle"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version Sum / FromIterator (owned and borrowed)",
        prod_tree: Leg::Trans("join_all_equals_the_sequential_fold"),
        prod_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner, 2026-07-26",
        ),
    },
    SurfaceRow {
        op: "Version Eq / Hash (canonical byte compare)",
        prod_tree: Leg::Law("eq_matches_causal_walk"),
        prod_fs: Leg::Excluded(
            "representation mechanics; equality-of-meaning rides every differential \
             compare (byte_equality_matches_bit_equality licenses the shortcut)",
        ),
        tree_fs: Leg::Excluded("representation mechanics"),
    },
    SurfaceRow {
        op: "Party Eq / Hash (canonical byte compare)",
        prod_tree: Leg::Law("byte_equality_matches_bit_equality"),
        prod_fs: Leg::Excluded(
            "representation mechanics; equality-of-meaning rides every differential \
             compare",
        ),
        tree_fs: Leg::Excluded("representation mechanics"),
    },
    SurfaceRow {
        op: "Clock | Version and Version | Clock (heterogeneous joins, |=)",
        prod_tree: Leg::Bound("heterogeneous_joins"),
        prod_fs: Leg::Trans("heterogeneous_joins"),
        tree_fs: Leg::Trans("heterogeneous_joins"),
    },
    SurfaceRow {
        op: "From<Party> for [Party; N] (consuming balanced split)",
        prod_tree: Leg::Law("forks_matches_from_array"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded("no oracle n-ary split; see the Party::forks row"),
    },
    SurfaceRow {
        op: "From<Clock> for [Clock; N] (consuming balanced split)",
        prod_tree: Leg::Law("clock_forks_matches_from_array"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(
            "no oracle n-ary split; composition of the party split and a version \
             clone per share — see the Clock::forks row",
        ),
    },
    SurfaceRow {
        op: "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        prod_tree: Leg::Law("forks_partial_drop_folds_back"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded("hand-out mechanics of the Rust API"),
    },
    codec_row("Party Display / FromStr / TryFrom literals"),
    codec_row("Version Display / FromStr / TryFrom literals"),
    codec_row("Clock Display / FromStr / TryFrom"),
    codec_row("serde / borsh impls (feature-gated, strict-decode pinned)"),
    SurfaceRow {
        op: "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display",
        prod_tree: Leg::Excluded(
            "not a paper object: order and arithmetic are bound to the in-test \
             alignment oracle (rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs, \
             rank_sum_equals_the_pairwise_fold) and to the laws::RANK_TRIPLE \
             monoid/order laws; the rank quantity itself is bound on all three \
             legs at Version::rank",
        ),
        prod_fs: Leg::Excluded("not a paper object; see the prod↔tree reason"),
        tree_fs: Leg::Excluded("not a paper object; see the prod↔tree reason"),
    },
    SurfaceRow {
        op: "Ticks ZERO / From / FromStr / Display / Add / Sum / Ord / Eq / Hash",
        prod_tree: Leg::Excluded(
            "not a paper object: the opaque count carrier for the ticks/min_ticks \
             surfaces, whose semantics are bound at the Version::ticks and \
             Version::min_ticks rows; the carrier's own arithmetic, order, and \
             text are law-pinned on production (ticks::tests, and the \
             laws::VERSION_PARTY / VERSION_PAIR_PARTY ticks laws quantify its \
             wide range through min_ticks-supplied counts)",
        ),
        prod_fs: Leg::Excluded("not a paper object; see the prod↔tree reason"),
        tree_fs: Leg::Excluded("not a paper object; see the prod↔tree reason"),
    },
    SurfaceRow {
        op: "Ranked Ord / From<Version> (byte tiebreak)",
        prod_tree: Leg::Law("ranked_linearly_extends_causality"),
        prod_fs: Leg::Excluded(
            "the tiebreak promises only some consistent total order extending \
             causality — representational by design",
        ),
        tree_fs: Leg::Excluded("representational by design; see the prod↔fs reason"),
    },
    SurfaceRow {
        op: "unbounded depth (beyond the differential grids)",
        prod_tree: Leg::Excluded(
            "the recursive oracle cannot build depth-100k trees; impl-only by \
             documented necessity, pinned by deep_tree_stack_safety",
        ),
        prod_fs: Leg::Excluded(
            "GRID_N caps function-space resolution; the premise is guarded by \
             grid_cap_is_never_reached",
        ),
        tree_fs: Leg::Excluded("GRID_N caps function-space resolution"),
    },
    SurfaceRow {
        op: "meter / error / iter plumbing",
        prod_tree: Leg::Excluded(
            "instrumentation and data plumbing, not ITC semantics; the meter board \
             and tier2 own their pinned suites",
        ),
        prod_fs: Leg::Excluded("instrumentation and data plumbing, not ITC semantics"),
        tree_fs: Leg::Excluded("instrumentation and data plumbing, not ITC semantics"),
    },
];
