//! Scratch heap probe for the min_ticks fix (never committed).
//!
//! Installs a counting global allocator in the lib test binary and reads the
//! peak live heap of one `min_ticks` above its baseline, exactly as the board
//! reads it, on right spines built from explicit leaf-height sequences. Each
//! reading also checks the value against an independent closed form.

use num_bigint::BigUint;
use peak_alloc::PeakAlloc;

use crate::testing::bridge::from_oracle_version;
use crate::testing::meter::registry::Shape;
use crate::testing::oracles::tree::Version as OV;
use crate::{Count, Version};

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// Board ceiling for one sample.
fn ceiling(encoded: usize) -> usize {
    1024 + 20 * encoded
}

/// Right spine `(a0, (a1, (a2, ...)))` over absolute leaf heights.
fn spine(leaves: &[BigUint]) -> OV {
    let mut v = OV::Leaf(leaves[leaves.len() - 1].clone());
    for a in leaves[..leaves.len() - 1].iter().rev() {
        v = OV::node(0u64, OV::Leaf(a.clone()), v);
    }
    v
}

/// Σ leaves − Σ internal-subtree minima for a right spine: node k's subtree
/// holds leaves k.., so its minimum is the suffix minimum from k.
fn spine_min_ticks(leaves: &[BigUint]) -> BigUint {
    let n = leaves.len();
    let mut suffix_min = leaves[n - 1].clone();
    let mut sum: BigUint = leaves.iter().sum();
    let mut minima = BigUint::ZERO;
    for k in (0..n - 1).rev() {
        if leaves[k] < suffix_min {
            suffix_min = leaves[k].clone();
        }
        minima += &suffix_min;
    }
    sum -= minima;
    sum
}

/// Peak heap of `min_ticks` above its baseline.
fn peak_min_ticks(v: &Version) -> (usize, Count) {
    HEAP.reset_peak_usage();
    let base = HEAP.current_usage();
    let m = v.min_ticks();
    let peak = HEAP.peak_usage().saturating_sub(base);
    (peak, m)
}

fn report(family: &str, leaves: &[BigUint]) {
    let ov = spine(leaves);
    let v = from_oracle_version(&ov);
    core::mem::forget(ov);
    let encoded = v.encode().len();
    let (peak, m) = peak_min_ticks(&v);
    let expected = Count(spine_min_ticks(leaves));
    let ok = m == expected;
    let c = ceiling(encoded);
    eprintln!(
        "PROBE {family} levels={} encoded={encoded} peak={peak} ratio={:.2} ceiling={c} {} value={}",
        leaves.len() - 1,
        peak as f64 / encoded as f64,
        if peak > c { "RED" } else { "green" },
        if ok { "ok" } else { "WRONG" },
    );
    assert!(ok, "{family}: min_ticks disagrees with the closed form");
}

fn pow2(b: u32) -> BigUint {
    BigUint::from(1u8) << b
}

/// Level counts on both sides of powers of two, where doubling storage is
/// fullest and emptiest.
fn sizes() -> Vec<usize> {
    let mut out = vec![2, 3, 5, 9, 17, 33, 65, 100, 129];
    for k in [8u32, 10, 12, 14, 16] {
        let p = 1usize << k;
        out.extend([p - 2, p + 2, p + p / 2 + 2]);
    }
    out
}

fn run_in_big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 31)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("join");
}

/// Plain rising spine: a_k = k.
#[test]
fn fixer_probe_rising() {
    run_in_big_stack(|| {
        for n in sizes() {
            let leaves: Vec<BigUint> = (0..=n as u64).map(BigUint::from).collect();
            report("rising", &leaves);
        }
    });
}

/// Jump-entered rising spine: a_0 = 0, a_k = J + k.
#[test]
fn fixer_probe_jump() {
    run_in_big_stack(|| {
        for jb in [16u32, 30, 31, 32, 40, 64, 200, 287, 300] {
            let j = pow2(jb);
            for n in sizes() {
                let leaves: Vec<BigUint> = (0..=n as u64)
                    .map(|k| if k == 0 { BigUint::ZERO } else { &j + k })
                    .collect();
                report(&format!("jump-2^{jb}"), &leaves);
            }
        }
    });
}

/// Wide-step rising spine: a_k = k * 2^s.
#[test]
fn fixer_probe_step() {
    run_in_big_stack(|| {
        for s in [30u32, 31, 32, 40, 63, 64, 65, 100, 287, 300] {
            let step = pow2(s);
            for n in sizes() {
                let leaves: Vec<BigUint> = (0..=n as u64).map(|k| &step * k).collect();
                report(&format!("step-2^{s}"), &leaves);
            }
        }
    });
}

/// Zigzag spine: a_k = (k mod 2) * 2^s; every step crosses 2^s.
#[test]
fn fixer_probe_zigzag() {
    run_in_big_stack(|| {
        for s in [30u32, 31, 32, 40, 64, 200] {
            let big = pow2(s);
            for n in sizes() {
                let leaves: Vec<BigUint> = (0..=n as u64)
                    .map(|k| {
                        if k % 2 == 1 {
                            big.clone()
                        } else {
                            BigUint::ZERO
                        }
                    })
                    .collect();
                report(&format!("zigzag-2^{s}"), &leaves);
            }
        }
    });
}

/// Rising zigzag: a_k = k + (k mod 2) * 2^s; positive boundaries on even
/// levels, and every step crosses 2^s.
#[test]
fn fixer_probe_rising_zigzag() {
    run_in_big_stack(|| {
        for s in [31u32, 32, 40, 64, 200] {
            let big = pow2(s);
            for n in sizes() {
                let leaves: Vec<BigUint> = (0..=n as u64)
                    .map(|k| {
                        let base = BigUint::from(k);
                        if k % 2 == 1 {
                            base + &big
                        } else {
                            base
                        }
                    })
                    .collect();
                report(&format!("rising-zigzag-2^{s}"), &leaves);
            }
        }
    });
}

/// The board's own family, through the registry, at the board's depths.
#[test]
fn fixer_probe_board_jr() {
    run_in_big_stack(|| {
        for d in [
            80usize, 147, 6680, 13346, 26680, 53346, 4096, 4098, 8192, 8194, 16384, 16386,
        ] {
            let v = Shape::JumpRisingSpine.build2(40, d).version();
            let encoded = v.encode().len();
            let (peak, _) = peak_min_ticks(&v);
            let c = ceiling(encoded);
            eprintln!(
                "PROBE board-jr d={d} encoded={encoded} peak={peak} ratio={:.2} ceiling={c} {}",
                peak as f64 / encoded as f64,
                if peak > c { "RED" } else { "green" },
            );
        }
    });
}
