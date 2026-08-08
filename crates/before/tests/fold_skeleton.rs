//! The hull fold on a shared deep skeleton: `span_all` over a
//! population whose *every* operand is a full-size adversarial spine.
//!
//! The board's fold rows ride the committed fold populations (scattered
//! single-tick forks, woven fork-tree leaves, staggered combs) — all
//! small operands, so the `O(D log k)` model's `D` axis is exercised
//! there by arity, not by operand depth. Here `D` itself doubles at
//! fixed arity: sixteen forks of one deep dense spine, each ticked
//! once, so meets never shrink and every combine carries the whole
//! skeleton. Per-byte flatness across the doubling is the model's `D`
//! leg on the one population shape the committed fold families leave
//! out.

#![cfg(all(feature = "meter", feature = "scan-meter", feature = "limb-meter"))]

use before::meter::registry::Shape;
use before::{meter, Party, Version};

/// One counter snapshot of a closure run.
fn counters(f: impl FnOnce()) -> (u64, u64, u64) {
    meter::reset_scan_bits();
    meter::reset_limb_ops();
    suanpan::touch_meter::reset();
    f();
    (
        meter::scan_bits(),
        meter::limb_ops(),
        suanpan::touch_meter::touches(),
    )
}

/// Growth of a per-byte reading across the level doubling.
fn growth(c0: u64, b0: usize, c1: u64, b1: usize) -> f64 {
    if c0 == 0 {
        return 1.0;
    }
    (c1 as f64 / b1 as f64) / (c0 as f64 / b0 as f64)
}

/// The per-byte flatness band across a doubling: the meter suite's
/// ×1.25 flatness convention with margin for amortization wobble.
const GROWTH_BOUND: f64 = 1.35;

/// `span_all` over a population sharing one deep adversarial skeleton
/// (every operand full-size, meets never shrink) stays per-byte flat
/// at fixed arity when the skeleton doubles.
#[test]
fn span_all_flat_on_shared_deep_skeleton() {
    let build = |lvl: u32| -> Vec<Version> {
        let base = Shape::Dense.packed1(2000 << lvl).version();
        let mut parties = vec![Party::seed()];
        while parties.len() < 16 {
            let mut next = Vec::new();
            for mut p in parties {
                let q = p.fork();
                next.push(p);
                next.push(q);
            }
            parties = next;
        }
        parties
            .iter()
            .map(|p| {
                let mut v = base.clone();
                v.tick(p);
                v
            })
            .collect()
    };
    let mut readings = Vec::new();
    for lvl in 0..2 {
        let pop = build(lvl);
        let bytes: usize = pop.iter().map(|v| v.encode().len()).sum();
        let (first, rest) = pop.split_first().expect("nonempty");
        let c = counters(|| {
            let _ = first.span_all(rest);
        });
        eprintln!(
            "MEASURED span_all_skeleton lvl{lvl}: bytes {bytes} scan {} limb {} touch {}",
            c.0, c.1, c.2
        );
        readings.push((c, bytes));
    }
    let ((c0, b0), (c1, b1)) = (readings[0], readings[1]);
    for (label, g) in ["scan", "limb", "touch"].iter().zip([
        growth(c0.0, b0, c1.0, b1),
        growth(c0.1, b0, c1.1, b1),
        growth(c0.2, b0, c1.2, b1),
    ]) {
        eprintln!("MEASURED span_all_skeleton growth {label}: {g:.3}");
        assert!(
            g <= GROWTH_BOUND,
            "span_all/{label}: per-byte growth {g:.3} across the skeleton \
             doubling exceeds {GROWTH_BOUND}"
        );
    }
}
