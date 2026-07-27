//! Membership pins for the bench judge's roster and ceiling declarations.
//!
//! The judge's enforcement is only as strong as two committed name sets:
//! the expected-verdict roster (`tools/benchjudge-expected.json`) and the
//! text-ceiling set the bench sidecar declares
//! (`benches/common/sidecar.rs`, `TEXT_CEILING_CELLS`). Both are plain
//! data a one-line edit could quietly reshape — un-rostering an owned red,
//! widening the text class to launder a superlinear cell — so this suite
//! pins their exact membership: any roster or class edit trips a test
//! whose diff a reviewer sees, alongside the edit itself.

// The sidecar module is shared bench-side code; this test target uses only
// its pinned text-ceiling set, so the harness plumbing is deliberately
// unused here.
#[allow(dead_code)]
#[path = "../benches/common/sidecar.rs"]
mod sidecar;

use serde_json::Value;

/// The committed roster, parsed from the workspace's `tools/` directory.
fn roster() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/benchjudge-expected.json"
    );
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("reading the roster at {path} failed: {err}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("the roster at {path} is not JSON: {err}"))
}

/// The roster's class member as a sorted list of cell IDs.
fn class(roster: &Value, name: &str) -> Vec<String> {
    let mut names: Vec<String> = roster[name]
        .as_array()
        .unwrap_or_else(|| panic!("roster `{name}` must be a list"))
        .iter()
        .map(|cell| cell.as_str().expect("cell IDs are strings").to_string())
        .collect();
    names.sort();
    names
}

/// The roster's red set is exactly the permanent schoolbook tripwire and
/// the hugeleaf display pair.
///
/// Every other cell — the designed diagonal and the board-red riders
/// alike — must fit under its own ceiling. The tripwire's red is the
/// known-quadratic conversion class it times, where green means the
/// tripwire went dark; the display pair's red is the conversion-dominated
/// hugeleaf-width render (measured e 1.39/1.42 at the general 1.3
/// ceiling, 2026-07-27), owned by the text column with the class question
/// open. Removing an owned red silences a standing judgment and adding
/// one launders a new regression as expected, so both directions must
/// show up as a diff of this pin.
#[test]
fn roster_red_membership_is_pinned() {
    let mut expected = vec![
        "clock_display/hugeleaf",
        "display_schoolbook/hugeleaf",
        "version_display/hugeleaf",
    ];
    expected.sort();
    assert_eq!(class(&roster(), "red"), expected);
}

/// The roster's boundary set is empty at this tip.
///
/// The class stays in the schema as enforcement vocabulary. Boundary
/// membership accepts either verdict, so adding a cell would exempt it
/// from judgment — any change must be a reviewed diff here.
#[test]
fn roster_boundary_membership_is_pinned() {
    assert_eq!(class(&roster(), "boundary"), [] as [&str; 0]);
}

/// The roster carries only the two expectation classes plus configuration
/// and notes.
///
/// A new member would be a new enforcement vocabulary (the judge refuses
/// unknown members at runtime; this pin catches the edit at test time,
/// before any bench runs).
#[test]
fn roster_schema_carries_expectations_only() {
    let roster = roster();
    let mut keys: Vec<&str> = roster
        .as_object()
        .expect("the roster is a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["boundary", "configuration", "notes", "red"]);
}

/// The sidecar's text-ceiling set is exactly the wide-display pair.
///
/// The text ceiling (1.7) exists for conversion-dominated rendering only,
/// and every other cell must stay judged at the general ceiling — widening
/// this set is the one remaining way to move a cell's ceiling, so it is
/// pinned here and asserted again at every sidecar write.
#[test]
fn sidecar_text_ceiling_set_is_the_wide_display_pair() {
    assert_eq!(
        sidecar::TEXT_CEILING_CELLS,
        [
            "version_display_wide/hugeleaf",
            "display_schoolbook/hugeleaf"
        ]
    );
}

/// The schoolbook tripwire is rostered red and the honest wide-display
/// cell is unrostered (green by default at its text ceiling).
///
/// The pair separates the conversion classes only while the quadratic
/// member is required red and the divide-and-conquer member is required
/// green.
#[test]
fn wide_display_pair_expectations_are_split() {
    let roster = roster();
    let red = class(&roster, "red");
    let boundary = class(&roster, "boundary");
    assert!(red.contains(&"display_schoolbook/hugeleaf".to_string()));
    for rostered in [&red, &boundary] {
        assert!(!rostered.contains(&"version_display_wide/hugeleaf".to_string()));
    }
}
