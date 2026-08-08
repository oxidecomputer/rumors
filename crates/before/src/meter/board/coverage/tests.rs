//! The board coverage tests: the tiling over the public surface.

use std::collections::{BTreeMap, BTreeSet};

use super::{BOARD_NOT_APPLICABLE, BOARD_PRICED};
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
