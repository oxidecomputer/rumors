//! Generator sanity: the strategies' structural invariants, judged natively
//! (no guest required), so a generator bug fails here before it can confuse
//! a fuel reading.

use std::collections::BTreeSet;

use proptest::prelude::*;

use fuzzfit_harness::bands::{judge_against, Band, Verdict, BANDS};
use fuzzfit_harness::ops::{Mirror, Op};
use fuzzfit_harness::strategies::{any_family, budget_for, build};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every generated program respects its family's budget: op count,
    /// total ticks, total forks, and fold width never exceed
    /// [`budget_for`]'s caps, whatever the dimensions drawn.
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

/// The pinned bands and the op vocabulary name the same kernels, pinned
/// here as an expectation list: every roster kernel has at least one
/// pinned band, and every pinned band prices a roster kernel.
///
/// A kernel added without a re-pin, and a band orphaned by a kernel
/// removal or rename, each fail by name in a diff a reviewer sees (the
/// `REFIT_COVERAGE` pattern) — before any generator has to happen to
/// sample the hole. The roster is one representative op per `Op`
/// variant: a variant added to the vocabulary belongs in this list, and
/// its kernel in the pinned bands.
#[test]
fn bands_and_op_roster_name_the_same_kernels() {
    let roster: Vec<Op> = vec![
        Op::ClockSeed { dst: 0 },
        Op::ClockTick { c: 0 },
        Op::ClockFork { dst: 0, src: 0 },
        Op::ClockJoin { a: 0, b: 0 },
        Op::ClockSend { c: 0 },
        Op::ClockRecv { c: 0, v: 0 },
        Op::ClockSync { a: 0, b: 0 },
        Op::ClockOwnVersion { dst: 0, src: 0 },
        Op::ClockVersion { dst: 0, src: 0 },
        Op::ClockIntoParts {
            dst_p: 0,
            dst_v: 0,
            src: 0,
        },
        Op::ClockFromParts { dst: 0, p: 0, v: 0 },
        Op::ClockEncode { src: 0 },
        Op::ClockDecode { dst: 0 },
        Op::VersionTick { v: 0, p: 0 },
        Op::VersionJoin { dst: 0, a: 0, b: 0 },
        Op::VersionMeet { dst: 0, a: 0, b: 0 },
        Op::VersionProject { dst: 0, v: 0, p: 0 },
        Op::VersionCmp { a: 0, b: 0 },
        Op::VersionConcurrent { a: 0, b: 0 },
        Op::VersionRank { dst: 0, src: 0 },
        Op::VersionDistance { dst: 0, a: 0, b: 0 },
        Op::VersionLag { dst: 0, a: 0, b: 0 },
        Op::VersionMinTicks { src: 0 },
        Op::VersionJoinAll {
            dst: 0,
            src: 0,
            n: 0,
        },
        Op::VersionMeetAll {
            dst: 0,
            src: 0,
            n: 0,
        },
        Op::VersionEncode { src: 0 },
        Op::VersionDecode { dst: 0 },
        Op::VersionDisplay { src: 0 },
        Op::VersionFromstr { dst: 0 },
        Op::PartySeed { dst: 0 },
        Op::PartyFork { dst: 0, src: 0 },
        Op::PartyForks {
            dst: 0,
            src: 0,
            n: 0,
        },
        Op::PartyJoin { a: 0, b: 0 },
        Op::PartyIsDisjoint { a: 0, b: 0 },
        Op::PartyCovers { a: 0, b: 0 },
        Op::PartyWithout { dst: 0, a: 0, b: 0 },
        Op::PartyEncode { src: 0 },
        Op::PartyDecode { dst: 0 },
        Op::PartyDisplay { src: 0 },
        Op::PartyFromstr { dst: 0 },
        Op::RankAdd { dst: 0, a: 0, b: 0 },
        Op::RankCmp { a: 0, b: 0 },
        Op::RankCheckedSub { dst: 0, a: 0, b: 0 },
        Op::RankDisplay { src: 0 },
    ];
    let kernels: BTreeSet<&'static str> = roster.iter().map(Op::kernel).collect();
    for kernel in &kernels {
        assert!(
            BANDS.iter().any(|band| band.kernel == *kernel),
            "kernel {kernel} has no pinned band: \
             re-pin with `just fuzzfit-calibrate` and commit src/bands.rs"
        );
    }
    for band in BANDS {
        assert!(
            kernels.contains(band.kernel),
            "pinned band {} prices no roster kernel: a stale pin or a roster hole; \
             re-pin with `just fuzzfit-calibrate` or extend the roster above",
            band.kernel
        );
    }
}

/// The judgment's own tripwire: against a pinned-linear band, a quadratic
/// fuel reading at scale must read `Above` and a dead-meter reading must
/// read `Below` — the two flags the enforcement leg exists to raise.
///
/// A judgment that passes either is decoration, so this fails the suite
/// before any fuzzing runs.
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
