//! The board coverage tests: the tiling over the public surface and the
//! red-triage buffer's acceptance assertion.

use std::collections::{BTreeMap, BTreeSet};

use super::{BOARD_EXPECTED_REDS, BOARD_NOT_APPLICABLE, BOARD_PRICED};
use crate::meter::board::{bench_cells, BenchMode};
use crate::testing::surface_coverage;

/// Every board operation name, from the board's own axis declarations at a tiny
/// build-only scale.
fn board_ops() -> BTreeSet<String> {
    bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| cell.op.to_owned())
        .collect()
}

/// The full public surface: every mechanically extracted `pub fn` plus every
/// coverage family row.
fn public_surface() -> BTreeSet<String> {
    let mut surface: BTreeSet<String> = surface_coverage::extract_public_fns();
    surface.extend(
        surface_coverage::FAMILY_SURFACE
            .iter()
            .map(|row| row.op.to_owned()),
    );
    surface
}

/// The board tiling: every public-surface row is priced by live board rows
/// ([`BOARD_PRICED`]) or excused in [`BOARD_NOT_APPLICABLE`] with a mechanism,
/// never both, never neither.
///
/// Both tables name only real surface rows, carry no duplicates, and every
/// cited board row exists on the board's operation axis, so a renamed or
/// retired row orphans the entries that leaned on it by name, and a new public
/// operation fails here until it is priced or excused.
#[test]
fn board_coverage_tiles_the_public_surface() {
    let surface = public_surface();
    let ops = board_ops();
    let mut priced: BTreeMap<&str, &[&str]> = BTreeMap::new();
    for (op, rows) in BOARD_PRICED {
        assert!(
            surface.contains(*op),
            "BOARD_PRICED names {op:?}, which is no public-surface row: \
             remove or rename the entry"
        );
        assert!(
            !rows.is_empty(),
            "{op}: a priced entry must cite at least one board row"
        );
        for row in *rows {
            assert!(ops.contains(*row), "{op}: cites unknown board row {row}");
        }
        assert!(
            priced.insert(op, rows).is_none(),
            "{op} appears twice in BOARD_PRICED"
        );
    }
    let mut na = BTreeMap::new();
    for (op, reason) in BOARD_NOT_APPLICABLE {
        assert!(
            surface.contains(*op),
            "BOARD_NOT_APPLICABLE names {op:?}, which is no public-surface row: \
             remove or rename the entry"
        );
        assert!(
            reason.len() >= 20,
            "{op}: the not-applicable reason is too thin to be a mechanism: {reason:?}"
        );
        assert!(
            na.insert(*op, *reason).is_none(),
            "{op} appears twice in BOARD_NOT_APPLICABLE"
        );
        assert!(
            !priced.contains_key(op),
            "{op}: priced by board rows AND excused in BOARD_NOT_APPLICABLE — \
             the tiling sides must stay disjoint; remove one"
        );
    }
    let untiled: Vec<&String> = surface
        .iter()
        .filter(|op| !priced.contains_key(op.as_str()) && !na.contains_key(op.as_str()))
        .collect();
    assert!(
        untiled.is_empty(),
        "public-surface rows neither priced by board rows nor excused in \
         BOARD_NOT_APPLICABLE (add them to one side of the tiling): {untiled:?}"
    );
    // The reverse leg: every board operation row prices some public
    // row, so the board carries no orphan row a rename could strand.
    let cited: BTreeSet<&str> = BOARD_PRICED
        .iter()
        .flat_map(|(_, rows)| rows.iter().copied())
        .collect();
    let orphans: Vec<String> = board_ops()
        .into_iter()
        .filter(|op| !cited.contains(op.as_str()))
        .collect();
    assert!(
        orphans.is_empty(),
        "board rows cited by no BOARD_PRICED entry (name the public operation \
         each prices, or retire the row): {orphans:?}"
    );
}

/// The red-triage buffer is empty at acceptance, and any in-flight entry names
/// a live board cell, exactly once, with a mechanism tag and a live-task
/// reference.
///
/// Red means untriaged, nothing else: every dashboard contradiction resolves to
/// a cure or an owner-declared model at the cell, so [`BOARD_EXPECTED_REDS`]
/// may hold an entry only while its triage is in flight (the `task` field names
/// the work), and this assertion is the acceptance teeth — a red that persists
/// across commits is a process failure, not a status.
#[test]
fn expected_red_buffer_is_an_empty_triage_buffer() {
    let cells: BTreeSet<(String, String)> = bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| (cell.op.to_owned(), cell.family.to_owned()))
        .collect();
    let mut seen = BTreeSet::new();
    for red in BOARD_EXPECTED_REDS {
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
            !red.task.trim().is_empty(),
            "{}/{} carries no live-task reference: an untriaged red may sit in \
             the buffer only while someone owns its triage",
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
    assert!(
        BOARD_EXPECTED_REDS.is_empty(),
        "the red-triage buffer is not empty: every entry is an untriaged \
         contradiction whose resolution (a cure, or an owner-declared model \
         at the cell) must land before acceptance: {:?}",
        BOARD_EXPECTED_REDS
            .iter()
            .map(|red| format!("{}/{} ({})", red.op, red.family, red.task))
            .collect::<Vec<_>>()
    );
}
