//! Bucketed inspection of the calibration corpus: per band key (kernel ×
//! outcome), per-decade median fuel-per-bit, the fit-free view of the same
//! samples the fitter sees.
//!
//! A log-log OLS slope over heteroscedastic per-step samples can read above
//! 1 while every decade's *median* cost per bit is flat or falling; this
//! tool prints the medians so a suspicious slope can be ground-truthed
//! before it is pinned or reported. Deterministic: same corpus as
//! `calibrate` at the same program count.
//!
//! Usage: `diag [programs] [kernel-substring] [family-substring]` — the
//! third argument restricts the corpus to families whose `Debug` rendering
//! contains the substring, separating a per-shape law from a family-mixture
//! artifact (different shapes dominating different size buckets).

use std::collections::BTreeMap;

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;

use fuzzfit_harness::drive::run_program;
use fuzzfit_harness::strategies::{any_family, build};

fn main() {
    let programs: usize = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("programs must be a number"))
        .unwrap_or(384);
    let filter = std::env::args().nth(2).unwrap_or_default();
    let family_filter = std::env::args().nth(3).unwrap_or_default();

    let mut runner = TestRunner::deterministic();
    let strategy = any_family();
    let mut by_key: BTreeMap<(&'static str, bool), Vec<(u64, u64)>> = BTreeMap::new();
    for case in 0..programs {
        let family = strategy
            .new_tree(&mut runner)
            .expect("family strategy cannot fail")
            .current();
        if !family_filter.is_empty() && !format!("{family:?}").contains(&family_filter) {
            continue;
        }
        let program = build(&family, case as u64);
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed program from {family:?}: {}", m.op));
        for s in samples {
            by_key
                .entry((s.kernel, s.rejected))
                .or_default()
                .push((s.denom_bits, s.fuel));
        }
    }

    for (&(kernel, rejected), samples) in &by_key {
        if !filter.is_empty() && !kernel.contains(&filter) {
            continue;
        }
        // Buckets of half a decade over the denominator.
        let mut buckets: BTreeMap<u32, Vec<f64>> = BTreeMap::new();
        for &(d, f) in samples {
            let key = (2.0 * (d.max(1) as f64).log10()).floor() as u32;
            buckets.entry(key).or_default().push(f as f64 / d as f64);
        }
        println!(
            "{kernel}{} ({} samples)",
            if rejected { " [err]" } else { "" },
            samples.len()
        );
        for (key, mut ratios) in buckets {
            ratios.sort_by(f64::total_cmp);
            let median = ratios[ratios.len() / 2];
            let p90 = ratios[(ratios.len() * 9) / 10];
            let max = ratios[ratios.len() - 1];
            println!(
                "  denom 10^{:>4.1}: n={:6}  fuel/bit median {:>10.1}  p90 {:>10.1}  max {:>10.1}",
                key as f64 / 2.0,
                ratios.len(),
                median,
                p90,
                max
            );
        }
    }
}
