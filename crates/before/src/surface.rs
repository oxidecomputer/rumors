//! The public surface of the library as a machine-readable enumeration.
//!
//! Public under the `meter` feature (with the other instrument-facing data) so
//! external instrument crates can bind their coverage tables to the same
//! roster.
//!
//! This module is the machine-readable roster the surface-coverage suite (the
//! crate's test-only differential architecture) enforces totality over: its
//! tests hold [`METHOD_SURFACE`] equal, name for name, to the inherent `pub fn`
//! surface extracted from the public-API source files, and hold every cited
//! test name resolvable to an executable binding. The rows live here, outside
//! the test-only tree, so external instrument crates can bind their own
//! coverage to the same roster — a coverage table keyed by these row names is
//! total over the public surface exactly as far as the coverage suite's
//! totality pins reach, with no second hand-maintained enumeration to drift.
//! Public under the `meter` feature (the instrument crates' feature) and never
//! part of a production build.
//!
//! Leg vocabulary, exclusion families, and the adequacy tripwires are the
//! coverage suite's business and are documented there; a row's dispositions are
//! carried here verbatim as the suite's committed record.

/// One leg's disposition: how (or whether) two of the three implementations are
/// compared for one operation.
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
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    }
}

/// Shorthand for a `causally` row: every predicate is a definitional
/// combination of `partial_cmp` verdicts, which are bound on all three
/// legs; binding the combinator adds sampling of a totally-derived form.
const fn causally_row(op: &'static str) -> SurfaceRow {
    const REASON: &str = "semantically a combinator over the bound causal order \
         (partial_cmp), law-pinned to it (atom_membership_matches_relations, \
         conjunction_is_intersection); unit-tested in causally/tests.rs";
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(REASON),
        prod_fs: Leg::Excluded(REASON),
        tree_fs: Leg::Excluded(REASON),
    }
}

/// Shorthand for a `causally` span row: the same disposition as
/// [`causally_row`], with the span placement law as the pin.
const fn span_row(op: &'static str) -> SurfaceRow {
    const REASON: &str = "semantically a combinator over the bound causal order \
         (partial_cmp), law-pinned to it (span_place_matches_relations); \
         unit-tested in causally/tests.rs";
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
     coverage while the hand-back contract stayed unbound — ratified by owner";

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
             mechanics — ratified by owner",
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
             production (forks_partial_drop_folds_back, \
             party_join_all_reunites_forks_at_any_width)",
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
        prod_fs: Leg::Excluded("linearity mechanics of the Rust API — ratified by owner"),
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
        op: "Version::ranked",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(
            "the method spelling of the borrowing Ranked view conversion; the law \
             pins the delegation",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the delegation"),
    },
    SurfaceRow {
        op: "Version::encode_rank",
        prod_tree: Leg::Excluded(
            "the identical fused rank-only emission as Ranked::encode_rank, \
             entered from the version; its doctest pins the emission back to \
             Version::rank through strict Rank::decode",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; see the Ranked::encode_rank row",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Version::encode_rank_to",
        prod_tree: Leg::Excluded(
            "the identical fused rank-only emission as the view's writer-sink \
             door, entered from the version; its doctest pins byte identity \
             with Rank::encode over the materialized rank",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; see the Ranked::encode_rank row",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
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
        op: "Version::join",
        prod_tree: Leg::Trans("join_method_is_the_operator"),
        prod_fs: Leg::Trans("join_method_is_the_operator"),
        tree_fs: Leg::Trans("join_method_is_the_operator"),
    },
    SurfaceRow {
        op: "Version::meet",
        prod_tree: Leg::Trans("meet_method_is_the_operator"),
        prod_fs: Leg::Trans("meet_method_is_the_operator"),
        tree_fs: Leg::Trans("meet_method_is_the_operator"),
    },
    SurfaceRow {
        op: "Version::join_all",
        prod_tree: Leg::Trans("join_all_equals_the_sequential_fold"),
        prod_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner",
        ),
        tree_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner",
        ),
    },
    SurfaceRow {
        op: "Version::meet_all",
        prod_tree: Leg::Bound("meet_all_matches_oracle"),
        prod_fs: Leg::Excluded(
            "n-ary pointwise-min realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner",
        ),
        tree_fs: Leg::Excluded(
            "n-ary pointwise-min realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner",
        ),
    },
    SurfaceRow {
        op: "Version::span",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(
            "definitionally the pair meet and join, both bound on all three legs; \
             the law pins the endpoints byte-identical to them",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the endpoints"),
    },
    SurfaceRow {
        op: "Version::span_all",
        prod_tree: Leg::Law("span_all_is_the_family_hull"),
        prod_fs: Leg::Excluded(
            "definitionally the two committed lattice folds (meet_all/join_all) over \
             {self} ∪ others; the law pins the endpoints byte-identical to them",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the endpoints"),
    },
    SurfaceRow {
        op: "Version::project",
        prod_tree: Leg::Trans("project_is_the_operator_spelling"),
        prod_fs: Leg::Trans("project_is_the_operator_spelling"),
        tree_fs: Leg::Trans("project_is_the_operator_spelling"),
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
        op: "Clock::sync_all",
        prod_tree: Leg::Law("sync_all_is_join_all_then_forks"),
        prod_fs: Leg::Excluded(
            "no n-ary reconcile in the function space; the operation is law-pinned \
             byte-identical to its composed spelling — the bound Clock::join_all \
             followed by the balanced re-share \
             (sync_all_is_join_all_then_forks) — and the organic-population \
             invariants ride sync_all_reconciles_one_world",
        ),
        tree_fs: Leg::Excluded(
            "no oracle n-ary sync; see the prod↔fs reason — the law pins the \
             composition of surfaces bound on their own rows \
             (sync_all_is_join_all_then_forks)",
        ),
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
        op: "Clock::absorb",
        prod_tree: Leg::Trans("absorb_is_the_anonymous_join"),
        prod_fs: Leg::Trans("absorb_is_the_anonymous_join"),
        tree_fs: Leg::Trans("absorb_is_the_anonymous_join"),
    },
    SurfaceRow {
        op: "Clock::recv_all",
        prod_tree: Leg::Trans("recv_all_is_joins_then_tick"),
        prod_fs: Leg::Trans("recv_all_is_joins_then_tick"),
        tree_fs: Leg::Trans("recv_all_is_joins_then_tick"),
    },
    SurfaceRow {
        op: "Clock::absorb_all",
        prod_tree: Leg::Trans("absorb_all_is_the_sequential_joins"),
        prod_fs: Leg::Trans("absorb_all_is_the_sequential_joins"),
        tree_fs: Leg::Trans("absorb_all_is_the_sequential_joins"),
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
        prod_fs: Leg::Excluded("linearity mechanics of the Rust API — ratified by owner"),
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
        op: "Rank::saturating_sub",
        prod_tree: Leg::Law("rank_saturating_sub_is_checked_sub_floored"),
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
        op: "Rank::encode",
        prod_tree: Leg::Excluded(
            "Rank is not a paper object and no wire form exists in the oracle; the \
             encoding's laws are production-side pins (the lexicographic-order and \
             suffix-safety proptests, the exhaustive bijectivity sweep, the boundary \
             goldens, the per-genre rejection witnesses, the provenance size pin)",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Rank::encode_to",
        prod_tree: Leg::Excluded(
            "the identical emission as Rank::encode with a writer sink; its doctest \
             pins byte identity with encode, and the borsh round-trip suite drives it \
             as the serializer",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Rank::decode",
        prod_tree: Leg::Excluded(
            "Rank is not a paper object and no wire form exists in the oracle; strict \
             decode is pinned production-side (round-trip inside the lexicographic \
             proptests, the exhaustive accept-or-reject sweep, the rejection \
             witnesses)",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Ranked::version",
        prod_tree: Leg::Law("ranked_sort_respects_causality"),
        prod_fs: Leg::Excluded("accessor over the rank view; law-pinned"),
        tree_fs: Leg::Excluded("accessor over the rank view; law-pinned"),
    },
    SurfaceRow {
        op: "Ranked::rank",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(
            "definitional delegation to Version::rank, which is bound on all three \
             legs; the law pins the delegation",
        ),
        tree_fs: Leg::Excluded("see the Version::rank row; the law pins the delegation"),
    },
    SurfaceRow {
        op: "Ranked::into_owned",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(
            "borrow-settling mechanics of the Rust API; the law pins value preservation",
        ),
        tree_fs: Leg::Excluded("borrow-settling mechanics of the Rust API"),
    },
    SurfaceRow {
        op: "Ranked::encode",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; the composite key is law-pinned as \
             the rank encoding followed by the version's canonical bytes, its byte \
             order pinned equal to the view's total order \
             (ranked_encoding_orders_like_ord)",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Ranked::encode_to",
        prod_tree: Leg::Excluded(
            "the identical composite emission as Ranked::encode with a writer sink; \
             its doctest pins byte identity with encode, and the borsh suite drives it \
             as the serializer",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Ranked::encode_rank",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; the fused rank-only emission is \
             law-pinned byte-identical to Rank::encode over the materialized rank",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Ranked::encode_rank_to",
        prod_tree: Leg::Excluded(
            "the identical fused rank-only emission as Ranked::encode_rank with a \
             writer sink; its doctest pins byte identity with encode_rank",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Ranked::decode",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; strict decode is pinned \
             production-side (the decode∘encode identity clause of the law, the \
             composite suffix-safety proptests, the per-genre rejection witnesses, \
             the rank-against-version verification the method documents)",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    // ───────────────────────────── causally ─────────────────────────────
    causally_row("causally::all"),
    causally_row("causally::after"),
    causally_row("causally::before"),
    causally_row("causally::since"),
    causally_row("causally::until"),
    causally_row("causally::strictly_after"),
    causally_row("causally::strictly_before"),
    causally_row("causally::delta"),
    causally_row("causally::toward"),
    causally_row("causally::Floor::contains"),
    causally_row("causally::Floor::or_concurrent"),
    causally_row("causally::Ceiling::contains"),
    causally_row("causally::Ceiling::or_concurrent"),
    causally_row("causally::Query::contains"),
    causally_row("causally::Query::coverage"),
    causally_row("causally::Query::into_owned"),
    span_row("Span::new"),
    SurfaceRow {
        op: "Span::at",
        prod_tree: Leg::Law("at_is_the_coincident_hull"),
        prod_fs: Leg::Excluded(
            "constructor mechanics of the Rust API; the law pins both coincident \
             doors to the bound pair hull",
        ),
        tree_fs: Leg::Excluded("constructor mechanics of the Rust API; law-pinned"),
    },
    span_row("Span::place"),
    span_row("Span::dominance"),
    span_row("Span::precedence"),
    span_row("Span::contains"),
    SurfaceRow {
        op: "Span::lo",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(
            "accessor over a stored endpoint; the hull laws pin the accessor \
             spelling to the committed lattice folds",
        ),
        tree_fs: Leg::Excluded("accessor over a stored endpoint; law-pinned"),
    },
    SurfaceRow {
        op: "Span::hi",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(
            "accessor over a stored endpoint; the hull laws pin the accessor \
             spelling to the committed lattice folds",
        ),
        tree_fs: Leg::Excluded("accessor over a stored endpoint; law-pinned"),
    },
    SurfaceRow {
        op: "Span::into_parts",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(
            "borrow-settling mechanics of the Rust API; the law pins value \
             preservation in (meet, join) order",
        ),
        tree_fs: Leg::Excluded("borrow-settling mechanics of the Rust API"),
    },
    SurfaceRow {
        op: "Span::reborrow",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(
            "borrow-lending mechanics of the Rust API; the law pins the \
             reborrowed endpoints byte-equal to the source's",
        ),
        tree_fs: Leg::Excluded("borrow-lending mechanics of the Rust API"),
    },
    SurfaceRow {
        op: "Span::union",
        prod_tree: Leg::Law("span_union_is_the_containment_join"),
        prod_fs: Leg::Excluded(
            "the method spelling of the `+` operator; the law pins it to the operator across every operand cell",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the spelling"),
    },
    SurfaceRow {
        op: "Span::intersect",
        prod_tree: Leg::Law("span_intersect_is_the_shared_segment"),
        prod_fs: Leg::Excluded(
            "the method spelling of the `*` operator; the law pins it to the operator across every operand cell",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the spelling"),
    },
    SurfaceRow {
        op: "Span::union_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(
            "definitionally the binary containment join folded over {self} ∪ others; the law pins the door to the bound operator at every arity, and span_union_of_points_is_span_all pins the all-coincident case to Version::span_all",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the laws pin the fold"),
    },
    SurfaceRow {
        op: "Span::intersect_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(
            "definitionally the binary containment meet folded through Option over {self} ∪ others; the law pins the door to the bound operator at every arity",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the laws pin the fold"),
    },
    SurfaceRow {
        op: "Span::join",
        prod_tree: Leg::Law("span_join_is_the_pointwise_join"),
        prod_fs: Leg::Excluded(
            "the method spelling of the `|` operator; the law pins it to the operator across every operand cell",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the spelling"),
    },
    SurfaceRow {
        op: "Span::meet",
        prod_tree: Leg::Law("span_meet_is_the_pointwise_meet"),
        prod_fs: Leg::Excluded(
            "the method spelling of the `&` operator; the law pins it to the operator across every operand cell",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the spelling"),
    },
    SurfaceRow {
        op: "Span::join_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(
            "definitionally the binary pointwise join folded over {self} ∪ others; the law pins the door to the bound operator at every arity",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the laws pin the fold"),
    },
    SurfaceRow {
        op: "Span::meet_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(
            "definitionally the binary pointwise meet folded over {self} ∪ others; the law pins the door to the bound operator at every arity",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the laws pin the fold"),
    },
    SurfaceRow {
        op: "Span::project",
        prod_tree: Leg::Trans("span_project_is_the_operator_spelling"),
        prod_fs: Leg::Trans("span_project_is_the_operator_spelling"),
        tree_fs: Leg::Trans("span_project_is_the_operator_spelling"),
    },
    SurfaceRow {
        op: "OwnSpan::lo",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "accessor handing out the bound OwnVersion view; the law pins it equal to the eagerly projected endpoint",
        ),
        tree_fs: Leg::Excluded("accessor over the bound projection; law-pinned"),
    },
    SurfaceRow {
        op: "OwnSpan::hi",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "accessor handing out the bound OwnVersion view; the law pins it equal to the eagerly projected endpoint",
        ),
        tree_fs: Leg::Excluded("accessor over the bound projection; law-pinned"),
    },
    SurfaceRow {
        op: "OwnSpan::place",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "semantically the nine-state transcription of the two bound masked comparisons (OwnVersion vs Version, bound on all three legs); the law pins every verdict to the eagerly projected span's",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the verdicts"),
    },
    SurfaceRow {
        op: "OwnSpan::dominance",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "the dominance coarsening over the bound masked comparisons; the law pins every verdict to the eagerly projected span's",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the verdicts"),
    },
    SurfaceRow {
        op: "OwnSpan::precedence",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "the precedence coarsening over the bound masked comparisons; the law pins every verdict to the eagerly projected span's",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the verdicts"),
    },
    SurfaceRow {
        op: "OwnSpan::contains",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "the membership coarsening over the bound masked comparisons; the law pins every verdict to the eagerly projected span's",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the verdicts"),
    },
    SurfaceRow {
        op: "OwnSpan::to_span",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "composition of the bound OwnVersion::to_version per endpoint; the law pins the materialized span, and projection monotonicity (projection_monotone_in_version) keeps the pair ordered",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the composition"),
    },
    SurfaceRow {
        op: "Span::into_owned",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(
            "borrow-settling mechanics of the Rust API; the law pins the \
             settled endpoints byte-equal in both borrow states",
        ),
        tree_fs: Leg::Excluded("borrow-settling mechanics of the Rust API"),
    },
    SurfaceRow {
        op: "Span::encode",
        prod_tree: Leg::Law("span_codec_roundtrip"),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; the composite is law-pinned as \
             the meet's encoding followed by the join's, with decode ∘ encode the \
             identity (span_codec_roundtrip) and the composite's prefix-freedom \
             pinned directly (span_encoding_is_prefix_free)",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Span::encode_to",
        prod_tree: Leg::Excluded(
            "the identical composite emission as Span::encode with a writer sink; \
             its doctest pins byte identity with encode, and the borsh suite drives \
             it as the serializer",
        ),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; representation is exactly what the \
             semantic domain quotients away — ratified by owner",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
    SurfaceRow {
        op: "Span::decode",
        prod_tree: Leg::Law("span_codec_roundtrip"),
        prod_fs: Leg::Excluded(
            "a function has no byte representation; strict fused decode is pinned \
             production-side (the decode ∘ encode identity clause of the law, the \
             fused-validate verdict identity against the composed \
             decode + decode + Span::new form over the exhaustive small scope and \
             arbitrary pairs, the per-genre rejection witnesses, and the span \
             decode meter rows' fusion pins)",
        ),
        tree_fs: Leg::Excluded("neither reference has a wire format"),
    },
];

/// The roster over the operator/trait surface the `pub fn` scan cannot
/// reach.
///
/// Rows here carry the leg dispositions by family; the concrete
/// impl inventory behind them is held mechanically total by the
/// surface-totality gate (`crates/before/surfacecheck`), which pins every
/// reachable trait impl by name against nightly rustdoc JSON. A new
/// operator impl is a deliberate API event: it fails that gate until its
/// pin — and, for a new family, a family row here — is added.
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
        op: "Version ^ Version (BitXor, owned and borrowed — the pair hull)",
        prod_tree: Leg::Trans("span_operator_matrix_is_the_method"),
        prod_fs: Leg::Excluded(
            "the operator spelling of Version::span, whose row carries the hull's \
             dispositions; the matrix law pins every cell's delegation",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the matrix law pins the delegation"),
    },
    SurfaceRow {
        op: "Span + Span (Add, owned and borrowed — the containment join)",
        prod_tree: Leg::Law("span_union_is_the_containment_join"),
        prod_fs: Leg::Excluded(
            "definitionally the bound meet/join over the corresponding endpoints; the law pins the endpoints, every operand cell, and coverage of both operands",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the endpoints"),
    },
    SurfaceRow {
        op: "Span * Span (Mul, owned and borrowed — the containment meet)",
        prod_tree: Leg::Law("span_intersect_is_the_shared_segment"),
        prod_fs: Leg::Excluded(
            "definitionally the bound join/meet over the corresponding endpoints, validated once; the law pins the endpoints, the None verdict, every operand cell, absorption, and shared-membership coherence",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the endpoints"),
    },
    SurfaceRow {
        op: "Span | Span (BitOr, owned and borrowed — the pointwise join)",
        prod_tree: Leg::Law("span_join_is_the_pointwise_join"),
        prod_fs: Leg::Excluded(
            "definitionally the bound join over both endpoint pairs; the law pins the endpoints, every operand cell, the identity, and the restriction to the version join on coincident operands",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the endpoints"),
    },
    SurfaceRow {
        op: "Span & Span (BitAnd, owned and borrowed — the pointwise meet)",
        prod_tree: Leg::Law("span_meet_is_the_pointwise_meet"),
        prod_fs: Leg::Excluded(
            "definitionally the bound meet over both endpoint pairs; the law pins the endpoints, every operand cell, pointwise absorption, and the restriction to the version meet on coincident operands",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the endpoints"),
    },
    SurfaceRow {
        op: "Span Sum / FromIterator (owned and borrowed — the union fold)",
        prod_tree: Leg::Law("span_sum_and_collect_are_the_union_fold"),
        prod_fs: Leg::Excluded(
            "definitionally the receiver-less entry to the union fold; the law pins both collection doors to Span::union_all and the empty iterator to None",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the doors"),
    },
    SurfaceRow {
        op: "Span Product (owned and borrowed — the intersection fold)",
        prod_tree: Leg::Law("span_product_is_the_intersect_fold"),
        prod_fs: Leg::Excluded(
            "definitionally the receiver-less entry to the intersection fold; the law pins the door to Span::intersect_all and the empty iterator to None",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the door"),
    },
    SurfaceRow {
        op: "&Span / &Party (Div — the lazy span projection view)",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "view construction: two borrows; every verdict and the materialization are law-pinned to the eagerly projected span, whose pieces are bound (Div on versions, OwnVersion::to_version)",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the view"),
    },
    SurfaceRow {
        op: "From<OwnSpan> for Span (explicit materialization)",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(
            "the From impl is to_span; the law pins both materialization doors to the eagerly projected span",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the doors"),
    },
    SurfaceRow {
        op: "From<Version> for Span (the coincident constructor, owned and borrowed)",
        prod_tree: Leg::Law("at_is_the_coincident_hull"),
        prod_fs: Leg::Excluded(
            "the consuming From impl is Span::at and the lending one stores two borrows of the version; the law pins every coincident door to the bound pair hull",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the doors"),
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
             on its prod↔tree leg — ratified by owner",
        ),
        tree_fs: Leg::Excluded(
            "n-ary pointwise-max realization not adopted; the operation stays bound \
             on its prod↔tree leg — ratified by owner",
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
        op: "Ranked comparisons and the Ranked / Rank From conversions (the total order)",
        prod_tree: Leg::Law("ranked_orders_by_rank_then_bytes"),
        prod_fs: Leg::Excluded(
            "every cell is the rank comparison — whose quantity is bound on all three \
             legs at Version::rank — completed on rank ties by the canonical-byte \
             tiebreak; the law pins the fused walk, the version-identity equality, \
             and the explicit Ranked::rank spelling of the rank question to it",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the delegation"),
    },
    SurfaceRow {
        op: "causally & conjunction (atoms and queries, every admitted pairing)",
        prod_tree: Leg::Law("conjunction_is_intersection"),
        prod_fs: Leg::Excluded(
            "semantically predicate intersection over the bound causal order; the law pins every polarity pairing pointwise, and conjunction_operand_forms_agree pins each typed merge path to the same predicate",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the laws pin the merge"),
    },
    SurfaceRow {
        op: "causally ! complement (atom negation into the polar hole)",
        prod_tree: Leg::Law("atom_membership_matches_relations"),
        prod_fs: Leg::Excluded(
            "O(1) hole mint over the atom's bound; the law's complement clause pins membership to the negated relation",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the complement"),
    },
    SurfaceRow {
        op: "From into Query (atoms, spans, versions, borrowed queries)",
        prod_tree: Leg::Law("conjunction_operand_forms_agree"),
        prod_fs: Leg::Excluded(
            "O(1) constructions through the cross-side merge (no comparison, no walk); the law pins the composed membership",
        ),
        tree_fs: Leg::Excluded("see the prod↔fs reason; the law pins the composition"),
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
