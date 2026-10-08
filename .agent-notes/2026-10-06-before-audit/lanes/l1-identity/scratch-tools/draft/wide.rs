//! Fork iterators at counts beyond the machine word.

use std::str::FromStr;

use before::{Clock, Count, Version};
use num_bigint::BigUint;
use proptest::prelude::*;

use crate::gen::{arb_set_upto, party};
use crate::model::Set;

/// The `i`-th share of the balanced partition of `p` into `n` shares, by one
/// logarithmic descent: ceil to the left, floor to the right.
pub fn share(p: &Set, n: &BigUint, i: &BigUint) -> Set {
    let (mut p, mut n, mut i) = (p.clone(), n.clone(), i.clone());
    let one = BigUint::from(1u8);
    while n > one {
        let (l, r) = p.fork();
        let left_n = (&n + 1u8) / 2u8;
        if i < left_n {
            p = l;
            n = left_n;
        } else {
            p = r;
            i -= &left_n;
            n /= 2u8;
        }
    }
    p
}

fn count(k: &BigUint) -> Count {
    Count::from_str(&k.to_string()).expect("decimal count")
}

/// Counts around the machine word and well beyond it.
fn arb_wide_k() -> impl Strategy<Value = BigUint> {
    let word = BigUint::from(usize::MAX);
    let w1 = word.clone();
    let w2 = word.clone();
    prop_oneof![
        // Crossing usize::MAX: some prefixes cross the Near-to-Exact boundary.
        (0u64..80).prop_map(move |d| &w1 + d - 3u8),
        // Just above twice the word: the first distant counts.
        (0u64..8).prop_map(move |d| &w2 * 2u8 + d - 2u8),
        // Arbitrary widths through 2^100.
        (1u32..=100, any::<u128>()).prop_map(|(bits, x)| BigUint::from(x >> (128 - bits)) + 1u8),
        // Powers of two and their neighbors.
        (1u32..=100, 0u8..3).prop_map(|(e, d)| (BigUint::from(1u8) << e) + d - 1u8),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Wide-count fork iterators yield the exact balanced shares, keep every
    /// untaken region in the borrower, and report sound size hints.
    #[test]
    fn forks_wide_counts(a in arb_set_upto(20), k in arb_wide_k(), taken in 0u64..40) {
        let n = &k + 1u8;
        let mut p = party(&a);
        let mut got = Vec::new();
        {
            let mut it = p.forks(count(&k));
            for t in 0..taken {
                let rem = &k - t;
                let (lo, hi) = it.size_hint();
                prop_assert!(BigUint::from(lo) <= rem, "size_hint lower bound {} above remaining {}", lo, rem);
                if let Some(hi) = hi {
                    prop_assert!(BigUint::from(hi) >= rem, "size_hint upper bound {} below remaining {}", hi, rem);
                }
                if rem <= BigUint::from(usize::MAX) && k <= BigUint::from(usize::MAX) {
                    let r = usize::try_from(&rem).unwrap();
                    prop_assert_eq!((lo, hi), (r, Some(r)));
                }
                got.push(it.next().expect("count not exhausted"));
            }
        }
        let mut residual = a.clone();
        for (t, s) in got.iter().enumerate() {
            let expect = share(&a, &n, &BigUint::from(t as u64 + 1));
            let s = Set::decode(s.as_bytes());
            prop_assert_eq!(&s, &expect, "share {}", t + 1);
            residual = residual.minus(&s);
        }
        prop_assert_eq!(Set::decode(p.as_bytes()), residual);
        // The residual is share 0 together with the untaken suffix, so it
        // always covers share 0.
        let zero = share(&a, &n, &BigUint::from(0u8));
        prop_assert!(Set::decode(p.as_bytes()).covers(&zero));
    }

    /// The clock iterator pairs the same wide-count shares with the version.
    #[test]
    fn clock_forks_wide_counts(a in arb_set_upto(12), k in arb_wide_k(), taken in 0u64..6) {
        let n = &k + 1u8;
        let mut c = Clock::from_parts(party(&a), Version::new());
        c.tick();
        let v = c.version().clone();
        let kids: Vec<Clock> = c.forks(count(&k)).take(taken as usize).collect();
        for (t, kid) in kids.iter().enumerate() {
            prop_assert_eq!(Set::decode(kid.party().as_bytes()), share(&a, &n, &BigUint::from(t as u64 + 1)));
            prop_assert_eq!(kid.version(), &v);
        }
    }
}
