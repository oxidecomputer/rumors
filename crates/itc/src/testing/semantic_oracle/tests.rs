//! Cross-check the three references — production impl, recursive tree [`oracle`], and the
//! function-space [`super`] model — by replaying one multi-seed op trace against all three
//! and requiring their final clock populations to agree on every pairwise comparison. Plus a
//! law suite proving the function-space operations are a sound ITC, and unit anchors pinning
//! the embedding against a value the paper states.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::{
    disjoint, ev_depth, ev_order, event, id_depth, id_order, join, lift_ev, lift_id, sum, Dyadic,
    Event, Id, SemClock, GRID_N,
};
use crate::codec::Base;
use crate::oracle;
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{world_strategy, Op};
use crate::Clock;

// ───────────────────────────── helpers ─────────────────────────────

/// Function equality at granularity `g` (both directions of the order are `Equal`).
fn ev_eq(a: &Event, b: &Event, g: u32) -> bool {
    ev_order(a, b, g) == Some(Ordering::Equal)
}
/// Id function equality at granularity `g`.
fn id_eq(a: &Id, b: &Id, g: u32) -> bool {
    id_order(a, b, g) == Some(Ordering::Equal)
}

/// A grid exponent that resolves an oracle event/id pair, with one level of headroom, capped
/// at [`GRID_N`]. The semantic event's step structure sits at the id's fork boundaries, so the
/// id depth bounds it too — hence both are folded in.
fn grid_for(parts: &[u32]) -> u32 {
    (parts.iter().copied().max().unwrap_or(0) + 1).min(GRID_N)
}

// ───────────────────────────── replay cross-check ─────────────────────────────

/// Play `ops` (starting from `seeds` independent seed clocks) against all three references in
/// lockstep, asserting they agree on every fallible op's outcome, and return the three final
/// populations (index-aligned). Multiple seeds start mutually non-disjoint, so the overlap and
/// concurrent arms are exercised; a `Join`/`Sync` on overlapping parties is a no-op in all
/// three (disjointness is invariant).
fn replay(seeds: usize, ops: &[Op]) -> (Vec<Clock>, Vec<oracle::Clock>, Vec<SemClock>) {
    let mut im: Vec<Clock> = (0..seeds).map(|_| Clock::seed()).collect();
    let mut or: Vec<oracle::Clock> = (0..seeds).map(|_| oracle::Clock::seed()).collect();
    let mut se: Vec<SemClock> = (0..seeds).map(|_| SemClock::seed()).collect();

    for op in ops {
        let n = im.len();
        match *op {
            Op::Tick(i) => {
                let i = i % n;
                im[i].tick();
                or[i].tick();
                se[i].tick();
            }
            Op::Fork(i) => {
                let i = i % n;
                let c = im[i].fork();
                im.push(c);
                let c = or[i].fork();
                or.push(c);
                let c = se[i].fork();
                se.push(c);
            }
            Op::Send(i, j) => {
                let (i, j) = (i % n, j % n);
                let m = im[i].send();
                im[j].receive(m);
                let m = or[i].send();
                or[j].receive(m);
                let m = se[i].send();
                se[j].receive(m);
            }
            Op::Sync(i, j) => {
                let (i, j) = (i % n, j % n);
                if i != j {
                    let (lo, hi) = (i.min(j), i.max(j));
                    let d_im = im[i].party().is_disjoint(im[j].party());
                    let d_or = or[i].party().is_disjoint(or[j].party());
                    let d_se = disjoint(&se[i].id, &se[j].id, GRID_N);
                    assert!(
                        d_im == d_or && d_or == d_se,
                        "sync disjointness disagreement"
                    );
                    if d_im {
                        let (a, b) = im.split_at_mut(hi);
                        assert!(a[lo].sync(&mut b[0]).is_ok());
                        let (a, b) = or.split_at_mut(hi);
                        assert!(a[lo].sync(&mut b[0]).is_ok());
                        let (a, b) = se.split_at_mut(hi);
                        assert!(a[lo].sync(&mut b[0], GRID_N).is_ok());
                    }
                }
            }
            Op::Join(i, j) => {
                if n > 1 {
                    let (i, j) = (i % n, j % n);
                    if i != j {
                        let d_im = im[i].party().is_disjoint(im[j].party());
                        let d_or = or[i].party().is_disjoint(or[j].party());
                        let d_se = disjoint(&se[i].id, &se[j].id, GRID_N);
                        assert!(
                            d_im == d_or && d_or == d_se,
                            "join disjointness disagreement"
                        );
                        if d_im {
                            let i2 = if j < i { i - 1 } else { i };
                            let v = im.remove(j);
                            assert!(im[i2].join(v).is_ok());
                            let v = or.remove(j);
                            assert!(or[i2].join(v).is_ok());
                            let v = se.remove(j);
                            assert!(se[i2].join(v, GRID_N).is_ok());
                        }
                    }
                }
            }
        }
    }
    (im, or, se)
}

proptest! {
    /// The keystone check. After the same op trace, every ordered pair of final clocks has the
    /// same comparison descriptor — (version causal order, party containment order, party
    /// disjointness) — under all three references. The function space computes its descriptor
    /// purely by sampling its closures; the impl and oracle use their native ops. Agreement
    /// across all three is the satisfaction condition: it is ITC's defining guarantee that the
    /// observable partial order is fixed by the operation sequence, independent of the (valid)
    /// fork/inflation policy each implementation chooses.
    ///
    /// Single seed, deliberately: that guarantee holds only for a *proper* ITC system, where
    /// ids partition one space and all live ids are disjoint. Multiple seeds would have several
    /// stamps each owning all of `[0,1)` — not a valid configuration — and a cross-lineage
    /// `receive` then entangles events on the overlapping region, where the add-one and minimal
    /// `grow` policies genuinely disagree on the order. (Disjointness/overlap and the
    /// join/sync-`Err` paths are id-algorithm behavior, exercised against the oracle elsewhere.)
    #[test]
    fn replay_matches_across_references(ops in world_strategy()) {
        let (im, or, se) = replay(1, &ops);
        let n = im.len();
        // One granularity for all the function-space scans: the deepest tree in the population
        // (capped, guarded by `grid_cap_is_never_reached`). A single seed's events live only on
        // the disjoint owned regions, so the id depth resolves them; the impl's *minimized*
        // event can be shallower than the add-one event, so its depth alone would under-resolve.
        let g = or
            .iter()
            .map(|c| id_depth(c.party()).max(ev_depth(&c.version())))
            .max()
            .map_or(0, |d| (d + 1).min(GRID_N));
        for i in 0..n {
            for j in 0..n {
                let (ovi, ovj) = (or[i].version(), or[j].version());
                let (ivi, ivj) = (im[i].version(), im[j].version());
                let d_impl = (
                    ivi.partial_cmp(&ivj),
                    im[i].party().partial_cmp(im[j].party()),
                    im[i].party().is_disjoint(im[j].party()),
                );
                let d_oracle = (
                    ovi.partial_cmp(&ovj),
                    or[i].party().partial_cmp(or[j].party()),
                    or[i].party().is_disjoint(or[j].party()),
                );
                let d_sem = (
                    ev_order(&se[i].ev, &se[j].ev, g),
                    id_order(&se[i].id, &se[j].id, g),
                    disjoint(&se[i].id, &se[j].id, g),
                );
                prop_assert_eq!(d_impl, d_oracle, "impl vs oracle at ({}, {})", i, j);
                prop_assert_eq!(d_oracle, d_sem, "function-space vs oracle at ({}, {})", i, j);
            }
        }
    }
}

// ───────────────────────────── law suite (the function-space model is a sound ITC) ─────────────────────────────

proptest! {
    /// The event order is a partial order: reflexive and transitive over arbitrary events
    /// (antisymmetry is the `Equal` case of the replay's order agreement).
    #[test]
    fn order_is_a_partial_order(
        a in arb_oracle_version(), b in arb_oracle_version(), c in arb_oracle_version(),
    ) {
        let g = grid_for(&[ev_depth(&a), ev_depth(&b), ev_depth(&c)]);
        let (fa, fb, fc) = (lift_ev(a), lift_ev(b), lift_ev(c));
        prop_assert_eq!(ev_order(&fa, &fa, g), Some(Ordering::Equal)); // reflexive
        // transitive: a ≤ b ≤ c ⇒ a ≤ c
        let le = |x: &Event, y: &Event| matches!(ev_order(x, y, g), Some(Ordering::Less | Ordering::Equal));
        if le(&fa, &fb) && le(&fb, &fc) {
            prop_assert!(le(&fa, &fc));
        }
    }

    /// `join` is the least upper bound: idempotent, commutative, associative, and an upper
    /// bound (`a ≤ a∨b`).
    #[test]
    fn join_is_the_lub(a in arb_oracle_version(), b in arb_oracle_version(), c in arb_oracle_version()) {
        let g = grid_for(&[ev_depth(&a), ev_depth(&b), ev_depth(&c)]);
        let (fa, fb, fc) = (lift_ev(a), lift_ev(b), lift_ev(c));
        prop_assert!(ev_eq(&join(fa.clone(), fa.clone()), &fa, g));                       // idempotent
        prop_assert!(ev_eq(&join(fa.clone(), fb.clone()), &join(fb.clone(), fa.clone()), g)); // commutative
        let l = join(join(fa.clone(), fb.clone()), fc.clone());
        let r = join(fa.clone(), join(fb.clone(), fc.clone()));
        prop_assert!(ev_eq(&l, &r, g));                                                   // associative
        let ab = join(fa.clone(), fb.clone());
        prop_assert!(
            matches!(ev_order(&fa, &ab, g), Some(Ordering::Less | Ordering::Equal)),
            "a is not <= a|b",
        );
    }

    /// `fork` partitions: the two halves are disjoint and recombine (`sum`) to the original;
    /// reproduces the paper's split exactly, so it also matches the oracle's `fork`.
    #[test]
    fn fork_partitions_and_matches_oracle(p in arb_oracle_party_nonempty()) {
        let g = grid_for(&[id_depth(&p)]);
        let i = lift_id(p.clone());
        let (l, r) = super::fork(i.clone());
        prop_assert!(disjoint(&l, &r, g), "fork halves overlap");
        prop_assert!(id_eq(&sum(l.clone(), r.clone()), &i, g), "fork halves do not recombine");
        // Reproduces the oracle's split as a set of two halves (order-independent, since the
        // kept/given convention is incidental).
        let mut keep = p.clone();
        let give = keep.fork();
        let (ok, og) = (lift_id(keep), lift_id(give));
        let matches = (id_eq(&l, &ok, g) && id_eq(&r, &og, g))
            || (id_eq(&l, &og, g) && id_eq(&r, &ok, g));
        prop_assert!(matches, "fork halves differ from the oracle's split");
    }

    /// `event` dominates, is local to the owned region, and strictly advances on a nonempty
    /// id (the §4 event condition).
    #[test]
    fn event_dominates_local_and_advances(p in arb_oracle_party_nonempty(), e in arb_oracle_version()) {
        let g = grid_for(&[id_depth(&p), ev_depth(&e) + 1]);
        let id = lift_id(p);
        let before = lift_ev(e);
        let after = event(&id, before.clone());
        let mut advanced = false;
        for k in 0..(1u64 << g) {
            let x = Dyadic::grid(k, g);
            let (vb, va) = (before(x), after(x));
            prop_assert!(va >= vb, "event decreased a sample");
            if !id(x) {
                prop_assert!(va == vb, "event grew where the id owns nothing");
            }
            if va > vb {
                advanced = true;
            }
        }
        prop_assert!(advanced, "event did not advance on a nonempty id");
    }

    /// `sum` of disjoint ids owns exactly their union (commutatively).
    #[test]
    fn sum_of_disjoint_is_union(p in arb_oracle_party_nonempty()) {
        // Fork a nonempty id into two disjoint halves, then sum them back two ways.
        let g = grid_for(&[id_depth(&p) + 1]);
        let i = lift_id(p);
        let (l, r) = super::fork(i.clone());
        prop_assert!(id_eq(&sum(l.clone(), r.clone()), &sum(r.clone(), l.clone()), g)); // commutative
        prop_assert!(id_eq(&sum(l, r), &i, g));                                          // union == original
    }
}

// ───────────────────────────── unit anchors ─────────────────────────────

/// The embedding reproduces the paper's worked function value: `⟦(1, 2, (0, (1, 0, 2), 0))⟧`
/// (§4, event tree graphical-notation example) samples to `[3, 3, 3, 3, 2, 4, 1, 1]` at depth
/// 3. Pins the embedding + sampling against a value the paper states, with no tree recursion
/// in the comparison.
#[test]
fn embedding_matches_paper_worked_value() {
    use oracle::Version as V;
    let e = V::Node(
        1u64.into(),
        Box::new(V::Leaf(2u64.into())),
        Box::new(V::Node(
            0u64.into(),
            Box::new(V::Node(
                1u64.into(),
                Box::new(V::Leaf(0u64.into())),
                Box::new(V::Leaf(2u64.into())),
            )),
            Box::new(V::Leaf(0u64.into())),
        )),
    );
    let f = lift_ev(e);
    let got: Vec<Base> = (0..8).map(|k| f(Dyadic::grid(k, 3))).collect();
    let want: Vec<Base> = [3u64, 3, 3, 3, 2, 4, 1, 1]
        .into_iter()
        .map(Base::from)
        .collect();
    assert_eq!(got, want);
}

/// The id and event are *genuine functions*: a lifted event is constant across distinct
/// non-grid dyadic points inside the same leaf interval (the step-function property, shown
/// directly rather than argued).
#[test]
fn lifted_event_is_constant_within_a_leaf_interval() {
    use oracle::Version as V;
    // Left leaf 7 over [0,1/2), right leaf 9 over [1/2,1).
    let f = lift_ev(V::Node(
        0u64.into(),
        Box::new(V::Leaf(7u64.into())),
        Box::new(V::Leaf(9u64.into())),
    ));
    // Two finer-than-needed points inside [0,1/2) agree; two inside [1/2,1) agree.
    assert_eq!(f(Dyadic::grid(1, 4)), Base::from(7u64)); // 1/16
    assert_eq!(f(Dyadic::grid(7, 4)), Base::from(7u64)); // 7/16
    assert_eq!(f(Dyadic::grid(9, 4)), Base::from(9u64)); // 9/16
    assert_eq!(f(Dyadic::grid(15, 4)), Base::from(9u64)); // 15/16
}

/// Guard the soundness premise: the chosen grid must fully resolve the trees under test, i.e.
/// the depth observed over a wide multi-seed op-trace sweep must stay below [`GRID_N`] (else
/// sampling could alias). Pins that headroom.
#[test]
fn grid_cap_is_never_reached() {
    use proptest::test_runner::{Config, TestRunner};
    use std::sync::atomic::{AtomicU32, Ordering as AOrd};
    let mut runner = TestRunner::new(Config {
        cases: 2000,
        ..Config::default()
    });
    let max_d = AtomicU32::new(0);
    runner
        .run(&(1usize..=4, world_strategy()), |(seeds, ops)| {
            let (_, or, _) = replay(seeds, &ops);
            for c in &or {
                max_d.fetch_max(ev_depth(&c.version()), AOrd::Relaxed);
                max_d.fetch_max(id_depth(c.party()), AOrd::Relaxed);
            }
            Ok(())
        })
        .unwrap();
    let observed = max_d.load(AOrd::Relaxed);
    assert!(
        observed < GRID_N,
        "op-trace reached depth {observed} ≥ GRID_N {GRID_N}; raise GRID_N so the scans stay \
         fully faithful",
    );
}
