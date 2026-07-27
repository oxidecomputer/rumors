//! The triangle suite: one committed roster binding every public operation
//! to the disposition of each differential leg.
//!
//! Three implementations cover the semantic surface: the production packed
//! implementation (*prod*), the recursive paper-transcription oracle in
//! [`crate::oracle`] (*tree* — the semantic definition of record), and the
//! function-space semantic oracle in [`super::semantic_oracle`] (*fs*). Three
//! legs connect them — prod↔tree, prod↔fs, tree↔fs — and this module holds
//! the *roster*: one row per public operation naming, for each leg, either
//! the primary test that binds it or the reason it is excluded. The roster
//! indexes the differentials that live beside the code they test; it never
//! re-implements one.
//!
//! # Tamper-evident totality
//!
//! [`METHOD_SURFACE`] must match, name for name, the inherent `pub fn`
//! surface extracted from the public-API source files
//! ([`extract_public_fns`], over [`SURFACE_SOURCES`]) — both directions, so
//! a *new* public operation fails the roster test until a named row is
//! added, and a removed operation orphans its row until the row is removed;
//! either way the reviewer sees a named diff. Operator and trait surfaces
//! (`|`, `&`, `/`, comparison matrices, `Display`/`FromStr`, serde/borsh)
//! are not reachable by that scan; they are rostered by family in
//! [`FAMILY_SURFACE`], whose totality is by review of this file alone.
//! Every test name a row cites must exist in the tree
//! ([`cited_test_names`] against a source scan), so a renamed or deleted
//! binding test fails the roster by name.
//!
//! # Leg vocabulary
//!
//! - [`Leg::Bound`]: a direct differential on that leg; the named test
//!   drives both sides.
//! - [`Leg::Law`]: pinned by an algebraic law on production alone (no
//!   reference on the right-hand side); used where no reference counterpart
//!   exists or the contract promises only a law.
//! - [`Leg::Trans`]: bound transitively — the operation reduces by
//!   definition to a bound one, or the leg is the composition of the other
//!   two bound legs; the named test anchors the reduction.
//! - [`Leg::Excluded`]: not bound, with the reason. The function-space
//!   boundary's exclusion dispositions are the owner's, marked
//!   "ratified by owner, 2026-07-26" at each reason.
//!
//! # Exclusion families
//!
//! Codecs and text (no wire format exists in the references; correctness is
//! production-side canonicality/round-trip/strict-rejection pins), batch
//! laziness (a batch equals its value-level ops), linearity and aliasing
//! mechanics (`Clone` references cannot express them; compile-fail tests
//! own them), `causally` (a definitional combinator over the bound causal
//! order), rank arithmetic (not a paper object; bound to the in-test
//! alignment oracle), n-ary hand-back mechanics (value identity and order
//! are not functions of the geometry), depth beyond the function-space grid
//! (`GRID_N` caps resolution; `deep_tree_stack_safety` is impl-only by
//! documented necessity), and the meter/error/iter plumbing.
//!
//! # Adequacy tripwires
//!
//! Each leg keeps a committed artifact proving its criterion can fail
//! ([`TRIPWIRES`], names checked live): prod↔tree keeps the fold seeds
//! replaying through the `join_all` differentials (the seeds are pinned
//! committed by `d1_seeds_stay_committed`) and the brute-force grow
//! reference as the independent fourth leg; prod↔fs keeps the grid-cap
//! premise guard; tree↔fs keeps the paper worked-value anchors. Named
//! obligation, not yet wired: a permanently-red known-bad artifact per leg
//! (the wrong-child-descent mutation is demonstrated in history, not
//! committed as a rostered red).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

/// One leg's disposition: how (or whether) two of the three
/// implementations are compared for one operation.
#[derive(Debug)]
pub(crate) enum Leg {
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
    pub(crate) fn cited(&self) -> Option<&'static str> {
        match self {
            Leg::Bound(t) | Leg::Law(t) | Leg::Trans(t) => Some(t),
            Leg::Excluded(_) => None,
        }
    }

    /// The exclusion reason, if this leg is excluded.
    pub(crate) fn exclusion_reason(&self) -> Option<&'static str> {
        match self {
            Leg::Excluded(reason) => Some(reason),
            _ => None,
        }
    }
}

/// One row of the roster: a public operation and its three leg
/// dispositions.
pub(crate) struct SurfaceRow {
    /// The operation, named as the extractor names it (`Type::fn`,
    /// `module::fn`) for [`METHOD_SURFACE`], or as a family description
    /// for [`FAMILY_SURFACE`].
    pub(crate) op: &'static str,
    pub(crate) prod_tree: Leg,
    pub(crate) prod_fs: Leg,
    pub(crate) tree_fs: Leg,
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

/// Shorthand for a batch-surface row: a batch equals its value-level ops
/// by construction, pinned by the operator matrices.
const fn batch_row(op: &'static str) -> SurfaceRow {
    const REASON: &str = "a batch equals its value-level ops (the oracle documents the \
         omission); pinned by representation_parity, batch_equals_value_level, \
         no_arith_batch_preserves_version, commit_on_drop";
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(REASON),
        prod_fs: Leg::Excluded(REASON),
        tree_fs: Leg::Excluded(REASON),
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
/// `roster_is_total_over_the_public_fn_surface` holds this equal, name for
/// name, to [`extract_public_fns`]'s listing.
pub(crate) const METHOD_SURFACE: &[SurfaceRow] = &[
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
             counterpart — pinned on production by dangerously_alias_aliases_region",
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
    batch_row("Version::batch"),
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
    batch_row("Clock::batch"),
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
    codec_row("Clock::encode"),
    codec_row("Clock::encode_to"),
    codec_row("Clock::decode"),
    codec_row("Clock::encoded_bits"),
    SurfaceRow {
        op: "Clock::dangerously_alias",
        prod_tree: Leg::Excluded(
            "linearity mechanics; an O(1) two-field composition over the party alias, \
             which dangerously_alias_aliases_region pins",
        ),
        prod_fs: Leg::Excluded(
            "linearity mechanics of the Rust API — ratified by owner, 2026-07-26",
        ),
        tree_fs: Leg::Excluded("linearity mechanics of the Rust API"),
    },
    // ───────────────────────────── Rank / Ranked ─────────────────────────────
    SurfaceRow {
        op: "Rank::checked_sub",
        prod_tree: Leg::Law("rank_monoid_and_order_laws"),
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
    // ───────────────────────────── batch modules ─────────────────────────────
    batch_row("batch::Version::tick"),
    batch_row("batch::Version::concurrent"),
    batch_row("batch::Version::snapshot"),
    batch_row("batch::Clock::tick"),
    batch_row("batch::Clock::fork"),
    batch_row("batch::Clock::join"),
    batch_row("batch::Clock::sync"),
    batch_row("batch::Clock::version"),
    batch_row("batch::Clock::party"),
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
pub(crate) const FAMILY_SURFACE: &[SurfaceRow] = &[
    SurfaceRow {
        op: "Version | Version (BitOr/BitOrAssign, the Batch operand matrix)",
        prod_tree: Leg::Bound("merge_arbitrary"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version & Version (BitAnd/BitAndAssign, the Batch operand matrix)",
        prod_tree: Leg::Bound("meet_arbitrary"),
        prod_fs: Leg::Trans("meet_realizes_pointwise_min"),
        tree_fs: Leg::Bound("meet_realizes_pointwise_min"),
    },
    SurfaceRow {
        op: "Version / &Party (Div/DivAssign — projection)",
        prod_tree: Leg::Bound("div_matches_oracle"),
        prod_fs: Leg::Trans("quotient_realizes_region_mask"),
        tree_fs: Leg::Bound("quotient_realizes_region_mask"),
    },
    SurfaceRow {
        op: "Version PartialOrd (the comparison matrix, all Version/Batch cells)",
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
             rank_sum_equals_the_pairwise_fold, rank_monoid_and_order_laws); the rank \
             quantity itself is bound on all three legs at Version::rank",
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

/// Per-leg adequacy tripwires: committed artifacts proving each leg's
/// criterion can fail. Names are checked live by the roster tests; the
/// prod↔tree seeds are additionally pinned committed by
/// `d1_seeds_stay_committed`.
pub(crate) const TRIPWIRES: &[(&str, &str)] = &[
    (
        "prod↔tree: the fold seeds replay through the join_all differentials",
        "join_all_matches_the_recursive_oracle",
    ),
    (
        "prod↔tree: the independent full-enumeration fourth leg for grow",
        "grow_matches_brute_force",
    ),
    (
        "prod↔fs: the grid-resolution premise guard",
        "grid_cap_is_never_reached",
    ),
    (
        "tree↔fs: the paper worked-value anchor",
        "embedding_matches_paper_worked_value",
    ),
    (
        "tree↔fs: the leaf-interval constancy anchor",
        "lifted_event_is_constant_within_a_leaf_interval",
    ),
];

/// A public-API source file the extractor scans, with the naming context
/// the file cannot carry itself.
pub(crate) struct SourceSpec {
    /// Path relative to the crate root.
    pub(crate) path: &'static str,
    /// Namespace for module-level `pub fn`s (`None`: the file must have
    /// none).
    pub(crate) module_prefix: Option<&'static str>,
    /// Override for the inherent-impl type name — for files whose local
    /// type name is not its public path (the two `Batch`es) or whose type
    /// lives under a public module.
    pub(crate) type_override: Option<&'static str>,
}

/// The public-API source files of record. A new public module with
/// inherent methods must be added here (and the roster test's coverage
/// note updated), which is itself a reviewed diff.
pub(crate) const SURFACE_SOURCES: &[SourceSpec] = &[
    SourceSpec {
        path: "src/party.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/clock.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/rank.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/ranked.rs",
        module_prefix: None,
        type_override: None,
    },
    SourceSpec {
        path: "src/version/batch.rs",
        module_prefix: None,
        type_override: Some("batch::Version"),
    },
    SourceSpec {
        path: "src/clock/batch.rs",
        module_prefix: None,
        type_override: Some("batch::Clock"),
    },
    SourceSpec {
        path: "src/party/forks.rs",
        module_prefix: None,
        type_override: Some("iter::Party"),
    },
    SourceSpec {
        path: "src/clock/forks.rs",
        module_prefix: None,
        type_override: Some("iter::Clock"),
    },
    SourceSpec {
        path: "src/causally.rs",
        module_prefix: Some("causally"),
        type_override: Some("causally::Range"),
    },
];

/// The crate root at test time.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Extract the inherent `pub fn` surface from [`SURFACE_SOURCES`], named
/// as the roster names it (`Type::fn` inside an inherent impl block,
/// `module::fn` at file top level).
///
/// A line scan, not a parser, resting on rustfmt-normalized shape: impl
/// headers at column 0 (trait impls contain ` for ` and cannot hold
/// `pub fn`s), inherent methods at one indent level. `pub fn` at an
/// unexpected position panics rather than silently vanishing from the
/// listing — the scan must never under-report the surface it exists to
/// pin.
pub(crate) fn extract_public_fns() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for spec in SURFACE_SOURCES {
        let path = crate_root().join(spec.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        // The public name of the current inherent impl block, if inside one.
        let mut current_type: Option<String> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("impl") {
                if line.contains(" for ") {
                    current_type = None; // trait impl: cannot hold `pub fn`
                } else {
                    current_type = parse_impl_self_type(rest)
                        .map(|name| spec.type_override.map(str::to_owned).unwrap_or(name));
                }
                continue;
            }
            if line == "}" {
                current_type = None;
                continue;
            }
            if let Some(rest) = line.strip_prefix("    pub fn ") {
                let name = fn_name(rest);
                let ty = current_type.as_deref().unwrap_or_else(|| {
                    panic!(
                        "{}: `pub fn {name}` outside an inherent impl block",
                        spec.path
                    )
                });
                out.insert(format!("{ty}::{name}"));
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub fn ") {
                let name = fn_name(rest);
                let prefix = spec.module_prefix.unwrap_or_else(|| {
                    panic!("{}: unexpected module-level `pub fn {name}`", spec.path)
                });
                out.insert(format!("{prefix}::{name}"));
            }
        }
    }
    out
}

/// The self-type name from an impl header's remainder (after `impl`):
/// skip a balanced generics list, then read the first identifier.
fn parse_impl_self_type(rest: &str) -> Option<String> {
    let mut chars = rest.chars().peekable();
    if chars.peek() == Some(&'<') {
        let mut depth = 0usize;
        for c in chars.by_ref() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    let name: String = chars
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The function name from the remainder after `pub fn `.
fn fn_name(rest: &str) -> &str {
    rest.split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
}

/// Every test name the roster and tripwires cite.
pub(crate) fn cited_test_names() -> BTreeSet<&'static str> {
    METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .flat_map(|row| {
            [&row.prod_tree, &row.prod_fs, &row.tree_fs]
                .into_iter()
                .filter_map(Leg::cited)
        })
        .chain(TRIPWIRES.iter().map(|(_, test)| *test))
        .collect()
}

/// Every `fn` name declared anywhere under `src/` — the haystack the
/// cited-name check searches.
pub(crate) fn declared_fn_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut stack = vec![crate_root().join("src")];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
                for line in text.lines() {
                    if let Some(pos) = line.find("fn ") {
                        // Require a word boundary before `fn` (start, space,
                        // or `(` for closures in macros).
                        let ok =
                            pos == 0 || line[..pos].ends_with(' ') || line[..pos].ends_with('(');
                        if ok {
                            let name = fn_name(&line[pos + 3..]);
                            if !name.is_empty() {
                                names.insert(name.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    names
}
