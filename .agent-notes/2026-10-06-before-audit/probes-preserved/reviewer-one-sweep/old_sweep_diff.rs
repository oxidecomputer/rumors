//! Reviewer experiment (never committed): the two-copy sweep from the base
//! commit, verbatim, compared byte for byte against the one-sweep code.
//!
//! `OLD_SWEEP_MUTANT` selects a deliberate defect in the *old* copy, so one
//! build proves the comparison can fail: `depth` opens the old `emit` at the
//! shallower cursor's depth; `relation` stops the old `hull_bits` folding
//! loop signs into its relation; `sticky` makes the old `emit` re-pick from
//! side A at ties.

use core::cmp::Ordering;

use proptest::prelude::*;
use rayon::prelude::*;

use super::{Extreme, Hull, Side};
use crate::testing::bridge::from_oracle_version;
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::meter::registry::Shape;
use crate::testing::{generators, optrace};
use crate::version::order::OrderState;
use crate::version::overlay::{advance_diff, OpenedPair};
use crate::version::io::regions::RegionReader;
use crate::version::io::writer::VersionWriter;
use crate::{Clock, Version};

fn mutant() -> Option<String> {
    std::env::var("OLD_SWEEP_MUTANT").ok()
}

/// The base commit's `Version::hull_bits`, verbatim except for the mutant hook.
fn old_hull_bits(this: &Version, other: &Version) -> Hull {
    struct Emission {
        extreme: Extreme,
        side: Side,
        out: VersionWriter,
    }
    let skip_loop_fold = mutant().as_deref() == Some("relation");

    let OpenedPair {
        a: mut cursor_a,
        b: mut cursor_b,
        mut diff,
        a_first,
        b_first,
    } = OpenedPair::open(this, other);

    let mut directions = OrderState::new();
    let sign = diff.cmp_zero();
    directions.fold(sign);

    let mut outputs = [
        Emission {
            extreme: Extreme::Lower,
            side: Side::A,
            out: VersionWriter::with_capacity(this.stored_len() + other.stored_len()),
        },
        Emission {
            extreme: Extreme::Higher,
            side: Side::A,
            out: VersionWriter::with_capacity(this.stored_len() + other.stored_len()),
        },
    ];
    for emission in &mut outputs {
        emission.side = emission.extreme.pick(sign, Side::A);
        let first = match emission.side {
            Side::A => &a_first,
            Side::B => &b_first,
        };
        emission
            .out
            .height(cursor_a.depth().max(cursor_b.depth()), first);
    }
    drop(a_first);
    drop(b_first);

    while !(cursor_a.done() && cursor_b.done()) {
        let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
        let sign = diff.cmp_zero();
        if !skip_loop_fold {
            directions.fold(sign);
        }
        let depth = cursor_a.depth().max(cursor_b.depth());
        for emission in &mut outputs {
            let new_side = emission.extreme.pick(sign, emission.side);
            let old_side = emission.side;
            emission.side = new_side;
            old_side.write_delta(
                &mut emission.out,
                depth,
                &diff,
                new_side,
                step_a.as_ref(),
                step_b.as_ref(),
            );
        }
    }

    let [lo, hi] = outputs;
    Hull {
        relation: directions.relation(),
        lo: lo.out.finish(),
        hi: hi.out.finish(),
    }
}

/// The base commit's `Extreme::emit`, verbatim except for the mutant hook.
fn old_emit(this: Extreme, a_bits: &Version, b_bits: &Version) -> Version {
    let m = mutant();
    let OpenedPair {
        a: mut cursor_a,
        b: mut cursor_b,
        mut diff,
        a_first,
        b_first,
    } = OpenedPair::open(a_bits, b_bits);

    let mut side = this.pick(diff.cmp_zero(), Side::A);
    let mut out = VersionWriter::with_capacity(a_bits.stored_len() + b_bits.stored_len());
    let first = match side {
        Side::A => &a_first,
        Side::B => &b_first,
    };
    let open_depth = if m.as_deref() == Some("depth") {
        cursor_a.depth().min(cursor_b.depth())
    } else {
        cursor_a.depth().max(cursor_b.depth())
    };
    out.height(open_depth, first);
    drop(a_first);
    drop(b_first);

    while !(cursor_a.done() && cursor_b.done()) {
        let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
        let current = if m.as_deref() == Some("sticky") { Side::A } else { side };
        let new_side = this.pick(diff.cmp_zero(), current);
        let old_side = side;
        side = new_side;
        old_side.write_delta(
            &mut out,
            cursor_a.depth().max(cursor_b.depth()),
            &diff,
            new_side,
            step_a.as_ref(),
            step_b.as_ref(),
        );
    }

    out.finish()
}

/// Compare old and new on one ordered pair and its reverse, directly through
/// the internal entries (so empty and equal operands, which the public
/// operations short-circuit, reach the sweep too).
fn check(a: &Version, b: &Version) {
    for (x, y) in [(a, b), (b, a)] {
        let new = x.hull_bits(y);
        let old = old_hull_bits(x, y);
        assert_eq!(new.relation, old.relation, "hull relation: {x:?} vs {y:?}");
        assert_eq!(new.lo.as_bytes(), old.lo.as_bytes(), "hull lo: {x:?} vs {y:?}");
        assert_eq!(new.hi.as_bytes(), old.hi.as_bytes(), "hull hi: {x:?} vs {y:?}");
        for extreme in [Extreme::Lower, Extreme::Higher] {
            let new = extreme.emit(x, y);
            let old = old_emit(extreme, x, y);
            let name = match extreme {
                Extreme::Lower => "Lower",
                Extreme::Higher => "Higher",
            };
            assert_eq!(new.as_bytes(), old.as_bytes(), "emit {name}: {x:?} vs {y:?}");
        }
    }
}

/// Check a pair plus the dominated shapes its join and meet supply.
fn check_closed(a: &Version, b: &Version) {
    check(a, b);
    let joined = Extreme::Higher.emit(a, b);
    let met = Extreme::Lower.emit(a, b);
    check(a, &joined);
    check(&met, b);
    check(&met, &joined);
}

#[test]
fn old_new_exhaustive_small_scope() {
    let pool: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let pairs = std::sync::atomic::AtomicU64::new(0);
    pool.par_iter().for_each(|a| {
        for b in &pool {
            check(a, b);
            pairs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });
    eprintln!(
        "old_new_exhaustive_small_scope: pool {} pairs {}",
        pool.len(),
        pairs.into_inner()
    );
}

#[test]
fn old_new_families() {
    let pool = vec![
        Version::new(),
        Shape::Dense.build1(1).version(),
        Shape::Dense.build1(2).version(),
        Shape::Dense.build1(64).version(),
        Shape::Dense.build1(512).version(),
        Shape::Bigroot.build2(7, 3).version(),
        Shape::Bigroot.build2(64, 16).version(),
        Shape::Hugeleaf.build1(1).version(),
        Shape::Hugeleaf.build1(64).version(),
        Shape::Hugeleaf.build1(600).version(),
        Shape::CliffComb.build2(3, 2).version(),
        Shape::CliffComb.build2(16, 16).version(),
        Shape::WideToothComb.build3(16, 8, 8).version(),
        Shape::CliffFan.build2(16, 8).version(),
        Shape::CancellingChain.build2(16, 8).version(),
        Shape::AltSpine.build1(3).version(),
        Shape::AltSpine.build1(64).version(),
        Shape::Staircase.build1(16).version(),
        Shape::Harmonic.build1(16).version(),
    ];
    pool.par_iter().for_each(|a| {
        for b in &pool {
            check_closed(a, b);
        }
    });
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(256),
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    #[test]
    fn old_new_arbitrary_pairs(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        check_closed(&from_oracle_version(&a), &from_oracle_version(&b));
    }

    #[test]
    fn old_new_organic_histories(ops in optrace::world_strategy_up_to(60)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for a in &clocks {
            for b in &clocks {
                check(a.version(), b.version());
            }
        }
    }
}

/// Sanity: the comparison sees every relation arm, so a relation defect can
/// propagate to an assertion.
#[test]
fn old_new_relation_census() {
    let pool: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let mut seen = [0u64; 4];
    for a in pool.iter().step_by(7) {
        for b in &pool {
            let idx = match a.hull_bits(b).relation {
                Some(Ordering::Less) => 0,
                Some(Ordering::Equal) => 1,
                Some(Ordering::Greater) => 2,
                None => 3,
            };
            seen[idx] += 1;
        }
    }
    eprintln!("relation census [Less, Equal, Greater, None]: {seen:?}");
    assert!(seen.iter().all(|&n| n > 0), "every relation arm occurs");
}
