//! The complexity-claims binding tests.
//!
//! They hold the roster total over the public surface, the prose tokens
//! present, the cited board rows alive, the superlinear claims equal to
//! the bench judge's red set, and the two non-linear classes' liveness
//! pins red-on-cure. The roster and the scanner live in the parent
//! module.

use std::collections::BTreeSet;

use super::{doc_index, Cells, Claim, Class, CLAIMS, NON_OPERATIONS};
use crate::meter::board::{self, BenchMode};
use crate::testing::triangle;

/// Every board operation name, from the board's own axis declarations at
/// a tiny build-only scale.
fn board_ops() -> BTreeSet<String> {
    board::bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| cell.op.to_owned())
        .collect()
}

/// The bench judge's committed expected-verdict roster
/// (`tools/benchjudge-expected.json`; its membership is pinned by
/// `tests/bench_judge_roster.rs`).
fn judge_roster() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/benchjudge-expected.json"
    );
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("reading the judge roster at {path} failed: {err}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("the judge roster at {path} is not JSON: {err}"))
}

/// The claims roster is total over the public surface, exactly.
///
/// Every mechanically extracted `pub fn` and every triangle family row
/// has one claim (or a place in the pinned non-operation list), and
/// nothing else does. A new public operation fails here until its
/// documented class is pinned; a removed one orphans its claim.
#[test]
fn claims_are_total_over_the_public_surface() {
    let mut surface: BTreeSet<String> = triangle::extract_public_fns();
    surface.extend(triangle::FAMILY_SURFACE.iter().map(|row| row.op.to_owned()));
    let mut claimed = BTreeSet::new();
    for claim in CLAIMS {
        assert!(
            claimed.insert(claim.op.to_owned()),
            "duplicate claim row: {}",
            claim.op
        );
    }
    claimed.extend(NON_OPERATIONS.iter().map(|op| (*op).to_owned()));
    let unclaimed: Vec<_> = surface.difference(&claimed).collect();
    let orphaned: Vec<_> = claimed.difference(&surface).collect();
    assert!(
        unclaimed.is_empty() && orphaned.is_empty(),
        "the claims roster and the public surface disagree:\n  \
         public operations with no complexity claim: {unclaimed:?}\n  \
         claims naming no public operation: {orphaned:?}"
    );
}

/// Every claim's `# Complexity` section exists at its recorded site and
/// carries its pinned Big-O tokens verbatim, so a class edit in the
/// rustdoc that skips this roster (or vice versa) is a named failure.
#[test]
fn complexity_sections_carry_their_pinned_tokens() {
    let index = doc_index();
    let mut errors = Vec::new();
    for claim in CLAIMS {
        for check in claim.checks {
            match index.section(claim.op, check.site) {
                Err(err) => errors.push(err),
                Ok(section) => {
                    for token in check.tokens {
                        if !section.contains(token) {
                            errors.push(format!(
                                "{}: the `# Complexity` section at {:?} lost its pinned \
                                 token {token}",
                                claim.op, check.site
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        errors.is_empty(),
        "rustdoc complexity sections drifted from the claims roster:\n  {}",
        errors.join("\n  ")
    );
}

/// Every board row a claim cites exists on the board's operation axis, so
/// a renamed or retired row orphans the claims that leaned on it by name.
#[test]
fn cited_board_rows_exist() {
    let ops = board_ops();
    let mut missing = Vec::new();
    for claim in CLAIMS {
        match &claim.cells {
            Cells::Board(cells) => {
                for (op, _) in *cells {
                    if !ops.contains(*op) {
                        missing.push(format!("{}: cites unknown board row {op}", claim.op));
                    }
                }
            }
            // Uncelled rows carry their reason as data, mirroring the
            // board module doc's coverage list; hold it non-empty.
            Cells::Uncelled(reason) => {
                assert!(
                    !reason.is_empty(),
                    "{}: an uncelled claim must state its reason",
                    claim.op
                );
            }
        }
    }
    assert!(
        missing.is_empty(),
        "claims cite board rows that do not exist:\n  {}",
        missing.join("\n  ")
    );
}

/// The board rows the rustdoc claims superlinear-in-time are exactly the
/// board rows on the bench judge's committed red set: curing a display
/// red (or rostering a new one) must reach the documentation through this
/// name.
#[test]
fn superlinear_time_claims_match_the_bench_judge_red_set() {
    let ops = board_ops();
    let claimed: BTreeSet<String> = CLAIMS
        .iter()
        .filter_map(|claim| match &claim.cells {
            Cells::Board(cells) => Some(*cells),
            Cells::Uncelled(_) => None,
        })
        .flat_map(|cells| {
            cells
                .iter()
                .filter(|(_, class)| *class == Class::SuperlinearTime)
                .map(|(op, _)| (*op).to_owned())
        })
        .collect();
    let rostered: BTreeSet<String> = judge_roster()["red"]
        .as_array()
        .expect("the judge roster's red class is a list")
        .iter()
        .map(|cell| {
            cell.as_str()
                .expect("cell IDs are strings")
                .split('/')
                .next()
                .expect("cell IDs are op/family")
                .to_owned()
        })
        // The judge also rosters non-board tripwire benches (the
        // schoolbook probe); only board rows bind rustdoc claims.
        .filter(|op| ops.contains(op))
        .collect();
    assert_eq!(
        claimed, rostered,
        "the rustdoc's superlinear-time claims and the bench judge's red set \
         disagree: update the claims roster and the `# Complexity` sections \
         together"
    );
}

/// The linear claims never cite a row the bench judge holds red: a board
/// row cannot be documented linear while its time leg is a rostered
/// superlinearity.
#[test]
fn linear_claims_cite_no_judge_red_row() {
    let rostered: BTreeSet<String> = judge_roster()["red"]
        .as_array()
        .expect("the judge roster's red class is a list")
        .iter()
        .map(|cell| {
            cell.as_str()
                .expect("cell IDs are strings")
                .split('/')
                .next()
                .expect("cell IDs are op/family")
                .to_owned()
        })
        .collect();
    let mut contradictions = Vec::new();
    for claim in CLAIMS {
        if let Cells::Board(cells) = &claim.cells {
            for (op, class) in *cells {
                if *class != Class::SuperlinearTime && rostered.contains(*op) {
                    contradictions.push(format!(
                        "{}: cites {op} as {class:?}, but the bench judge holds it red",
                        claim.op
                    ));
                }
            }
        }
    }
    assert!(
        contradictions.is_empty(),
        "linear claims contradict the judge's red set:\n  {}",
        contradictions.join("\n  ")
    );
}

/// Big-integer limb work of one full render of the wide left-full shape
/// (the board's mirror-wide event side) at spine scale `s`.
#[cfg(feature = "limb-meter")]
fn render_limb_ops(s: usize) -> u64 {
    let version = crate::meter::wide_tail(s, s).version();
    crate::meter::reset_limb_ops();
    std::hint::black_box(version.to_string());
    crate::meter::limb_ops()
}

/// The render merge's superlinearity is alive.
///
/// `Display` limb work on the wide left-full shape grows super-linearly
/// across a doubling, which is exactly what the rustdoc's "summary-merge
/// cost that grows faster than the operand" sentence and the claims
/// roster's `SuperlinearTime` class describe. When the render-merge cure
/// lands this pin reads red, and the rustdoc, the claims roster, and this
/// floor must move in one change.
///
/// Deterministic counter, dev profile; linear rendering would read ~2.0
/// across the doubling, and the current merge reads x2.93 (8 558 ->
/// 25 114 ops, measured 2026-07-27 at this shape and scale). The floor
/// sits midway in that gap, so only a class change (never noise — the
/// counter is exact) crosses it.
#[cfg(feature = "limb-meter")]
#[test]
fn render_merge_superlinearity_is_alive() {
    /// Halfway between linear growth (~2.0x) and the measured x2.93.
    const MIN_GROWTH: f64 = 2.45;
    let (lo, hi) = (render_limb_ops(500), render_limb_ops(1000));
    let growth = hi as f64 / lo.max(1) as f64;
    assert!(
        growth >= MIN_GROWTH,
        "the render merge's limb work grew only x{growth:.2} across a doubling \
         ({lo} -> {hi} ops; superlinear read >= x{MIN_GROWTH}, linear ~x2.0): \
         the documented superlinearity is gone, so update the Display \
         `# Complexity` sections, the claims roster, and this pin together"
    );
}

/// Packed-stream scan work of one `Version::join_all` over the scatter
/// fold population (the board's shape: `n` balanced-forked single-tick
/// versions, evens before odds).
#[cfg(feature = "scan-meter")]
fn fold_scan_bits(n: usize) -> u64 {
    use crate::{Party, Version};
    let mut parties = vec![Party::seed()];
    while parties.len() < n {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    let versions: Vec<Version> = parties
        .iter()
        .map(|p| {
            let mut v = Version::new();
            v.tick(p);
            v
        })
        .collect();
    let (evens, odds): (Vec<_>, Vec<_>) = versions
        .into_iter()
        .enumerate()
        .partition(|(i, _)| i % 2 == 0);
    let scattered: Vec<Version> = evens.into_iter().chain(odds).map(|(_, v)| v).collect();
    crate::meter::reset_scan_bits();
    std::hint::black_box(Version::join_all(scattered));
    crate::meter::scan_bits()
}

/// The n-ary fold's log factor is alive.
///
/// `Version::join_all`'s scan work on the scatter population grows faster
/// than its input across a x4 population growth — the balanced
/// reduction's `O(D log k)`, which is what the fold operations'
/// `# Complexity` sections and the claims roster's `FoldLog` class
/// document. If an n-cursor merge (or any linear fold) lands, this pin
/// reads red, and the rustdoc, the claims roster, and this floor must
/// move in one change.
///
/// Deterministic counter, dev profile; a linear fold would read ~4.0
/// across the x4 growth, `D log k` predicts `4 x log(4n)/log(n)` (5.0 at
/// n = 256), and the current reduction reads x5.16 (51 354 -> 264 730
/// bits, measured 2026-07-27). The floor sits midway between linear and
/// measured.
#[cfg(feature = "scan-meter")]
#[test]
fn fold_log_factor_is_alive() {
    /// Halfway between linear growth (~4.0x) and the measured x5.16.
    const MIN_GROWTH: f64 = 4.6;
    let (lo, hi) = (fold_scan_bits(256), fold_scan_bits(1024));
    let growth = hi as f64 / lo.max(1) as f64;
    assert!(
        growth >= MIN_GROWTH,
        "join_all's scan work grew only x{growth:.2} across a x4 population \
         growth ({lo} -> {hi} bits; the log factor reads >= x{MIN_GROWTH}, a \
         linear fold ~x4.0): the documented `O(D log k)` overstates, so \
         update the fold `# Complexity` sections, the claims roster, and \
         this pin together"
    );
}

/// The tripwire the roster's own vocabulary rests on: a doc block whose
/// `# Complexity` section is missing, or whose section lost a pinned
/// token, is detected — the scanner is not vacuously green.
#[test]
fn scanner_detects_missing_sections_and_tokens() {
    assert_eq!(
        super::section_of("Summary line.\n\nNo sections here.\n"),
        None,
        "a block with no Complexity section must scan as missing"
    );
    let section =
        super::section_of("Summary.\n\n# Complexity\n\n`O(|v|)` time.\n\n# Panics\n\nNever.\n")
            .expect("the section exists");
    assert!(
        section.contains("`O(|v|)`") && !section.contains("Never"),
        "the section slice must carry its own tokens and end at the next heading"
    );
}

/// A `Claim` is inspectable in failure messages (`Site` derives `Debug`);
/// keep the type checked so the roster stays printable.
#[test]
fn claim_rows_are_printable() {
    let row: &Claim = &CLAIMS[0];
    assert!(!format!("{:?}", row.checks[0].site).is_empty());
}

/// The class-binding seal (review #37, F1's categorical fix): no linear
/// claim cites a board cell standing red on an exponent mechanism, and
/// every counter-superlinear claim keeps at least one.
///
/// The bench judge's red set binds only wall time, and the
/// `version_min_ticks` time legs sit under the judge's resolution at
/// bench scales — so before this seal, a counter-superlinear kernel
/// could keep a `Linear` rustdoc claim with every gate green: at
/// `395f0e72` the min_ticks claim read `Class::Linear` while its
/// pure-comb, reveal-comb, and ascend-cliff board cells read touch/limb
/// exponents 1.58–1.98 on the release boards of record. Run against that
/// state (the mutation demonstration: flip the min_ticks claim back to
/// `Class::Linear`, or mark any Linear-cited cell `exponent: true` in
/// [`board::BOARD_EXPECTED_REDS`]) this test fails naming the
/// contradiction — verified by mutation before this seal landed.
///
/// The reverse leg keeps the class honest: a `SuperlinearCounter` claim
/// whose operation no longer has a standing exponent red is decoration,
/// so the cure that flips the board pins must move the class back to
/// linear in the same change.
#[test]
fn linear_claims_cite_no_exponent_red_board_cell() {
    let exponent_red_ops: BTreeSet<&str> = board::BOARD_EXPECTED_REDS
        .iter()
        .filter(|red| red.exponent)
        .map(|red| red.op)
        .collect();
    let mut contradictions = Vec::new();
    for claim in CLAIMS {
        let Cells::Board(cells) = &claim.cells else {
            continue;
        };
        for (op, class) in *cells {
            match class {
                // Every class that claims the cell scales as its model
                // says (linear, linear-I/O, or the declared fold log)
                // is contradicted by a standing exponent-mechanism red.
                Class::Linear | Class::LinearIo | Class::FoldLog => {
                    if exponent_red_ops.contains(*op) {
                        contradictions.push(format!(
                            "{}: cites {op} as {class:?}, but the board holds it red on an \
                             exponent mechanism (BOARD_EXPECTED_REDS)",
                            claim.op
                        ));
                    }
                }
                // The counter-superlinear class must keep its witness.
                Class::SuperlinearCounter => {
                    if !exponent_red_ops.contains(*op) {
                        contradictions.push(format!(
                            "{}: claims {op} SuperlinearCounter with no standing \
                             exponent-mechanism board red: the class is decoration, move it \
                             back to a linear class with the cure",
                            claim.op
                        ));
                    }
                }
                // Judge-rostered superlinear time: bound by the set
                // equality above; a deterministic exponent red on the
                // same operation is consistent with the class.
                Class::SuperlinearTime => {}
            }
        }
    }
    assert!(
        contradictions.is_empty(),
        "the claims roster contradicts the board's mechanism-tagged red set:\n  {}",
        contradictions.join("\n  ")
    );
}

/// The expected-red roster's own hygiene: every entry names a live board
/// cell exactly once and carries at least one mechanism, and every bench
/// rider is a rostered red (a rider exists to keep a standing red's time
/// leg judged, so an unrostered rider is a stale census).
#[test]
fn expected_red_roster_names_live_cells() {
    let cells: BTreeSet<(String, String)> = board::bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| (cell.op.to_owned(), cell.family.to_owned()))
        .collect();
    let mut seen = BTreeSet::new();
    for red in board::BOARD_EXPECTED_REDS {
        assert!(
            cells.contains(&(red.op.to_owned(), red.family.to_owned())),
            "{}/{} in BOARD_EXPECTED_REDS names no live board cell",
            red.op,
            red.family
        );
        assert!(
            red.exponent || red.constant,
            "{}/{} carries no mechanism tag",
            red.op,
            red.family
        );
        assert!(
            seen.insert((red.op, red.family)),
            "{}/{} appears twice in BOARD_EXPECTED_REDS",
            red.op,
            red.family
        );
    }
    for (op, family) in board::BOARD_RED_BENCH_RIDERS {
        assert!(
            board::BOARD_EXPECTED_REDS
                .iter()
                .any(|red| red.op == *op && red.family == *family),
            "rider {op}/{family} is not a rostered standing red: re-realize the census"
        );
    }
}
