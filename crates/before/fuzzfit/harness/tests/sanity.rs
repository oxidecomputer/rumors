//! Native checks of generated program structure and fuel-band judgment.

use proptest::prelude::*;

use fuzzfit_harness::bands::{judge_against, Band, Verdict};
use fuzzfit_harness::ops::{Mirror, Op};
use fuzzfit_harness::strategies::{any_family, budget_for, build};

proptest! {
    /// Every generated program stays within its operation and width budgets.
    #[test]
    fn programs_respect_the_budget(family in any_family(), seed in any::<u64>()) {
        let budget = budget_for(&family);
        let program = build(&family, seed);
        prop_assert!(program.len() <= budget.max_ops, "{} ops", program.len());
        let mut ticks = 0u32;
        let mut forks = 0u32;
        for op in &program {
            match op {
                Op::ClockTick { .. } | Op::VersionTick { .. } => ticks += 1,
                Op::ClockFork { .. } | Op::PartyFork { .. } => forks += 1,
                Op::PartyForks { n, .. } => forks += n,
                Op::VersionJoinAll { n, .. } | Op::VersionMeetAll { n, .. } => {
                    prop_assert!(*n <= budget.max_fold, "fold width {n}");
                }
                _ => {}
            }
        }
        prop_assert!(ticks <= budget.max_ticks, "{ticks} ticks");
        prop_assert!(forks <= budget.max_forks, "{forks} forks");
    }

    /// Every generated program executes in the native mirror without misuse.
    ///
    /// All remaining registers must also contain canonical encoded values.
    #[test]
    fn programs_are_well_formed(family in any_family(), seed in any::<u64>()) {
        let program = build(&family, seed);
        let mut mirror = Mirror::new();
        for op in &program {
            let step = mirror.step(op);
            prop_assert!(step.is_ok(), "malformed op {:?}", op);
            prop_assert!(step.expect("checked").denom_bits >= 1);
        }
        // Every live register's bytes must be canonical for its value type.
        for (reg, tag) in mirror.live_regs() {
            let bytes = mirror.snapshot(reg).expect("live");
            match tag {
                b'v' => {
                    let v = before::Version::decode(bytes.as_slice());
                    prop_assert!(v.is_ok(), "version r{reg} does not round-trip");
                    prop_assert_eq!(v.expect("checked").encode(), bytes);
                }
                b'p' => {
                    let p = before::Party::decode(bytes.as_slice());
                    prop_assert!(p.is_ok(), "party r{reg} does not round-trip");
                    prop_assert_eq!(p.expect("checked").encode(), bytes);
                }
                b'c' => {
                    let c = before::Clock::decode(bytes.as_slice());
                    prop_assert!(c.is_ok(), "clock r{reg} does not round-trip");
                    prop_assert_eq!(c.expect("checked").encode(), bytes);
                }
                b'r' => prop_assert!(!bytes.is_empty()),
                _ => unreachable!("mirror tags are v/p/c/r"),
            }
        }
    }

    /// A family and seed determine the generated program exactly.
    #[test]
    fn generation_is_deterministic(family in any_family(), seed in any::<u64>()) {
        prop_assert_eq!(build(&family, seed), build(&family, seed));
    }
}

/// Band judgment rejects quadratic growth and a constant dead-meter reading.
#[test]
fn judgment_flags_quadratic_and_dead_readings() {
    // A synthetic linear band: fuel ≈ 100 · d (slope 1, intercept 2),
    // width +0.3/-0.3, calibrated over 10³..10⁶ bits.
    let band = Band {
        kernel: "synthetic_linear",
        rejected: false,
        slope: 1.0,
        intercept: 2.0,
        width_above: 0.3,
        width_below: 0.3,
        min_denom: 1_000,
        max_denom: 1_000_000,
        samples: 1000,
        constant: false,
    };
    // Linear readings remain in band, including above the calibration range.
    assert_eq!(judge_against(&band, 10_000, 1_000_000), Verdict::InBand);
    assert_eq!(
        judge_against(&band, 100_000_000, 10_000_000_000),
        Verdict::InBand
    );
    // The quadratic mechanism: fuel = d² / 10⁴ crosses the ceiling well
    // inside the calibrated range and reads Above from there up.
    assert_eq!(
        judge_against(&band, 1_000_000, 100_000_000_000),
        Verdict::Above
    );
    // The dead meter: nop-level fuel on a large case reads Below.
    assert_eq!(judge_against(&band, 1_000_000, 2), Verdict::Below);
    // Below the calibrated floor: not judged.
    assert_eq!(judge_against(&band, 999, 1), Verdict::BelowFloor);
}
