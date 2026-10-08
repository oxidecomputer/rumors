//! Reviewer scratch probe; never committed.
//!
//! Measures `min_ticks` on two shapes where a narrow record is pushed and
//! popped repeatedly directly above an outer minimum whose own offset is wide,
//! and on `wide_arming` at the board's sizes, reporting a limb counter fed by
//! temporary hooks and the peak heap.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};

use num_bigint::{BigInt, BigUint};
use peak_alloc::PeakAlloc;

use super::{ev_leaf, ev_leaf_wide, wide_arming, Encoding};
use crate::bits::BitsWriter;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

static ENABLED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
static LIMBS: AtomicU64 = AtomicU64::new(0);

/// Count the limbs of two offset operands read by suspended-record storage.
pub(crate) fn count_limbs(a: &BigInt, b: &BigInt) {
    LIMBS.fetch_add(a.bits().div_ceil(64) + b.bits().div_ceil(64), Relaxed);
}

/// Print the heap held and the peak so far, relative to the call's start.
pub(crate) fn mark(label: &str) {
    if ENABLED.load(Relaxed) {
        let base = BASE.load(Relaxed);
        let current = HEAP.current_usage().saturating_sub(base);
        let peak = HEAP.peak_usage().saturating_sub(base);
        eprintln!("MARK {label}: current={current} peak_so_far={peak}");
    }
}

fn pow2(b: usize) -> BigUint {
    BigUint::from(1u8) << b
}

/// The left-deep comb `M_k` over teeth `T_1 = (0, 1)` and `T_i = 1 + (0, 1)`,
/// with `M_k` lifted by `top` relative to its parent.
fn comb(bits: &mut BitsWriter, k: usize, top: &BigUint) {
    assert!(k >= 2);
    bits.push(true);
    bits.write_gamma(top); // M_k
    for _ in 0..k - 2 {
        bits.push(true);
        bits.write_gamma(&BigUint::ZERO); // M_{k-1} .. M_2
    }
    bits.push(true);
    bits.write_gamma(&BigUint::ZERO); // T_1
    ev_leaf(bits, 0);
    ev_leaf(bits, 1);
    for _ in 2..=k {
        bits.push(true);
        bits.write_gamma(&BigUint::from(1u8)); // T_i
        ev_leaf(bits, 0);
        ev_leaf(bits, 1);
    }
}

/// Leaves `0, J, J+1, J+2, (J+2, J+3) x (k-1)` with `J = 2^b`: the outer
/// minimum `A = J` keeps its wide offset at prefix 0, and the comb's records
/// sit at the next prefix with offsets near zero.
fn cross_prefix(b: usize, k: usize) -> Encoding {
    let mut bits = BitsWriter::new();
    bits.push(true);
    bits.write_gamma(&BigUint::ZERO); // R
    ev_leaf(&mut bits, 0); // Z = 0
    bits.push(true);
    bits.write_gamma(&pow2(b)); // N0 at J
    ev_leaf(&mut bits, 0); // A = J
    comb(&mut bits, k, &BigUint::from(1u8));
    Encoding::from_bits(bits)
}

/// Leaves `H, 1, H+1, H+2, (H+2, H+3) x (k-1)` with `H = 2^b`: the outer
/// minimum `A = 1` has offset `1 - H` at prefix 0, and the comb's records
/// share that prefix with offsets near zero.
fn same_prefix(b: usize, k: usize) -> Encoding {
    let mut bits = BitsWriter::new();
    bits.push(true);
    bits.write_gamma(&BigUint::from(1u8)); // R at 1
    ev_leaf_wide(&mut bits, &(pow2(b) - 1u8)); // Z = H
    bits.push(true);
    bits.write_gamma(&BigUint::ZERO); // N0 at 1
    ev_leaf(&mut bits, 0); // A = 1
    comb(&mut bits, k, &pow2(b)); // M at H + 1
    Encoding::from_bits(bits)
}

fn measure(name: &str, encoding: &Encoding) {
    let version = encoding.version();
    let bytes = version.encode().len();
    LIMBS.store(0, Relaxed);
    BASE.store(HEAP.current_usage(), Relaxed);
    HEAP.reset_peak_usage();
    ENABLED.store(true, Relaxed);
    let answer = version.min_ticks();
    ENABLED.store(false, Relaxed);
    let peak = HEAP.peak_usage().saturating_sub(BASE.load(Relaxed));
    let limbs = LIMBS.load(Relaxed);
    eprintln!(
        "PROBE {name} bytes={bytes} limbs={limbs} limbs_per_byte={:.2} peak={peak} \
         peak_per_byte={:.2} answer_bits={} answer_low={:x}",
        limbs as f64 / bytes as f64,
        peak as f64 / bytes as f64,
        answer.0.bits(),
        answer.0.iter_u64_digits().next().unwrap_or(0),
    );
}

#[test]
fn reviewer_probe_circulation() {
    for k in [250, 500, 1000, 2000] {
        measure(&format!("cross-prefix b={} k={k}", 64 * k), &cross_prefix(64 * k, k));
    }
    for k in [250, 500, 1000, 2000] {
        measure(&format!("same-prefix b={} k={k}", 64 * k), &same_prefix(64 * k, k));
    }
}

#[test]
fn reviewer_probe_wide_arming() {
    for s in [587, 1174, 2348, 4696] {
        measure(&format!("wide-arming s={s}"), &wide_arming(s, s));
    }
}
