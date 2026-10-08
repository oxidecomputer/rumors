//! The clock overlay against an independent refinement.

use before::shape::Rise;
use before::{Clock, Version};
use proptest::prelude::*;

use crate::gen::{arb_set, party};
use crate::model::{MAXD, ONE};

/// The coarsest common refinement of the model's party regions and the
/// version's plateaus: `(depth, rise on a plateau's first cell, owned)`.
fn refine(regions: &[(bool, u64)], plateaus: &[(Option<Rise>, u64)]) -> Vec<(u64, Option<Rise>, bool)> {
    let width = |d: u64| -> u128 {
        assert!(d <= u64::from(MAXD), "depth beyond the model resolution");
        ONE >> d
    };
    let mut out = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    let mut pos = 0u128;
    let mut a_end = width(regions[0].1);
    let mut b_end = width(plateaus[0].1);
    let mut first = true;
    while pos < ONE {
        let depth = regions[i].1.max(plateaus[j].1);
        let end = a_end.min(b_end);
        assert_eq!(end - pos, width(depth), "tilings nest");
        out.push((depth, if first { plateaus[j].0.clone() } else { None }, regions[i].0));
        first = false;
        pos = end;
        if a_end == end && pos < ONE {
            i += 1;
            a_end = pos + width(regions[i].1);
        }
        if b_end == end && pos < ONE {
            j += 1;
            first = true;
            b_end = pos + width(plateaus[j].1);
        }
    }
    out
}

/// A version built by ticking arbitrary parties, so its plateaus interleave
/// with the clock's own regions.
fn arb_version() -> impl Strategy<Value = Version> {
    proptest::collection::vec((arb_set(), 0u8..4), 0..5).prop_map(|steps| {
        let mut v = Version::new();
        for (s, n) in steps {
            party(&s).ticks(&mut v, u64::from(n));
        }
        v
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// `Clock::shape` is the refinement of the party's regions by the
    /// version's plateaus, carrying each plateau's rise on its first cell.
    #[test]
    fn overlay_is_the_refinement(a in arb_set(), v in arb_version()) {
        let clock = Clock::from_parts(party(&a), v.clone());
        let got: Vec<(u64, Option<Rise>, bool)> =
            clock.shape().map(|(p, owned)| (p.depth, p.rise, owned)).collect();
        let plateaus: Vec<(Option<Rise>, u64)> = v.shape().map(|p| (p.rise, p.depth)).collect();
        let expect = refine(&a.regions(), &plateaus);
        prop_assert_eq!(got, expect);
    }
}
