//! Checks that the resource board's coverage tables are internally complete.

use std::collections::{BTreeMap, BTreeSet};

use super::{BOARD_NOT_APPLICABLE, BOARD_PRICED};

/// Every board operation name, from the board's own axis declarations at a tiny
/// build-only scale.
fn board_ops() -> BTreeSet<String> {
    super::super::ops::ops()
        .into_iter()
        .map(|op| op.name.to_owned())
        .collect()
}

/// Every coverage entry is either measured or explicitly inapplicable, never
/// both, and every measurement is used.
///
/// The compiler-derived `surface-totality` gate separately holds the
/// function-like entries against the public API and pins every public trait
/// implementation. This test checks the relationships internal to the board
/// without maintaining another copy of the API.
#[test]
fn board_coverage_is_consistent() {
    let ops = board_ops();
    let mut priced: BTreeMap<&str, &[&str]> = BTreeMap::new();
    for (op, rows) in BOARD_PRICED {
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
