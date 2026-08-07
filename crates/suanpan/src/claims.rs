//! The claims roster: every public operation's committed cost row — its
//! crate-page table cell and the test evidence behind it.
//!
//! The rustdoc's `# Complexity` sections are review-maintained prose,
//! each stating its own bound inline; the roster binds the legs that
//! rot silently without a name, and the binding tests
//! (`claims/tests.rs`) hold them together:
//!
//! - **Table ↔ roster**: the crate page's operations table is scanned
//!   row by row; every row's cost cell must byte-equal the
//!   [`Claim::table_cost`] of each operation the row names, and every
//!   row must be named by some claim — the table was twice found wrong
//!   in review before this binding existed, so it is held to the roster
//!   like any other committed data.
//! - **Claim ↔ evidence**: every claim either names committed `#[test]`
//!   witnesses (checked to exist, by name, in their files — suanpan's own
//!   touch-metered pins, plus the `accum_streams` digit-touch bands that
//!   live beside the consumer in `before/tests/meter.rs`) or carries a
//!   mechanism-based exclusion reason.
//! - **Totality**: the extracted `pub fn` surface plus the family rows
//!   ([`FAMILY_SURFACE`]) equals the roster's op set exactly, both
//!   directions, so a new public operation fails here until its cost
//!   row is pinned.
//!
//! The cost vocabulary the table cells use — digit touches, amortized,
//! the byte sizes `|x|`, the written span — is defined on the crate
//! page (the metering section and the table preamble/footnote).

use surface_scan::SourceSpec;

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

/// One public operation's pinned cost row.
pub(crate) struct Claim {
    /// The operation, named exactly as the surface extractor names it
    /// (or a [`FAMILY_SURFACE`] row).
    pub(crate) op: &'static str,
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

/// Suanpan's own touch-metered pins.
const OWN: &str = "src/accumulator/tests/metered.rs";

/// The digit-touch stream bands committed beside the consumer.
const BANDS: &str = "../before/tests/meter.rs";

/// Shorthand for a word-scale operation with no table row, excluded
/// from instrumentation with its mechanism.
const fn constant(op: &'static str, reason: &'static str) -> Claim {
    Claim {
        op,
        table_cost: None,
        evidence: Evidence::Excluded(reason),
    }
}

/// The claims roster of record: one row per public operation, named as
/// the surface extractor (or [`FAMILY_SURFACE`]) names it.
///
/// The binding tests hold it total, its table cells byte-equal to the
/// crate page's, and its cited witnesses alive.
pub(crate) const CLAIMS: &[Claim] = &[
    // ─────────────────────────── construction ───────────────────────────
    constant(
        "Accumulator::new",
        "allocates the one-digit buffer: word-scale, no input axis to measure against",
    ),
    // ─────────────────────── machine-word deltas ────────────────────────
    Claim {
        op: "Accumulator::add_small",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(BANDS, "accum_comb_touches_flat")]),
    },
    Claim {
        op: "Accumulator::sub_small",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(BANDS, "accum_comb_touches_flat")]),
    },
    Claim {
        op: "Accumulator::add_u64",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "u64_comb_touches_are_flat_and_exact")]),
    },
    Claim {
        op: "Accumulator::sub_u64",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "u64_comb_touches_are_flat_and_exact")]),
    },
    // ─────────────────────────── wide deltas ────────────────────────────
    Claim {
        op: "Accumulator::add_wide",
        table_cost: Some(r"amortized O(\|delta\|), whatever the held width"),
        evidence: Evidence::Witnessed(&[
            (OWN, "wide_writes_cost_the_operand_at_any_held_width"),
            (BANDS, "accum_wide_tooth_touches_flat"),
            (BANDS, "accum_cancelling_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::sub_wide",
        table_cost: Some(r"amortized O(\|delta\|), whatever the held width"),
        evidence: Evidence::Witnessed(&[
            (OWN, "wide_writes_cost_the_operand_at_any_held_width"),
            (BANDS, "accum_wide_tooth_touches_flat"),
            (BANDS, "accum_cancelling_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::add_wide_shl",
        table_cost: Some(r"amortized O(\|delta\|), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    Claim {
        op: "Accumulator::sub_wide_shl",
        table_cost: Some(r"amortized O(\|delta\|), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    Claim {
        op: "Accumulator::add_u64_shl",
        table_cost: Some("amortized O(1), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    Claim {
        op: "Accumulator::sub_u64_shl",
        table_cost: Some("amortized O(1), independent of the shift"),
        evidence: Evidence::Witnessed(&[(
            OWN,
            "alternating_shifted_writes_cost_the_operand_not_the_gap",
        )]),
    },
    // ─────────────────────── magnitude dispatches ───────────────────────
    Claim {
        op: "Accumulator::add_magnitude",
        table_cost: Some(r"word-scale: amortized O(1); wide: amortized O(\|delta\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "magnitude_dispatch_costs_its_width_path")]),
    },
    Claim {
        op: "Accumulator::sub_magnitude",
        table_cost: Some(r"word-scale: amortized O(1); wide: amortized O(\|delta\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "magnitude_dispatch_costs_its_width_path")]),
    },
    Claim {
        op: "Accumulator::add_magnitude_shl",
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
        table_cost: Some(r"amortized O(\|other\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::sub_accum",
        table_cost: Some(r"amortized O(\|other\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::add_accum_shl",
        table_cost: Some(r"amortized O(\|other\|), independent of the shift"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::sub_accum_shl",
        table_cost: Some(r"amortized O(\|other\|), independent of the shift"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    Claim {
        op: "Accumulator::merge_into_wider",
        table_cost: Some(r"amortized O(min(\|self\|, \|other\|))"),
        evidence: Evidence::Witnessed(&[(OWN, "accumulator_operand_rows_cost_the_operand")]),
    },
    // ───────────────────────── sign queries ─────────────────────────────
    Claim {
        op: "Accumulator::sign",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[
            (OWN, "no_collapse_fold_re_scans_the_prefix"),
            (OWN, "sign_fold_skips_certified_runs"),
            (BANDS, "accum_static_prefix_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::is_negative",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[
            (OWN, "no_collapse_fold_re_scans_the_prefix"),
            (BANDS, "accum_static_prefix_touches_flat"),
        ]),
    },
    Claim {
        op: "Accumulator::sign_dominates_word",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "domination_reads_cost_one_touch_after_the_first")]),
    },
    Claim {
        op: "Accumulator::sign_dominates_at",
        table_cost: Some("amortized O(1)"),
        evidence: Evidence::Witnessed(&[(OWN, "domination_reads_cost_one_touch_after_the_first")]),
    },
    // ─────────────────────────── O(1) probes ────────────────────────────
    Claim {
        op: "Accumulator::is_literally_zero",
        table_cost: Some("O(1)"),
        evidence: Evidence::Excluded(
            "two field reads: no digit is touched, and there is no input axis to measure \
             against",
        ),
    },
    Claim {
        op: "Accumulator::digit_count",
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
        table_cost: Some(r"O(\|self\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::negate",
        table_cost: Some(r"O(\|self\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::reset",
        table_cost: Some(r"O(\|self\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::sign_magnitude",
        table_cost: Some(r"O(\|self\|)"),
        evidence: Evidence::Witnessed(&[(OWN, "held_width_rows_cost_the_held_digits")]),
    },
    Claim {
        op: "Accumulator::sign_magnitude_shl",
        table_cost: Some("O(w), w the written span since the last reset"),
        evidence: Evidence::Witnessed(&[
            (OWN, "scaled_read_costs_the_written_span"),
            (OWN, "scaled_read_costs_the_span_not_the_write_count"),
        ]),
    },
    // ──────────────────────────── Limbs ─────────────────────────────────
    Claim {
        op: "Limbs::new",
        table_cost: None,
        evidence: Evidence::Excluded(
            "builds a chunk iterator over a borrowed word slice: no digit axis and no \
             allocation",
        ),
    },
    Claim {
        op: "Limbs iteration (Iterator / DoubleEndedIterator)",
        table_cost: None,
        evidence: Evidence::Excluded(
            "each step packs at most two borrowed storage words into one limb: word-scale \
             by construction, outside the digit-touch denomination",
        ),
    },
    // ─────────────────────────── Magnitude ──────────────────────────────
    Claim {
        op: "Magnitude (the caller-implemented width seam)",
        table_cost: None,
        evidence: Evidence::Excluded(
            "a trait contract on implementors — to_word must be O(1), the dispatch read \
             the small path's accounting assumes free; the crate prices its own impls, \
             not callers'",
        ),
    },
    Claim {
        op: "Magnitude for UBig (the word-fit dispatch)",
        table_cost: None,
        evidence: Evidence::Excluded(
            "the word-fit probe reads dashu's stored-word count (at most two words \
             compared): word-scale, outside the digit-touch denomination",
        ),
    },
    // ──────────────────── derived and re-exported ───────────────────────
    Claim {
        op: "Accumulator Clone / Debug / Default (derived surface)",
        table_cost: None,
        evidence: Evidence::Excluded(
            "derived traversals of the digit buffer and ledger: they read digits without \
             the read-modify-write the touch meter denominates, so the buffer-order cost \
             is structural, stated at the type doc",
        ),
    },
    Claim {
        op: "UBig (re-export)",
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
/// Reads the raw source (the table lives under its own heading in the
/// crate docs), locates the unique `| Operation | Cost |` header, and
/// takes the contiguous table rows after its separator line.
pub(crate) fn cost_table() -> Vec<(String, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs");
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("reading {path}: {error}"));
    let doc_lines: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("//!"))
        .map(|line| line.strip_prefix(' ').unwrap_or(line).trim_end())
        .collect();
    let header = doc_lines
        .iter()
        .position(|line| *line == "| Operation | Cost |")
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
        // Cells split on unescaped pipes only: `\|` inside a cell is
        // markdown's literal bar (the size notation `|x|` uses it).
        let masked = line.replace("\\|", "\u{0}");
        let cells: Vec<String> = masked
            .split('|')
            .map(|cell| cell.replace('\u{0}', "\\|"))
            .collect();
        assert_eq!(cells.len(), 4, "a table row is `| ops | cost |`: {line:?}");
        rows.push((cells[1].trim().to_owned(), cells[2].trim().to_owned()));
    }
    assert!(!rows.is_empty(), "the operations table has data rows");
    rows
}
