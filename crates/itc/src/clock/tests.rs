//! Clock-level tests.
//!
//! Phase 3 (observers): `has_seen` / `happens_before` / `concurrent_with` agree with
//! the oracle. Phase 6: the master differential harness (groups B 6 & 21), protocol
//! semantics (group E 22–27), and batch equivalence / laziness / commit-on-drop
//! (group G 30–32, plus the F 29 mid-batch comparison).

use proptest::prelude::*;

use crate::oracle;
use crate::test_support::{
    from_oracle_clock, from_oracle_party, from_oracle_version, run, to_oracle_clock,
    to_oracle_version, world_strategy, Op,
};
use crate::Clock;

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

// ───────────────────────────── master differential harness ─────────────────────────────

proptest! {
    /// B6 + B21. A seed-derived op trace, applied in lockstep to the oracle and the
    /// impl, agrees structurally on every live clock after every step, and all live
    /// impl parties stay pairwise disjoint (so `join`/`sync` never error in correct
    /// usage). Agreement is by structural lowering — `to_oracle_clock` rebuilds the
    /// oracle's tree shape from the impl's internal packed bits — not via the byte
    /// codec, which the per-trace round-trip below exercises separately.
    #[test]
    fn b6_master_differential(ops in world_strategy()) {
        let mut ora: Vec<oracle::Clock> = vec![oracle::Clock::seed()];
        let mut imp: Vec<Clock> = vec![Clock::seed()];

        for op in &ops {
            let n = ora.len();
            match *op {
                Op::Tick(i) => {
                    let i = i % n;
                    ora[i].tick();
                    imp[i].tick();
                }
                Op::Fork(i) => {
                    let i = i % n;
                    let oc = ora[i].fork();
                    let ic = imp[i].fork();
                    ora.push(oc);
                    imp.push(ic);
                }
                Op::Send(i, j) => {
                    let (i, j) = (i % n, j % n);
                    let om = ora[i].send();
                    let im = imp[i].send();
                    ora[j].receive(om);
                    imp[j].receive(im);
                }
                Op::Sync(i, j) => {
                    let (i, j) = (i % n, j % n);
                    if i != j {
                        let hi = i.max(j);
                        let lo = i.min(j);
                        {
                            let (a, b) = ora.split_at_mut(hi);
                            a[lo].sync(&mut b[0]).expect("seed-derived parties are disjoint");
                        }
                        {
                            let (a, b) = imp.split_at_mut(hi);
                            a[lo].sync(&mut b[0]).expect("seed-derived parties are disjoint");
                        }
                    }
                }
                Op::Join(i, j) => {
                    if n > 1 {
                        let (i, j) = (i % n, j % n);
                        if i != j {
                            let ov = ora.remove(j);
                            let iv = imp.remove(j);
                            let i2 = if j < i { i - 1 } else { i };
                            ora[i2].join(ov).expect("seed-derived parties are disjoint");
                            imp[i2].join(iv).expect("seed-derived parties are disjoint");
                        }
                    }
                }
            }

            // Structural agreement on every live clock.
            prop_assert_eq!(ora.len(), imp.len());
            for (o, m) in ora.iter().zip(imp.iter()) {
                let (op_tree, ov_tree) = o.trees();
                let (mp_tree, mv_tree) = to_oracle_clock(m);
                prop_assert_eq!(&mp_tree, op_tree);
                prop_assert_eq!(&mv_tree, ov_tree);
            }

            // Disjointness invariant: all live impl parties pairwise disjoint.
            for a in 0..imp.len() {
                for b in (a + 1)..imp.len() {
                    prop_assert!(imp[a].party().is_disjoint(imp[b].party()));
                }
            }
        }

        // Per-trace codec exercise: every live clock round-trips through decode∘encode
        // and stays structurally identical (also confirms each encoding is canonical,
        // since `decode` strictly rejects non-normal-form input).
        for m in &imp {
            let back = Clock::decode(&m.encode()).expect("impl encodings are canonical");
            prop_assert_eq!(to_oracle_clock(&back), to_oracle_clock(m));
        }
    }
}

// ───────────────────────────── protocol semantics (group E) ─────────────────────────────

proptest! {
    /// E22. `fork` preserves the version on both halves.
    #[test]
    fn e22_fork_preserves_version(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut c = from_oracle_clock(&cs[i % n]);
        let before = c.version();
        let child = c.fork();
        prop_assert!(c.version() == before);
        prop_assert!(child.version() == before);
    }

    /// E23. `version()` (peek) does not advance the clock; the returned `Version`
    /// equals the clock's own and repeated peeks are stable.
    #[test]
    fn e23_peek_does_not_advance(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let c = from_oracle_clock(&cs[i % n]);
        let before = c.encode();
        let v1 = c.version();
        let v2 = c.version();
        prop_assert!(v1 == v2);
        prop_assert_eq!(c.encode(), before);
    }

    /// E24. `receive(msg)` with `msg <= self` (here `msg == self.version()`) equals a
    /// bare `tick`: an own-message receive is benign, and the party is unchanged.
    #[test]
    fn e24_own_receive_is_tick(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut received = from_oracle_clock(&cs[i % n]);
        let mut ticked = from_oracle_clock(&cs[i % n]);
        let own = received.version();
        received.receive(own);
        ticked.tick();
        prop_assert!(received.version() == ticked.version());
        prop_assert!(received.party() == ticked.party());
    }

    /// E25. After `a.sync(&mut b)`: both end at the oracle's result, their versions are
    /// equal, their parties are disjoint, and re-joining the two parties recovers the
    /// pre-sync merged party.
    #[test]
    fn e25_sync(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        if n < 2 {
            return Ok(());
        }
        // Derive two distinct members directly rather than rejecting collisions —
        // small populations collide often, and `prop_assume` would blow the reject
        // cap under a high case count (see oracle `o12_sync`).
        let i = i % n;
        let j = (i + 1 + j % (n - 1)) % n;

        // Oracle expectation, and the pre-sync merged party.
        let mut oa = cs[i].clone();
        let mut ob = cs[j].clone();
        oa.sync(&mut ob).expect("disjoint");
        let mut merged = cs[i].party().clone();
        merged.join(cs[j].party().clone()).expect("disjoint");

        // Impl sync.
        let mut ia = from_oracle_clock(&cs[i]);
        let mut ib = from_oracle_clock(&cs[j]);
        ia.sync(&mut ib).expect("disjoint");

        // Structural agreement with the oracle on both sides.
        let (oap, oav) = oa.trees();
        let (obp, obv) = ob.trees();
        prop_assert_eq!(to_oracle_clock(&ia), (oap.clone(), oav.clone()));
        prop_assert_eq!(to_oracle_clock(&ib), (obp.clone(), obv.clone()));

        // Versions equal, parties disjoint.
        prop_assert!(ia.version() == ib.version());
        prop_assert!(ia.party().is_disjoint(ib.party()));

        // Re-joining the re-split parties recovers the pre-sync merged party.
        let (pa, _) = ia.into_parts();
        let (pb, _) = ib.into_parts();
        let mut rejoined = pa;
        rejoined.join(pb).expect("disjoint after re-split");
        prop_assert!(rejoined == from_oracle_party(&merged));
    }

    /// E26 + E27. The heterogeneous joins `Version|Version`, `Clock|Version`, and
    /// `Version|Clock` all match the oracle. The latter two encode the
    /// anonymous-as-party-0 identity: the version merges, the party is untouched.
    #[test]
    fn e26_heterogeneous_joins(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (i, j) = (i % n, j % n);
        let ov_i = cs[i].version();
        let ov_j = cs[j].version();

        // Version | Version.
        let exp_vv = ov_i.clone() | ov_j.clone();
        let got_vv = from_oracle_version(&ov_i) | from_oracle_version(&ov_j);
        prop_assert_eq!(to_oracle_version(&got_vv), exp_vv);

        // Clock | Version (party from the clock, versions joined).
        let exp_cv = cs[i].clone() | ov_j.clone();
        let got_cv = from_oracle_clock(&cs[i]) | from_oracle_version(&ov_j);
        let (cvp, cvv) = exp_cv.trees();
        prop_assert_eq!(to_oracle_clock(&got_cv), (cvp.clone(), cvv.clone()));

        // Version | Clock (party from the clock, versions joined).
        let exp_vc = ov_i.clone() | cs[j].clone();
        let got_vc = from_oracle_version(&ov_i) | from_oracle_clock(&cs[j]);
        let (vcp, vcv) = exp_vc.trees();
        prop_assert_eq!(to_oracle_clock(&got_vc), (vcp.clone(), vcv.clone()));
    }
}

// ───────────────────────── batch equivalence / laziness (group G) ─────────────────────────

proptest! {
    /// G30. A batch of ops equals the same ops applied as value-level calls.
    #[test]
    fn g30_batch_equals_value_level(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (i, j) = (i % n, j % n);
        let msg = from_oracle_version(&cs[j].version());

        let mut batched = from_oracle_clock(&cs[i]);
        {
            let mut b = batched.batch();
            b.tick();
            b.merge(&msg);
            b.tick();
        }

        let mut value_level = from_oracle_clock(&cs[i]);
        value_level.tick();
        value_level |= msg.clone();
        value_level.tick();

        prop_assert!(batched.version() == value_level.version());
        prop_assert!(batched.party() == value_level.party());
    }

    /// G31. A batch with no event arithmetic (created-and-dropped, or fork-only)
    /// leaves the version unchanged — the working form is never materialized.
    #[test]
    fn g31_no_arith_batch_preserves_version(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut c = from_oracle_clock(&cs[i % n]);

        let before = c.version();
        {
            let _b = c.batch();
        }
        prop_assert!(c.version() == before);

        let before_fork = c.version();
        {
            let mut b = c.batch();
            let _child = b.fork();
        }
        prop_assert!(c.version() == before_fork);
    }

    /// G32 + F29. The commit happens on drop, and mid-batch comparison already reflects
    /// the uncommitted tick: `batch.version()` equals the post-tick value before drop,
    /// and the underlying clock equals it after drop.
    #[test]
    fn g32_commit_on_drop(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut c = from_oracle_clock(&cs[i % n]);

        let expected = {
            let mut e = from_oracle_clock(&cs[i % n]);
            e.tick();
            e.version()
        };

        let mut b = c.batch();
        b.tick();
        prop_assert!(b.version() == &expected);
        drop(b);
        prop_assert!(c.version() == expected);
    }
}
