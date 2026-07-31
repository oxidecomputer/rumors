use crate::ops::ROSTER;
use crate::sample::cell_rng;

use super::{run_op, slice_arity, split_budget, Plan, Samplers};

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
/// arity-and-composition draw, the distinct-pair rejection, and the
/// three-way split with in-guest fork preparation.
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
    ] {
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

/// The slice arity draw reaches every count the budget can feed.
///
/// Uniform over `1..=total`, so at a small total every arity appears
/// across a modest draw budget — a draw that silently skipped an arity
/// band would skew every slice column's measure and hide the fold's
/// boundary arities (the drain first combines at 3, the merged–merged
/// carry at 4, the drain of two merged groups at 6: all reachable only
/// if no count is skipped).
#[test]
fn slice_arity_reaches_every_count() {
    let mut rng = cell_rng(0xc0de, "arity-reach", 9, 0);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..2_000 {
        seen.insert(slice_arity(9, &mut rng));
    }
    assert_eq!(
        seen,
        (1..=9).collect::<std::collections::BTreeSet<_>>(),
        "every arity in 1..=9 must be reachable"
    );
}

/// The slice arity draw is uniform over `1..=total`, not merely total.
///
/// The module doc claims *uniform* stratification (equal representation
/// per arity), and the atlas's slice-column measure rests on it — but a
/// reachability check alone cannot pin uniformity: a biased-but-total
/// draw (e.g. the min of two `gen_range` draws, `P(1) = 17/81` vs the
/// uniform `1/9` at total 9, chi-square 723.2 at the committed seed
/// over 2000 draws) reaches
/// every count in the same budget and passes it. This one-sided
/// chi-square pin (8 degrees of freedom, mean 8 + 6σ = 32, the sampler
/// pins' own 6σ idiom) reads red on any such skew while a fixed seed
/// keeps the run deterministic.
#[test]
fn slice_arity_draws_uniformly_over_every_count() {
    const TOTAL: usize = 9;
    const DRAWS: usize = 2_000;
    let mut rng = cell_rng(0xc0de, "arity-uniformity", TOTAL, 0);
    let mut observed = [0u64; TOTAL];
    for _ in 0..DRAWS {
        observed[slice_arity(TOTAL, &mut rng) - 1] += 1;
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
