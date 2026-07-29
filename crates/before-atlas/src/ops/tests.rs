use std::collections::BTreeSet;

use before::surface::{FAMILY_SURFACE, METHOD_SURFACE};

use super::{EXEMPTIONS, ROSTER};

/// The atlas's totality binding: panels plus exemptions tile the triangle
/// roster exactly — every roster row is either claimed by some panel's
/// `covers` list or carries a committed exemption reason, every claim and
/// every exemption names a real roster row, and no row is both. The
/// triangle suite pins the roster to the extracted public `pub fn`
/// surface, so through this test a new public operation cannot ship
/// without an atlas panel or a reviewed exemption, and a renamed one
/// fails here by name (a stale exemption included).
#[test]
fn panels_and_exemptions_tile_the_triangle_roster() {
    let surface: BTreeSet<&str> = METHOD_SURFACE
        .iter()
        .chain(FAMILY_SURFACE)
        .map(|row| row.op)
        .collect();
    let covered: BTreeSet<&str> = ROSTER
        .iter()
        .flat_map(|op| op.covers.iter().copied())
        .collect();
    let exempted: BTreeSet<&str> = EXEMPTIONS.iter().map(|(op, _)| *op).collect();

    for op in &covered {
        assert!(
            surface.contains(op),
            "stale covers claim: no triangle roster row is named {op:?}"
        );
    }
    for op in &exempted {
        assert!(
            surface.contains(op),
            "stale exemption: no triangle roster row is named {op:?}"
        );
    }
    for op in &surface {
        assert!(
            covered.contains(op) || exempted.contains(op),
            "triangle roster row {op:?} has neither an atlas panel covering it nor a \
             committed exemption"
        );
    }
    let contradictions: Vec<&&str> = covered.intersection(&exempted).collect();
    assert!(
        contradictions.is_empty(),
        "rows both covered by a panel and exempted (keep exactly one): {contradictions:?}"
    );
}

/// Every panel claims at least one roster row, and panel names (the
/// output file stems) are unique — a duplicated stem would silently
/// overwrite a sibling's render.
#[test]
fn panels_claim_rows_and_have_unique_names() {
    let mut names = BTreeSet::new();
    for op in ROSTER {
        assert!(
            !op.covers.is_empty(),
            "{}: a panel must claim at least one triangle roster row",
            op.name
        );
        assert!(names.insert(op.name), "duplicate roster name {}", op.name);
    }
}

/// Every exemption states a nonempty reason and names each roster row at
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
