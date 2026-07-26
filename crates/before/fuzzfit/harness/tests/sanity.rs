//! Generator sanity: the strategies' structural invariants, judged natively
//! (no guest required), so a generator bug fails here before it can confuse
//! a fuel reading.

use proptest::prelude::*;

use fuzzfit_harness::bands::{judge_against, Band, Verdict};
use fuzzfit_harness::ops::{Mirror, Op};
use fuzzfit_harness::strategies::{any_family, build, BUDGET};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every generated program respects the budget: op count, total ticks,
    /// total forks, and fold width never exceed [`BUDGET`], whatever the
    /// family and dimensions drawn.
    #[test]
    fn programs_respect_the_budget(family in any_family(), seed in any::<u64>()) {
        let program = build(&family, seed);
        prop_assert!(program.len() <= BUDGET.max_ops, "{} ops", program.len());
        let mut ticks = 0u32;
        let mut forks = 0u32;
        for op in &program {
            match op {
                Op::ClockTick { .. } | Op::VersionTick { .. } => ticks += 1,
                Op::ClockFork { .. } | Op::PartyFork { .. } => forks += 1,
                Op::PartyForks { n, .. } => forks += n,
                Op::VersionJoinAll { n, .. } | Op::VersionMeetAll { n, .. } => {
                    prop_assert!(*n <= BUDGET.max_fold, "fold width {n}");
                }
                _ => {}
            }
        }
        prop_assert!(ticks <= BUDGET.max_ticks, "{ticks} ticks");
        prop_assert!(forks <= BUDGET.max_forks, "{forks} forks");
    }

    /// Every generated program is well-formed: the native mirror executes
    /// it end to end without a register-file violation, i.e. the builder's
    /// liveness model matches real consumption (linearity by construction).
    #[test]
    fn programs_are_well_formed(family in any_family(), seed in any::<u64>()) {
        let program = build(&family, seed);
        let mut mirror = Mirror::new();
        for op in &program {
            let step = mirror.step(op);
            prop_assert!(step.is_ok(), "malformed op {:?}", op);
            prop_assert!(step.expect("checked").denom_bits >= 1);
        }
        // Every live register's canonical bytes round-trip through the
        // codec: the constructed values are honest packed values.
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

    /// Program generation is a pure function of (family, seed): the replay
    /// determinism the enforcement leg's shrinking rests on.
    #[test]
    fn generation_is_deterministic(family in any_family(), seed in any::<u64>()) {
        prop_assert_eq!(build(&family, seed), build(&family, seed));
    }
}

/// The judgment's own tripwire: against a pinned-linear band, a quadratic
/// fuel reading at scale must read `Above` and a dead-meter reading must
/// read `Below` — the two flags the enforcement leg exists to raise. A
/// judgment that passes either is decoration, so this fails the suite
/// before any fuzzing runs.
#[test]
fn judgment_flags_quadratic_and_dead_readings() {
    // A synthetic linear band: fuel ≈ 100 · d (slope 1, intercept 2),
    // width ±0.3, calibrated over 10³..10⁶ bits.
    let band = Band {
        kernel: "synthetic_linear",
        slope: 1.0,
        intercept: 2.0,
        width: 0.3,
        min_denom: 1_000,
        max_denom: 1_000_000,
        samples: 1000,
        constant: false,
    };
    // In-band: an honest linear reading, and one at the extrapolated top.
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
