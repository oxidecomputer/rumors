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
//! Leg vocabulary and the adequacy tripwires are the coverage suite's
//! business and are documented there; the exclusion families are the closed
//! vocabulary [`Exclusion`] documents variant by variant. A row's
//! dispositions are carried here verbatim as the suite's committed record.

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
    /// Not bound: the exclusion family the decision belongs to.
    Excluded(Exclusion),
}

impl Leg {
    /// The test name this disposition cites, if any.
    pub fn cited(&self) -> Option<&'static str> {
        match self {
            Leg::Bound(t) | Leg::Law(t) | Leg::Trans(t) => Some(t),
            Leg::Excluded(_) => None,
        }
    }

    /// The exclusion family, if this leg is excluded.
    pub fn exclusion(&self) -> Option<&Exclusion> {
        match self {
            Leg::Excluded(family) => Some(family),
            _ => None,
        }
    }
}

/// Why a leg is not bound: the roster's closed vocabulary of exclusion
/// families.
///
/// An exclusion is a documented boundary decision, never a bare opt-out.
/// Each variant is one family whose argument is defended once, in the
/// variant's documentation; a new exclusion picks a rostered family or
/// widens this enum — a reviewed API event, exactly as a new law-group
/// signature is. The function-space families' non-adoption dispositions
/// are the owner's, ratified where each variant's documentation says so.
///
/// Payloads carry the production-side pins an instance rests on *where the
/// row's own `Bound`/`Law`/`Trans` legs do not already carry them* — the
/// coverage suite resolves every payload name exactly as it resolves
/// citations, and each family's structural obligation (a live guard, a
/// binding row) is enforced there too.
#[derive(Debug)]
pub enum Exclusion {
    /// No wire format exists in the paper or either reference.
    ///
    /// Byte representation is exactly what the semantic domain quotients
    /// away (fs non-adoption ratified by owner), so codec, text, and
    /// writer-sink doors are pinned production-side — round-trips,
    /// strict-rejection batteries, canonicality and mutation sweeps,
    /// format goldens, and, for each writer-sink door, its doctest
    /// pinning byte identity with the buffer door.
    NoWireFormatInReferences {
        /// The load-bearing production-side pins, by test or law name.
        pins: &'static [&'static str],
    },
    /// A definitional combination of surfaces bound on their own rows.
    ///
    /// The operation is a combination, spelling, accessor, or coarsening
    /// of bound surfaces, and a law pins the reduction on production;
    /// binding the combinator would only re-sample a totally-derived
    /// form.
    DefinitionalCombinator {
        /// The pinning laws or tests the row's own legs do not carry.
        pins: &'static [&'static str],
    },
    /// No n-ary counterpart exists in either reference.
    ///
    /// No oracle n-ary split or reconcile exists, and the pointwise n-ary
    /// realizations are not adopted (ratified by owner). For the fallible
    /// folds a verdict-only binding would read as coverage while the
    /// hand-back contract — value identity and order against the fixed
    /// accumulator, not a function of the geometry — stayed unbound. The
    /// n-ary doors are law-pinned on production at every arity instead.
    NAryNotInReferences {
        /// The arity-quantified production laws the row's legs do not
        /// carry.
        pins: &'static [&'static str],
    },
    /// Linearity, borrowing, and adjacent Rust-API mechanics.
    ///
    /// Aliasing doors, borrow settling and lending, and hand-out
    /// iterators — shapes the `Clone` references cannot
    /// express. The hazard side is owned by the compile-time pins
    /// (`static_assertions` beside the `Party`/`Clock` definitions and
    /// the array-split `compile_fail` doctest twins).
    LinearityMechanics {
        /// The value-preservation pins the row's legs do not carry.
        pins: &'static [&'static str],
    },
    /// Not an object of the paper's model.
    ///
    /// The excluded surface is a carrier or instrument whose model-facing
    /// semantics are bound elsewhere — `bound_at` names the roster row
    /// (or suite) carrying the quantity, and the carrier's own
    /// arithmetic, order, and text are pinned on production.
    NotAPaperObject {
        /// The roster row, test, or law where the quantity is bound.
        bound_at: &'static str,
        /// The carrier's own production-side pins, where the row's legs
        /// do not carry them.
        pins: &'static [&'static str],
    },
    /// A reference capacity cap.
    ///
    /// The input regime lies beyond what a reference can build or resolve
    /// (the function space's `GRID_N` resolution, the recursive oracle's
    /// stack depth), so the leg is impl-only by documented necessity.
    GridCap {
        /// The live premise guard or capacity witness, by test name —
        /// checked to be an executable test, so the cap's premise cannot
        /// rot silently.
        guard: &'static str,
    },
    /// Representation mechanics: `Eq`/`Hash` ride canonical bytes, and
    /// equality-of-meaning already rides every differential compare.
    RepresentationMechanics {
        /// The law licensing the byte-compare shortcut.
        license: &'static str,
    },
}

impl Exclusion {
    /// Every family name, for the inhabitation census (an empty family is
    /// a dead category); [`family`](Exclusion::family)'s exhaustive match
    /// beside this list keeps the two in one diff when the vocabulary
    /// widens.
    pub const FAMILIES: &'static [&'static str] = &[
        "NoWireFormatInReferences",
        "DefinitionalCombinator",
        "NAryNotInReferences",
        "LinearityMechanics",
        "NotAPaperObject",
        "GridCap",
        "RepresentationMechanics",
    ];

    /// The family's name, as [`FAMILIES`](Exclusion::FAMILIES) spells it.
    pub fn family(&self) -> &'static str {
        match self {
            Exclusion::NoWireFormatInReferences { .. } => "NoWireFormatInReferences",
            Exclusion::DefinitionalCombinator { .. } => "DefinitionalCombinator",
            Exclusion::NAryNotInReferences { .. } => "NAryNotInReferences",
            Exclusion::LinearityMechanics { .. } => "LinearityMechanics",
            Exclusion::NotAPaperObject { .. } => "NotAPaperObject",
            Exclusion::GridCap { .. } => "GridCap",
            Exclusion::RepresentationMechanics { .. } => "RepresentationMechanics",
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

/// The generic codec doors' production-side pins.
///
/// The round-trip and byte-view laws and the decode totality sweep shared
/// by every type's wire surface (each type's own rejection batteries and
/// goldens live in its codec suites).
const CODEC_PINS: &[&str] = &[
    "decode_encode_arbitrary",
    "as_bytes_matches_encode",
    "decode_never_panics",
];

/// Shorthand for a codec/text method row: representation is exactly what
/// both references quotient away, so all three legs are excluded and
/// correctness lives in the production-side pins.
const fn codec_row(op: &'static str) -> SurfaceRow {
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: CODEC_PINS }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    }
}

/// The rank wire form's production-side pins.
///
/// The wire-form laws, the suffix-safety proptest, the exhaustive
/// small-scope sweep, the format goldens, the per-genre rejection
/// witnesses, and the provenance size pin.
const RANK_WIRE_PINS: &[&str] = &[
    "rank_lex_order",
    "rank_codec_roundtrip",
    "rank_encoding_prefix_free",
    "rank_lex_encoding_is_suffix_safe",
    "rank_encoding_exhaustive_small_scope",
    "rank_encoding_known_values",
    "rank_decoding_rejects_each_genre",
    "rank_encoding_size_is_provenance_linear",
];

/// Shorthand for a plain writer-sink codec door (`encode_to` on `Party`,
/// `Version`, and `Clock`).
///
/// The identical emission as `encode` with a writer sink, with the
/// agreement pinned across all three types by `encode_to_matches_encode`
/// beside each door's doctest.
const fn encode_to_row(op: &'static str) -> SurfaceRow {
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: &["encode_to_matches_encode"],
        }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    }
}

/// The `causally` combinators' pinning laws.
///
/// Every predicate is a definitional combination of `partial_cmp`
/// verdicts, bound on all three legs, and these laws pin the combination
/// (the predicates are also unit-tested in `causally/tests.rs`).
const CAUSALLY_PINS: &[&str] = &[
    "atom_membership_matches_relations",
    "conjunction_is_intersection",
];

/// Shorthand for a `causally` row: a definitional combinator over the
/// bound causal order, law-pinned on every leg.
const fn causally_row(op: &'static str) -> SurfaceRow {
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: CAUSALLY_PINS,
        }),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: CAUSALLY_PINS,
        }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: CAUSALLY_PINS,
        }),
    }
}

/// The span verdict surfaces' pinning laws: the nine-state placement as a
/// pure transcription of the two endpoint comparisons, and each coarsening
/// pinned to it.
const SPAN_PLACE_PINS: &[&str] = &[
    "span_place_matches_relations",
    "span_dominance_coarsens_place",
    "span_precedence_coarsens_place",
    "span_contains_matches_place",
];

/// Shorthand for a `causally` span row: the same disposition as
/// [`causally_row`], with the span placement laws as the pins.
const fn span_row(op: &'static str) -> SurfaceRow {
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: SPAN_PLACE_PINS,
        }),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: SPAN_PLACE_PINS,
        }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: SPAN_PLACE_PINS,
        }),
    }
}

/// The n-ary hand-back exclusion, shared by the `join_all`/`forks` family
/// of rows.
///
/// The family variant's documentation carries the half-binding rationale,
/// and the pins carry the arity-quantified best-effort laws that bind the
/// hand-back contract on production.
const HANDBACK: Exclusion = Exclusion::NAryNotInReferences {
    pins: &[
        "party_join_all_is_best_effort_at_any_width",
        "join_overlap_hands_back",
    ],
};

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
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["is_seed_iff_equals_seed"],
        }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["is_seed_iff_equals_seed"],
        }),
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
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences {
            pins: &[
                "forks_partial_drop_folds_back",
                "party_join_all_reunites_forks_at_any_width",
            ],
        }),
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
        prod_tree: Leg::Excluded(Exclusion::LinearityMechanics {
            pins: &["alias_is_byte_identical_overlap"],
        }),
        prod_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    codec_row("Party::encode"),
    encode_to_row("Party::encode_to"),
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
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["is_empty_iff_new"],
        }),
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
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::encode_rank",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::encode_rank_to",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
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
        prod_tree: Leg::Trans("join_all_is_the_sequential_pair_fold"),
        prod_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::meet_all",
        prod_tree: Leg::Bound("meet_all_matches_oracle"),
        prod_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::span",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::span_all",
        prod_tree: Leg::Law("span_all_is_the_family_hull"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::project",
        prod_tree: Leg::Trans("project_is_the_operator_spelling"),
        prod_fs: Leg::Trans("project_is_the_operator_spelling"),
        tree_fs: Leg::Trans("project_is_the_operator_spelling"),
    },
    codec_row("Version::encode"),
    encode_to_row("Version::encode_to"),
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
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
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
        prod_fs: Leg::Excluded(Exclusion::NAryNotInReferences {
            pins: &["sync_all_reconciles_one_world"],
        }),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
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
    encode_to_row("Clock::encode_to"),
    codec_row("Clock::decode"),
    codec_row("Clock::encoded_bits"),
    SurfaceRow {
        op: "Clock::dangerously_alias",
        prod_tree: Leg::Excluded(Exclusion::LinearityMechanics {
            pins: &["alias_is_byte_identical_overlap"],
        }),
        prod_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    // ───────────────────────────── Rank / Ranked ─────────────────────────────
    SurfaceRow {
        op: "Rank::checked_sub",
        prod_tree: Leg::Law("rank_checked_sub_iff_dominated"),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::rank",
            pins: &["rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs"],
        }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::rank",
            pins: &[],
        }),
    },
    SurfaceRow {
        op: "Rank::saturating_sub",
        prod_tree: Leg::Law("rank_saturating_sub_is_checked_sub_floored"),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::rank",
            pins: &["rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs"],
        }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::rank",
            pins: &[],
        }),
    },
    SurfaceRow {
        op: "Rank::encode",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: RANK_WIRE_PINS,
        }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Rank::encode_to",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Rank::decode",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: RANK_WIRE_PINS,
        }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::version",
        prod_tree: Leg::Law("ranked_sort_respects_causality"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::rank",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::into_owned",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::encode",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: &["ranked_encoding_orders_like_ord"],
        }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::encode_to",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::encode_rank",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::encode_rank_to",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked::decode",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: &[
                "ranked_composite_encoding_is_suffix_safe",
                "ranked_composite_key_is_suffix_safe_at_the_tiebreak_seam",
                "ranked_decode_rejects_each_genre",
                "ranked_composite_bit_flip_rejects_or_decodes_canonically",
            ],
        }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
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
    SurfaceRow {
        op: "Span::new",
        prod_tree: Leg::Law("span_gate_admits_exactly_the_ordered"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::at",
        prod_tree: Leg::Law("at_is_the_coincident_hull"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    span_row("Span::place"),
    span_row("Span::dominance"),
    span_row("Span::precedence"),
    span_row("Span::contains"),
    SurfaceRow {
        op: "Span::lo",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::hi",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::into_parts",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::reborrow",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::union",
        prod_tree: Leg::Law("span_union_is_the_containment_join"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::intersect",
        prod_tree: Leg::Law("span_intersect_is_the_shared_segment"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::union_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["span_union_of_points_is_span_all"],
        }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::intersect_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::join",
        prod_tree: Leg::Law("span_join_is_the_pointwise_join"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::meet",
        prod_tree: Leg::Law("span_meet_is_the_pointwise_meet"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::join_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::meet_all",
        prod_tree: Leg::Law("span_folds_match_the_sequential_operators"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
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
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "OwnSpan::hi",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "OwnSpan::place",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "OwnSpan::dominance",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "OwnSpan::precedence",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "OwnSpan::contains",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "OwnSpan::to_span",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["projection_monotone_in_version"],
        }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::into_owned",
        prod_tree: Leg::Law("span_is_the_pair_hull"),
        prod_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::encode",
        prod_tree: Leg::Law("span_codec_roundtrip"),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: &["span_encoding_is_prefix_free"],
        }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::encode_to",
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Span::decode",
        prod_tree: Leg::Law("span_codec_roundtrip"),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences {
            pins: &[
                "span_decode_verdict_matches_the_composed_form",
                "span_decode_verdict_matches_the_composed_form_exhaustively",
                "span_decode_rejects_each_genre",
            ],
        }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
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
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span + Span (Add, owned and borrowed — the containment join)",
        prod_tree: Leg::Law("span_union_is_the_containment_join"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span * Span (Mul, owned and borrowed — the containment meet)",
        prod_tree: Leg::Law("span_intersect_is_the_shared_segment"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span | Span (BitOr, owned and borrowed — the pointwise join)",
        prod_tree: Leg::Law("span_join_is_the_pointwise_join"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span & Span (BitAnd, owned and borrowed — the pointwise meet)",
        prod_tree: Leg::Law("span_meet_is_the_pointwise_meet"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span Sum / FromIterator (owned and borrowed — the union fold)",
        prod_tree: Leg::Law("span_sum_and_collect_are_the_union_fold"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span Product (owned and borrowed — the intersection fold)",
        prod_tree: Leg::Law("span_product_is_the_intersect_fold"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "&Span / &Party (Div — the lazy span projection view)",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "From<OwnSpan> for Span (explicit materialization)",
        prod_tree: Leg::Law("own_span_matches_the_projected_span"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "From<Version> for Span (the coincident constructor, owned and borrowed)",
        prod_tree: Leg::Law("at_is_the_coincident_hull"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
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
        prod_tree: Leg::Trans("version_sum_is_the_sequential_pair_fold"),
        prod_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "Version Eq / Hash (canonical byte compare)",
        prod_tree: Leg::Law("eq_matches_causal_walk"),
        prod_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "byte_equality_matches_bit_equality" }),
        tree_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "byte_equality_matches_bit_equality" }),
    },
    SurfaceRow {
        op: "Party Eq / Hash (canonical byte compare)",
        prod_tree: Leg::Law("byte_equality_matches_bit_equality"),
        prod_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "party_eq_iff_bytes_eq" }),
        tree_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "party_eq_iff_bytes_eq" }),
    },
    SurfaceRow {
        op: "Clock Eq / Hash (canonical byte compare)",
        prod_tree: Leg::Law("clock_eq_iff_bytes_eq"),
        prod_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "clock_eq_iff_bytes_eq" }),
        tree_fs: Leg::Excluded(Exclusion::RepresentationMechanics { license: "clock_eq_iff_bytes_eq" }),
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
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "From<Clock> for [Clock; N] (consuming balanced split)",
        prod_tree: Leg::Law("clock_forks_matches_from_array"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        prod_tree: Leg::Law("forks_partial_drop_folds_back"),
        prod_fs: Leg::Excluded(HANDBACK),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    codec_row("Party Display / FromStr / TryFrom literals"),
    codec_row("Version Display / FromStr / TryFrom literals"),
    codec_row("Clock Display / FromStr / TryFrom"),
    codec_row("serde / borsh impls (feature-gated, strict-decode pinned)"),
    SurfaceRow {
        op: "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::rank", pins: &["rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs", "rank_sum_equals_the_pairwise_fold"] }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::rank", pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::rank", pins: &[] }),
    },
    SurfaceRow {
        op: "Ticks ZERO / From / FromStr / Display / Add / Sum / Ord / Eq / Hash",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::min_ticks", pins: &["addition_behaves_like_the_naturals", "text_round_trips", "ticks_agrees_with_iterated_ticks", "ticks_composes"] }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::min_ticks", pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::min_ticks", pins: &[] }),
    },
    SurfaceRow {
        op: "Ranked comparisons and the Ranked / Rank From conversions (the total order)",
        prod_tree: Leg::Law("ranked_orders_by_rank_then_bytes"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "causally & conjunction (atoms and queries, every admitted pairing)",
        prod_tree: Leg::Law("conjunction_is_intersection"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &["conjunction_operand_forms_agree"] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "causally ! complement (atom negation into the polar hole)",
        prod_tree: Leg::Law("atom_membership_matches_relations"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "From into Query (atoms, spans, versions, borrowed queries)",
        prod_tree: Leg::Law("conjunction_operand_forms_agree"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "unbounded depth (beyond the differential grids)",
        prod_tree: Leg::Excluded(Exclusion::GridCap { guard: "deep_tree_stack_safety" }),
        prod_fs: Leg::Excluded(Exclusion::GridCap { guard: "grid_cap_is_never_reached" }),
        tree_fs: Leg::Excluded(Exclusion::GridCap { guard: "grid_cap_is_never_reached" }),
    },
    SurfaceRow {
        op: "meter instrumentation plumbing",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "board_coverage_tiles_the_public_surface",
            pins: &[],
        }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "board_coverage_tiles_the_public_surface",
            pins: &[],
        }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "board_coverage_tiles_the_public_surface",
            pins: &[],
        }),
    },
    SurfaceRow {
        op: "error verdict types (Decode / Parse / Crossed)",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::decode",
            pins: &[
                "span_gate_admits_exactly_the_ordered",
                "rank_decoding_rejects_each_genre",
                "span_decode_rejects_each_genre",
                "from_str_is_strict_about_shape",
            ],
        }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::decode",
            pins: &[],
        }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::decode",
            pins: &[],
        }),
    },
];
