//! The complexity-claims roster: every public operation's documented
//! asymptotic class, bound to the amplification board's verdicts.
//!
//! The public rustdoc states each operation's cost in a uniform
//! `# Complexity` section whose lead is a Big-O token over user-held
//! denominators (packed sizes, text bytes, result sizes). Prose cannot be
//! checked; tokens can — so the roster here pins, per operation, the exact
//! token strings its section must carry and the board rows whose verdicts
//! witness the claimed class, and the tests hold three legs together:
//!
//! - **Prose ↔ roster** ([`doc_index`]): a source scan over the same files
//!   the triangle suite pins ([`triangle::SURFACE_SOURCES`]) locates each
//!   operation's `# Complexity` section — on the `pub fn`, the type doc,
//!   the module doc, or a documented trait impl, as the roster's
//!   [`Site`] records — and requires the pinned tokens verbatim. Editing a
//!   section's class without this roster is a named failure; the sentences
//!   after the tokens are explanation, uniformly non-normative.
//! - **Roster ↔ board** : every cited board row must exist in the board's
//!   own operation axis ([`board::bench_cells`]), and the set of rows
//!   claimed superlinear-in-time must equal the bench judge's committed
//!   red set (`tools/benchjudge-expected.json`, itself membership-pinned
//!   by `tests/bench_judge_roster.rs`) — both directions, so curing a red
//!   or flipping a class reaches the rustdoc through a failing name here.
//! - **Roster ↔ mechanism-tagged reds** (the class-binding seal): no
//!   linear-class claim (`Linear`, `LinearIo`, `FoldLog`) may cite an
//!   operation with a standing exponent-mechanism red in
//!   [`board::BOARD_EXPECTED_REDS`], and every [`Class::SuperlinearCounter`]
//!   claim must keep one — the judge's red set binds wall time only, and
//!   this leg binds the deterministic counters' verdicts, so a
//!   counter-superlinear kernel whose wall constant hides under the
//!   judge's resolution can no longer keep a linear claim.
//! - **Class liveness**: every non-linear class keeps a deterministic
//!   growth pin proving the documented behavior still exists — the
//!   render merge's superlinear limb growth on the wide left-full shape
//!   and the n-ary fold's log factor on the scatter population live in
//!   this suite. A cure landing flips the pin red, forcing roster and
//!   rustdoc to move in the same change.
//!
//! Totality rides the triangle surface: every name in
//! [`triangle::extract_public_fns`] and [`triangle::FAMILY_SURFACE`] has
//! exactly one claim row (or a place in [`NON_OPERATIONS`], the family
//! rows that are dispositions rather than operations), so a new public
//! operation fails this roster until its documented class is pinned.

use std::collections::BTreeMap;
use std::fs;

use super::triangle;

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
    /// The members: the display pair (value conversion plus the render
    /// merge).
    SuperlinearTime,
    /// Superlinear in the deterministic work counters on committed
    /// board families, while absent from the bench judge's red roster.
    ///
    /// A standing exponent-mechanism red in
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
}

/// Where an operation's `# Complexity` section lives.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Site {
    /// The doc block of the `pub fn` the triangle extractor names; the
    /// operation's own name locates it.
    Fn,
    /// The doc block of a `pub struct` — `(file, local type name)`.
    TypeDoc(&'static str, &'static str),
    /// The module doc (`//!`) of the named file.
    ModuleDoc(&'static str),
    /// The doc block of a trait/operator impl — `(file, a substring of
    /// the impl header line)`.
    ImplDoc(&'static str, &'static str),
}

/// One prose check: a site that must carry a `# Complexity` section
/// containing every listed token verbatim.
pub(crate) struct Check {
    pub(crate) site: Site,
    pub(crate) tokens: &'static [&'static str],
}

/// The board leg of one claim: the rows witnessing the class, or the
/// reason none exists (mirroring the board module doc's coverage list).
pub(crate) enum Cells {
    /// `(board operation name, the class its verdict witnesses)`.
    Board(&'static [(&'static str, Class)]),
    /// No board row of its own, with the reason.
    Uncelled(&'static str),
}

/// One public operation's pinned complexity claim.
pub(crate) struct Claim {
    /// The operation, named exactly as the triangle surface names it.
    pub(crate) op: &'static str,
    pub(crate) checks: &'static [Check],
    pub(crate) cells: Cells,
}

/// Triangle family rows that are coverage dispositions, not operations:
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
            tokens: &["`O(1)`"],
        }],
        cells: Cells::Uncelled("word-scale: no input axis to measure against"),
    }
}

/// Shorthand for a causally-module operation: the module doc carries one
/// note for all of them.
const fn causally(op: &'static str, cells: Cells) -> Claim {
    Claim {
        op,
        checks: &[Check {
            site: Site::ModuleDoc("src/causally.rs"),
            tokens: &["`O(1)`", "`O(|a| + |b|)`"],
        }],
        cells,
    }
}

/// The reason the causally constructors cite no board row.
const CAUSALLY_CONSTRUCTOR: &str =
    "stores two borrows; the comparison cost is on the membership predicates \
     (the causally_contains row prices them)";

/// The reason the causally compositions (a start paired with an end) cite
/// no board row.
const CAUSALLY_COMPOSITION: &str =
    "stores two borrows plus at most one validating causal comparison, the \
     identical comparison the causally_contains row prices";

/// The claims roster of record. One row per public operation, named as
/// the triangle surface names it; the tests hold it total, its sites
/// carrying the pinned tokens, and its cited cells alive on the board.
pub(crate) const CLAIMS: &[Claim] = &[
    // ───────────────────────────── Party ─────────────────────────────
    constant("Party::seed"),
    constant("Party::is_seed"),
    Claim {
        op: "Party::tick",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|v| + |p|)`"],
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
            tokens: &["`O(|v| + |p| + log n)`"],
        }],
        cells: Cells::Board(&[("version_ticks", Class::Linear)]),
    },
    Claim {
        op: "Party::fork",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|p|)`"],
        }],
        cells: Cells::Board(&[("party_fork", Class::Linear)]),
    },
    Claim {
        op: "Party::forks",
        checks: &[
            Check {
                site: Site::Fn,
                tokens: &["`O(S)`"],
            },
            Check {
                site: Site::TypeDoc("src/party/forks.rs", "Forks"),
                tokens: &["`O(S)`", "`O(log n)`"],
            },
        ],
        cells: Cells::Uncelled(
            "iterates the measured fork on shrinking operands; no board row of \
             its own (the board module doc's coverage list)",
        ),
    },
    Claim {
        op: "Party::join",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|a| + |b|)`"],
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
            tokens: &["`O(D log k + B log |p|)`", "`O(D)`"],
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
            tokens: &["`O(|a| + |b|)`"],
        }],
        cells: Cells::Board(&[("party_disjoint", Class::Linear)]),
    },
    Claim {
        op: "Party::covers",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|a| + |b|)`"],
        }],
        cells: Cells::Board(&[("party_covers", Class::Linear)]),
    },
    Claim {
        op: "Party::without",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|a| + |b|)`"],
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
            tokens: &["`O(|p|)`"],
        }],
        cells: Cells::Uncelled("one byte copy (the board module doc's coverage list)"),
    },
    Claim {
        op: "Party::encode",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|p|)`"],
        }],
        cells: Cells::Board(&[("party_encode", Class::Linear)]),
    },
    Claim {
        op: "Party::encode_to",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|p|)`"],
        }],
        cells: Cells::Board(&[("party_encode", Class::Linear)]),
    },
    constant("Party::encoded_bits"),
    Claim {
        op: "Party::decode",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(n)`"],
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
            tokens: &["`O(|v| + |p|)`"],
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
            tokens: &["`O(|v| + |p| + log n)`"],
        }],
        cells: Cells::Board(&[("version_ticks", Class::Linear)]),
    },
    Claim {
        op: "Version::concurrent",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|a| + |b|)`"],
        }],
        cells: Cells::Board(&[("version_concurrent", Class::Linear)]),
    },
    Claim {
        op: "Version::min_ticks",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|v|)`"],
        }],
        cells: Cells::Board(&[("version_min_ticks", Class::Linear)]),
    },
    Claim {
        op: "Version::rank",
        checks: &[Check {
            site: Site::Fn,
            tokens: &[
                "`O(|v|)` space",
                "`Θ(|v|²)`",
                "`Ω(M(|v|))`",
                "`O(|v| log |v|)`",
            ],
        }],
        // The three-part time claim, each part with a committed
        // witness: Θ(|v|²) worst case is held tight from below by two
        // red pins in tests/meter.rs — `ledger_wide_arming_pin` (the
        // ledger settle's aggregate product: one arming as wide as
        // the input ahead of a suffix as dense) and
        // `answer_embedded_product` (the close-time settle with zero
        // armings: the plateau-puncture rank IS a wide × dense
        // product) — and from above by the `quadratic_ceiling`
        // probes, whose module doc derives the O(|v|²) ceiling over
        // every committed superlinear family (each settle product is
        // charged to the widest arming of its left half; window-sum
        // density is subadditive over disjoint entry ranges; every
        // other charge is O(|v| log |v|)). Ω(M(|v|)) is mandatory
        // because the plateau-puncture answer embeds the product; the
        // O(|v| log |v|) leg is the dense-suffix/promo-rearm flatness
        // bands' reading on every O(1)-wide-parked family. Neither
        // pin family is a board family — the board reads every
        // committed rank row green, so the class-binding seal
        // (SuperlinearCounter needs a standing exponent-mechanism red
        // in BOARD_EXPECTED_REDS) keeps this row Linear-classed with
        // the rustdoc stating the worst case. FoldLog would be wrong
        // in the other direction: the settle's log factor is real but
        // never dominates a committed reading, and "linear × log" is
        // not an upper bound while the pins stand. A settle reaching
        // the multiplication bound flips the wide-arming pin and
        // revisits this class; no cure flips the plateau-puncture pin
        // below M.
        cells: Cells::Board(&[("version_rank", Class::Linear)]),
    },
    Claim {
        op: "Version::distance",
        checks: &[Check {
            site: Site::Fn,
            tokens: &[
                "`O(|a| + |b|)` space",
                "`Θ((|a| + |b|)²)`",
                "`Ω(M(|a| + |b|))`",
                "`O((|a| + |b|) log (|a| + |b|))`",
            ],
        }],
        // The pair form of rank's three-part claim — one shared
        // integrator, so the same witnesses, ceiling derivation
        // (`quadratic_ceiling`, including its pair probe), and class
        // reasoning as Version::rank above.
        cells: Cells::Board(&[("version_distance", Class::Linear)]),
    },
    Claim {
        op: "Version::lag",
        checks: &[Check {
            site: Site::Fn,
            tokens: &[
                "`O(|a| + |b|)` space",
                "`Θ((|a| + |b|)²)`",
                "`Ω(M(|a| + |b|))`",
                "`O((|a| + |b|) log (|a| + |b|))`",
            ],
        }],
        // As Version::distance above (one shared co-sweep).
        cells: Cells::Board(&[("version_lag", Class::Linear)]),
    },
    Claim {
        op: "Version::join_all",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(D log k)`", "`O(D)`"],
        }],
        cells: Cells::Board(&[("version_join_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version::meet_all",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(D log k)`", "`O(D)`"],
        }],
        // The join fold's balanced reduction over the meet emitter; the
        // non-shrinking-accumulator worst case is held flat by the
        // `meet_fold` band and its sequential-reduce tripwire
        // (`tests/meter.rs`) on the meet-shade population.
        cells: Cells::Board(&[("version_meet_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version::encode",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|v|)`"],
        }],
        cells: Cells::Board(&[("version_encode", Class::Linear)]),
    },
    Claim {
        op: "Version::encode_to",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|v|)`"],
        }],
        cells: Cells::Board(&[("version_encode", Class::Linear)]),
    },
    Claim {
        op: "Version::decode",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(n)`"],
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
            tokens: &["`O(|c|)`"],
        }],
        cells: Cells::Board(&[("clock_tick", Class::Linear)]),
    },
    Claim {
        op: "Clock::ticks",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|c| + log n)`"],
        }],
        cells: Cells::Board(&[("version_ticks", Class::Linear)]),
    },
    Claim {
        op: "Clock::fork",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|c|)`"],
        }],
        cells: Cells::Board(&[("clock_fork", Class::Linear)]),
    },
    Claim {
        op: "Clock::forks",
        checks: &[
            Check {
                site: Site::Fn,
                tokens: &["`O(S + n·|v|)`"],
            },
            Check {
                site: Site::TypeDoc("src/clock/forks.rs", "Forks"),
                tokens: &["`O(S + n·|v|)`"],
            },
        ],
        cells: Cells::Uncelled(
            "iterates the measured fork on shrinking operands, one version \
             clone per child; no board row of its own (the board module doc's \
             coverage list)",
        ),
    },
    Claim {
        op: "Clock::join",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|a| + |b|)`"],
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
            tokens: &["`O(D log k + B log |c|)`", "`O(D)`"],
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
            tokens: &["`O(|a| + |b|)`"],
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
            tokens: &["`O(|c|)`"],
        }],
        cells: Cells::Board(&[("clock_tick", Class::Linear)]),
    },
    Claim {
        op: "Clock::recv",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|c| + |v|)`"],
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
            tokens: &["`O(1)`"],
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
            tokens: &["`O(|v| + |p| + |r|)`"],
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
            tokens: &["`O(|c|)`"],
        }],
        cells: Cells::Board(&[("clock_encode", Class::Linear)]),
    },
    Claim {
        op: "Clock::encode_to",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(|c|)`"],
        }],
        cells: Cells::Board(&[("clock_encode", Class::Linear)]),
    },
    Claim {
        op: "Clock::decode",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(n)`"],
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
            tokens: &["`O(|c|)`"],
        }],
        cells: Cells::Uncelled("one byte copy per part (the board module doc's coverage list)"),
    },
    // ───────────────────────────── Rank / Ranked ─────────────────────────────
    Claim {
        op: "Rank::checked_sub",
        checks: &[Check {
            site: Site::Fn,
            tokens: &["`O(‖a‖ + ‖b‖)`"],
        }],
        cells: Cells::Board(&[("rank_pair_ops", Class::Linear)]),
    },
    constant("Ranked::version"),
    constant("Ranked::rank"),
    constant("Ranked::into_parts"),
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
        Cells::Board(&[("causally_contains", Class::Linear)]),
    ),
    // ─────────────────────── operator/trait families ───────────────────────
    Claim {
        op: "Version | Version (BitOr/BitOrAssign, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            tokens: &["`O(|a| + |b|)`"],
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
            tokens: &["`O(|a| + |b|)`"],
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
            tokens: &["`O(1)`"],
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
            tokens: &["`O(|v| + |p| + |w|)`"],
        }],
        cells: Cells::Board(&[("own_version_cmp", Class::Linear)]),
    },
    Claim {
        op: "OwnVersion vs OwnVersion comparisons (the four-stream co-walk, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version/own.rs", "OwnVersion"),
            tokens: &["`O(|v₁| + |p₁| + |v₂| + |p₂|)`"],
        }],
        cells: Cells::Board(&[("own_version_pair_cmp", Class::Linear)]),
    },
    Claim {
        op: "From<OwnVersion> for Version (explicit materialization)",
        checks: &[Check {
            site: Site::ImplDoc("src/version/own.rs", "From<OwnVersion<'_>> for Version"),
            tokens: &["`O(|v| + |p| + |r|)`"],
        }],
        cells: Cells::Board(&[("own_version_to_version", Class::LinearIo)]),
    },
    Claim {
        op: "Version PartialOrd (the comparison matrix, owned and borrowed)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            tokens: &["`O(|a| + |b|)`"],
        }],
        cells: Cells::Board(&[("version_cmp", Class::Linear)]),
    },
    Claim {
        op: "Version Sum / FromIterator (owned and borrowed)",
        checks: &[
            Check {
                site: Site::ImplDoc("src/version.rs", "impl Sum<Version> for Version"),
                tokens: &["`O(D log k)`", "`O(D)`"],
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl<'a> Sum<&'a Version> for Version"),
                tokens: &["`O(D log k)`", "`O(D)`"],
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl FromIterator<Version> for Version"),
                tokens: &["`O(D log k)`", "`O(D)`"],
            },
            Check {
                site: Site::ImplDoc(
                    "src/version.rs",
                    "impl<'a> FromIterator<&'a Version> for Version",
                ),
                tokens: &["`O(D log k)`", "`O(D)`"],
            },
        ],
        cells: Cells::Board(&[("version_join_all", Class::FoldLog)]),
    },
    Claim {
        op: "Version Eq / Hash (canonical byte compare)",
        checks: &[Check {
            site: Site::TypeDoc("src/version.rs", "Version"),
            tokens: &["`O(|v|)`"],
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
            tokens: &["`O(|p|)`"],
        }],
        cells: Cells::Board(&[("party_hash", Class::Linear)]),
    },
    Claim {
        op: "Clock | Version and Version | Clock (heterogeneous joins, |=)",
        checks: &[Check {
            site: Site::TypeDoc("src/clock.rs", "Clock"),
            tokens: &["`O(|c| + |v|)`"],
        }],
        cells: Cells::Board(&[("clock_recv", Class::Linear)]),
    },
    Claim {
        op: "From<Party> for [Party; N] (consuming balanced split)",
        checks: &[Check {
            site: Site::ImplDoc("src/party/forks.rs", "From<Party> for [Party; N]"),
            tokens: &["`O(S)`"],
        }],
        cells: Cells::Uncelled(
            "the forks machinery consuming its operand; no board row of its own \
             (the board module doc's coverage list)",
        ),
    },
    Claim {
        op: "iter::Party / iter::Clock (Forks iterators, drop folds back)",
        checks: &[
            Check {
                site: Site::TypeDoc("src/party/forks.rs", "Forks"),
                tokens: &["`O(S)`", "`O(log n)`"],
            },
            Check {
                site: Site::TypeDoc("src/clock/forks.rs", "Forks"),
                tokens: &["`O(S + n·|v|)`"],
            },
        ],
        cells: Cells::Uncelled(
            "iterates the measured fork on shrinking operands; no board row of \
             its own (the board module doc's coverage list)",
        ),
    },
    Claim {
        op: "Party Display / FromStr / TryFrom literals",
        checks: &[
            Check {
                site: Site::ImplDoc("src/party.rs", "impl core::fmt::Display for Party"),
                tokens: &["`O(|p| + t)`"],
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "impl core::str::FromStr for Party"),
                tokens: &["`O(t + |p|)`"],
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "impl TryFrom<u8> for Party"),
                tokens: &["`O(1)`"],
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "impl TryFrom<bool> for Party"),
                tokens: &["`O(1)`"],
            },
            Check {
                site: Site::ImplDoc("src/party.rs", "TryFrom<(T, S)> for Party"),
                tokens: &["`O(|p|)`"],
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
                tokens: &["`O(|v| + t)`", "superlinear"],
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl core::str::FromStr for Version"),
                tokens: &["`O(t + |v|)`", "superlinear"],
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "impl TryFrom<u64> for Version"),
                tokens: &["`O(1)`"],
            },
            Check {
                site: Site::ImplDoc("src/version.rs", "TryFrom<(u64, T, S)> for Version"),
                tokens: &["`O(|v|)`"],
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
                tokens: &["`O(|c| + t)`", "superlinear"],
            },
            Check {
                site: Site::ImplDoc("src/clock.rs", "impl core::str::FromStr for Clock"),
                tokens: &["`O(t + |c|)`", "superlinear"],
            },
            Check {
                site: Site::ImplDoc("src/clock.rs", "TryFrom<(I, E)> for Clock"),
                tokens: &["`O(|c|)`"],
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
        ]),
    },
    Claim {
        op: "Rank ZERO / Add / AddAssign / Sum / Ord / Eq / Hash / Display",
        checks: &[Check {
            site: Site::TypeDoc("src/version/rank.rs", "Rank"),
            tokens: &["`O(‖a‖ + ‖b‖)`", "`O(N)`", "superlinear"],
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
            tokens: &[
                "`O(1)`",
                "`O(‖n‖)`",
                "`O(‖a‖ + ‖b‖)`",
                "`O(N)`",
                "superlinear",
            ],
        }],
        cells: Cells::Uncelled(
            "an opaque count carrier: construction and arithmetic are \
             word-to-width-scale with no packed-input axis; the operations \
             denominated in it are celled at their own rows (version_ticks, \
             version_min_ticks)",
        ),
    },
    Claim {
        op: "Ranked Ord / From<Version> (byte tiebreak)",
        checks: &[Check {
            site: Site::TypeDoc("src/version/ranked.rs", "Ranked"),
            tokens: &["`O(|v|)`", "`O(1)`"],
        }],
        cells: Cells::Board(&[("version_rank", Class::Linear)]),
    },
];

/// The `# Complexity` sections the surface files carry, scanned from
/// source with the triangle extractor's line discipline.
pub(crate) struct DocIndex {
    /// `Type::fn` / `module::fn` name → its doc block's Complexity
    /// section, if the block has one.
    pub(crate) fns: BTreeMap<String, Option<String>>,
    /// `(file, local type name)` → the `pub struct`'s section.
    pub(crate) structs: BTreeMap<(String, String), Option<String>>,
    /// `(file, impl header line)` → the impl's section, for every
    /// documented column-0 impl.
    pub(crate) impls: Vec<(String, String, Option<String>)>,
    /// file → the module doc's section.
    pub(crate) modules: BTreeMap<String, Option<String>>,
}

impl DocIndex {
    /// The section at `site`, or an error naming what is missing.
    pub(crate) fn section(&self, op: &str, site: Site) -> Result<&str, String> {
        let found = match site {
            Site::Fn => self
                .fns
                .get(op)
                .ok_or_else(|| format!("{op}: no `pub fn` doc block found by the scanner"))?,
            Site::TypeDoc(file, ty) => self
                .structs
                .get(&(file.to_owned(), ty.to_owned()))
                .ok_or_else(|| format!("{op}: no `pub struct {ty}` found in {file}"))?,
            Site::ModuleDoc(file) => self
                .modules
                .get(file)
                .ok_or_else(|| format!("{op}: no module doc found in {file}"))?,
            Site::ImplDoc(file, header) => {
                let mut matches = self
                    .impls
                    .iter()
                    .filter(|(f, h, _)| f == file && h.contains(header));
                let (_, _, section) = matches.next().ok_or_else(|| {
                    format!("{op}: no impl header containing `{header}` in {file}")
                })?;
                if matches.next().is_some() {
                    return Err(format!(
                        "{op}: impl header substring `{header}` is ambiguous in {file}"
                    ));
                }
                section
            }
        };
        found.as_deref().ok_or_else(|| {
            format!("{op}: the doc block at its roster site has no `# Complexity` section")
        })
    }
}

/// Scan every triangle surface file for doc blocks and their
/// `# Complexity` sections.
///
/// The same rustfmt-normalized line discipline as
/// [`triangle::extract_public_fns`]: column-0 `impl` headers open inherent
/// or trait impls, `pub fn` appears at column 0 (module level) or one
/// indent (inherent methods), and a doc block is the contiguous `///` run
/// (attributes transparent) directly above its item.
pub(crate) fn doc_index() -> DocIndex {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut index = DocIndex {
        fns: BTreeMap::new(),
        structs: BTreeMap::new(),
        impls: Vec::new(),
        modules: BTreeMap::new(),
    };
    for spec in triangle::SURFACE_SOURCES {
        let path = root.join(spec.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let mut module_doc = String::new();
        let mut doc = String::new();
        let mut current_type: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("//!") {
                module_doc.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                module_doc.push('\n');
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("///") {
                doc.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                doc.push('\n');
                continue;
            }
            // Attributes and plain comments sit between a doc block and
            // its item without detaching it (rustc ignores both), so the
            // scan treats them as transparent.
            if trimmed.starts_with("#[") || trimmed.starts_with("#!") || trimmed.starts_with("//") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("impl") {
                if line.contains(" for ") {
                    index
                        .impls
                        .push((spec.path.to_owned(), line.to_owned(), section_of(&doc)));
                    current_type = None;
                } else {
                    current_type = triangle::parse_impl_self_type(rest);
                }
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("    pub fn ") {
                if let Some(ty) = current_type.as_deref() {
                    let ty = spec.type_override.unwrap_or(ty);
                    let name = format!("{ty}::{}", triangle::fn_name(rest));
                    index.fns.insert(name, section_of(&doc));
                }
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub fn ") {
                if let Some(prefix) = spec.module_prefix {
                    let name = format!("{prefix}::{}", triangle::fn_name(rest));
                    index.fns.insert(name, section_of(&doc));
                }
                doc.clear();
                continue;
            }
            if let Some(rest) = line.strip_prefix("pub struct ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                index
                    .structs
                    .insert((spec.path.to_owned(), name), section_of(&doc));
                doc.clear();
                continue;
            }
            if line == "}" {
                current_type = None;
            }
            doc.clear();
        }
        index
            .modules
            .insert(spec.path.to_owned(), section_of(&module_doc));
    }
    index
}

/// The `# Complexity` section of one doc block: the lines from its
/// heading to the next heading or example fence. [`None`] when the block
/// has no such section.
fn section_of(doc: &str) -> Option<String> {
    let mut lines = doc.lines();
    lines.by_ref().find(|l| l.trim() == "# Complexity")?;
    let section: Vec<&str> = lines
        .take_while(|l| !l.trim_start().starts_with("# ") && !l.trim_start().starts_with("```"))
        .collect();
    Some(section.join("\n"))
}
