use crate::ops::ROSTER;

use super::{run_op, Plan, Samplers};

/// A run is a pure function of (guest wasm, plan): two executions of the
/// same plan read byte-identical fuel in the same cell order.
///
/// Every cell RNG is seeded from its coordinates and every measurement
/// runs in a fresh guest, so rayon's nondeterministic scheduling has
/// nothing to perturb — neither a draw, nor a reading, nor the order the
/// collected samples land in. This is the atlas's stamped determinism
/// contract; a construction change that leaks state between samples or
/// re-derives a cell's RNG from execution order fails it.
#[test]
fn run_op_is_deterministic_and_ordered() {
    let plan = Plan {
        base_seed: 0x5eed,
        samples_per_column: 4,
        max_bytes: 8,
    };
    let samplers = Samplers::build(&plan);
    // One unary version row, one binary party row: both samplers, both
    // arities, the split rule, and the version rejection path.
    for name in ["version_rank", "party_covers"] {
        let op = ROSTER
            .iter()
            .find(|op| op.name == name)
            .expect("determinism ops are roster rows");
        let a = run_op(&plan, &samplers, op);
        let b = run_op(&plan, &samplers, op);

        let cells = |atlas: &super::OpAtlas| -> Vec<(usize, u64, u64)> {
            atlas
                .samples
                .iter()
                .map(|s| (s.size, s.fuel, s.rejected))
                .collect()
        };
        assert_eq!(
            cells(&a),
            cells(&b),
            "{name}: bulk cells must replay exactly"
        );

        let overlay = |atlas: &super::OpAtlas| -> Vec<(&'static str, usize, u64)> {
            atlas
                .overlay
                .iter()
                .map(|p| (p.family, p.size, p.fuel))
                .collect()
        };
        assert_eq!(
            overlay(&a),
            overlay(&b),
            "{name}: overlay points must replay exactly"
        );

        // The collected order is the plan's declared order: columns in
        // grid order, sample indices in sequence within each column.
        let expected: Vec<usize> = plan
            .columns(op.operands.len())
            .into_iter()
            .flat_map(|size| std::iter::repeat_n(size, plan.samples_per_column))
            .collect();
        let got: Vec<usize> = a.samples.iter().map(|s| s.size).collect();
        assert_eq!(got, expected, "{name}: samples must land in plan order");
    }
}
