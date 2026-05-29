//! Phase 3 clock-observer tests: `has_seen` / `happens_before` / `concurrent_with`
//! agree with the oracle across generated populations.

use proptest::prelude::*;

use crate::test_support::{from_oracle_clock, from_oracle_version, run, world_strategy};

proptest! {
    /// The clock observers match the oracle's: `has_seen` is `msg <= version`,
    /// `happens_before` is the strict causal order, and `concurrent_with` is
    /// incomparability.
    #[test]
    fn clock_observers_match_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (oa, ob) = (&cs[i % n], &cs[j % n]);

        let ia = from_oracle_clock(oa);
        let ib = from_oracle_clock(ob);
        let msg_oracle = ob.version();
        let msg = from_oracle_version(&msg_oracle);

        prop_assert_eq!(ia.has_seen(&msg), oa.has_seen(&msg_oracle));
        prop_assert_eq!(ia.happens_before(&ib), oa.happens_before(ob));
        prop_assert_eq!(ia.concurrent_with(&ib), oa.concurrent_with(ob));
    }
}
