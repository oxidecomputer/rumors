use std::collections::BTreeSet;

use super::overlay_inputs;
use crate::ops::ROSTER;

/// The ranked-comparison overlay reaches both expensive settle shapes without
/// admitting identical operands that return before the rank walk.
#[test]
fn ranked_comparison_families_reach_the_measured_path() {
    let op = ROSTER
        .iter()
        .find(|op| op.name == "ranked_cmp")
        .expect("ranked comparison has a fuelscape panel");
    let overlays = overlay_inputs(op, 4096);

    assert!(
        overlays
            .iter()
            .all(|family| family.inputs[0] != family.inputs[1]),
        "every overlay must reach comparison rather than the identity fast path"
    );

    let names: BTreeSet<_> = overlays.iter().map(|family| family.family).collect();
    assert!(
        names.contains("wide_arming × empty"),
        "the overlay must exercise promoted wide arithmetic"
    );
    assert!(
        names.contains("plateau_puncture × empty"),
        "the overlay must exercise the answer-embedded product"
    );
}
