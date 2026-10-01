use std::collections::BTreeSet;

use before::testing::meter::board::{BOARD_NOT_APPLICABLE, BOARD_PRICED};

use super::{EXEMPTIONS, ROSTER};

/// Panels plus exemptions tile the board's public-operation inventory.
///
/// Every board operation is either claimed by some panel's `covers` list or
/// carries a committed exemption reason, every claim and every exemption
/// names a real operation, and no operation is both. The compiler-derived
/// surface check separately pins the function entries to the public API and
/// every trait implementation to its census. A renamed board entry fails here
/// by name, including a stale exemption.
#[test]
fn panels_and_exemptions_tile_the_board_inventory() {
    let surface: BTreeSet<&str> = BOARD_PRICED
        .iter()
        .map(|(operation, _)| *operation)
        .chain(BOARD_NOT_APPLICABLE.iter().map(|(operation, _)| *operation))
        .collect();
    let covered: BTreeSet<&str> = ROSTER
        .iter()
        .flat_map(|op| op.covers.iter().copied())
        .collect();
    let exempted: BTreeSet<&str> = EXEMPTIONS.iter().map(|(op, _)| *op).collect();

    for op in &covered {
        assert!(
            surface.contains(op),
            "stale covers claim: no board operation is named {op:?}"
        );
    }
    for op in &exempted {
        assert!(
            surface.contains(op),
            "stale exemption: no board operation is named {op:?}"
        );
    }
    for op in &surface {
        assert!(
            covered.contains(op) || exempted.contains(op),
            "board operation {op:?} has neither an atlas panel covering it nor a \
             committed exemption"
        );
    }
    let contradictions: Vec<&&str> = covered.intersection(&exempted).collect();
    assert!(
        contradictions.is_empty(),
        "rows both covered by a panel and exempted (keep exactly one): {contradictions:?}"
    );
}

/// Every panel claims at least one public operation, and panel names (the
/// output file stems) are unique — a duplicated stem would silently
/// overwrite a sibling's render.
#[test]
fn panels_claim_rows_and_have_unique_names() {
    let mut names = BTreeSet::new();
    for op in ROSTER {
        assert!(
            !op.covers.is_empty(),
            "{}: a panel must claim at least one public operation",
            op.name
        );
        assert!(names.insert(op.name), "duplicate roster name {}", op.name);
    }
}

/// Every exemption states a nonempty reason and names each operation at
/// most once — the table is the reviewed artifact, so a blank or
/// duplicated line is a bookkeeping bug.
#[test]
fn exemptions_are_reasoned_and_unique() {
    let mut names = BTreeSet::new();
    for (op, reason) in EXEMPTIONS {
        assert!(
            !reason.trim().is_empty(),
            "{op}: exemption without a reason"
        );
        assert!(names.insert(*op), "duplicate exemption for {op}");
    }
}
