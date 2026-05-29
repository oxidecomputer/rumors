//! Clock-level tests.
//!
//! Phase 3 (observers): `has_seen` / `happens_before` / `concurrent_with` agree with
//! the oracle. Phase 6: the master differential harness (groups B 6 & 21), protocol
//! semantics (group E 22–27), and batch equivalence / laziness / commit-on-drop
//! (group G 30–32, plus the F 29 mid-batch comparison).

use proptest::prelude::*;

use crate::oracle;
use crate::test_support::{
    deep_left_spine_party, from_oracle_clock, from_oracle_party, from_oracle_version, run,
    step_impl, to_oracle_clock, to_oracle_party, to_oracle_version, world_strategy, Op,
};
use crate::{Clock, ParseError, Party, Version};

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

// ───────────────────────── normal-form invariant (group A 5) ─────────────────────────

proptest! {
    /// A5. Every value produced by every op is in canonical normal form, checked after
    /// every step of a seed-derived impl-only trace (lowered to oracle trees, which
    /// carry the `is_normal` predicate).
    #[test]
    fn a5_ops_preserve_normal_form(ops in world_strategy()) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
            for c in &imp {
                let (p, v) = to_oracle_clock(c);
                prop_assert!(p.is_normal(), "party not normal: {p:?}");
                prop_assert!(v.is_normal(), "version not normal: {v:?}");
            }
        }
    }
}

// ───────────────────────────── robustness (group H) ─────────────────────────────

/// H33. Deep structures (a depth-100k id spine, and the deep event tree a tick builds
/// over it) survive every op plus the codec and the `Debug` printer with no stack
/// overflow — the proof that every traversal is iterative. Impl-only: the recursive
/// oracle cannot build or even drop a tree this deep (oracle agreement at bounded depth
/// is the master harness's job, §8).
#[test]
fn h33_deep_tree_stack_safety() {
    const DEPTH: usize = 100_000;
    let party = deep_left_spine_party(DEPTH);
    let mut clock = Clock::from_parts(party, Version::new());

    // Codec over a deep id round-trips to canonical bytes.
    let bytes = clock.encode();
    let decoded = Clock::decode(&bytes).expect("deep id encodes to canonical bytes");
    assert_eq!(decoded.encode(), bytes);

    // Ticks build, then refine, a deep event tree (unpack / fill / grow / repack).
    clock.tick();
    clock.tick();

    // Observing ops over the deep version do not overflow.
    let v = clock.version();
    assert_eq!(v.partial_cmp(&v), Some(core::cmp::Ordering::Equal));
    assert_eq!(v.clone() | v.clone(), v);

    // Codec over a deep id + deep event tree round-trips.
    let bytes = clock.encode();
    assert_eq!(
        Clock::decode(&bytes)
            .expect("deep clock encodes canonically")
            .encode(),
        bytes
    );

    // Fork (deep split + snapshot) yields a disjoint child; join restores the whole.
    let child = clock.fork();
    assert!(clock.party().is_disjoint(child.party()));
    clock.join(child).expect("fork halves are disjoint");

    // The iterative Debug pretty-printer must not overflow either.
    assert!(!format!("{clock:?}").is_empty());
}

proptest! {
    /// H34. `decode` of arbitrary bytes never panics; it returns `Ok` or `Err`. Any
    /// accepted value is canonical: re-encoding then decoding yields it again.
    #[test]
    fn h34_decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(p) = Party::decode(&bytes) {
            prop_assert_eq!(Party::decode(&p.encode()).ok(), Some(p));
        }
        if let Ok(v) = Version::decode(&bytes) {
            prop_assert_eq!(Version::decode(&v.encode()).ok(), Some(v));
        }
        if let Ok(c) = Clock::decode(&bytes) {
            let re = Clock::decode(&c.encode()).expect("re-encode of an accepted clock is canonical");
            prop_assert_eq!(re.encode(), c.encode());
        }
    }
}

// ───────────────────────────── worked example (group I) ─────────────────────────────

/// I35 (paper §5.1). The paper's example run, step by step: seed forks to two; one
/// ticks then forks; the other ticks twice; one of three ticks while the other two
/// sync; finally all rejoin to the whole space and a tick collapses the event tree to a
/// single integer. Mirrors the oracle's `o15_worked_example` on the impl.
#[test]
fn i35_worked_example() {
    // Whole-space region check, computed structurally (parties are not `Clone`).
    let region = |clocks: &[&Clock]| {
        let mut acc = oracle::Party::Leaf(false);
        for c in clocks {
            acc.join(to_oracle_party(c.party()))
                .expect("participants own disjoint regions");
        }
        acc
    };

    // seed -> fork into two.
    let mut p1 = Clock::seed();
    let mut p2 = p1.fork();

    // p1 suffers one event, then forks.
    p1.tick();
    let mut p1a = p1.fork();
    let mut p1b = p1;

    // p2 suffers two events.
    p2.tick();
    p2.tick();

    // Three participants covering the whole space.
    assert_eq!(region(&[&p1a, &p1b, &p2]), oracle::Party::seed());

    // One participant ticks; the other two sync.
    let before = p1a.version();
    p1a.tick();
    assert!(p1a.version() > before);

    let merged_region = {
        let mut acc = to_oracle_party(p1b.party());
        acc.join(to_oracle_party(p2.party())).expect("disjoint");
        acc
    };
    p1b.sync(&mut p2).expect("disjoint");

    // Sync reconciled histories and preserved total ownership of the two halves.
    assert!(p1b.version() == p2.version());
    let mut rejoined = to_oracle_party(p1b.party());
    rejoined
        .join(to_oracle_party(p2.party()))
        .expect("disjoint");
    assert_eq!(rejoined, merged_region);
    assert_eq!(region(&[&p1a, &p1b, &p2]), oracle::Party::seed());

    // Rejoin all three (recovering id = 1) and tick: the id owns the whole space, so the
    // event tree collapses to a single integer.
    let mut whole = p1a;
    whole.join(p1b).expect("disjoint");
    whole.join(p2).expect("disjoint");
    assert_eq!(to_oracle_party(whole.party()), oracle::Party::seed());
    whole.tick();
    assert!(
        matches!(
            to_oracle_version(&whole.version()),
            oracle::Version::Leaf(_)
        ),
        "post-join event should collapse to a single integer, got {:?}",
        whole.version()
    );
}

// ───────────────────── Display / FromStr / TryFrom (paper notation) ─────────────────────

proptest! {
    /// `Display` then `FromStr` round-trips for every type, and the printed form is the
    /// canonical paper notation (re-parsing yields the same value).
    #[test]
    fn display_fromstr_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        let ps = p.to_string();
        prop_assert_eq!(ps.parse::<Party>().expect("Display is valid paper notation"), p);

        let vs = v.to_string();
        prop_assert_eq!(vs.parse::<Version>().expect("Display is valid paper notation"), v);

        let cstr = c.to_string();
        let cparsed: Clock = cstr.parse().expect("Display is valid paper notation");
        prop_assert_eq!(cparsed.encode(), c.encode());
    }
}

/// Display renders the paper's notation exactly (id `0/1/(l, r)`, event `n/(n, e1, e2)`,
/// stamp `(i, e)`), matching the paper's §5 examples.
#[test]
fn display_matches_paper_notation() {
    assert_eq!(Party::seed().to_string(), "1");
    assert_eq!(Version::new().to_string(), "0");
    assert_eq!(Clock::seed().to_string(), "(1, 0)");

    let id: Party = "((0, (1, 0)), (1, 0))".parse().unwrap();
    assert_eq!(id.to_string(), "((0, (1, 0)), (1, 0))");

    let ev: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
    assert_eq!(ev.to_string(), "(1, 2, (0, (1, 0, 2), 0))");

    // Debug is the same as Display.
    assert_eq!(format!("{id:?}"), "((0, (1, 0)), (1, 0))");
    assert_eq!(format!("{ev:?}"), "(1, 2, (0, (1, 0, 2), 0))");
    assert_eq!(
        format!("{:?}", Clock::seed()),
        "Clock { party: 1, version: 0 }"
    );
}

/// `TryFrom` literals build the same values as the equivalent paper-notation strings,
/// grounding out in the `u8`/`u64` base cases.
#[test]
fn tryfrom_literals_build_values() {
    let p = Party::try_from((1u8, (0u8, 1u8))).unwrap();
    assert_eq!(p, "(1, (0, 1))".parse::<Party>().unwrap());

    let v = Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64))).unwrap();
    assert_eq!(v, "(1, 0, (2, 0, 1))".parse::<Version>().unwrap());

    let c = Clock::try_from(((1u8, 0u8), 5u64)).unwrap();
    assert_eq!(c.encode(), "((1, 0), 5)".parse::<Clock>().unwrap().encode());

    // Base cases. `1` is a valid party; `0` is anonymous on its own but fine as a
    // sub-tree (see the `(0, 1)` cases above).
    assert_eq!(Party::try_from(1u8).unwrap().to_string(), "1");
    assert_eq!(Party::try_from(0u8), Err(ParseError::Anonymous));
    assert_eq!(Version::try_from(7u64).unwrap().to_string(), "7");
}

/// `FromStr` and `TryFrom` reject both malformed input and well-formed-but-denormal
/// input, mirroring `decode`'s strictness.
#[test]
fn fromstr_tryfrom_reject_denormal_and_syntax() {
    // Denormal (well-formed but not canonical).
    assert_eq!("(1, 1)".parse::<Party>(), Err(ParseError::NotCanonical));
    assert_eq!(Party::try_from((1u8, 1u8)), Err(ParseError::NotCanonical));
    assert_eq!(
        "(5, 3, 3)".parse::<Version>(),
        Err(ParseError::NotCanonical)
    );
    assert_eq!(
        "(1, 2, 3)".parse::<Version>(),
        Err(ParseError::NotCanonical)
    );
    assert_eq!(
        Version::try_from((1u64, 2u64, 3u64)),
        Err(ParseError::NotCanonical)
    );

    // Syntax (malformed).
    assert_eq!("(1, 2".parse::<Party>(), Err(ParseError::Syntax)); // unbalanced
    assert_eq!("2".parse::<Party>(), Err(ParseError::Syntax)); // id leaves are only 0/1
    assert_eq!(Party::try_from(2u8), Err(ParseError::Syntax));
    assert_eq!("".parse::<Version>(), Err(ParseError::Syntax)); // empty
    assert_eq!("(1, 0)".parse::<Version>(), Err(ParseError::Syntax)); // event needs 3 parts
    assert_eq!(
        "(99999999999999999999, 0, 0)".parse::<Version>(),
        Err(ParseError::Syntax) // integer overflows u64: rejected, not panicked
    );
    assert_eq!("(café, 0)".parse::<Clock>().err(), Some(ParseError::Syntax)); // non-ASCII byte

    // Anonymous identity `0` is rejected as a standalone party (but allowed as a
    // sub-tree, exercised in `tryfrom_literals_build_values`).
    assert_eq!("0".parse::<Party>(), Err(ParseError::Anonymous));
    assert_eq!(Party::try_from(0u8), Err(ParseError::Anonymous));
    assert_eq!("(0, 1)".parse::<Party>().unwrap().to_string(), "(0, 1)"); // 0 as sub-tree: ok
                                                                          // Clock has no `PartialEq`, so compare the error directly.
    assert_eq!("(0, 5)".parse::<Clock>().err(), Some(ParseError::Anonymous)); // anonymous party
    assert_eq!(
        Clock::try_from((0u8, 5u64)).err(),
        Some(ParseError::Anonymous)
    );

    // Whitespace is tolerated.
    assert_eq!(
        " ( 1 , ( 0 , 1 ) ) ".parse::<Party>().unwrap().to_string(),
        "(1, (0, 1))"
    );
}

// ───────────────────────────── serde (group I, feature-gated) ─────────────────────────────

#[cfg(feature = "serde")]
proptest! {
    /// Every value round-trips through serde (here `serde_json`), since it serializes as
    /// its canonical encoding and deserializes back through the strict validator.
    #[test]
    fn serde_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        let p2: Party = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        let v2: Version = serde_json::from_slice(&serde_json::to_vec(&v).unwrap()).unwrap();
        let c2: Clock = serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();

        prop_assert_eq!(p2, p);
        prop_assert_eq!(v2, v);
        prop_assert_eq!(c2.encode(), c.encode());
    }
}
