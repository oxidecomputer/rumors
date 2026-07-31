//! Answer-embedded shapes for the measure folds (`min_ticks`, `rank`).
//!
//! Inputs whose *answer* is a wide counter while the attacker-controlled
//! knob is a long tail of tiny deltas riding it — a coupling no board
//! family drives (the committed families vary one axis at a time).
//!
//! Two constructed families, both through the public API only:
//!
//! - **wide-base tiny-tail** `WT(n, w)`: one `ticks(seed, 10^w)` base
//!   raise over the whole id space, then `n` forked parties tick once
//!   each on alternating leaves. Every leaf height and every subtree
//!   minimum embeds the wide answer; the fold must not re-touch the
//!   wide component per leaf (the freeze/epoch-ledger discipline's
//!   funding claim, attacked at its answer-embedded corner).
//! - **wide ladder** `WL(n, w)`: `n` forked parties each tick
//!   `10^w + i` — near-equal wide subtree minima, forcing the min web
//!   to discriminate wide values that differ only at the low end.
//!
//! The tripwire is the *mixed second difference* over a 2x2 (n, w)
//! grid: for an additive cost `a*n + b*w` it vanishes; for a product
//! law `c*n*w` it is a quarter of the top cell. Bounding it at a tenth
//! of the top cell refutes any n x w coupling while tolerating
//! amortization wobble.

#![cfg(all(feature = "meter", feature = "scan-meter", feature = "limb-meter"))]

use std::str::FromStr;

use before::{meter, Party, Ticks, Version};

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

/// `n` parties tiling the seed's space, by repeated fork.
fn fork_parties(n: usize) -> Vec<Party> {
    fork_parties_from(Party::seed(), n)
}

/// `n` parties tiling the given party's space, by repeated fork.
fn fork_parties_from(seed: Party, n: usize) -> Vec<Party> {
    let mut parties = vec![seed];
    while parties.len() < n {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    parties
}

/// `10^w` as a tick count.
fn wide(w: usize) -> Ticks {
    Ticks::from_str(&format!("1{}", "0".repeat(w))).expect("a digit run parses")
}

/// Wide-base tiny-tail: one wide raise over the whole space, then `n`
/// single ticks on alternating forked leaves.
fn wt(n: usize, w: usize) -> Version {
    let mut v = Version::new();
    // One wide raise over the whole space while the seed still owns it:
    // a single wide root delta, before the space is carved into leaves.
    let seed = Party::seed();
    v.ticks(&seed, wide(w));
    let parties = fork_parties_from(seed, n);
    for p in parties.iter().step_by(2) {
        v.tick(p);
    }
    v
}

/// Wide ladder: `n` forked parties each raise their own leaf by a
/// near-equal wide count (`10^w + i`).
fn wl(n: usize, w: usize) -> Version {
    let parties = fork_parties(n);
    let mut v = Version::new();
    let base = wide(w);
    let mut bump = Ticks::ZERO;
    for p in &parties {
        v.ticks(p, &base + &bump);
        bump = &bump + &Ticks::from(1u64);
    }
    v
}

/// One grid cell: (scan, limb, touch) for both measure folds plus the
/// packed size.
fn measure(v: &Version) -> ((u64, u64, u64), (u64, u64, u64), usize) {
    let mt = counters(|| {
        let _ = v.min_ticks();
    });
    let rk = counters(|| {
        let _ = v.rank();
    });
    (mt, rk, v.encode().len())
}

/// The mixed second difference must stay under a tenth of the top
/// cell: no `n x w` product term in the fold's cost.
fn assert_no_product(name: &str, op: &str, label: &str, t: [u64; 4]) {
    // t = [f(n,w), f(2n,w), f(n,2w), f(2n,2w)]
    let mixed = t[3] as f64 - t[2] as f64 - t[1] as f64 + t[0] as f64;
    let bound = 0.10 * t[3] as f64;
    eprintln!("MEASURED {name}/{op}/{label}: grid {t:?} mixed {mixed:.0} bound {bound:.0}");
    assert!(
        mixed.abs() <= bound,
        "{name}/{op}/{label}: mixed second difference {mixed:.0} exceeds {bound:.0} — \
         an n x w product term in an answer-embedded fold"
    );
}

/// Neither measure fold couples the tail length to the answer width on
/// the wide-base tiny-tail family: the wide component is paid once,
/// not per leaf.
#[test]
fn measure_folds_are_additive_on_wide_base_tiny_tail() {
    let (n0, w0) = (128usize, 2000usize);
    let grid = [
        wt(n0, w0),
        wt(2 * n0, w0),
        wt(n0, 2 * w0),
        wt(2 * n0, 2 * w0),
    ];
    let cells: Vec<_> = grid.iter().map(measure).collect();
    for (i, c) in cells.iter().enumerate() {
        eprintln!(
            "MEASURED wt cell{i}: bytes {} min_ticks {:?} rank {:?}",
            c.2, c.0, c.1
        );
    }
    for (op, pick) in [("min_ticks", 0usize), ("rank", 1usize)] {
        for (label, idx) in [("scan", 0usize), ("limb", 1), ("touch", 2)] {
            let t: Vec<u64> = cells
                .iter()
                .map(|c| {
                    let trio = if pick == 0 { c.0 } else { c.1 };
                    [trio.0, trio.1, trio.2][idx]
                })
                .collect();
            assert_no_product("wt", op, label, [t[0], t[1], t[2], t[3]]);
        }
    }
}

/// Both measure folds stay per-input-byte flat on the wide-ladder
/// family.
///
/// Near-equal wide subtree minima are each discriminated within the
/// input bits that spelled them. Unlike the wide-base family, here the
/// *input itself* carries the n x w product (every rung stores its own
/// wide count), so the honest check is flatness per packed byte across
/// all four grid cells.
#[test]
fn measure_folds_are_flat_per_byte_on_wide_ladder() {
    let (n0, w0) = (64usize, 1500usize);
    let grid = [
        wl(n0, w0),
        wl(2 * n0, w0),
        wl(n0, 2 * w0),
        wl(2 * n0, 2 * w0),
    ];
    let cells: Vec<_> = grid.iter().map(measure).collect();
    for (i, c) in cells.iter().enumerate() {
        eprintln!(
            "MEASURED wl cell{i}: bytes {} min_ticks {:?} rank {:?}",
            c.2, c.0, c.1
        );
    }
    for (op, pick) in [("min_ticks", 0usize), ("rank", 1usize)] {
        for (label, idx) in [("scan", 0usize), ("limb", 1), ("touch", 2)] {
            let per_byte: Vec<f64> = cells
                .iter()
                .map(|c| {
                    let trio = if pick == 0 { c.0 } else { c.1 };
                    [trio.0, trio.1, trio.2][idx] as f64 / c.2 as f64
                })
                .collect();
            let base = per_byte[0];
            eprintln!("MEASURED wl/{op}/{label}: per-byte {per_byte:?}");
            for (i, r) in per_byte.iter().enumerate() {
                assert!(
                    *r <= base * 1.10,
                    "wl/{op}/{label}: cell{i} per-byte {r:.3} grows past base {base:.3} x1.10"
                );
            }
        }
    }
}
