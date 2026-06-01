//! Clock-level tests.
//!
//! Observers: `has_seen` / `happens_before` / `concurrent_with` agree with the
//! oracle. The master differential harness, protocol semantics, and batch
//! equivalence / laziness / commit-on-drop (plus the mid-batch comparison).

use proptest::prelude::*;

use super::Batch;
use crate::oracle;
use crate::testing::bridge::{
    from_oracle_clock, from_oracle_party, from_oracle_version, to_oracle_clock, to_oracle_party,
    to_oracle_version,
};
use crate::testing::generators::deep_left_spine_party;
use crate::testing::optrace::{run, step_impl, world_strategy, Op};
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

        prop_assert_eq!(ia.version() >= &msg, oa.version() >= msg_oracle);
        prop_assert_eq!(ia.version() < ib.version(), oa.version() < ob.version());
        prop_assert_eq!(ia.version().concurrent(ib.version()), oa.concurrent_with(ob));
    }
}

// ───────────────────────────── master differential harness ─────────────────────────────

proptest! {
    /// A seed-derived op trace, applied in lockstep to the oracle and the
    /// impl, agrees structurally on every live clock after every step, and all live
    /// impl parties stay pairwise disjoint (so `join`/`sync` never error in correct
    /// usage). Agreement is by structural lowering — `to_oracle_clock` rebuilds the
    /// oracle's tree shape from the impl's internal packed bits — not via the byte
    /// codec, which the per-trace round-trip below exercises separately.
    #[test]
    fn master_differential(ops in world_strategy()) {
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
                    let im = imp[i].send().clone();
                    ora[j].receive(om);
                    imp[j].receive(&im);
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

// ───────────────────────────── protocol semantics ─────────────────────────────

proptest! {
    /// `fork` preserves the version on both halves.
    #[test]
    fn fork_preserves_version(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut c = from_oracle_clock(&cs[i % n]);
        let before = c.version().clone();
        let child = c.fork();
        prop_assert!(c.version() == &before);
        prop_assert!(child.version() == &before);
    }

    /// `version()` (peek) does not advance the clock; the returned `Version`
    /// equals the clock's own and repeated peeks are stable.
    #[test]
    fn peek_does_not_advance(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let c = from_oracle_clock(&cs[i % n]);
        let before = c.encode();
        let v1 = c.version();
        let v2 = c.version();
        prop_assert!(v1 == v2);
        prop_assert_eq!(c.encode(), before);
    }

    /// `receive(msg)` with `msg <= self` (here `msg == self.version()`) equals a
    /// bare `tick`: an own-message receive is benign, and the party is unchanged.
    #[test]
    fn own_receive_is_tick(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut received = from_oracle_clock(&cs[i % n]);
        let mut ticked = from_oracle_clock(&cs[i % n]);
        let own = received.version().clone();
        received.receive(&own);
        ticked.tick();
        prop_assert!(received.version() == ticked.version());
        prop_assert!(received.party() == ticked.party());
    }

    /// After `a.sync(&mut b)`: both end at the oracle's result, their versions are
    /// equal, their parties are disjoint, and re-joining the two parties recovers the
    /// pre-sync merged party.
    #[test]
    fn sync(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        if n < 2 {
            return Ok(());
        }
        // Derive two distinct members directly rather than rejecting collisions —
        // small populations collide often, and `prop_assume` would blow the reject
        // cap under a high case count (see the oracle `sync` test).
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

    /// The heterogeneous joins `Version|Version`, `Clock|Version`, and
    /// `Version|Clock` all match the oracle. The latter two encode the
    /// anonymous-as-party-0 identity: the version merges, the party is untouched.
    #[test]
    fn heterogeneous_joins(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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

    /// Assigning forms. The `Clock` assigning / batch join surfaces merge the
    /// version and leave the party untouched, matching the oracle — complementing the
    /// by-value `Clock | Version` above. Covers `Clock |= Version`, the `From<&mut
    /// Clock>` batch conversion, the `clock::Batch |= &Version` operator (committed on
    /// drop), and the `clock::Batch::party` accessor.
    #[test]
    fn clock_assign_join_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (i, j) = (i % n, j % n);
        let msg_oracle = cs[j].version();

        // Oracle expectation: party unchanged, version merged.
        let expected = cs[i].clone() | msg_oracle.clone();
        let (ep, ev) = expected.trees();

        // `Clock |= Version`.
        let mut assign = from_oracle_clock(&cs[i]);
        assign |= from_oracle_version(&msg_oracle);
        prop_assert_eq!(to_oracle_clock(&assign), (ep.clone(), ev.clone()));

        // `clock::Batch |= &Version`, over a batch built via `From<&mut Clock>`,
        // committed on drop. The `party` accessor reflects the unchanged party
        // mid-session.
        let msg = from_oracle_version(&msg_oracle);
        let mut batched = from_oracle_clock(&cs[i]);
        {
            let mut batch: Batch = (&mut batched).into();
            batch |= &msg;
            prop_assert_eq!(to_oracle_party(batch.party()), ep.clone());
        }
        prop_assert_eq!(to_oracle_clock(&batched), (ep.clone(), ev.clone()));
    }
}

// ───────────────────────── batch equivalence / laziness ─────────────────────────

proptest! {
    /// A batch of ops equals the same ops applied as value-level calls.
    #[test]
    fn batch_equals_value_level(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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

    /// A batch with no event arithmetic (created-and-dropped, or fork-only)
    /// leaves the version unchanged — the working form is never materialized.
    #[test]
    fn no_arith_batch_preserves_version(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut c = from_oracle_clock(&cs[i % n]);

        let before = c.version().clone();
        {
            let _b = c.batch();
        }
        prop_assert!(c.version() == &before);

        let before_fork = c.version().clone();
        {
            let mut b = c.batch();
            let _child = b.fork();
        }
        prop_assert!(c.version().clone() == before_fork);
    }

    /// The commit happens on drop, and mid-batch comparison already reflects
    /// the uncommitted tick: `batch.version()` equals the post-tick value before drop,
    /// and the underlying clock equals it after drop.
    #[test]
    fn commit_on_drop(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut c = from_oracle_clock(&cs[i % n]);

        let expected = {
            let mut e = from_oracle_clock(&cs[i % n]);
            e.tick();
            e.version().clone()
        };

        let mut b = c.batch();
        b.tick();
        prop_assert!(b.version() == &expected);
        drop(b);
        prop_assert!(c.version() == &expected);
    }
}

// ───────────────────────── normal-form invariant ─────────────────────────

proptest! {
    /// Every value produced by every op is in canonical normal form, checked after
    /// every step of a seed-derived impl-only trace (lowered to oracle trees, which
    /// carry the `is_normal` predicate).
    #[test]
    fn ops_preserve_normal_form(ops in world_strategy()) {
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

// ───────────────────────────── robustness ─────────────────────────────

/// Deep structures (a depth-100k id spine, and the deep event tree a tick builds
/// over it) survive *every* public op plus the codec and the `Debug` printer with no
/// stack overflow — the proof that every traversal is iterative. Beyond the single-clock
/// ops (tick / fork / join / partial_cmp / `|` / encode / decode / Debug), this drives
/// the composite and observer ops that operate on deep structures: `sync` between two
/// deep clocks, `send`/`receive` of a deep version, and each clock observer
/// (`has_seen` / `happens_before` / `concurrent_with`) at depth. Impl-only: the
/// recursive oracle cannot build or even drop a tree this deep (oracle agreement at
/// bounded depth is the master differential harness's job).
#[test]
fn deep_tree_stack_safety() {
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
    assert_eq!(v.partial_cmp(v), Some(core::cmp::Ordering::Equal));
    assert_eq!(v.clone() | v.clone(), *v);

    // Codec over a deep id + deep event tree round-trips.
    let bytes = clock.encode();
    assert_eq!(
        Clock::decode(&bytes)
            .expect("deep clock encodes canonically")
            .encode(),
        bytes
    );

    // `send`/`receive` over the deep clock: `send` extracts a deep version (and ticks the
    // clock's event tree), and a self-`receive` (the sent message is `<= self`) merges a
    // deep version into a deep clock without overflow.
    let msg = clock.send().clone();
    clock.receive(&msg);

    // Observers over a deep clock and a deep message do not overflow: `has_seen` lowers to
    // a deep `causal_cmp` against the version, and the clock-vs-clock observers compare two
    // deep versions.
    let sent = clock.send().clone();
    assert!(clock.version() >= &sent);
    assert!(!(clock.version() >= clock.version()));
    assert!(!(clock.version().partial_cmp(clock.version()).is_none()));

    // Fork (deep split + snapshot) yields a disjoint child; both halves stay deep.
    let mut child = clock.fork();
    assert!(clock.party().is_disjoint(child.party()));

    // `sync` between two deep clocks is the most complex composite (fork + join + merge of
    // deep structures). Drive it and assert it reconciles without overflow: post-sync the
    // two versions are equal, the parties stay disjoint, and the observers agree they are
    // neither strictly ordered nor concurrent.
    clock.sync(&mut child).expect("fork halves are disjoint");
    assert!(clock.version() == child.version());
    assert!(clock.party().is_disjoint(child.party()));
    assert!(!(clock.version() >= child.version()));
    assert!(!(clock.version().concurrent(child.version())));

    // join restores the whole from the two deep halves.
    clock.join(child).expect("fork halves are disjoint");

    // The iterative Debug pretty-printer must not overflow either.
    assert!(!format!("{clock:?}").is_empty());
}

proptest! {
    /// `decode` of arbitrary bytes never panics; it returns `Ok` or `Err`.
    /// Any accepted value satisfies the keystone invariant `decode(b) == Ok(x) ⟹
    /// is_normal(x)`: lowering it to the oracle yields a normal-form tree. This — not the
    /// re-encode round-trip alone — is what makes the byte-equality `Eq`/`Hash` sound: a
    /// non-normal accept would give two distinct byte strings for one logical value. The
    /// re-encode-then-decode round-trip is also asserted (canonical encoding is stable).
    #[test]
    fn decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(p) = Party::decode(&bytes) {
            prop_assert!(to_oracle_party(&p).is_normal(), "accepted a non-normal Party");
            prop_assert_eq!(Party::decode(&p.encode()).ok(), Some(p));
        }
        if let Ok(v) = Version::decode(&bytes) {
            prop_assert!(to_oracle_version(&v).is_normal(), "accepted a non-normal Version");
            prop_assert_eq!(Version::decode(&v.encode()).ok(), Some(v));
        }
        if let Ok(c) = Clock::decode(&bytes) {
            let (p, v) = to_oracle_clock(&c);
            prop_assert!(p.is_normal() && v.is_normal(), "accepted a non-normal Clock");
            let re = Clock::decode(&c.encode()).expect("re-encode of an accepted clock is canonical");
            prop_assert_eq!(re.encode(), c.encode());
        }
    }
}

// ─────────────────────── decoded-component canonicity (regression) ───────────────────────
//
// `Clock::encode` lays the id directly before the event, so the event begins at a
// generally non-byte-aligned bit offset. A `decode` that extracts the event with
// `slice.to_bitvec()` keeps that head offset (rather than shifting to bit 0), leaving the
// recovered `Version`'s packed stream non-canonical: `version().encode()` mis-packs it and
// `Version::decode` then disagrees. Whole-clock round-trips hide this, because
// `Clock::encode` re-aligns each component via `extend_from_bitslice`; the bug only shows
// when a component extracted from a decoded clock is encoded on its own.

/// The seed's id is two bits, so its event starts at a non-byte-aligned offset. Decoding
/// the seed and re-encoding the recovered version must reproduce the canonical encoding
/// (and survive its own `decode`), not an offset-shifted one.
#[test]
fn decoded_seed_version_encodes_canonically() {
    let seed = Clock::seed();
    let decoded = Clock::decode(&seed.encode()).unwrap();
    assert_eq!(decoded.version().encode(), seed.version().encode());
    assert_eq!(
        &Version::decode(&decoded.version().encode()).unwrap(),
        seed.version(),
    );
}

proptest! {
    /// For any seed-derived clock, decoding it preserves each component's canonical
    /// byte encoding, and the extracted party and version each round-trip through their
    /// own `decode`. Guards the whole class of non-byte-aligned offset extraction.
    #[test]
    fn decode_preserves_component_canonicity(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let original = from_oracle_clock(&cs[i % n]);
        let decoded = Clock::decode(&original.encode()).expect("re-decode canonical clock");

        prop_assert_eq!(decoded.party().encode(), original.party().encode());
        prop_assert_eq!(decoded.version().encode(), original.version().encode());

        let v = decoded.version();
        prop_assert_eq!(&Version::decode(&v.encode()).unwrap(), v);
        let p_bytes = decoded.party().encode();
        prop_assert_eq!(Party::decode(&p_bytes).unwrap().encode(), p_bytes);
    }
}

// ───────────────────────────── worked example ─────────────────────────────

/// Paper §5.1. The paper's example run, step by step: seed forks to two; one
/// ticks then forks; the other ticks twice; one of three ticks while the other two
/// sync; finally all rejoin to the whole space and a tick collapses the event tree to a
/// single integer. Mirrors the oracle's `worked_example` on the impl.
#[test]
fn worked_example() {
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
    let before = p1a.version().clone();
    p1a.tick();
    assert!(p1a.version() > &before);

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

    // Arbitrary-precision bases round-trip: a base past `u64::MAX` (2^64) parses,
    // re-renders, and decodes unchanged — there is no integer-width cap.
    let wide: Version = "(18446744073709551616, 0, 1)".parse().unwrap();
    assert_eq!(wide.to_string(), "(18446744073709551616, 0, 1)");
    assert_eq!(Version::decode(&wide.encode()).unwrap(), wide);

    // Debug is the same as Display.
    assert_eq!(format!("{id:?}"), "((0, (1, 0)), (1, 0))");
    assert_eq!(format!("{ev:?}"), "(1, 2, (0, (1, 0, 2), 0))");
    assert_eq!(
        format!("{:?}", Clock::seed()),
        "Clock { party: 1, version: 0 }"
    );
}

/// `TryFrom` literals build the same values as the equivalent paper-notation strings,
/// grounding out in the `bool`/`u8`/`u64` base cases.
#[test]
fn tryfrom_literals_build_values() {
    let p = Party::try_from((1, (0, 1))).unwrap();
    assert_eq!(p, "(1, (0, 1))".parse::<Party>().unwrap());

    let p = Party::try_from((true, false)).unwrap();
    assert_eq!(p, "(1, 0)".parse::<Party>().unwrap());

    let v = Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64))).unwrap();
    assert_eq!(v, "(1, 0, (2, 0, 1))".parse::<Version>().unwrap());

    let c = Clock::try_from(((1u8, 0u8), 5u64)).unwrap();
    assert_eq!(c.encode(), "((1, 0), 5)".parse::<Clock>().unwrap().encode());

    // Base cases. `1` is a valid party; `0` is anonymous on its own but fine as a
    // sub-tree (see the `(0, 1)` cases above).
    assert_eq!(Party::try_from(1u8).unwrap().to_string(), "1");
    assert_eq!(Party::try_from(0u8), Err(ParseError::Anonymous));
    assert_eq!(Party::try_from(false), Err(ParseError::Anonymous));
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

// ───────────────────────────── serde (feature-gated) ─────────────────────────────

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

    /// `serde_json` represents `serialize_bytes` as a JSON number-array, decoded back via
    /// `visit_seq` — so it never exercises the binary `serialize_bytes`/`visit_bytes` path.
    /// Pin that path through two non-JSON formats: `postcard` (non-self-describing,
    /// length-prefixed bytes) and `ciborium` (self-describing CBOR, which emits a *typed*
    /// byte-string — CBOR major type 2). Every type must round-trip through both: the
    /// serialized form is the canonical encoding, deserialization re-validates it, and the
    /// CBOR typed-bytes path is the one `serde_json` alone can never reach.
    #[test]
    fn serde_roundtrip_binary_formats(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let p = from_oracle_party(cs[i % n].party());
        let v = from_oracle_version(&cs[i % n].version());
        let c = from_oracle_clock(&cs[i % n]);

        // postcard: non-self-describing binary format.
        let p2: Party = postcard::from_bytes(&postcard::to_allocvec(&p).unwrap()).unwrap();
        let v2: Version = postcard::from_bytes(&postcard::to_allocvec(&v).unwrap()).unwrap();
        let c2: Clock = postcard::from_bytes(&postcard::to_allocvec(&c).unwrap()).unwrap();
        prop_assert_eq!(&p2, &p);
        prop_assert_eq!(&v2, &v);
        prop_assert_eq!(c2.encode(), c.encode());

        // ciborium: self-describing CBOR. Each value must serialize as a byte string
        // (major type 2) and deserialize back through `Vec<u8>`'s `visit_bytes`.
        let cbor = |bytes: &[u8]| -> u8 { bytes[0] >> 5 };

        let mut b = Vec::new();
        ciborium::ser::into_writer(&p, &mut b).unwrap();
        prop_assert_eq!(cbor(&b), 2, "Party did not serialize as a CBOR byte string");
        let p3: Party = ciborium::de::from_reader(&b[..]).unwrap();
        prop_assert_eq!(&p3, &p);

        let mut b = Vec::new();
        ciborium::ser::into_writer(&v, &mut b).unwrap();
        prop_assert_eq!(cbor(&b), 2, "Version did not serialize as a CBOR byte string");
        let v3: Version = ciborium::de::from_reader(&b[..]).unwrap();
        prop_assert_eq!(&v3, &v);

        let mut b = Vec::new();
        ciborium::ser::into_writer(&c, &mut b).unwrap();
        prop_assert_eq!(cbor(&b), 2, "Clock did not serialize as a CBOR byte string");
        let c3: Clock = ciborium::de::from_reader(&b[..]).unwrap();
        prop_assert_eq!(c3.encode(), c.encode());
    }
}
