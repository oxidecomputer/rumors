use crate::ops::ROSTER;
use crate::sample::cell_rng;

use super::{draw_arity, run_op, split_budget, Plan, Samplers};

/// A run is a pure function of (guest wasm, plan): two executions of the
/// same plan read byte-identical fuel in the same cell order.
///
/// Every cell RNG is seeded from its coordinates and every measurement
/// runs in a fresh guest, so rayon's nondeterministic scheduling has
/// nothing to perturb — neither a draw, nor a reading, nor the order the
/// collected samples land in. This is the atlas's stamped determinism
/// contract; a construction change that leaks state between samples or
/// re-derives a cell's RNG from execution order fails it. The op list
/// walks every input space: unary and binary packed draws (both
/// samplers, the split rule, the version rejection path), the slice
/// arity-and-composition draw, the distinct-pair rejection, the
/// three-way split with in-guest fork preparation, and both
/// variable-arity fold draws (the party-plus-version-slice clock fold
/// and the guest-split party shares).
#[test]
fn run_op_is_deterministic_and_ordered() {
    let plan = Plan {
        base_seed: 0x5eed,
        samples_per_column: 4,
        max_bytes: 8,
    };
    let samplers = Samplers::build(&plan);
    for name in [
        "version_rank",
        "party_covers",
        "version_join_all",
        "party_without",
        "clock_sync",
        "clock_join_all",
        "party_join_all",
    ] {
        let op = ROSTER
            .iter()
            .find(|op| op.name == name)
            .expect("determinism ops are roster rows");
        let a = run_op(&plan, &samplers, op);
        let b = run_op(&plan, &samplers, op);

        let cells = |atlas: &super::OpAtlas| -> Vec<(usize, usize, u64, u64)> {
            atlas
                .samples
                .iter()
                .map(|s| (s.size, s.arity, s.fuel, s.rejected))
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
            .columns(op.inputs.min_bytes())
            .into_iter()
            .flat_map(|size| std::iter::repeat_n(size, plan.samples_per_column))
            .collect();
        let got: Vec<usize> = a.samples.iter().map(|s| s.size).collect();
        assert_eq!(got, expected, "{name}: samples must land in plan order");
    }
}

/// The budget split is a composition: `parts` positive part sizes that
/// sum to the total, for every feasible (total, parts) pair in the small
/// grid — the invariant every multi-operand draw rests on.
#[test]
fn split_budget_yields_positive_compositions() {
    let mut rng = cell_rng(0xc0de, "split", 0, 0);
    for total in 1..=24usize {
        for parts in 1..=total.min(6) {
            for _ in 0..50 {
                let sizes = split_budget(total, parts, &mut rng);
                assert_eq!(sizes.len(), parts, "split into {parts} of {total}");
                assert_eq!(
                    sizes.iter().sum::<usize>(),
                    total,
                    "parts must sum to the total"
                );
                assert!(sizes.iter().all(|&s| s >= 1), "every operand needs a byte");
            }
        }
    }
}

/// The arity draw reaches every count under its cap.
///
/// Uniform over `1..=cap`, so at a small cap every arity appears across
/// a modest draw budget — a draw that silently skipped an arity band
/// would skew every variable-arity column's measure and hide the fold's
/// boundary arities (the drain first combines at 3, the merged–merged
/// carry at 4, the drain of two merged groups at 6: all reachable only
/// if no count is skipped).
#[test]
fn arity_draw_reaches_every_count() {
    let mut rng = cell_rng(0xc0de, "arity-reach", 9, 0);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..2_000 {
        seen.insert(draw_arity(9, &mut rng));
    }
    assert_eq!(
        seen,
        (1..=9).collect::<std::collections::BTreeSet<_>>(),
        "every arity in 1..=9 must be reachable"
    );
}

/// The arity draw is uniform over `1..=cap`, not merely total.
///
/// The module doc claims *uniform* stratification (equal representation
/// per arity), and every variable-arity column's measure rests on it —
/// but a reachability check alone cannot pin uniformity: a
/// biased-but-total draw (e.g. the min of two `gen_range` draws,
/// `P(1) = 17/81` vs the uniform `1/9` at cap 9, chi-square 723.2 at
/// the committed seed over 2000 draws) reaches
/// every count in the same budget and passes it. This one-sided
/// chi-square pin (8 degrees of freedom, mean 8 + 6σ = 32, the sampler
/// pins' own 6σ idiom) reads red on any such skew while a fixed seed
/// keeps the run deterministic.
#[test]
fn arity_draw_is_uniform_over_every_count() {
    const TOTAL: usize = 9;
    const DRAWS: usize = 2_000;
    let mut rng = cell_rng(0xc0de, "arity-uniformity", TOTAL, 0);
    let mut observed = [0u64; TOTAL];
    for _ in 0..DRAWS {
        observed[draw_arity(TOTAL, &mut rng) - 1] += 1;
    }
    let expected = DRAWS as f64 / TOTAL as f64;
    let chi2: f64 = observed
        .iter()
        .map(|&o| {
            let d = o as f64 - expected;
            d * d / expected
        })
        .sum();
    // 8 degrees of freedom: mean 8, variance 16, so 8 + 6·4 = 32.
    let threshold = 32.0;
    assert!(
        chi2 <= threshold,
        "chi-square {chi2:.1} exceeds {threshold:.1} over {TOTAL} arities"
    );
}

/// The fold panels can read the fold's arity out of the bulk cloud: at
/// a fixed size column, samples that drew a larger arity burn more
/// fuel.
///
/// This is what the variable-arity draw buys the two guest-split fold
/// rows. With the arity pinned to a constant, the balanced fold's
/// log-arity factor is the same in every sample, and the panel is
/// structurally incapable of separating a fold that costs `O(n)` from
/// one that costs `O(n · log k)`; the per-sample draw puts the arity
/// axis inside each column, where the spread and the reference slopes
/// can price it. Fuel is deterministic wasm instruction metering, so
/// the comparison is exact at the committed seed and load-immune.
#[test]
fn fold_rows_expose_the_arity_axis_in_fuel() {
    let plan = Plan {
        base_seed: 0x5eed,
        samples_per_column: 32,
        max_bytes: 32,
    };
    let samplers = Samplers::build(&plan);
    for name in ["party_join_all", "clock_join_all"] {
        let op = ROSTER
            .iter()
            .find(|op| op.name == name)
            .expect("fold rows are roster rows");
        let atlas = run_op(&plan, &samplers, op);
        let mut column: Vec<(usize, u64)> = atlas
            .samples
            .iter()
            .filter(|s| s.size == plan.max_bytes)
            .map(|s| (s.arity, s.fuel))
            .collect();
        column.sort_unstable();
        let arities: std::collections::BTreeSet<usize> =
            column.iter().map(|&(arity, _)| arity).collect();
        assert!(
            arities.len() > 1,
            "{name}: the drawn arity must vary within a column, got only {arities:?}"
        );
        let half = column.len() / 2;
        let mean = |cells: &[(usize, u64)]| {
            cells.iter().map(|&(_, fuel)| fuel).sum::<u64>() as f64 / cells.len() as f64
        };
        let (low, high) = (mean(&column[..half]), mean(&column[half..]));
        assert!(
            high > low,
            "{name}: mean fuel must rise with the drawn arity at a fixed size \
             (low-arity half {low:.0}, high-arity half {high:.0})"
        );
    }
}

/// The split reaches every composition.
///
/// Cut-point sampling is uniform over the compositions, so at (6, 3)
/// all ten compositions of 6 into 3 positive parts appear across a
/// modest draw budget. A split rule that biased away from an extreme
/// (e.g. never producing (4, 1, 1)) would silently skew every
/// multi-operand column's measure.
#[test]
fn split_budget_reaches_every_composition() {
    let mut rng = cell_rng(0xc0de, "split-reach", 6, 3);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..2_000 {
        seen.insert(split_budget(6, 3, &mut rng));
    }
    // C(5, 2) = 10 compositions of 6 into 3 positive parts.
    assert_eq!(
        seen.len(),
        10,
        "all compositions of 6 into 3 must be reachable"
    );
}
