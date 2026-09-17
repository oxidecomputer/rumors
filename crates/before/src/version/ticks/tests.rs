//! Properties of [`Ticks`] construction, ordering, limbs, and addition.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::Ticks;

/// Every unsigned machine width converts in and agrees on the value: one count
/// per numeral, whatever type spelled it.
#[test]
fn from_impls_agree_across_widths() {
    let want = Ticks::from(200u8);
    assert_eq!(Ticks::from(200u16), want);
    assert_eq!(Ticks::from(200u32), want);
    assert_eq!(Ticks::from(200u64), want);
    assert_eq!(Ticks::from(200u128), want);
    assert_eq!(Ticks::from(200usize), want);
    assert_eq!(want.to_string(), "200");
}

/// `ZERO` is `From<0>`, the `Default`, and renders as `"0"`.
#[test]
fn zero_forms_agree() {
    assert_eq!(Ticks::ZERO, Ticks::from(0u64));
    assert_eq!(Ticks::default(), Ticks::ZERO);
    assert_eq!(Ticks::ZERO.to_string(), "0");
}

/// Addition constructs counts wider than `u128`, which retain their order and
/// decimal rendering.
#[test]
fn wide_counts_order_and_render() {
    let text = "115792089237316195423570985008687907853269984665640564039457584007913129639936"; // 2^256
    let mut wide = Ticks::from(1u8);
    for _ in 0..256 {
        wide = &wide + &wide;
    }
    assert_eq!(wide.to_string(), text);
    assert!(wide > Ticks::from(u128::MAX));
}

proptest! {
    /// Addition is commutative, associative, and monotone, `ZERO` is the
    /// identity, and `Sum` equals the pairwise fold — the naturals' laws on
    /// the opaque carrier.
    #[test]
    fn addition_behaves_like_the_naturals(a in any::<u128>(), b in any::<u128>(), c in any::<u128>()) {
        let (ta, tb, tc) = (Ticks::from(a), Ticks::from(b), Ticks::from(c));
        prop_assert_eq!(&ta + &tb, &tb + &ta);
        prop_assert_eq!(&(&ta + &tb) + &tc, &ta + &(&tb + &tc));
        prop_assert_eq!(&ta + &Ticks::ZERO, ta.clone());
        prop_assert!(&ta + &tb >= ta);
        let summed: Ticks = [ta.clone(), tb.clone(), tc.clone()].into_iter().sum();
        prop_assert_eq!(summed, &(&ta + &tb) + &tc);
    }
}

proptest! {
    /// The limb spelling is exact and canonical: `Σ limbᵢ · 2^(64·i)`
    /// rebuilds the count, no trailing zero limb appears, the zero count
    /// yields no limbs, and the exact-size length stays truthful at
    /// every step of the drain.
    #[test]
    fn limbs_respell_the_count(base in crate::testing::generators::arb_magnitude()) {
        let count = Ticks(base);
        let limbs: Vec<u64> = count.limbs().collect();
        if let Some(last) = limbs.last() {
            prop_assert_ne!(*last, 0u64, "no trailing zero limbs");
        } else {
            prop_assert_eq!(&count, &Ticks::ZERO);
        }
        let mut rebuilt = BigUint::ZERO;
        for (index, limb) in limbs.iter().enumerate() {
            rebuilt += &(BigUint::from(*limb) << (64 * index as u32));
        }
        prop_assert_eq!(&Ticks(rebuilt), &count);
        let mut iter = count.limbs();
        let mut remaining = limbs.len();
        prop_assert_eq!(iter.len(), remaining);
        while iter.next().is_some() {
            remaining -= 1;
            prop_assert_eq!(iter.len(), remaining);
        }
        prop_assert_eq!(iter.len(), 0);
        prop_assert_eq!(iter.next(), None); // fused past the end
    }

    /// `u64::try_from` answers every machine-range count with its value
    /// and every wider count with `TooWide`, in agreement with the limb
    /// spelling's length.
    #[test]
    fn u64_conversion_matches_the_range(base in crate::testing::generators::arb_magnitude()) {
        let count = Ticks(base);
        match u64::try_from(&count) {
            Ok(word) => prop_assert_eq!(&Ticks::from(word), &count),
            Err(_) => prop_assert!(count.limbs().len() > 1, "only wide counts refuse"),
        }
    }
}
