//! L3 audit heap probes (explore branch only).
//!
//! Installs a counting global allocator in the lib test binary and reads the
//! peak live heap of one `tick` above its baseline, across sizes, on families
//! built to maximize memo links and suspended levels per input bit.

use num_bigint::BigUint;
use peak_alloc::PeakAlloc;

use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::oracles::tree::{Party as OP, Version as OV};
use crate::{Party, Version};

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn leaf(h: u64) -> OV {
    OV::Leaf(BigUint::from(h))
}

/// Sibling sites under one outermost lookahead: a balanced tree of ordinary
/// nodes whose leaves are minimal sites `(1, (1, 0))` over `(L, c)`, with `c`
/// alternating so each sibling link is nonzero (or constant for the control).
fn siblings(n: usize, alternate: bool) -> (OP, OV) {
    fn rec(lo: usize, hi: usize, alternate: bool) -> (OP, OV) {
        if hi - lo == 1 {
            let c = if alternate { 1 + (lo as u64 % 2) } else { 1 };
            let p = OP::node(OP::Leaf(true), OP::node(OP::Leaf(true), OP::Leaf(false)));
            let v = OV::node(0u64, leaf(0), leaf(c));
            return (p, v);
        }
        let mid = (lo + hi) / 2;
        let (pl, vl) = rec(lo, mid, alternate);
        let (pr, vr) = rec(mid, hi, alternate);
        (OP::node(pl, pr), OV::node(0u64, vl, vr))
    }
    let (px, vx) = rec(0, n, alternate);
    (OP::node(OP::Leaf(true), px), OV::node(0u64, leaf(0), vx))
}

/// A chain of nested sites, each beside a lower unowned leaf, so every
/// level's minimum differs from its enclosing one.
///
/// Level k: party `(1, (S_{k+1}, 0))`... built inside-out. Depth `n`.
fn chain(n: usize) -> (OP, OV) {
    // Innermost: a site over (L, c).
    let mut p = OP::node(OP::Leaf(true), OP::node(OP::Leaf(true), OP::Leaf(false)));
    let mut v = OV::node(0u64, leaf(n as u64 + 5), leaf(n as u64 + 2));
    for k in (0..n).rev() {
        // Range R_k = node(site_{k+1}, e_k) with e_k = k + 1, under party
        // (site, 0); the site at level k is (1, R_k) over (L_k, R_k).
        let range_p = OP::node(p, OP::Leaf(false));
        let range_v = OV::node(0u64, v, leaf(k as u64 + 1));
        p = OP::node(OP::Leaf(true), range_p);
        v = OV::node(0u64, leaf(n as u64 + 5), range_v);
    }
    (p, v)
}

/// A right spine whose preorder leaves rise by one: `(0, (1, (2, …)))`.
///
/// Every nested subtree arms its minimum strictly above its parent's, so a
/// range-minimum tracker holds one positive boundary per level.
fn rising_right_spine(n: usize) -> OV {
    let mut v = leaf(n as u64);
    for k in (0..n).rev() {
        v = OV::node(0u64, leaf(k as u64), v);
    }
    v
}

/// Peak heap of `min_ticks` above its baseline, and the input bytes.
fn measure_min_ticks(v: &Version) -> (usize, usize) {
    let input = v.stored_len().div_ceil(8) as usize;
    HEAP.reset_peak_usage();
    let base = HEAP.current_usage();
    let m = v.min_ticks();
    let peak = HEAP.peak_usage().saturating_sub(base);
    drop(m);
    (peak, input)
}

/// Prints peak-heap readings for `min_ticks` on the rising right spine
/// (diagnostic), at sizes on both sides of powers of two.
#[test]
fn l3_heap_min_ticks_rising_spine() {
    std::thread::Builder::new()
        .stack_size(1 << 31)
        .spawn(|| {
            for k in [10u32, 12, 14, 16, 18] {
                for n in [(1usize << k) - 1, (1 << k) + 1, (1 << k) + (1 << (k - 1))] {
                    let ov = rising_right_spine(n);
                    let v = from_oracle_version(&ov);
                    core::mem::forget(ov);
                    let (peak, input) = measure_min_ticks(&v);
                    eprintln!(
                        "HEAP min_ticks rising-right-spine n={n} input_bytes={input} peak={peak} ratio={:.2}",
                        peak as f64 / input as f64
                    );
                    // The tick walk over the same spine, seed party (one collapse).
                    let (tpeak, tinput, tout) = measure(&v, &Party::seed());
                    eprintln!(
                        "HEAP tick-seed rising-right-spine n={n} input_bytes={tinput} output_bytes={tout} peak={tpeak} ratio={:.2}",
                        tpeak as f64 / tinput as f64
                    );
                }
            }
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// Peak heap of one tick above its baseline, and the input bytes.
fn measure(v: &Version, p: &Party) -> (usize, usize, usize) {
    let input = (v.stored_len() + p.stored_len()).div_ceil(8) as usize;
    let mut w = v.clone();
    HEAP.reset_peak_usage();
    let base = HEAP.current_usage();
    w.tick(p);
    let peak = HEAP.peak_usage().saturating_sub(base);
    let out = w.stored_len().div_ceil(8) as usize;
    drop(w);
    (peak, input, out)
}

fn report(name: &str, build: impl Fn(usize) -> (OP, OV), sizes: &[usize]) {
    for &n in sizes {
        let (op, ov) = build(n);
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        // The oracle trees are deep; leak them rather than drop recursively.
        core::mem::forget((op, ov));
        let (peak, input, out) = measure(&v, &p);
        eprintln!(
            "HEAP {name} n={n} input_bytes={input} output_bytes={out} peak={peak} ratio={:.2}",
            peak as f64 / input as f64
        );
    }
}

/// Prints peak-heap readings for the memo-dense families (diagnostic).
#[test]
fn l3_heap_memo_families() {
    std::thread::Builder::new()
        .stack_size(1 << 31)
        .spawn(|| {
            let sizes = [1 << 10, 1 << 12, 1 << 14, 1 << 16];
            report("siblings-alternating", |n| siblings(n, true), &sizes);
            report("siblings-constant", |n| siblings(n, false), &sizes);
            report("chain", chain, &sizes);
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// Prints `min_ticks` peak heap on the rising right spine at every size within
/// a few levels of each power of two, beside the board's per-sample ceiling
/// `1024 + 20 * input_bytes` (diagnostic).
#[test]
fn l3_heap_min_ticks_rising_spine_fine() {
    std::thread::Builder::new()
        .stack_size(1 << 31)
        .spawn(|| {
            for k in [8u32, 10, 12, 14, 16, 18] {
                for d in 0..=8usize {
                    let n = (1usize << k) - 4 + d;
                    let ov = rising_right_spine(n);
                    let v = from_oracle_version(&ov);
                    core::mem::forget(ov);
                    let encoded = v.encode().len();
                    let (peak, input) = measure_min_ticks(&v);
                    let ceiling = 1024 + 20 * encoded;
                    eprintln!(
                        "FINE min_ticks rising-right-spine n={n} stored_bytes={input} encoded_bytes={encoded} peak={peak} ratio={:.2} board_ceiling={ceiling} {}",
                        peak as f64 / encoded as f64,
                        if peak > ceiling { "RED" } else { "green" }
                    );
                }
            }
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// A right spine whose first leaf is 0 and whose later preorder leaves rise by
/// one from `jump`: `(0, (jump + 1, (jump + 2, …)))`.
///
/// One wide entry code puts every later leaf's offset from the frozen height
/// prefix outside `i32` without crossing the freeze allowance.
fn jump_rising_right_spine(n: usize, jump: &BigUint) -> OV {
    let mut v = OV::Leaf(jump + BigUint::from(n as u64));
    for k in (1..n).rev() {
        v = OV::node(0u64, OV::Leaf(jump + BigUint::from(k as u64)), v);
    }
    OV::node(0u64, leaf(0), v)
}

/// Prints `min_ticks` peak heap on the jump-entered rising spine beside the
/// board's per-sample ceiling (diagnostic).
#[test]
fn l3_heap_min_ticks_jump_rising_spine() {
    std::thread::Builder::new()
        .stack_size(1 << 31)
        .spawn(|| {
            for jump_bits in [16u32, 31, 40, 200] {
                let jump = BigUint::from(1u8) << jump_bits;
                for n in [1usize << 10, 1 << 12, 1 << 14, 1 << 16, (1 << 16) + (1 << 15)] {
                    let ov = jump_rising_right_spine(n, &jump);
                    let v = from_oracle_version(&ov);
                    core::mem::forget(ov);
                    let encoded = v.encode().len();
                    let (peak, _) = measure_min_ticks(&v);
                    let ceiling = 1024 + 20 * encoded;
                    eprintln!(
                        "JUMP min_ticks jump=2^{jump_bits} n={n} encoded_bytes={encoded} peak={peak} ratio={:.2} board_ceiling={ceiling} {}",
                        peak as f64 / encoded as f64,
                        if peak > ceiling { "RED" } else { "green" }
                    );
                }
            }
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// Samples co-generated pairs and prints the tick and `ticks(2^70)` peak heap
/// readings closest to (or over) the board's per-sample ceiling
/// `1024 + 20 * input_bytes` (diagnostic; always passes).
#[test]
fn l3_heap_search() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;

    use super::l3_probe::{arb_pair, arb_palette, arb_spine, arb_wide, build};

    fn search<S: Strategy<Value = super::l3_probe::Pair>>(name: &str, strat: S, n: usize) {
        let mut runner = TestRunner::deterministic();
        let palette = arb_palette();
        let mut best: Vec<(f64, String)> = Vec::new();
        let wide = crate::Count((BigUint::from(1u8) << 70u32) + 3u8);
        for _ in 0..n {
            let pair = strat.new_tree(&mut runner).unwrap().current();
            let pal = palette.new_tree(&mut runner).unwrap().current();
            let (op, ov) = build(&pair, &pal);
            let op = if op.is_empty() { OP::Leaf(true) } else { op };
            let v = from_oracle_version(&ov);
            let p = from_oracle_party(&op);
            let input = (v.encode().len() + p.encode().len()) as f64;
            for (kind, run) in [("tick", 0u8), ("ticks2^70", 1u8)] {
                let mut w = v.clone();
                HEAP.reset_peak_usage();
                let base = HEAP.current_usage();
                if run == 0 {
                    w.tick(&p);
                } else {
                    w.ticks(&p, wide.clone());
                }
                let peak = HEAP.peak_usage().saturating_sub(base) as f64;
                drop(w);
                let score = peak / (1024.0 + 20.0 * input);
                let shape: String = format!("{ov:?} | {op:?}").chars().take(240).collect();
                best.push((score, format!("{kind} peak={peak} input={input} {}", shape)));
            }
        }
        best.sort_by(|a, b| b.0.partial_cmp(&a.0).expect("finite"));
        for (score, line) in best.into_iter().take(4) {
            eprintln!("HEAPSEARCH {name} ceiling-fraction={score:.3} {line}");
        }
    }
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(|| {
            search("spine", arb_spine(200), 3000);
            search("bushy", arb_pair(10, 128), 3000);
            search("wide", arb_wide(40, 400), 300);
        })
        .expect("spawn")
        .join()
        .expect("join");
}
