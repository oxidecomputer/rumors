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

/// `Sum` over no owned counts is [`Count::ZERO`], as the `Sum<Count>` impl
/// documents.
#[test]
fn empty_owned_sum_is_zero() {
    assert_eq!(core::iter::empty::<Count>().sum::<Count>(), Count::ZERO);
}

/// `Sum` over no borrowed counts is [`Count::ZERO`], as the `Sum<&Count>` impl
/// documents.
#[test]
fn empty_borrowed_sum_is_zero() {
    assert_eq!(core::iter::empty::<&Count>().sum::<Count>(), Count::ZERO);
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

/// Every unsigned primitive's boundary converts, renders, and respells
/// exactly: 0, 1, and `MAX - 1`, `MAX`, and `MAX + 1` of each width.
///
/// Each count is built from its value's 64-bit digits through public
/// addition alone. Its limbs must equal those digits, with the size hint
/// equal to the limb count; its `Display` and `parse` must round-trip the
/// decimal; and each `TryFrom`, borrowed and owned, must succeed exactly when
/// the value is at most the destination's `MAX`, judged by comparing
/// arbitrary-precision values rather than by the conversion `Count` delegates
/// to, and must then yield the value and convert back to the count. The
/// `usize` row repeats the `u64` row on a 64-bit host and the `u32` row on a
/// 32-bit one, so the sweep keeps both meanings in view.
#[test]
fn every_primitive_boundary_converts_exactly() {
    let maxima = [
        BigUint::from(u8::MAX),
        BigUint::from(u16::MAX),
        BigUint::from(u32::MAX),
        BigUint::from(u64::MAX),
        BigUint::from(u128::MAX),
        BigUint::from(usize::MAX),
    ];
    let mut values = vec![BigUint::ZERO, BigUint::from(1u8)];
    for max in &maxima {
        values.extend([max - 1u8, max.clone(), max + 1u8]);
    }

    let one = Count::from(1u8);
    for value in &values {
        let mut count = Count::ZERO;
        for digit in value.iter_u64_digits().rev() {
            for _ in 0..64 {
                count += count.clone();
            }
            count += Count::from(digit);
        }

        let limbs: Vec<u64> = count.limbs().collect();
        assert_eq!(
            limbs,
            value.iter_u64_digits().collect::<Vec<_>>(),
            "limbs of {value}"
        );
        assert_eq!(
            count.limbs().len(),
            limbs.len(),
            "exact size of {value}'s limbs"
        );
        assert_eq!(count.to_string(), value.to_string(), "Display of {value}");
        assert_eq!(
            count.to_string().parse::<Count>().ok(),
            Some(count.clone()),
            "parse of {value}"
        );

        // Each conversion, borrowed and owned, must succeed exactly when the
        // value fits, and then yield the value and convert back to the count.
        macro_rules! check_conversion {
            ($t:ty) => {{
                let fits = *value <= BigUint::from(<$t>::MAX);
                let name = stringify!($t);
                for converted in [
                    <$t>::try_from(&count).ok(),
                    <$t>::try_from(count.clone()).ok(),
                ] {
                    assert_eq!(converted.is_some(), fits, "{value} into {name}");
                    if let Some(narrow) = converted {
                        assert_eq!(BigUint::from(narrow), *value, "{value} through {name}");
                        assert_eq!(Count::from(narrow), count, "{value} back from {name}");
                    }
                }
            }};
        }
        check_conversion!(u8);
        check_conversion!(u16);
        check_conversion!(u32);
        check_conversion!(u64);
        check_conversion!(u128);
        check_conversion!(usize);

        assert_eq!(
            count.checked_sub(&one).is_some(),
            *value >= BigUint::from(1u8),
            "{value} - 1"
        );
        assert_eq!(
            (&count + &one).checked_sub(&one),
            Some(count.clone()),
            "{value} + 1 - 1"
        );
    }
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

    /// Every spelling of addition computes the natural-number sum, for counts
    /// of every size: the four owned and borrowed `+` cells, `+=` with an
    /// owned or borrowed right operand, and `Sum` over owned or borrowed
    /// items.
    #[test]
    fn addition_spellings_match_the_naturals(
        a in crate::testing::generators::arb_magnitude(),
        b in crate::testing::generators::arb_magnitude(),
    ) {
        let expected = Count(&a + &b);
        let (ta, tb) = (Count(a), Count(b));
        prop_assert_eq!(&ta + &tb, expected.clone());
        prop_assert_eq!(ta.clone() + tb.clone(), expected.clone());
        prop_assert_eq!(ta.clone() + &tb, expected.clone());
        prop_assert_eq!(&ta + tb.clone(), expected.clone());
        let mut by_ref = ta.clone();
        by_ref += &tb;
        prop_assert_eq!(by_ref, expected.clone());
        let mut by_value = ta.clone();
        by_value += tb.clone();
        prop_assert_eq!(by_value, expected.clone());
        prop_assert_eq!([&ta, &tb].into_iter().sum::<Count>(), expected.clone());
        prop_assert_eq!([ta, tb].into_iter().sum::<Count>(), expected);
    }

    /// `Debug` renders a count exactly as `Display` does, as its
    /// documentation states: for counts of every size, both in the default
    /// format and when width and alignment flags apply.
    ///
    /// A `Debug` that formats through a fresh formatter, such as
    /// `write!(f, "{self}")`, matches only the default format.
    #[test]
    fn debug_is_display(base in crate::testing::generators::arb_magnitude()) {
        let count = Count(base);
        prop_assert_eq!(format!("{count:?}"), count.to_string());
        prop_assert_eq!(format!("{count:>40?}"), format!("{count:>40}"));
    }
}
