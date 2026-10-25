//! A join that recovers one wide payload at every level of a spine.

use focused_fuel_harness::{assert_flat, measure_per, Probe};

/// Join work stays linear when one wide payload survives a collapse at every
/// level: doubling the spine's depth and the flat version's width together
/// leaves fuel per encoded input bit flat, in both operand orders.
///
/// Form: flat per unit (F-lin) across a doubling of the depth, with the
/// operands' encoded bits as the unit.
#[test]
fn reanchor_join_is_linear_per_input_bit() {
    for probe in [Probe::ReanchorJoinFlatFirst, Probe::ReanchorJoinSpineFirst] {
        assert_flat(
            &format!("{probe:?} per input bit"),
            measure_per(probe, 32, 0),
            measure_per(probe, 64, 0),
        );
    }
}
