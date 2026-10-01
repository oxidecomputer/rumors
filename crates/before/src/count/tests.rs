//! Properties of [`Count`] construction, arithmetic, conversions, and limbs.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::Count;

/// Every unsigned machine width converts in and agrees on the value: one count
/// per numeral, whatever type spelled it.
#[test]
fn from_impls_agree_across_widths() {
    let want = Count::from(200u8);
    assert_eq!(Count::from(200u16), want);
    assert_eq!(Count::from(200u32), want);
    assert_eq!(Count::from(200u64), want);
    assert_eq!(Count::from(200u128), want);
    assert_eq!(Count::from(200usize), want);
    assert_eq!(want.to_string(), "200");
}

/// `ZERO` is `From<0>`, the `Default`, and renders as `"0"`.
#[test]
fn zero_forms_agree() {
    assert_eq!(Count::ZERO, Count::from(0u64));
    assert_eq!(Count::default(), Count::ZERO);
    assert_eq!(Count::ZERO.to_string(), "0");
}

/// Addition constructs counts wider than `u128`, which retain their order and
/// decimal rendering.
#[test]
fn wide_counts_order_and_render() {
    let text = "115792089237316195423570985008687907853269984665640564039457584007913129639936"; // 2^256
    let mut wide = Count::from(1u8);
    for _ in 0..256 {
        wide = &wide + &wide;
    }
    assert_eq!(wide.to_string(), text);
    assert!(wide > Count::from(u128::MAX));
}

proptest! {
    /// Addition is commutative, associative, and monotone, `ZERO` is the
    /// identity, and `Sum` equals the pairwise fold — the naturals' laws on
    /// the opaque carrier.
    #[test]
    fn addition_behaves_like_the_naturals(a in any::<u128>(), b in any::<u128>(), c in any::<u128>()) {
        let (ta, tb, tc) = (Count::from(a), Count::from(b), Count::from(c));
        prop_assert_eq!(&ta + &tb, &tb + &ta);
        prop_assert_eq!(&(&ta + &tb) + &tc, &ta + &(&tb + &tc));
        prop_assert_eq!(&ta + &Count::ZERO, ta.clone());
        prop_assert!(&ta + &tb >= ta);
        let summed: Count = [ta.clone(), tb.clone(), tc.clone()].into_iter().sum();
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
        let count = Count(base);
        let limbs: Vec<u64> = count.limbs().collect();
        if let Some(last) = limbs.last() {
            prop_assert_ne!(*last, 0u64, "no trailing zero limbs");
        } else {
            prop_assert_eq!(&count, &Count::ZERO);
        }
        let mut rebuilt = BigUint::ZERO;
        for (index, limb) in limbs.iter().enumerate() {
            rebuilt += &(BigUint::from(*limb) << (64 * index as u32));
        }
        prop_assert_eq!(&Count(rebuilt), &count);
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

    /// Every owned and borrowed unsigned conversion succeeds exactly when the
    /// arbitrary-precision value fits its destination type.
    #[test]
    fn unsigned_conversions_match_their_ranges(base in crate::testing::generators::arb_magnitude()) {
        let count = Count(base.clone());
        macro_rules! check {
            ($t:ty) => {{
                let expected = <$t>::try_from(&base).ok();
                prop_assert_eq!(<$t>::try_from(&count).ok(), expected);
                prop_assert_eq!(<$t>::try_from(count.clone()).ok(), expected);
            }};
        }
        check!(u8);
        check!(u16);
        check!(u32);
        check!(u64);
        check!(u128);
        check!(usize);
    }

    /// Checked and saturating subtraction agree with natural-number
    /// subtraction at zero, equal, narrow, and arbitrary-width values.
    #[test]
    fn subtraction_matches_the_naturals(
        a in crate::testing::generators::arb_magnitude(),
        b in crate::testing::generators::arb_magnitude(),
    ) {
        let (ta, tb) = (Count(a.clone()), Count(b.clone()));
        if a < b {
            prop_assert_eq!(ta.checked_sub(&tb), None);
            prop_assert_eq!(ta.saturating_sub(&tb), Count::ZERO);
        } else {
            let expected = Count(a - b);
            prop_assert_eq!(ta.checked_sub(&tb), Some(expected.clone()));
            prop_assert_eq!(ta.saturating_sub(&tb), expected);
        }
    }
}
