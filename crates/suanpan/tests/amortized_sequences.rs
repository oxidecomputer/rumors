//! A sign-flip sequence that distinguishes additive work from a
//! rounds-by-width product.
//!
//! Hold `-1` at digit 0,
//! then alternate `add_limbs_shl([1], d)` / `sign()` /
//! `sub_limbs_shl([1], d)` / `sign()`. Every write is one operand limb
//! (amortized `O(1)` touches, shift-independent); every read
//! flips the sign of a value whose decisive digits sit `d` bits apart.
//! Re-scanning the gap on each flip would therefore cost `n × d/32`, while the
//! intended bound is `O(n + d/32)`.
//!
//! The test uses the mixed second difference over a 2×2 `(n, d)` grid:
//! zero for any additive `a·n + b·d` cost, a quarter of the top cell
//! for a product `c·n·d`. Bounded at a tenth of the top cell.

#![cfg(feature = "touch-meter")]

use suanpan::{touch_meter, Accumulator};

fn touches(metered: impl FnOnce()) -> u64 {
    touch_meter::reset();
    metered();
    touch_meter::touches()
}

/// Total touches for the wide sign-flip oscillation.
fn s1(rounds: usize, d_bits: u64) -> u64 {
    let mut acc = Accumulator::new();
    acc.sub_u64(1);
    touches(|| {
        for _ in 0..rounds {
            acc.add_limbs_shl([1], d_bits);
            assert_eq!(acc.sign(), std::cmp::Ordering::Greater);
            acc.sub_limbs_shl([1], d_bits);
            assert_eq!(acc.sign(), std::cmp::Ordering::Less);
        }
    })
}

/// Assert that a two-dimensional measurement grid has no product term.
fn assert_no_product(name: &str, grid: [u64; 4]) {
    let mixed = grid[3] as f64 - grid[2] as f64 - grid[1] as f64 + grid[0] as f64;
    let bound = 0.10 * grid[3] as f64;
    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed:.0} bound {bound:.0}");
    assert!(
        mixed.abs() <= bound,
        "{name}: mixed second difference {mixed:.0} exceeds {bound:.0} — \
         an n x d product term under an amortized-O(1) claim"
    );
}

/// The sign fold's amortized `O(1)` claim survives a worst-case stream
/// flipping the sign of a wide value with one-limb writes: no
/// per-round re-certification of the width.
#[test]
fn sign_flip_oscillation_has_no_width_product() {
    let (n0, d0) = (2048usize, 32_768u64);
    assert_no_product(
        "s1_sign_flip",
        [
            s1(n0, d0),
            s1(2 * n0, d0),
            s1(n0, 2 * d0),
            s1(2 * n0, 2 * d0),
        ],
    );
}
