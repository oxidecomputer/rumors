//! Machine-readable coverage decisions for the public API.
//!
//! Each row states how an operation is compared with the recursive and
//! function-space references, or why a comparison does not apply. Tests keep
//! [`METHOD_SURFACE`] synchronized with the public methods and verify every
//! cited test name. The roster is available under `meter` so other verification
//! tools can use the same coverage decisions.

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

/// A reason that one reference comparison does not apply.
///
/// Each variant records a distinct boundary of the references. Its payload
/// names the production tests or laws that verify the behavior instead. The
/// coverage suite checks every name.
#[derive(Debug)]
pub enum Exclusion {
    /// The references define values but not their byte or text encodings.
    ///
    /// Production tests cover round trips, canonical output, invalid input,
    /// fixed examples, and agreement between buffered and streaming encoders.
    NoWireFormatInReferences {
        /// Tests or laws that verify the production representation.
        pins: &'static [&'static str],
    },
    /// The operation is defined entirely in terms of separately verified ones.
    ///
    /// A production law verifies the definition, so another reference
    /// comparison would duplicate existing coverage.
    DefinitionalCombinator {
        /// Tests or laws that verify the definition.
        pins: &'static [&'static str],
    },
    /// Neither reference defines an operation over a collection.
    ///
    /// The references provide no collection form of split or reconciliation,
    /// and no function-space equivalent is adopted (ratified by owner).
    /// Production laws therefore verify these operations for every input count,
    /// including the values returned after an error.
    NAryNotInReferences {
        /// Production laws quantified over the input count.
        pins: &'static [&'static str],
    },
    /// The behavior depends on Rust ownership or borrowing.
    ///
    /// The value-based references cannot express linear ownership, borrowed
    /// iteration, or compile-time alias prevention. Production tests and
    /// compile-fail examples verify those properties.
    LinearityMechanics {
        /// Production tests or laws for the runtime behavior.
        pins: &'static [&'static str],
    },
    /// The type or operation has no counterpart in the paper's model.
    ///
    /// `bound_at` names where its semantic quantity is compared. Production
    /// tests cover its own arithmetic, ordering, or formatting.
    NotAPaperObject {
        /// The roster row, test, or law where the quantity is bound.
        bound_at: &'static str,
        /// Tests or laws for behavior not covered at `bound_at`.
        pins: &'static [&'static str],
    },
    /// A valid input exceeds a reference implementation's capacity.
    ///
    /// The production behavior is tested directly because the reference cannot
    /// construct or resolve this input size.
    GridCap {
        /// A test that demonstrates the reference limit.
        guard: &'static str,
    },
    /// Canonical encoding makes representation equality equal value equality.
    RepresentationMechanics {
        /// The law that verifies this equivalence.
        license: &'static str,
    },
}

impl Exclusion {
    /// Every exclusion variant name.
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

/// Production tests used when a reference has no wire format.
const CODEC_PINS: &[&str] = &[
    "decode_encode_arbitrary",
    "as_bytes_matches_encode",
    "decode_never_panics",
];

/// Build a row for an encoding operation absent from both references.
const fn codec_row(op: &'static str) -> SurfaceRow {
    SurfaceRow {
        op,
        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: CODEC_PINS }),
        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
    }
}

/// Production tests for the rank encoding.
const RANK_WIRE_PINS: &[&str] = &[
    "rank_lex_order",
    "rank_codec_roundtrip",
    "rank_encoding_prefix_free",
    "rank_lex_encoding_is_suffix_safe",
    "rank_encoding_exhaustive_small_scope",
    "rank_encoding_known_values",
    "rank_decoding_rejects_each_malformed_input_class",
    "rank_encoding_size_is_provenance_linear",
];

/// Build a row for an encoder that writes to a caller-provided sink.
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

/// Laws defining causal predicates in terms of `partial_cmp`.
const CAUSALLY_PINS: &[&str] = &[
    "atom_membership_matches_relations",
    "conjunction_is_intersection",
];

/// Build a row for a causal predicate defined by the laws above.
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

/// Laws defining span placement and its coarser relations.
const SPAN_PLACE_PINS: &[&str] = &[
    "span_place_matches_relations",
    "span_dominance_coarsens_place",
    "span_precedence_coarsens_place",
    "span_contains_matches_place",
];

/// Build a row for a span predicate defined by the laws above.
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

/// Laws for collection operations absent from the references.
const NARY_REFERENCE_GAP: Exclusion = Exclusion::NAryNotInReferences {
    pins: &[
        "party_join_all_err_conserves_the_region_union",
        "clock_join_all_err_conserves_the_region_union",
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
        prod_tree: Leg::Bound("party_is_seed_matches_the_oracle"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["is_seed_iff_equals_seed"],
        }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator {
            pins: &["is_seed_iff_equals_seed"],
        }),
    },
    SurfaceRow {
        op: "Party::tick",
        prod_tree: Leg::Trans("version_tick_matches_the_oracle"),
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
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences {
            pins: &[
                "forks_partial_drop_conserves_party",
                "party_join_all_reunites_forks_at_any_width",
            ],
        }),
    },
    SurfaceRow {
        op: "Party::join",
        prod_tree: Leg::Bound("join_arbitrary"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("party_disjoint_join_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Party::join_all",
        prod_tree: Leg::Bound("join_all_matches_the_recursive_oracle"),
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
        tree_fs: Leg::Excluded(NARY_REFERENCE_GAP),
    },
    SurfaceRow {
        op: "Party::is_disjoint",
        prod_tree: Leg::Bound("party_disjointness_matches_the_oracle"),
        prod_fs: Leg::Trans("party_disjointness_matches_the_oracle"),
        tree_fs: Leg::Bound("party_disjointness_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Party::covers",
        prod_tree: Leg::Bound("party_covers_matches_the_oracle"),
        prod_fs: Leg::Trans("party_covers_matches_the_oracle"),
        tree_fs: Leg::Bound("party_covers_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Party::without",
        prod_tree: Leg::Bound("party_without_matches_the_oracle"),
        prod_fs: Leg::Trans("party_without_matches_the_oracle"),
        tree_fs: Leg::Bound("party_without_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Party::shape",
        prod_tree: Leg::Bound("party_shape_matches_the_oracle"),
        prod_fs: Leg::Trans("party_shape_matches_the_oracle"),
        tree_fs: Leg::Bound("party_shape_matches_the_oracle"),
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
        prod_tree: Leg::Bound("version_tick_matches_the_oracle"),
        prod_fs: Leg::Bound("event_dominates_local_and_advances"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version::ticks",
        prod_tree: Leg::Bound("version_ticks_matches_the_oracle"),
        prod_fs: Leg::Trans("ticks_agrees_with_iterated_ticks"),
        tree_fs: Leg::Trans("version_ticks_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Version::concurrent",
        prod_tree: Leg::Bound("version_concurrency_matches_the_oracle"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version::min_ticks",
        prod_tree: Leg::Bound("version_min_ticks_matches_the_oracle"),
        prod_fs: Leg::Trans("version_min_ticks_matches_the_oracle"),
        tree_fs: Leg::Bound("version_min_ticks_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Version::rank",
        prod_tree: Leg::Bound("version_rank_matches_the_oracle"),
        prod_fs: Leg::Trans("version_rank_matches_the_oracle"),
        tree_fs: Leg::Bound("version_rank_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Version::ranked",
        prod_tree: Leg::Law("ranked_carries_own_rank"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Version::shape",
        prod_tree: Leg::Bound("version_shape_matches_the_oracle"),
        prod_fs: Leg::Trans("version_shape_matches_the_oracle"),
        tree_fs: Leg::Bound("version_shape_matches_the_oracle"),
    },
    // ─────────────────── the shape module and its counts ───────────────────
    SurfaceRow {
        op: "shape::combine",
        prod_tree: Leg::Bound("shape_combine_matches_the_oracle"),
        prod_fs: Leg::Trans("shape_combine_matches_the_oracle"),
        tree_fs: Leg::Bound("shape_combine_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Ticks::limbs",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::min_ticks",
            pins: &[
                "limbs_respell_the_count",
                "u64_conversion_matches_the_range",
            ],
        }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::min_ticks",
            pins: &[],
        }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::min_ticks",
            pins: &[],
        }),
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
        prod_tree: Leg::Bound("version_distance_matches_the_oracle"),
        prod_fs: Leg::Trans("version_distance_matches_the_oracle"),
        tree_fs: Leg::Bound("version_distance_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Version::lag",
        prod_tree: Leg::Bound("version_lag_matches_the_oracle"),
        prod_fs: Leg::Trans("version_lag_matches_the_oracle"),
        tree_fs: Leg::Bound("version_lag_matches_the_oracle"),
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
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
        tree_fs: Leg::Excluded(NARY_REFERENCE_GAP),
    },
    SurfaceRow {
        op: "Clock::forks",
        prod_tree: Leg::Trans("join_all_agrees_with_oracle_on_forked_and_aliased_populations"),
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
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
        op: "Clock::shape",
        prod_tree: Leg::Bound("clock_shape_matches_the_oracle"),
        prod_fs: Leg::Trans("clock_shape_matches_the_oracle"),
        tree_fs: Leg::Bound("clock_shape_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Clock::own_version",
        prod_tree: Leg::Bound("clock_own_version_matches_the_oracle"),
        prod_fs: Leg::Trans("clock_own_version_matches_the_oracle"),
        tree_fs: Leg::Bound("clock_own_version_matches_the_oracle"),
    },
    SurfaceRow {
        op: "OwnVersion::to_version",
        prod_tree: Leg::Bound("version_projection_matches_the_oracle"),
        prod_fs: Leg::Trans("version_projection_matches_the_oracle"),
        tree_fs: Leg::Bound("version_projection_matches_the_oracle"),
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
                "ranked_composite_key_is_suffix_safe_at_the_tiebreak_boundary",
                "ranked_decode_rejects_each_composite_error",
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
                "span_decode_rejects_each_malformed_input_class",
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
        prod_tree: Leg::Bound("version_join_matches_the_oracle"),
        prod_fs: Leg::Bound("replay_matches_across_references"),
        tree_fs: Leg::Bound("replay_matches_across_references"),
    },
    SurfaceRow {
        op: "Version & Version (BitAnd/BitAndAssign, owned and borrowed)",
        prod_tree: Leg::Bound("version_meet_matches_the_oracle"),
        prod_fs: Leg::Trans("version_meet_matches_the_oracle"),
        tree_fs: Leg::Bound("version_meet_matches_the_oracle"),
    },
    SurfaceRow {
        op: "Version ^ Version (BitXor, owned and borrowed — the pair hull)",
        prod_tree: Leg::Trans("span_operator_matrix_is_the_method"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span + impl Into<Span> / Version + Span (Add/AddAssign, owned and borrowed — the containment join)",
        prod_tree: Leg::Law("span_union_is_the_containment_join"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span * Span (Mul, owned and borrowed — the containment meet; partial, so no MulAssign and no Into widening)",
        prod_tree: Leg::Law("span_intersect_is_the_shared_segment"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span | impl Into<Span> / Version | Span (BitOr/BitOrAssign, owned and borrowed — the pointwise join)",
        prod_tree: Leg::Law("span_join_is_the_pointwise_join"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span & impl Into<Span> / Version & Span (BitAnd/BitAndAssign, owned and borrowed — the pointwise meet)",
        prod_tree: Leg::Law("span_meet_is_the_pointwise_meet"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span Sum / FromIterator (spans or versions, owned and borrowed — the union fold)",
        prod_tree: Leg::Law("span_sum_and_collect_are_the_union_fold"),
        prod_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::DefinitionalCombinator { pins: &[] }),
    },
    SurfaceRow {
        op: "Span Product (spans only, owned and borrowed — the intersection fold)",
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
        prod_tree: Leg::Bound("version_projection_matches_the_oracle"),
        prod_fs: Leg::Trans("version_projection_matches_the_oracle"),
        tree_fs: Leg::Bound("version_projection_matches_the_oracle"),
    },
    SurfaceRow {
        op: "OwnVersion vs Version comparisons (PartialEq/PartialOrd, both directions, owned and borrowed)",
        prod_tree: Leg::Bound("own_version_cmp_matches_the_oracle"),
        prod_fs: Leg::Trans("own_version_cmp_matches_materialized"),
        tree_fs: Leg::Trans("version_projection_matches_the_oracle"),
    },
    SurfaceRow {
        op: "OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)",
        prod_tree: Leg::Bound("own_version_pair_cmp_matches_the_oracle"),
        prod_fs: Leg::Trans("own_version_pair_cmp_matches_materialized"),
        tree_fs: Leg::Trans("version_projection_matches_the_oracle"),
    },
    SurfaceRow {
        op: "From<OwnVersion> for Version (explicit materialization)",
        prod_tree: Leg::Trans("from_impl_is_to_version"),
        prod_fs: Leg::Trans("from_impl_is_to_version"),
        tree_fs: Leg::Trans("from_impl_is_to_version"),
    },
    SurfaceRow {
        op: "Version PartialOrd (the comparison matrix, owned and borrowed)",
        prod_tree: Leg::Bound("version_order_matches_the_oracle"),
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
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "From<Clock> for [Clock; N] (consuming balanced split)",
        prod_tree: Leg::Law("clock_forks_matches_from_array"),
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
        tree_fs: Leg::Excluded(Exclusion::NAryNotInReferences { pins: &[] }),
    },
    SurfaceRow {
        op: "iter::Party / iter::Clock (fork iterators and partial-drop conservation)",
        prod_tree: Leg::Law("forks_partial_drop_conserves_party"),
        prod_fs: Leg::Excluded(NARY_REFERENCE_GAP),
        tree_fs: Leg::Excluded(Exclusion::LinearityMechanics { pins: &[] }),
    },
    codec_row("serde / borsh impls (feature-gated, strict-decode pinned)"),
    SurfaceRow {
        op: "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display / FromStr",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::rank", pins: &["rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs", "rank_sum_equals_the_pairwise_fold"] }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::rank", pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::rank", pins: &[] }),
    },
    SurfaceRow {
        op: "Ticks ZERO / From / TryFrom / Display / Add / Sum / Ord / Eq / Hash",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::min_ticks", pins: &["addition_behaves_like_the_naturals", "ticks_agrees_with_iterated_ticks", "ticks_composes", "u64_conversion_matches_the_range"] }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::min_ticks", pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::min_ticks", pins: &[] }),
    },
    SurfaceRow {
        op: "shape iterators (Plateaus / Regions / Overlay / Cells / Limbs: Iterator, FusedIterator, ExactSizeIterator)",
        prod_tree: Leg::Trans("version_shape_matches_the_oracle"),
        prod_fs: Leg::Trans("version_shape_matches_the_oracle"),
        tree_fs: Leg::Trans("version_shape_matches_the_oracle"),
    },
    SurfaceRow {
        op: "shape item types (Plateau / Rise / Region / Cell: Clone, Eq, Debug)",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::shape", pins: &[] }),
        prod_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::shape", pins: &[] }),
        tree_fs: Leg::Excluded(Exclusion::NotAPaperObject { bound_at: "Version::shape", pins: &[] }),
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
        op: "error verdict types (Decode / Crossed)",
        prod_tree: Leg::Excluded(Exclusion::NotAPaperObject {
            bound_at: "Version::decode",
            pins: &[
                "span_gate_admits_exactly_the_ordered",
                "rank_decoding_rejects_each_malformed_input_class",
                "span_decode_rejects_each_malformed_input_class",
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
