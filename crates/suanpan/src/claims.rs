//! The complexity-claims roster: every public operation's documented
//! cost, bound to committed instruments.
//!
//! The rustdoc states each operation's cost in a uniform `# Complexity`
//! section. Prose cannot be checked; a rendered line can — so the roster
//! here pins, per operation, a structured [`Bound`] whose rendering must
//! appear **verbatim as the opening line** of the section at each
//! recorded [`Site`], plus the committed test evidence behind the claim,
//! and the binding tests (`claims/tests.rs`) hold the legs together:
//!
//! - **Prose ↔ roster**: the shared doc scanner locates every section and
//!   byte-compares its first line against the roster's rendered claim
//!   line; the prose after the opening line is explanation, uniformly
//!   non-normative.
//! - **Table ↔ roster**: the crate page's operations table is scanned
//!   row by row; every row's cost cell must byte-equal the
//!   [`Claim::table_cost`] of each operation the row names, and every
//!   row must be named by some claim — the table was twice found wrong
//!   in review before this binding existed, so it is held to the roster
//!   like any other claim site.
//! - **Claim ↔ evidence**: every claim either names committed `#[test]`
//!   witnesses (checked to exist, by name, in their files — suanpan's own
//!   touch-metered pins, plus the `accum_streams` digit-touch bands that
//!   live beside the consumer in `before/tests/meter.rs`) or carries a
//!   mechanism-based exclusion reason.
//! - **Totality**: the extracted `pub fn` surface plus the family rows
//!   ([`FAMILY_SURFACE`]) equals the roster's op set exactly, both
//!   directions, so a new public operation fails here until its
//!   documented cost is pinned.
//!
//! # The rendered lines' vocabulary
//!
//! Costs are denominated in *digit touches* (the crate page's metering
//! section; the `touch-meter` counter is the gauge). *Amortized* bounds
//! hold over the whole operation sequence, per the crate page's table
//! preamble. An *operand limb* is one 64-bit word of a wide operand's
//! value; *held digits* is [`digit_count`](crate::Accumulator::digit_count);
//! the *written span* is defined at the crate page's table footnote.
//! Almost every row is a [`Bound::Custom`]: the amortized digit-touch
//! denomination is this crate's own vocabulary, which no shared template
//! carries — each custom line states that reason as committed data.

use complexity_claims::{Bound, Check, Site, SourceSpec};

#[cfg(test)]
mod tests;

/// The public-API sources of record: every module carrying `pub` items
/// (the crate root re-exports them and declares no `pub fn` of its own).
pub(crate) const SOURCES: &[SourceSpec] = &[
    SourceSpec {
        path: "src/accumulator.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/limbs.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/magnitude.rs",
        module_prefix: None,
        type_overrides: &[],
    },
    SourceSpec {
        path: "src/touch_meter.rs",
        module_prefix: Some("touch_meter"),
        type_overrides: &[],
    },
];

/// The committed backing of one claim: named test evidence, or the
/// mechanism-based reason none is needed.
pub(crate) enum Evidence {
    /// `(manifest-relative file, #[test] fn name)` pairs, each required
    /// to exist as an attributed test in its file.
    Witnessed(&'static [(&'static str, &'static str)]),
    /// No instrument, with the mechanism that makes one meaningless
    /// (word-scale work, or work outside the digit-touch denomination).
    Excluded(&'static str),
}

/// One public operation's pinned complexity claim.
pub(crate) struct Claim {
    /// The operation, named exactly as the surface extractor names it
    /// (or a [`FAMILY_SURFACE`] row).
    pub(crate) op: &'static str,
    /// The `# Complexity` sections whose opening lines this claim pins.
    pub(crate) checks: &'static [Check],
    /// The crate-page table row's cost cell, verbatim, when the
    /// operation has a table row.
    pub(crate) table_cost: Option<&'static str>,
    /// The committed instruments behind the claim.
    pub(crate) evidence: Evidence,
}

/// Trait/derive/re-export surface the `pub fn` extractor cannot reach,
/// rostered by family; totality of this list is by review of this file
/// against the crate's `pub` items.
pub(crate) const FAMILY_SURFACE: &[&str] = &[
    "Limbs iteration (Iterator / DoubleEndedIterator)",
    "Magnitude (the caller-implemented width seam)",
    "Magnitude for UBig (the word-fit dispatch)",
    "Accumulator Clone / Debug / Default (derived surface)",
    "UBig (re-export)",
];

/// The reason nearly every row renders through [`Bound::Custom`]: the
/// crate's cost vocabulary is its own denomination.
const DIGIT_DENOMINATED: &str =
    "the amortized digit-touch denomination is this crate's own vocabulary; no shared \
     template carries it";

/// Suanpan's own touch-metered pins.
const OWN: &str = "src/accumulator/tests.rs";

/// The digit-touch stream bands committed beside the consumer.
const BANDS: &str = "../before/tests/meter.rs";

/// The machine-word delta rows.
const WORD: Bound = Bound::Custom {
    line: "Amortized `O(1)` digit touches.",
    reason: DIGIT_DENOMINATED,
};

/// The unshifted wide rows.
const WIDE: Bound = Bound::Custom {
    line: "Amortized `O(operand limbs)` digit touches, whatever the held width.",
    reason: DIGIT_DENOMINATED,
};

/// The shifted wide rows.
const WIDE_SHL: Bound = Bound::Custom {
    line: "Amortized `O(operand limbs)` digit touches, independent of the shift; the digit \
           buffer grows to cover the shifted positions.",
    reason: DIGIT_DENOMINATED,
};

/// The shifted machine-word rows.
const WORD_SHL: Bound = Bound::Custom {
    line: "Amortized `O(1)` digit touches, independent of the shift; the digit buffer grows \
           to cover the shifted positions.",
    reason: DIGIT_DENOMINATED,
};

/// The width-dispatching magnitude rows.
const MAGNITUDE: Bound = Bound::Custom {
    line: "Word-scale operands amortized `O(1)` digit touches, wide operands amortized \
           `O(operand limbs)`.",
    reason: DIGIT_DENOMINATED,
};

/// The shifted magnitude rows.
const MAGNITUDE_SHL: Bound = Bound::Custom {
    line: "Word-scale operands amortized `O(1)` digit touches, wide operands amortized \
           `O(operand limbs)`, independent of the shift; the digit buffer grows to cover \
           the shifted positions.",
    reason: DIGIT_DENOMINATED,
};

/// The unshifted accumulator-operand rows.
const ACCUM: Bound = Bound::Custom {
    line: "Amortized `O(the operand's held digits)` digit touches, whatever the receiver's \
           width.",
    reason: DIGIT_DENOMINATED,
};

/// The shifted accumulator-operand rows.
const ACCUM_SHL: Bound = Bound::Custom {
    line: "Amortized `O(the operand's held digits)` digit touches, independent of the \
           shift; the digit buffer grows to cover the shifted positions.",
    reason: DIGIT_DENOMINATED,
};

/// The width-ordered merge.
const MERGE: Bound = Bound::Custom {
    line: "Amortized `O(the narrower operand's held digits)` digit touches, plus an `O(1)` \
           buffer swap.",
    reason: DIGIT_DENOMINATED,
};

/// The sign-query rows (the collapsing fold).
const SIGN: Bound = Bound::Custom {
    line: "Amortized `O(1)` digit touches.",
    reason: DIGIT_DENOMINATED,
};

/// The per-call held-width rows.
const HELD: Bound = Bound::Custom {
    line: "`O(held digits)` digit touches.",
    reason: DIGIT_DENOMINATED,
};

/// The in-place scale: held-width touches, shift-independent.
const SHL: Bound = Bound::Custom {
    line: "`O(held digits)` digit touches, independent of the shift; the digit buffer \
           grows to cover the shifted positions.",
    reason: DIGIT_DENOMINATED,
};

/// The normalized read-out.
const READ_OUT: Bound = Bound::Custom {
    line: "`O(held digits)` digit touches and a same-order magnitude allocation.",
    reason: DIGIT_DENOMINATED,
};

/// The scaled read-out: span-denominated, the row the census corrected.
const SCALED_READ: Bound = Bound::Custom {
    line: "`O(the written span)` digit touches — every digit from the lowest position \
           written since the last reset up to the top, never-written gaps included — and \
           a same-order magnitude allocation.",
    reason: DIGIT_DENOMINATED,
};

/// The `Limbs` type doc's line.
const LIMBS_TYPE: Bound = Bound::Custom {
    line: "Construction and each step `O(1)`.",
    reason: "an iterator's per-step cost fits no whole-operation template",
};

/// The `Accumulator` type doc's derived-surface line.
const ACC_TYPE: Bound = Bound::Custom {
    line: "`Clone` and `Debug` `O(the digit buffer: the highest position ever written)`; \
           `Default` `O(1)`.",
    reason: "the derived surface is buffer-denominated (the buffer never shrinks), which \
             no shared template carries",
};

/// The `Magnitude for UBig` impl doc's line.
const UBIG_IMPL: Bound = Bound::Custom {
    line: "`to_word` and `as_wide` `O(1)`.",
    reason: "two trait methods priced together at their impl doc; no template names a pair",
};

/// Shorthand for a word-scale operation: `O(1)` on its own doc block,
/// excluded from instrumentation with its mechanism.
const fn constant(op: &'static str, reason: &'static str) -> Claim {
    Claim {
        op,
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        table_cost: None,
        evidence: Evidence::Excluded(reason),
    }
}

/// The claims roster of record: one row per public operation, named as
/// the surface extractor (or [`FAMILY_SURFACE`]) names it.
///
/// The binding tests hold it total, its sites' opening lines and table
/// cells byte-equal to the roster, and its cited witnesses alive.
pub(crate) const CLAIMS: &[Claim] = &[
    // ─────────────────────────── construction ───────────────────────────
    constant(
        "Accumulator::new",
        "allocates the one-digit buffer: word-scale, no input axis to measure against",
    ),
    // ─────────────────────── machine-word deltas ────────────────────────
    Claim {
        op: "Accumulator::add_small",
        checks: &[Check {
            site: Site::Fn,
            bound: WORD,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(BANDS, "accum_comb_touches_flat")]),
    },
    Claim {
        op: "Accumulator::sub_small",
        checks: &[Check {
            site: Site::Fn,
            bound: WORD,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(BANDS, "accum_comb_touches_flat")]),
    },
    Claim {
        op: "Accumulator::add_u64",
        checks: &[Check {
            site: Site::Fn,
            bound: WORD,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "u64_comb_touches_are_flat_and_exact")]),
    },
    Claim {
        op: "Accumulator::sub_u64",
        checks: &[Check {
            site: Site::Fn,
            bound: WORD,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "u64_comb_touches_are_flat_and_exact")]),
    },
    // ─────────────────────────── wide deltas ────────────────────────────
    Claim {
        op: "Accumulator::add_wide",
        checks: &[Check {
            site: Site::Fn,
            bound: WIDE,
        }],
        table_cost: Some("amortized O(operand limbs), whatever the held width"),
        evidence: Evidence::Witnessed(&[
            (OWN, "wide_writes_cost_the_operand_at_any_held_width"),
            (BANDS, "accum_wide_tooth_touches_flat"),
            (BANDS, "accum_cancelling_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::sub_wide",
        checks: &[Check {
            site: Site::Fn,
            bound: WIDE,
        }],
        table_cost: Some("amortized O(operand limbs), whatever the held width"),
        evidence: Evidence::Witnessed(&[
            (OWN, "wide_writes_cost_the_operand_at_any_held_width"),
            (BANDS, "accum_wide_tooth_touches_flat"),
            (BANDS, "accum_cancelling_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::add_wide_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: WIDE_SHL,
        }],
        table_cost: Some("amortized O(operand limbs), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    Claim {
        op: "Accumulator::sub_wide_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: WIDE_SHL,
        }],
        table_cost: Some("amortized O(operand limbs), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    Claim {
        op: "Accumulator::add_u64_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: WORD_SHL,
        }],
        table_cost: Some("amortized O(1), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    Claim {
        op: "Accumulator::sub_u64_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: WORD_SHL,
        }],
        table_cost: Some("amortized O(1), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    // ─────────────────────── magnitude dispatches ───────────────────────
    Claim {
        op: "Accumulator::add_magnitude",
        checks: &[Check {
            site: Site::Fn,
            bound: MAGNITUDE,
        }],
        table_cost: Some("word-scale: amortized O(1); wide: amortized O(operand limbs)"),
        evidence: Evidence::Witnessed(&[(OWN, "magnitude_dispatch_costs_its_width_path")]),
    },
    Claim {
        op: "Accumulator::sub_magnitude",
        checks: &[Check {
            site: Site::Fn,
            bound: MAGNITUDE,
        }],
        table_cost: Some("word-scale: amortized O(1); wide: amortized O(operand limbs)"),
        evidence: Evidence::Witnessed(&[(OWN, "magnitude_dispatch_costs_its_width_path")]),
    },
    Claim {
        op: "Accumulator::add_magnitude_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: MAGNITUDE_SHL,
        }],
        table_cost: Some(
            "as [`add_magnitude`](Accumulator::add_magnitude)/\
             [`sub_magnitude`](Accumulator::sub_magnitude), at any shift",
        ),
        evidence: Evidence::Witnessed(&[
            (
                OWN,
                "alternating_shifted_writes_cost_the_operand_not_the_gap",
            ),
            (OWN, "magnitude_dispatch_costs_its_width_path"),
        ]),
    },
    Claim {
        op: "Accumulator::sub_magnitude_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: MAGNITUDE_SHL,
        }],
        table_cost: Some(
            "as [`add_magnitude`](Accumulator::add_magnitude)/\
             [`sub_magnitude`](Accumulator::sub_magnitude), at any shift",
        ),
        evidence: Evidence::Witnessed(&[
            (
                OWN,
                "alternating_shifted_writes_cost_the_operand_not_the_gap",
            ),
            (OWN, "magnitude_dispatch_costs_its_width_path"),
        ]),
    },
    // ─────────────────────── accumulator operands ───────────────────────
    Claim {
        op: "Accumulator::add_accum",
        checks: &[Check {
            site: Site::Fn,
            bound: ACCUM,
        }],
        table_cost: Some("amortized O(operand's held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::sub_accum",
        checks: &[Check {
            site: Site::Fn,
            bound: ACCUM,
        }],
        table_cost: Some("amortized O(operand's held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::add_accum_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: ACCUM_SHL,
        }],
        table_cost: Some("amortized O(operand's held digits), independent of the shift"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::sub_accum_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: ACCUM_SHL,
        }],
        table_cost: Some("amortized O(operand's held digits), independent of the shift"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::merge_into_wider",
        checks: &[Check {
            site: Site::Fn,
            bound: MERGE,
        }],
        table_cost: Some("amortized O(narrower operand's held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    // ───────────────────────── sign queries ─────────────────────────────
    Claim {
        op: "Accumulator::sign",
        checks: &[Check {
            site: Site::Fn,
            bound: SIGN,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[
            (OWN, "no_collapse_fold_re_scans_the_prefix"),
            (OWN, "sign_fold_skips_certified_runs"),
            (BANDS, "accum_static_prefix_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::is_negative",
        checks: &[Check {
            site: Site::Fn,
            bound: SIGN,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[
            (OWN, "no_collapse_fold_re_scans_the_prefix"),
            (BANDS, "accum_static_prefix_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::sign_dominates_word",
        checks: &[Check {
            site: Site::Fn,
            bound: SIGN,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "domination_reads_cost_one_touch_after_the_first")]),
    },
    Claim {
        op: "Accumulator::sign_dominates_at",
        checks: &[Check {
            site: Site::Fn,
            bound: SIGN,
        }],
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "domination_reads_cost_one_touch_after_the_first")]),
    },
    // ─────────────────────────── O(1) probes ────────────────────────────
    Claim {
        op: "Accumulator::is_literally_zero",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        table_cost: Some("O(1)"),
        evidence: Evidence::Excluded(
            "two field reads: no digit is touched, and there is no input axis to measure \
             against",
        ),
    },
    Claim {
        op: "Accumulator::digit_count",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        table_cost: Some("O(1)"),
        evidence: Evidence::Excluded(
            "one field read plus an increment; the exact-top maintenance it rests on is \
             priced on the writes (top_settlement_steps_are_metered pins the settle \
             scan's metering)",
        ),
    },
    // ─────────────────────── held-width operations ──────────────────────
    Claim {
        op: "Accumulator::shl",
        checks: &[Check {
            site: Site::Fn,
            bound: SHL,
        }],
        table_cost: Some("O(held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::negate",
        checks: &[Check {
            site: Site::Fn,
            bound: HELD,
        }],
        table_cost: Some("O(held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::reset",
        checks: &[Check {
            site: Site::Fn,
            bound: HELD,
        }],
        table_cost: Some("O(held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::sign_magnitude",
        checks: &[Check {
            site: Site::Fn,
            bound: READ_OUT,
        }],
        table_cost: Some("O(held digits)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::sign_magnitude_shl",
        checks: &[Check {
            site: Site::Fn,
            bound: SCALED_READ,
        }],
        table_cost: Some("O(the written span since the last reset)"),
        evidence: Evidence::Witnessed(&[
            (OWN, "scaled_read_costs_the_written_span"),
            (OWN, "scaled_read_costs_the_span_not_the_write_count"),
        ]),
    },
    // ──────────────────────────── Limbs ─────────────────────────────────
    Claim {
        op: "Limbs::new",
        checks: &[Check {
            site: Site::Fn,
            bound: Bound::Constant,
        }],
        table_cost: None,
        evidence: Evidence::Excluded(
            "builds a chunk iterator over a borrowed word slice: no digit axis and no \
             allocation",
        ),
    },
    Claim {
        op: "Limbs iteration (Iterator / DoubleEndedIterator)",
        checks: &[Check {
            site: Site::TypeDoc("src/limbs.rs", "Limbs"),
            bound: LIMBS_TYPE,
        }],
        table_cost: None,
        evidence: Evidence::Excluded(
            "each step packs at most two borrowed storage words into one limb: word-scale \
             by construction, outside the digit-touch denomination",
        ),
    },
    // ─────────────────────────── Magnitude ──────────────────────────────
    Claim {
        op: "Magnitude (the caller-implemented width seam)",
        checks: &[],
        table_cost: None,
        evidence: Evidence::Excluded(
            "a trait contract on implementors — to_word must be O(1), the dispatch read \
             the small path's accounting assumes free; the crate prices its own impls, \
             not callers'",
        ),
    },
    Claim {
        op: "Magnitude for UBig (the word-fit dispatch)",
        checks: &[Check {
            site: Site::ImplDoc("src/magnitude.rs", "impl Magnitude for UBig"),
            bound: UBIG_IMPL,
        }],
        table_cost: None,
        evidence: Evidence::Excluded(
            "the word-fit probe reads dashu's stored-word count (at most two words \
             compared): word-scale, outside the digit-touch denomination",
        ),
    },
    // ──────────────────── derived and re-exported ───────────────────────
    Claim {
        op: "Accumulator Clone / Debug / Default (derived surface)",
        checks: &[Check {
            site: Site::TypeDoc("src/accumulator.rs", "Accumulator"),
            bound: ACC_TYPE,
        }],
        table_cost: None,
        evidence: Evidence::Excluded(
            "derived traversals of the digit buffer and ledger: they read digits without \
             the read-modify-write the touch meter denominates, so the buffer-order cost \
             is structural, stated at the type doc",
        ),
    },
    Claim {
        op: "UBig (re-export)",
        checks: &[],
        table_cost: None,
        evidence: Evidence::Excluded(
            "dashu-int's own type, re-exported so callers can name the compiled-against \
             version; its costs are dashu's to state",
        ),
    },
    // ─────────────────────────── touch_meter ────────────────────────────
    constant(
        "touch_meter::touches",
        "one relaxed atomic load: word-scale",
    ),
    constant("touch_meter::reset", "one relaxed atomic store: word-scale"),
];

/// The crate page's operations table, parsed from the module doc: one
/// `(operations cell, cost cell)` pair per data row.
///
/// Reads the raw source (the doc scanner keeps only `# Complexity`
/// sections, and the table lives under its own heading), locates the
/// unique `| Operation | Cost |` header, and takes the contiguous table
/// rows after its separator line.
pub(crate) fn cost_table() -> Vec<(String, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    let doc_lines: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("//!"))
        .map(|l| l.strip_prefix(' ').unwrap_or(l).trim_end())
        .collect();
    let header = doc_lines
        .iter()
        .position(|l| *l == "| Operation | Cost |")
        .expect("the crate page carries the operations table header");
    assert_eq!(
        doc_lines.get(header + 1),
        Some(&"|---|---|"),
        "the table header is followed by its separator"
    );
    let mut rows = Vec::new();
    for line in &doc_lines[header + 2..] {
        if !line.starts_with('|') {
            break;
        }
        let cells: Vec<&str> = line.split('|').collect();
        assert_eq!(cells.len(), 4, "a table row is `| ops | cost |`: {line:?}");
        rows.push((cells[1].trim().to_owned(), cells[2].trim().to_owned()));
    }
    assert!(!rows.is_empty(), "the operations table has data rows");
    rows
}
