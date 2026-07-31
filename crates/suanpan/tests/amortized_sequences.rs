//! An adversarial operation sequence against the roster's *amortized*
//! claims. An amortized bound holds over any operation sequence the
//! caller can write, so the attack is a sequence: cheap writes that
//! repeatedly force the expensive event the amortization must fund.
//!
//! **Wide sign-flip oscillation** `S1(n, d)`: hold `-1` at digit 0,
//! then alternate `add_wide_shl(1, d)` / `sign()` /
//! `sub_wide_shl(1, d)` / `sign()`. Every write is one operand limb
//! (claimed amortized `O(1)` touches, shift-independent); every read
//! flips the sign of a value whose decisive digits sit `d` bits apart.
//! If each flip re-certified the gap, the sequence would cost
//! `n × d/32` against a claimed `O(n + d/32)` — and every committed
//! band holds its sign at one polarity throughout, so no other
//! instrument ever flips sign under load. (The dual parking attack —
//! the value held at a full-width carry boundary with `±1` oscillation
//! across the cliff — is the committed `accum_comb_touches_flat` band
//! in `before`'s meter suite, which drives the same trajectory with
//! sign reads interleaved.)
//!
//! The tripwire is the mixed second difference over a 2×2 (n, d) grid:
//! zero for any additive `a·n + b·d` cost, a quarter of the top cell
//! for a product `c·n·d`. Bounded at a tenth of the top cell.

#![cfg(feature = "touch-meter")]

use suanpan::{touch_meter, Accumulator, UBig};

fn touches(f: impl FnOnce()) -> u64 {
    touch_meter::reset();
    f();
    touch_meter::touches()
}

/// Total touches for the wide sign-flip oscillation.
fn s1(n: usize, d_bits: u64) -> u64 {
    let one = UBig::from(1u8);
    let mut a = Accumulator::new();
    a.sub_u64(1);
    touches(|| {
        for _ in 0..n {
            a.add_wide_shl(&one, d_bits);
            assert_eq!(a.sign(), std::cmp::Ordering::Greater);
            a.sub_wide_shl(&one, d_bits);
            assert_eq!(a.sign(), std::cmp::Ordering::Less);
        }
    })
}

/// The no-product tripwire over the 2x2 grid.
fn assert_no_product(name: &str, t: [u64; 4]) {
    let mixed = t[3] as f64 - t[2] as f64 - t[1] as f64 + t[0] as f64;
    let bound = 0.10 * t[3] as f64;
    eprintln!("MEASURED {name}: grid {t:?} mixed {mixed:.0} bound {bound:.0}");
    assert!(
        mixed.abs() <= bound,
        "{name}: mixed second difference {mixed:.0} exceeds {bound:.0} — \
         an n x d product term under an amortized-O(1) claim"
    );
}

/// The sign fold's amortized `O(1)` claim survives an adversary
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
