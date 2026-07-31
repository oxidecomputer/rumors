//! The complexity-claims roster: every public operation's documented
//! asymptotic class, bound to the amplification board's verdicts.
//!
//! The public rustdoc states each operation's cost in a uniform
//! `# Complexity` section. Prose cannot be checked; a rendered line can —
//! so the roster here pins, per operation, a structured [`Bound`] whose
//! rendering must appear **verbatim as the terminal line** of the
//! section at each recorded site, plus the board rows whose verdicts
//! witness the claimed class, and the tests hold the legs together:
//!
//! - **Prose ↔ roster** ([`doc_index`]): a source scan over the same files
//!   the surface-coverage suite pins
//!   ([`surface_coverage::SURFACE_SOURCES`]) locates each operation's
//!   `# Complexity` section — on the `pub fn`, the type doc, the module
//!   doc, or a documented trait impl, as the roster's [`Site`] records —
//!   and byte-compares its last line against the roster's rendered
//!   `**Complexity**:` line. Editing a section's class without this
//!   roster is a named failure; the prose above the terminal line is
//!   explanation, uniformly non-normative.
//! - **Roster ↔ board**: every cited board row must exist in the board's
//!   own operation axis ([`board::bench_cells`]).
//! - **Class ↔ evidence** (the class contracts): every [`Class`] variant
//!   declares one [`ClassContract`] — its stance toward exponent-mechanism
//!   entries in the board's red-triage buffer
//!   ([`board::BOARD_EXPECTED_REDS`]), whether it claims a
//!   bench-judge-rostered time leg (`tools/benchjudge-expected.json`,
//!   membership-pinned by `tests/bench_judge_roster.rs`), its defining
//!   rendered token, and its named witness tests — and one test enforces
//!   every contract the same way, so curing a red, flipping a class, or
//!   retiring a witness reaches the rustdoc through a failing name here.
//!   The exhaustive match in [`Class::contract`] makes a contract-less
//!   class a compile error; the judge's red set binds wall time, the
//!   exponent-red stances bind the deterministic counters' verdicts, so a
//!   counter-superlinear kernel whose wall constant hides under the
//!   judge's resolution cannot keep a flat-counter class.
//! - **Class liveness**: every non-linear class's contract names a
//!   deterministic pin proving the documented behavior still exists —
//!   the render merge's superlinear limb growth on the wide left-full
//!   shape, the n-ary fold's log factor on the scatter population, and
//!   the `MulBound` claims' answer-embedded product (the
//!   plateau-puncture rank equals a wide × dense closed form whose
//!   factors scale with the input *and* stay incompressible under the
//!   settle's own compaction) live in this suite. A cure landing flips
//!   the pin red, forcing roster and rustdoc to move in the same
//!   change.
//!
//! Totality rides the coverage surface: every name in
//! [`surface_coverage::extract_public_fns`] and
//! [`surface_coverage::FAMILY_SURFACE`] has exactly one claim row (or a
//! place in [`NON_OPERATIONS`], the family rows that are dispositions
//! rather than operations), so a new public operation fails this roster
//! until its documented class is pinned.
//!
//! # The rendered line
//!
//! [`Bound::render`] emits the one normative sentence per site, in a
//! uniform vocabulary: `n` is the operation's packed input size (`a`/`b`
//! for a pair's), `t` text bytes, `S` an n-ary hand-out's total share
//! size, `D`/`k` a fold's total packed input and operand count (`B` its
//! both-present node count), `M(·)` the arithmetic backend's
//! integer-multiplication bound, and `‖·‖` a value's numeric size where
//! no packed encoding exists. A bare `O(...)` covers time and space;
//! forms that split the two say so. [`Bound::Custom`] is the escape
//! hatch for a row whose honest bound fits no template — every use
//! states its reason beside the line. Rows sharing one doc site (a type
//! doc pricing a whole operator matrix, the `causally` module doc) share
//! one `Bound`, so the shared section carries one line and the binding
//! test holds every such row to it.

use super::surface_coverage;

// The bound vocabulary, its renderer, the doc-section scanner, and the
// witness scanner are the workspace-shared claims machinery (the
// `complexity-claims` crate); the roster rows, the class contracts, and
// the board/judge bindings below are before's own.
pub(crate) use ::complexity_claims::{section_of, Bound, Check, DocIndex, Site};

#[cfg(test)]
mod tests;

/// The asymptotic class a board row's verdict witnesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    /// Amortized linear in the packed inputs (input-denominated rows).
    Linear,
    /// Linear in the inputs plus the output the operation must
    /// materialize (the I/O-denominated rows: text, output-dominated
    /// projection).
    LinearIo,
    /// Linear in the total packed input times the logarithm of the
    /// operand count: the balanced n-ary reduction's log factor.
    ///
    /// Visible on the fold rows' deterministic exponents and
    /// scale-growing constants and judged there under the board's
    /// declared fold model — which, on the party fold, also carries the
    /// indexed overlap test's per-node search allowance, the
    /// `B log |p|` term the fold rustdoc states.
    FoldLog,
    /// Linear space, superlinear worst-case time, red on the bench
    /// judge's committed roster.
    ///
    /// The membership is structural, never a prose roster: the
    /// operations whose value conversion or render merge delegates
    /// superlinear work below the counters — the [`CLAIMS`] rows
    /// citing this class are the enumeration of record.
    SuperlinearTime,
    /// Superlinear in the deterministic work counters on committed
    /// board families, while absent from the bench judge's red roster.
    ///
    /// A standing exponent-mechanism entry in
    /// [`board::BOARD_EXPECTED_REDS`] with the rustdoc stating the
    /// superlinear worst case; the operation's wall constant sits under
    /// the judge's resolution at bench scales, so the counter leg is
    /// the one that sees the class.
    ///
    /// The class-binding tests hold it live in both directions: a claim
    /// in this class must cite at least one exponent-red board cell
    /// (else the class is decoration and the claim flips to a linear
    /// one), and no linear claim may cite any. Currently unpopulated:
    /// the class and its seal stand ready for the next
    /// counter-superlinear finding, and the mutation tests keep both
    /// directions honest meanwhile.
    SuperlinearCounter,
    /// Superlinear worst-case time at the arithmetic backend's
    /// integer-multiplication bound, delegated below every
    /// deterministic counter: the counters legitimately read flat on
    /// the very families that witness the worst case.
    ///
    /// The membership is structural, never a prose roster: the
    /// settle-delegating query folds (single-stream and pair, one
    /// shared integrator whose settle products ride the backend's
    /// multiplication) and the `Ranked` key surface built on them —
    /// the [`CLAIMS`] rows citing this class are the enumeration of
    /// record. No board or judge red can exist for this class — the
    /// counters price the fold's own traffic, which *is* linear, and
    /// the multiplication's wall share sits far under the judge's
    /// resolution at bench scales — so its evidence is structural,
    /// named by its contract's witnesses: the wide × dense flatness
    /// bands, the committed schoolbook kernels failing beside them,
    /// and the answer-embedded-product liveness pins. An answer that
    /// stops embedding the product dissolves the class back to a
    /// linear one in the same change.
    MulBound,
}

/// A class's stance toward the board's standing exponent-mechanism
/// reds ([`board::BOARD_EXPECTED_REDS`] entries tagged `exponent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedStance {
    /// A cited cell with a standing exponent red contradicts the
    /// class: its counters are claimed to scale as the class's model.
    Forbidden,
    /// The class exists *because* of a standing exponent red: a claim
    /// without one is decoration.
    Required,
    /// The class makes no counter claim; a standing exponent red on
    /// the same operation is consistent with it.
    Allowed,
}

/// The uniform mechanical bindings of one [`Class`]: what the binding
/// tests enforce for every claim citing it.
///
/// Every variant declares one contract (the exhaustive match in
/// [`Class::contract`] makes a contract-less class a compile error),
/// and one test (`classes_satisfy_their_contracts`) enforces all of
/// them the same way — no class's binding is bespoke machinery.
pub(crate) struct ClassContract {
    /// Stance toward the board's exponent-mechanism reds.
    pub(crate) exponent_reds: RedStance,
    /// Whether the class claims a bench-judge-rostered superlinear
    /// time leg: the rows cited under judge-red classes must equal the
    /// judge roster's red set exactly, and no other class may cite a
    /// rostered row.
    pub(crate) judge_red: bool,
    /// The class-defining token: every claim citing the class must
    /// render a `**Complexity**:` line containing it.
    pub(crate) token: Option<&'static str>,
    /// Whether the defining token is exclusive to the class: a claim
    /// rendering it without citing the class is unclassed prose.
    pub(crate) token_exclusive: bool,
    /// Committed witnesses that must exist in the tree by name, as
    /// `(manifest-relative file, test fn name)`.
    ///
    /// Measurement pins and adequacy kernels, each required to be a
    /// `#[test]`-attributed function in its file (a mention in prose
    /// does not count), so a deletion or rename fails a reviewed name
    /// here, never silently.
    pub(crate) witnesses: &'static [(&'static str, &'static str)],
}

impl Class {
    /// Every class, for contract iteration (the compiler holds
    /// [`Class::contract`] total; this list holds the witness sweep
    /// total — a new variant joins both in one edit).
    pub(crate) const ALL: &'static [Class] = &[
        Class::Linear,
        Class::LinearIo,
        Class::FoldLog,
        Class::SuperlinearTime,
        Class::SuperlinearCounter,
        Class::MulBound,
    ];

    /// The class's uniform mechanical binding.
    pub(crate) fn contract(self) -> ClassContract {
        match self {
            // The plain linear classes: flat counters, no judge red,
            // no defining token beyond the Big-O lead itself.
            Class::Linear | Class::LinearIo => ClassContract {
                exponent_reds: RedStance::Forbidden,
                judge_red: false,
                token: None,
                token_exclusive: false,
                witnesses: &[],
            },
            // The fold classes promise the balanced reduction's log
            // factor: the `log k` token is theirs alone, and the
            // scatter-population growth pin keeps the factor alive.
            Class::FoldLog => ClassContract {
                exponent_reds: RedStance::Forbidden,
                judge_red: false,
                token: Some("log k"),
                token_exclusive: true,
                witnesses: &[(
                    "src/testing/complexity_claims/tests.rs",
                    "fold_log_factor_is_alive",
                )],
            },
            // Judge-rostered superlinear time. The token is not
            // exclusive: type docs legitimately note superlinear
            // rendering costs (Rank, Ticks) while their own cells
            // stay linear.
            Class::SuperlinearTime => ClassContract {
                exponent_reds: RedStance::Allowed,
                judge_red: true,
                token: Some("superlinear"),
                token_exclusive: false,
                witnesses: &[(
                    "src/testing/complexity_claims/tests.rs",
                    "render_merge_superlinearity_is_alive",
                )],
            },
            // Counter-witnessed superlinearity: the standing exponent
            // red is the class's whole evidence (currently
            // unpopulated; the decoration fixture keeps the stance's
            // reverse leg honest).
            Class::SuperlinearCounter => ClassContract {
                exponent_reds: RedStance::Required,
                judge_red: false,
                token: Some("superlinear"),
                token_exclusive: false,
                witnesses: &[],
            },
            // The multiplication-bound delegation: flat counters by
            // design (a standing exponent red means the delegation
            // failed and the honest home is SuperlinearCounter), the
            // Ω(M(·)) floor token exclusively its own, and the
            // committed evidence named through both settle doors —
            // rank's single-stream fold (both wide × dense flatness
            // bands, both schoolbook kernels, the
            // answer-embedded-product liveness pin) and the pair
            // co-sweep, whose claims (distance, lag, the ranked
            // comparisons) enter the settle through a distinct entry
            // point (the pair flatness band and the pair embedding
            // pin).
            Class::MulBound => ClassContract {
                exponent_reds: RedStance::Forbidden,
                judge_red: false,
                token: Some("Ω(M("),
                token_exclusive: true,
                witnesses: &[
                    ("tests/meter.rs", "rank_wide_arming_is_flat_per_unit"),
                    ("tests/meter.rs", "rank_plateau_puncture_is_flat_per_unit"),
                    ("tests/meter.rs", "pair_plateau_train_is_flat_per_unit"),
                    (
                        "src/version/skyline/query/tests.rs",
                        "schoolbook_settle_reads_superlinear_on_wide_arming",
                    ),
                    (
                        "src/version/skyline/query/tests.rs",
                        "schoolbook_settle_reads_superlinear_on_plateau_puncture",
                    ),
                    (
                        "src/testing/complexity_claims/tests.rs",
                        "mul_bound_embedding_is_alive",
                    ),
                    (
                        "src/testing/complexity_claims/tests.rs",
                        "mul_bound_pair_embedding_is_alive",
                    ),
                    (
                        "src/testing/complexity_claims/tests.rs",
                        "mul_bound_key_embedding_is_alive",
                    ),
                ],
            },
        }
    }
}

/// The board leg of one claim: the rows witnessing the class, or the
/// reason none exists (mirroring the board's coverage table).
pub(crate) enum Cells {
    /// `(board operation name, the class its verdict witnesses)`.
    Board(&'static [(&'static str, Class)]),
    /// No board row of its own, with the reason.
    Uncelled(&'static str),
}

/// One public operation's pinned complexity claim.
pub(crate) struct Claim {
    /// The operation, named exactly as the coverage surface names it.
    pub(crate) op: &'static str,
    pub(crate) checks: &'static [Check],
    pub(crate) cells: Cells,
}

/// Surface family rows that are coverage dispositions, not operations:
/// they carry no cost contract of their own, so they have no claim row.
pub(crate) const NON_OPERATIONS: &[&str] = &[
    "unbounded depth (beyond the differential grids)",
    "meter / error / iter plumbing",
];

/// Shorthand for a word-scale operation: `O(1)` on its own doc block, no
/// board row (nothing scales).
const fn constant(op: &'static str) -> Claim {
    Claim {
        op,
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        cells: Cells::Uncelled("word-scale: no input axis to measure against"),
    }
}

/// The `causally` module doc's shared line: eighteen rows price the
/// same three facts, so they share one bound at one site.
///
/// The deriving constructors (`Version::span`/`Version::span_all`) are
/// priced at their own fn docs, not here.
const CAUSALLY_BOUND: Bound = Bound::Custom {
    line: "borrowing constructors `O(1)` (the deriving `span`/`span_all` priced on `Version`); validation at most one causal comparison; placement one fused pass `O(v + s + e)`.",
    reason: "one module-doc section prices every constructor and predicate together",
};

/// Shorthand for a causally-module operation: the module doc carries one
/// note (and one rendered line) for all of them.
const fn causally(op: &'static str, cells: Cells) -> Claim {
    Claim {
        op,
        checks: &[Check {
            site: Site::ModuleDoc("src/causally.rs"),
            bound: CAUSALLY_BOUND,
        }],
        cells,
    }
}

/// The tick-count bounds: a pair (or single) packed operand plus the
/// count's width. No template names a count axis, so the ticks trio
/// carries the count term as a stated custom line.
const TICKS_PAIR_BOUND: Bound = Bound::Custom {
    line: "`O(a + b + log m)`, `m` the tick count.",
    reason: "the fused multi-tick adds only the count's width; no template names a count axis",
};

/// [`TICKS_PAIR_BOUND`]'s single-operand form, for the clock spelling.
const TICKS_SINGLE_BOUND: Bound = Bound::Custom {
    line: "`O(n + log m)`, `m` the tick count.",
    reason: "the fused multi-tick adds only the count's width; no template names a count axis",
};

/// The party split's hand-out bound, shared by `Party::forks`'s own doc
/// and the consuming array split: the denominator is the shares
/// produced, not a packed operand.
const PARTY_SPLIT_BOUND: Bound = Bound::Custom {
    line: "`O(S)`, `S` the shares' total packed size.",
    reason: "an n-ary hand-out is denominated in its produced shares, not one packed operand",
};

/// The party `Forks` iterator's type-doc bound: the drain plus the
/// early-drop rejoin.
const PARTY_FORKS_TYPE_BOUND: Bound = Bound::Custom {
    line: "a full drain `O(S)`, `S` the shares' total packed size; an early drop rejoins in \
           `O(log n)` joins.",
    reason: "an n-ary hand-out is denominated in its produced shares, not one packed operand",
};

/// The clock split's bound, shared by `Clock::forks`'s own doc and the
/// clock `Forks` iterator's type doc: the party split plus one version
/// clone per child.
// 2026-07-30, the Bytes-backed at-rest form: was `O(S + n·|v|)` — a
// version clone was a byte copy; it is now a refcount bump, so the
// per-child term drops to a constant.
const CLOCK_SPLIT_BOUND: Bound = Bound::Custom {
    line: "`O(S + n)`: the party split plus one `O(1)` version clone per child.",
    reason: "an n-ary hand-out denominated in its shares and per-child clones, not one \
             packed operand",
};

/// The explicit projection materialization's bound, shared by
/// `OwnVersion::to_version` and the `From` impl: the output is not
/// bounded by a constant factor of the operands.
const PROJECTION_BOUND: Bound = Bound::Custom {
    line: "`O(|v| + |p| + |r|)`, `|r|` the result's packed size.",
    reason: "output-dominated: the result's size is not derivable from the operands, so the \
             honest denominator names it",
};

/// The `Version` type doc's shared line: four family rows (the join and
/// meet operators, the comparison matrix, `Eq`/`Hash`) price one
/// section.
const VERSION_TYPE_BOUND: Bound = Bound::Custom {
    line: "every comparison, join, and meet `O(a + b)`; hashing `O(n)`.",
    reason: "one type-doc section prices the whole operator matrix",
};

/// The `OwnVersion` type doc's shared line: both fused-comparison
/// family rows price one section.
const OWN_VERSION_TYPE_BOUND: Bound = Bound::Custom {
    line: "construction `O(1)`; view vs version `O(|v| + |p| + |w|)`; view vs view \
           `O(|v₁| + |p₁| + |v₂| + |p₂|)`.",
    reason: "one type-doc section prices both fused co-walks and the O(1) construction",
};

/// The reason the causally constructors cite no board row.
const CAUSALLY_CONSTRUCTOR: &str =
    "stores two borrows; the comparison cost is on the membership predicates \
     (the causally_contains row prices them)";

/// The reason the causally compositions (a start paired with an end) cite
/// no board row.
const CAUSALLY_COMPOSITION: &str =
    "stores two borrows plus at most one validating causal comparison, the \
     identical comparison the causally_contains row prices";

/// The claims roster of record: one row per public operation, named as
/// the coverage surface names it.
///
/// The tests hold it total, its sites' terminal lines byte-equal to
/// the rendered bounds, and its cited cells alive on the board.
pub(crate) const CLAIMS: &[Claim] = &[
    // ───────────────────────────── Party ─────────────────────────────
    constant("Party::seed"),
    constant("Party::is_seed"),
    Claim {
        op: "Party::tick",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[
            ("version_tick", Class::Linear),
            ("version_tick_adv_party", Class::Linear),
        ]),
    },
    Claim {
        op: "Party::ticks",
        checks: &[Check {
            site: Site::Fn,
            bound: TICKS_PAIR_BOUND,
        }],
        cells: Cells::Board(&[("version_ticks", Class::Linear)]),
    },
    Claim {
        op: "Party::fork",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("party_fork", Class::Linear)]),
    },
    Claim {
        op: "Party::forks",
        checks: &[
            Check {
                site: Site::Fn,
                bound: PARTY_SPLIT_BOUND,
            },
            Check {
                site: Site::TypeDoc("src/party/forks.rs", "Forks"),
                bound: PARTY_FORKS_TYPE_BOUND,
            },
        ],
        cells: Cells::Uncelled(
            "iterates the measured fork on shrinking operands; no board row of \
             its own (the board's coverage table)",
        ),
    },
    Claim {
        op: "Party::join",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[
            ("party_join", Class::Linear),
            ("party_join_overlap", Class::Linear),
        ]),
    },
    Claim {
        op: "Party::join_all",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::FoldSearch,
        }],
        cells: Cells::Board(&[
            ("party_join_all", Class::FoldLog),
            ("party_join_all_overlap", Class::Linear),
        ]),
    },
    Claim {
        op: "Party::is_disjoint",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[("party_disjoint", Class::Linear)]),
    },
    Claim {
        op: "Party::covers",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[("party_covers", Class::Linear)]),
    },
    Claim {
        op: "Party::without",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[
            ("party_without", Class::Linear),
            ("party_without_none", Class::Linear),
        ]),
    },
    Claim {
        op: "Party::dangerously_alias",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        // 2026-07-30, the Bytes-backed at-rest form: was Linear (one
        // byte copy); the alias now shares the refcounted stored buffer.
        cells: Cells::Uncelled("one refcount bump (the board's coverage table)"),
    },
    Claim {
        op: "Party::encode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("party_encode", Class::Linear)]),
    },
    Claim {
        op: "Party::encode_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("party_encode", Class::Linear)]),
    },
    constant("Party::encoded_bits"),
    Claim {
        op: "Party::decode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[
            ("party_decode", Class::Linear),
            ("party_decode_truncated", Class::Linear),
            ("party_decode_trailing", Class::Linear),
            ("party_decode_noncanon", Class::Linear),
        ]),
    },
    constant("Party::as_bytes"),
    // ───────────────────────────── Version ─────────────────────────────
    constant("Version::new"),
    constant("Version::is_empty"),
    Claim {
        op: "Version::tick",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[
            ("version_tick", Class::Linear),
            ("version_tick_adv_party", Class::Linear),
        ]),
    },
    Claim {
        op: "Version::ticks",
        checks: &[Check {
            site: Site::Fn,
            bound: TICKS_PAIR_BOUND,
        }],
        cells: Cells::Board(&[("version_ticks", Class::Linear)]),
    },
    Claim {
        op: "Version::concurrent",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[("version_concurrent", Class::Linear)]),
    },
    Claim {
        op: "Version::min_ticks",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("version_min_ticks", Class::Linear)]),
    },
    Claim {
        op: "Version::rank",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        // The three-part time claim, each part with a committed
        // witness: the O(M(|v|) · log |v|) worst case is the settle's
        // bound (the query module doc derives it: every settle
        // product delegated cluster-wise to the backend's
        // multiplication, re-associated through a mass-balanced
        // product tree whose per-node products telescope under the
        // backend's power-law tiers — the tree-depth log survives
        // only past its quasilinear threshold), held from above by
        // the ledger_wide_arming and answer_embedded_product flatness
        // bands in tests/meter.rs (both wide × dense families flat
        // per byte in the fold's own traffic) with the schoolbook
        // kernel committed and failing beside them (the query fold's
        // test suite). Ω(M(|v|)) is mandatory because the
        // puncture-product family embeds arbitrary integer products
        // in exact answers — the committed reduction proptest and
        // the band's exact-rank leg witness it; the O(|v| log |v|) leg is
        // the dense-suffix/promo-rearm flatness bands' reading on
        // every O(1)-wide-parked family. Neither witness family is a
        // board family — the board reads every committed rank row
        // green — legitimately: the multiplication runs inside the
        // backend, below the limb shim, so no counter or judge red
        // can witness the worst case. The row is MulBound-classed:
        // the class carries exactly this structure (rustdoc worst
        // case at the multiplication bound, flat counters, the
        // embedding held alive by mul_bound_embedding_is_alive), so
        // neither Linear (a false claim against a proven Ω(M(|v|))
        // worst case) nor SuperlinearCounter/SuperlinearTime (their
        // seals demand reds that cannot honestly exist here) fits.
        cells: Cells::Board(&[("version_rank", Class::MulBound)]),
    },
    constant("Version::ranked"),
    Claim {
        op: "Version::distance",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBoundPair,
        }],
        // The pair form of rank's three-part claim — one shared
        // integrator, so the same witnesses (plus the settle_flatness
        // pair probe, which drives both settle sites through the
        // public distance and lag) and class reasoning as
        // Version::rank above.
        cells: Cells::Board(&[("version_distance", Class::MulBound)]),
    },
    Claim {
        op: "Version::lag",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBoundPair,
        }],
        // As Version::distance above (one shared co-sweep).
        cells: Cells::Board(&[("version_lag", Class::MulBound)]),
    },
    Claim {
        op: "Version::join_all",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Fold,
        }],
        cells: Cells::Board(&[("version_join_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version::meet_all",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Fold,
        }],
        // The join fold's balanced reduction over the meet emitter; the
        // non-shrinking-accumulator worst case is held flat by the
        // `meet_fold` band and its sequential-reduce tripwire
        // (`tests/meter.rs`) on the meet-shade population.
        cells: Cells::Board(&[("version_meet_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version::span",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        // The fused pair hull has its own row: one sweep feeds both
        // endpoints, priced directly rather than as the join/meet
        // composition it undercuts (the span scan-identity pins in
        // tests/meter.rs hold the undercut exact).
        cells: Cells::Board(&[("version_span", Class::Linear)]),
    },
    Claim {
        op: "Version::span_all",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Fold,
        }],
        // The hull fold has its own row: one balanced reduction
        // carrying both endpoints, leaf combines fused.
        cells: Cells::Board(&[("version_span_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version::encode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("version_encode", Class::Linear)]),
    },
    Claim {
        op: "Version::encode_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("version_encode", Class::Linear)]),
    },
    Claim {
        op: "Version::decode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[
            ("version_decode", Class::Linear),
            ("version_decode_truncated", Class::Linear),
            ("version_decode_trailing", Class::Linear),
            ("version_decode_noncanon", Class::Linear),
        ]),
    },
    constant("Version::encoded_bits"),
    constant("Version::as_bytes"),
    // ───────────────────────────── Clock ─────────────────────────────
    constant("Clock::seed"),
    Claim {
        op: "Clock::tick",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("clock_tick", Class::Linear)]),
    },
    Claim {
        op: "Clock::ticks",
        checks: &[Check {
            site: Site::Fn,
            bound: TICKS_SINGLE_BOUND,
        }],
        cells: Cells::Board(&[("version_ticks", Class::Linear)]),
    },
    Claim {
        op: "Clock::fork",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("clock_fork", Class::Linear)]),
    },
    Claim {
        op: "Clock::forks",
        checks: &[
            Check {
                site: Site::Fn,
                bound: CLOCK_SPLIT_BOUND,
            },
            Check {
                site: Site::TypeDoc("src/clock/forks.rs", "Forks"),
                bound: CLOCK_SPLIT_BOUND,
            },
        ],
        cells: Cells::Uncelled(
            "iterates the measured fork on shrinking operands, one version \
             clone per child; no board row of its own (the board's coverage \
             table)",
        ),
    },
    Claim {
        op: "Clock::join",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[
            ("clock_join", Class::Linear),
            ("clock_join_overlap", Class::Linear),
        ]),
    },
    Claim {
        op: "Clock::join_all",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::FoldSearch,
        }],
        cells: Cells::Board(&[
            ("version_join_all", Class::FoldLog),
            ("party_join_all", Class::FoldLog),
        ]),
    },
    Claim {
        op: "Clock::sync",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[
            ("clock_sync", Class::Linear),
            ("clock_sync_overlap", Class::Linear),
        ]),
    },
    Claim {
        op: "Clock::send",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("clock_tick", Class::Linear)]),
    },
    Claim {
        op: "Clock::recv",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::LinearPair,
        }],
        cells: Cells::Board(&[("clock_recv", Class::Linear)]),
    },
    constant("Clock::from_parts"),
    constant("Clock::into_parts"),
    constant("Clock::party"),
    constant("Clock::version"),
    Claim {
        op: "Clock::own_version",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        cells: Cells::Uncelled(
            "view construction: two borrows; the materialization and fused \
             comparison costs live on the OwnVersion rows",
        ),
    },
    Claim {
        op: "OwnVersion::to_version",
        checks: &[Check {
            site: Site::Fn,
            bound: PROJECTION_BOUND,
        }],
        cells: Cells::Board(&[
            ("own_version_to_version", Class::LinearIo),
            ("clock_own_version_to_version", Class::LinearIo),
        ]),
    },
    Claim {
        op: "Clock::encode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("clock_encode", Class::Linear)]),
    },
    Claim {
        op: "Clock::encode_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("clock_encode", Class::Linear)]),
    },
    Claim {
        op: "Clock::decode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[
            ("clock_decode", Class::Linear),
            ("clock_decode_truncated", Class::Linear),
            ("clock_decode_trailing", Class::Linear),
        ]),
    },
    constant("Clock::encoded_bits"),
    Claim {
        op: "Clock::dangerously_alias",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        // 2026-07-30, the Bytes-backed at-rest form: was Linear (one
        // byte copy per part); the alias now shares each part's
        // refcounted stored buffer.
        cells: Cells::Uncelled("one refcount bump per part (the board's coverage table)"),
    },
    // ───────────────────────────── Rank / Ranked ─────────────────────────────
    Claim {
        op: "Rank::checked_sub",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Custom {
                line: "`O(‖a‖ + ‖b‖)`, the operands' numeric sizes.",
                reason: "the operands are in-memory ranks; costs are \
                         value-content-denominated",
            },
        }],
        cells: Cells::Board(&[("rank_pair_ops", Class::Linear)]),
    },
    Claim {
        op: "Rank::encode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Custom {
                line: "`O(‖r‖)` time and space; the output is at most `9⁄8 · ‖r‖ + O(log ‖r‖)` \
                       bits.",
                reason: "the operand is an in-memory rank denominated by value content; \
                         the output is mandatory, so the honest bound names it",
            },
        }],
        cells: Cells::Board(&[("rank_encode", Class::LinearIo)]),
    },
    Claim {
        op: "Rank::encode_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Custom {
                line: "`O(‖r‖)` time and space; the output is at most `9⁄8 · ‖r‖ + O(log ‖r‖)` \
                       bits.",
                reason: "the identical emission with a writer sink; the rank_encode cell \
                         prices it",
            },
        }],
        cells: Cells::Board(&[("rank_encode", Class::LinearIo)]),
    },
    Claim {
        op: "Rank::decode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("rank_decode", Class::Linear)]),
    },
    constant("Ranked::version"),
    Claim {
        op: "Ranked::to_rank",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        // Definitional delegation: one rank fold over the viewed
        // version, so the row of record is Version::rank's.
        cells: Cells::Board(&[("version_rank", Class::MulBound)]),
    },
    // 2026-07-30, the Bytes-backed at-rest form: was `O(n)` when
    // borrowed — settling a borrow cloned the version's bytes; a clone
    // is now a refcount bump, so the settle is constant either way.
    Claim {
        op: "Ranked::into_owned",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        cells: Cells::Uncelled(
            "one refcount bump when borrowed (the board's coverage table)",
        ),
    },
    Claim {
        op: "Ranked::encode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        cells: Cells::Board(&[("ranked_encode", Class::MulBound)]),
    },
    Claim {
        op: "Ranked::encode_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        // The identical composite emission with a writer sink; the
        // ranked_encode cell prices it.
        cells: Cells::Board(&[("ranked_encode", Class::MulBound)]),
    },
    Claim {
        op: "Ranked::encode_rank",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        cells: Cells::Board(&[("ranked_encode_rank", Class::MulBound)]),
    },
    Claim {
        op: "Ranked::encode_rank_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        // The identical fused rank-only fold and emission with a
        // writer sink; the ranked_encode_rank cell prices it.
        cells: Cells::Board(&[("ranked_encode_rank", Class::MulBound)]),
    },
    Claim {
        op: "Ranked::decode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::MulBound,
        }],
        // One strict linear parse plus the verifying rank fold over
        // the decoded version: the fold's class governs.
        cells: Cells::Board(&[("ranked_decode", Class::MulBound)]),
    },
    // ───────────────────────────── causally ─────────────────────────────
    causally("causally::all", Cells::Uncelled(CAUSALLY_CONSTRUCTOR)),
    causally("causally::since", Cells::Uncelled(CAUSALLY_CONSTRUCTOR)),
    causally(
        "causally::not_before",
        Cells::Uncelled(CAUSALLY_CONSTRUCTOR),
    ),
    causally("causally::known_at", Cells::Uncelled(CAUSALLY_CONSTRUCTOR)),
    causally("causally::before", Cells::Uncelled(CAUSALLY_CONSTRUCTOR)),
    causally("causally::delta", Cells::Uncelled(CAUSALLY_COMPOSITION)),
    causally(
        "causally::delta_before",
        Cells::Uncelled(CAUSALLY_COMPOSITION),
    ),
    causally(
        "causally::Range::since",
        Cells::Uncelled(CAUSALLY_COMPOSITION),
    ),
    causally(
        "causally::Range::not_before",
        Cells::Uncelled(CAUSALLY_COMPOSITION),
    ),
    causally(
        "causally::Range::known_at",
        Cells::Uncelled(CAUSALLY_COMPOSITION),
    ),
    causally(
        "causally::Range::before",
        Cells::Uncelled(CAUSALLY_COMPOSITION),
    ),
    causally(
        "causally::Range::contains",
        Cells::Board(&[("causally_contains", Class::Linear)]),
    ),
    causally(
        "causally::Range::placement_of",
        Cells::Board(&[("range_bounded", Class::Linear)]),
    ),
    causally(
        "causally::Range::bounded",
        Cells::Board(&[("range_bounded", Class::Linear)]),
    ),
    causally(
        "causally::Span::new",
        Cells::Uncelled(CAUSALLY_COMPOSITION),
    ),
    causally(
        "causally::Span::new_unchecked",
        Cells::Uncelled(
            "stores two borrows and performs no comparison at all: the trusted \
             door's debug assertion sits outside the cost contract",
        ),
    ),
    causally(
        "causally::Span::place",
        Cells::Board(&[("span_place", Class::Linear)]),
    ),
    causally(
        "causally::Span::dominance_of",
        Cells::Board(&[("span_dominance", Class::Linear)]),
    ),
    constant("causally::Span::meet"),
    constant("causally::Span::join"),
    // 2026-07-30, the Bytes-backed at-rest form: was `O(n)` when
    // borrowed — settling a borrow cloned each endpoint's bytes; a
    // clone is now a refcount bump, so the settle is constant either
    // way.
    Claim {
        op: "causally::Span::into_parts",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        cells: Cells::Uncelled(
            "one refcount bump per borrowed endpoint (the board's coverage table)",
        ),
    },
    constant("causally::Span::reborrow"),
    // 2026-07-30, the Bytes-backed at-rest form: was `O(n)` when
    // borrowed, as `into_parts`.
    Claim {
        op: "causally::Span::into_owned",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        cells: Cells::Uncelled(
            "one refcount bump per borrowed endpoint (the board's coverage table)",
        ),
    },
    Claim {
        op: "causally::Span::encode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("span_encode", Class::Linear)]),
    },
    Claim {
        op: "causally::Span::encode_to",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        // The identical composite emission with a writer sink; the
        // span_encode cell prices it.
        cells: Cells::Board(&[("span_encode", Class::Linear)]),
    },
    Claim {
        op: "causally::Span::decode",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[
            ("span_decode", Class::Linear),
            ("span_decode_truncated", Class::Linear),
            ("span_decode_trailing", Class::Linear),
            ("span_decode_crossed", Class::Linear),
        ]),
    },
    // ─────────────────────── operator/trait families ───────────────────────
    Claim {
        op: "Version | Version (BitOr/BitOrAssign, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            bound: VERSION_TYPE_BOUND,
        }],
        cells: Cells::Board(&[
            ("version_join", Class::Linear),
            ("version_join_assign", Class::Linear),
        ]),
    },
    Claim {
        op: "Version & Version (BitAnd/BitAndAssign, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            bound: VERSION_TYPE_BOUND,
        }],
        cells: Cells::Board(&[
            ("version_meet", Class::Linear),
            ("version_meet_assign", Class::Linear),
        ]),
    },
    Claim {
        op: "&Version / &Party (Div — the lazy projection view)",
        checks: &[Check {
            site: Site::ImplDoc("src/version.rs", "Div<&'a Party> for &'a Version"),
            bound: Bound::Constant,
        }],
        cells: Cells::Uncelled(
            "view construction: two borrows; the materialization and fused \
             comparison costs live on the OwnVersion rows",
        ),
    },
    Claim {
        op: "OwnVersion vs Version comparisons (PartialEq/PartialOrd, both directions, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version/own.rs", "OwnVersion"),
            bound: OWN_VERSION_TYPE_BOUND,
        }],
        cells: Cells::Board(&[("own_version_cmp", Class::Linear)]),
    },
    Claim {
        op: "OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version/own.rs", "OwnVersion"),
            bound: OWN_VERSION_TYPE_BOUND,
        }],
        cells: Cells::Board(&[("own_version_pair_cmp", Class::Linear)]),
    },
    Claim {
        op: "From<OwnVersion> for Version (explicit materialization)",
        checks: &[Check {
            site: Site::ImplDoc("src/version/own.rs", "From<OwnVersion<'_>> for Version"),
            bound: PROJECTION_BOUND,
        }],
        cells: Cells::Board(&[("own_version_to_version", Class::LinearIo)]),
    },
    Claim {
        op: "Version PartialOrd (the comparison matrix, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            bound: VERSION_TYPE_BOUND,
        }],
        cells: Cells::Board(&[("version_cmp", Class::Linear)]),
    },
    Claim {
        op: "Version Sum / FromIterator (owned and borrowed)",
        checks: &[
            Check {
                site: Site::ImplDoc("src/version.rs", "impl Sum<Version> for Version"),
                bound: Bound::Fold,
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl<'a> Sum<&'a Version> for Version"),
                bound: Bound::Fold,
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl FromIterator<Version> for Version"),
                bound: Bound::Fold,
            },
            Check {
                site: Site::ImplDoc(
                    "src/version.rs",
                    "impl<'a> FromIterator<&'a Version> for Version",
                ),
                bound: Bound::Fold,
            },
        ],
        cells: Cells::Board(&[("version_join_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version Eq / Hash (canonical byte compare)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            bound: VERSION_TYPE_BOUND,
        }],
        cells: Cells::Board(&[
            ("version_eq", Class::Linear),
            ("version_hash", Class::Linear),
        ]),
    },
    Claim {
        op: "Party Eq / Hash (canonical byte compare)",
        checks: &[Check {
            site: Site::TypeDoc("src/party.rs", "Party"),
            bound: Bound::Custom {
                line: "`==` and hashing `O(n)`; every other cost is on its operation.",
                reason: "one type-doc section prices the derived byte-compare surface",
            },
        }],
        cells: Cells::Board(&[("party_hash", Class::Linear)]),
    },
    Claim {
        op: "Clock | Version and Version | Clock (heterogeneous joins, |=)",
        checks: &[Check {
            site: Site::TypeDoc("src/clock.rs", "Clock"),
            bound: Bound::Custom {
                line: "the heterogeneous joins `O(a + b)`; `==` and hashing `O(n)`.",
                reason: "one type-doc section prices the operator matrix and the \
                         byte-compare surface together",
            },
        }],
        cells: Cells::Board(&[
            ("clock_recv", Class::Linear),
            ("clock_hash", Class::Linear),
        ]),
    },
    Claim {
        op: "From<Party> for [Party; N] (consuming balanced split)",
        checks: &[Check {
            site: Site::ImplDoc("src/party/forks.rs", "From<Party> for [Party; N]"),
            bound: PARTY_SPLIT_BOUND,
        }],
        cells: Cells::Uncelled(
            "the forks machinery consuming its operand; no board row of its own \
             (the board's coverage table)",
        ),
    },
    Claim {
        op: "From<Clock> for [Clock; N] (consuming balanced split)",
        checks: &[Check {
            site: Site::ImplDoc("src/clock/forks.rs", "From<Clock> for [Clock; N]"),
            bound: CLOCK_SPLIT_BOUND,
        }],
        cells: Cells::Uncelled(
            "the clock forks machinery consuming its operand; no board row of its \
             own (the board's coverage table)",
        ),
    },
    Claim {
        op: "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        checks: &[
            Check {
                site: Site::TypeDoc("src/party/forks.rs", "Forks"),
                bound: PARTY_FORKS_TYPE_BOUND,
            },
            Check {
                site: Site::TypeDoc("src/clock/forks.rs", "Forks"),
                bound: CLOCK_SPLIT_BOUND,
            },
        ],
        cells: Cells::Uncelled(
            "iterates the measured fork on shrinking operands; no board row of \
             its own (the board's coverage table)",
        ),
    },
    Claim {
        op: "Party Display / FromStr / TryFrom literals",
        checks: &[
            Check {
                site: Site::ImplDoc("src/party.rs", "impl core::fmt::Display for Party"),
                bound: Bound::TextRender,
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "impl core::str::FromStr for Party"),
                bound: Bound::TextParse,
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "impl TryFrom<u8> for Party"),
                bound: Bound::Constant,
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "impl TryFrom<bool> for Party"),
                bound: Bound::Constant,
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "TryFrom<(T, S)> for Party"),
                bound: Bound::Linear,
            },
        ],
        cells: Cells::Board(&[
            ("party_display", Class::LinearIo),
            ("party_from_str", Class::LinearIo),
            ("party_parse_trailing", Class::LinearIo),
            ("party_parse_noncanon", Class::LinearIo),
        ]),
    },
    Claim {
        op: "Version Display / FromStr / TryFrom literals",
        checks: &[
            Check {
                site: Site::ImplDoc("src/version.rs", "impl core::fmt::Display for Version"),
                bound: Bound::Custom {
                    line: "`O(n + t)` space; time superlinear in the spelled value widths \
                           (decimal conversion plus the render merge).",
                    reason: "the honest time class is superlinear; no linear template may \
                             carry it",
                },
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl core::str::FromStr for Version"),
                bound: Bound::Custom {
                    line: "`O(t + n)` space; time superlinear in the spelled value widths \
                           (decimal-to-binary conversion).",
                    reason: "the honest time class is superlinear; no linear template may \
                             carry it",
                },
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl TryFrom<u64> for Version"),
                bound: Bound::Constant,
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "TryFrom<(u64, T, S)> for Version"),
                bound: Bound::Linear,
            },
        ],
        cells: Cells::Board(&[
            ("version_display", Class::SuperlinearTime),
            ("version_from_str", Class::LinearIo),
            ("version_parse_trailing", Class::LinearIo),
            ("version_parse_noncanon", Class::LinearIo),
        ]),
    },
    Claim {
        op: "Clock Display / FromStr / TryFrom",
        checks: &[
            Check {
                site: Site::ImplDoc("src/clock.rs", "impl core::fmt::Display for Clock"),
                bound: Bound::Custom {
                    line: "`O(n + t)` space; time superlinear on the version side (as \
                           `Version`'s `Display`).",
                    reason: "the honest time class is superlinear; no linear template may \
                             carry it",
                },
            },
            Check {
                site: Site::ImplDoc("src/clock.rs", "impl core::str::FromStr for Clock"),
                bound: Bound::Custom {
                    line: "`O(t + n)` space; time superlinear in the spelled value widths \
                           (decimal-to-binary conversion).",
                    reason: "the honest time class is superlinear; no linear template may \
                             carry it",
                },
            },
            Check {
                site: Site::ImplDoc("src/clock.rs", "TryFrom<(I, E)> for Clock"),
                bound: Bound::Linear,
            },
        ],
        cells: Cells::Board(&[
            ("clock_display", Class::SuperlinearTime),
            ("clock_from_str", Class::LinearIo),
            ("clock_parse_trailing", Class::LinearIo),
        ]),
    },
    Claim {
        op: "serde / borsh impls (feature-gated, strict-decode pinned)",
        checks: &[],
        cells: Cells::Board(&[
            ("version_encode", Class::Linear),
            ("version_decode", Class::Linear),
            ("party_encode", Class::Linear),
            ("party_decode", Class::Linear),
            ("clock_encode", Class::Linear),
            ("clock_decode", Class::Linear),
            ("rank_encode", Class::Linear),
            ("rank_decode", Class::Linear),
            ("ranked_encode", Class::Linear),
            ("ranked_decode", Class::Linear),
            ("span_encode", Class::Linear),
            ("span_decode", Class::Linear),
        ]),
    },
    Claim {
        op: "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display",
        checks: &[Check {
            site: Site::TypeDoc("src/version/rank.rs", "Rank"),
            bound: Bound::Custom {
                line: "comparison and addition `O(‖a‖ + ‖b‖)`, `Sum` `O(N)`; `Display` \
                       superlinear in the numerator width (decimal conversion).",
                reason: "Rank's arithmetic operands are in-memory values with no packed \
                         operand form (the canonical wire form is priced on its own \
                         encode/decode rows); one type-doc section prices the \
                         value-content-denominated surface",
            },
        }],
        cells: Cells::Board(&[
            ("rank_pair_ops", Class::Linear),
            ("rank_sum", Class::Linear),
        ]),
    },
    Claim {
        op: "Ticks ZERO / From / FromStr / Display / Add / Sum / Ord / Eq / Hash",
        checks: &[Check {
            site: Site::TypeDoc("src/version/ticks.rs", "Ticks"),
            bound: Bound::Custom {
                line: "construction `O(1)`; comparison and hashing `O(‖n‖)`; addition \
                       `O(‖a‖ + ‖b‖)`, `Sum` `O(N)`; text superlinear in the count's \
                       width (decimal conversion).",
                reason: "Ticks has no packed encoding; one type-doc section prices its \
                         value-content-denominated surface",
            },
        }],
        cells: Cells::Uncelled(
            "an opaque count carrier: construction and arithmetic are \
             word-to-width-scale with no packed-input axis; the operations \
             denominated in it are celled at their own rows (version_ticks, \
             version_min_ticks)",
        ),
    },
    Claim {
        op: "Ranked comparisons and the Ranked / Rank From conversions (the total order)",
        checks: &[Check {
            site: Site::TypeDoc("src/version/ranked.rs", "Ranked"),
            bound: Bound::MulBoundPair,
        }],
        // The fused signed co-sweep is distance/lag's integrator with
        // constant orientation (the rank-tie byte tiebreak is one
        // comparison inside the same linear space bound), so the pair
        // claim and its witnesses are theirs; the ranked_cmp row
        // drives it whole-surface.
        cells: Cells::Board(&[("ranked_cmp", Class::MulBound)]),
    },
];

/// Scan every coverage surface file for doc blocks and their
/// `# Complexity` sections.
///
/// The shared scanner over [`surface_coverage::SURFACE_SOURCES`], with
/// the same rustfmt-normalized line discipline as the surface extractor
/// (see [`complexity_claims::doc_index`]'s docs).
pub(crate) fn doc_index() -> DocIndex {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ::complexity_claims::doc_index(&root, surface_coverage::SURFACE_SOURCES)
}
