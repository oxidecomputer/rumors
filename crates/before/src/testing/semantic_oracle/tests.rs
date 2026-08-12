//! Cross-check the three references by replaying one single-seed op trace
//! against all three and requiring their final clock populations to agree on
//! every pairwise comparison.
//!
//! The three references are the production impl, the recursive tree [`oracle`],
//! and the function-space [`super`] model. Also a law suite proving the
//! function-space operations are a sound ITC, and unit anchors pinning the
//! embedding against a value the paper states.

use std::cmp::Ordering;
use std::sync::Arc;

use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

use super::{
    descend, disjoint, ev_depth, ev_order, ev_res, event, id_depth, id_order, id_res, join,
    lift_ev, lift_id, meet, rank, seed_id, sum, Dyadic, Event, FunctionClock, Id, GRID_N,
};
use crate::codec::Base;
use crate::oracle;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::MAX_TRACE_OPS;
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
/// at [`GRID_N`].
///
/// The semantic event's step structure sits at the id's fork boundaries, so the
/// id depth bounds it too — hence both are folded in.
fn grid_for(parts: &[u32]) -> u32 {
    (parts.iter().copied().max().unwrap_or(0) + 1).min(GRID_N)
}

// ───────────────────────────── replay cross-check ─────────────────────────────

/// Play `ops` (starting from `seeds` independent seed clocks) against all three
/// references in lockstep, asserting they agree on every fallible op's outcome,
/// and return the three final populations (index-aligned).
///
/// Every caller passes `seeds = 1`: the invariance the keystone asserts holds
/// only for a proper single-seed system (see
/// [`replay_matches_across_references`]). A `Join`/`Sync` on overlapping
/// parties is a no-op in all three (disjointness is invariant).
fn replay(
    seeds: usize,
    ops: &[Op],
    rng: &mut StdRng,
) -> (Vec<Clock>, Vec<oracle::Clock>, Vec<FunctionClock>) {
    let mut im: Vec<Clock> = (0..seeds).map(|_| Clock::seed()).collect();
    let mut or: Vec<oracle::Clock> = (0..seeds).map(|_| oracle::Clock::seed()).collect();
    let mut se: Vec<FunctionClock> = (0..seeds).map(|_| FunctionClock::seed()).collect();

    for op in ops {
        let n = im.len();
        match *op {
            Op::Tick(i) => {
                let i = i % n;
                im[i].tick();
                or[i].tick();
                se[i].tick(rng);
            }
            Op::Ticks(i, k) => {
                let i = i % n;
                im[i].ticks(u64::from(k));
                // Both references iterate: the semantic clock has no
                // fused form, and the oracle's is the literal loop.
                for _ in 0..k {
                    or[i].tick();
                    se[i].tick(rng);
                }
            }
            Op::Fork(i) => {
                let i = i % n;
                let c = im[i].fork();
                im.push(c);
                let c = or[i].fork();
                or.push(c);
                let c = se[i].fork(rng);
                se.push(c);
            }
            Op::Send(i, j) => {
                let (i, j) = (i % n, j % n);
                let m = im[i].send().clone();
                im[j].recv(&m);
                let m = or[i].send();
                or[j].receive(m);
                let m = se[i].send(rng);
                se[j].receive(m, rng);
            }
            Op::Sync(i, j) => {
                let (i, j) = (i % n, j % n);
                if i != j {
                    let (lo, hi) = (i.min(j), i.max(j));
                    let d_im = im[i].party().is_disjoint(im[j].party());
                    let d_or = or[i].party().is_disjoint(or[j].party());
                    let g = se[i].id.res_ceiling().max(se[j].id.res_ceiling());
                    let d_se = disjoint(&se[i].id, &se[j].id, g);
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
                        assert!(a[lo].sync(&mut b[0], rng).is_ok());
                    }
                }
            }
            Op::Join(i, j) => {
                if n > 1 {
                    let (i, j) = (i % n, j % n);
                    if i != j {
                        let d_im = im[i].party().is_disjoint(im[j].party());
                        let d_or = or[i].party().is_disjoint(or[j].party());
                        let g = se[i].id.res_ceiling().max(se[j].id.res_ceiling());
                        let d_se = disjoint(&se[i].id, &se[j].id, g);
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
                            assert!(se[i2].join(v).is_ok());
                        }
                    }
                }
            }
        }
    }
    (im, or, se)
}

proptest! {
    /// The keystone check.
    ///
    /// After the same op trace, every ordered pair of final clocks has the same
    /// comparison descriptor — (version causal order, party disjointness) —
    /// under all three references. The function space computes its descriptor
    /// purely by sampling its closures; the impl and oracle use their native
    /// ops. Agreement across all three is the satisfaction condition: it is
    /// ITC's defining guarantee that the observable partial order is fixed by
    /// the operation sequence, independent of the (valid) fork/inflation policy
    /// each implementation chooses.
    ///
    /// Single seed, deliberately: that guarantee holds only for a *proper* ITC
    /// system, where ids partition one space and all live ids are disjoint.
    /// Multiple seeds would have several stamps each owning all of `[0,1)` —
    /// not a valid configuration — and a cross-lineage `receive` then entangles
    /// events on the overlapping region, where the various §4-valid inflation
    /// policies genuinely disagree on the order. (Disjointness/overlap and the
    /// join/sync-`Err` paths are id-algorithm behavior, exercised against the
    /// oracle elsewhere.)
    ///
    /// The trace carries a `seed`: the function space's `fork`/`event` choices
    /// are *random* (any §4-valid split/inflation), so this asserts agreement
    /// for an arbitrary valid policy, not just one fixed instantiation. The
    /// seed makes a failure replay deterministically.
    #[test]
    fn replay_matches_across_references(ops in world_strategy(), seed in any::<u64>()) {
        let (im, or, se) = replay(1, &ops, &mut StdRng::seed_from_u64(seed));
        let n = im.len();
        // One granularity for all the function-space scans: the finest boundary
        // actually present in the population's own closures (probed, then
        // capped — guarded by `grid_cap_is_never_reached`). Sampling at that
        // level resolves every step exactly, so the random policy's
        // region/inflation boundaries are never aliased.
        let g = se
            .iter()
            .map(|c| id_res(&c.id).max(ev_res(&c.ev)))
            .max()
            .map_or(0, |d| d.min(GRID_N));
        for i in 0..n {
            for j in 0..n {
                let (ovi, ovj) = (or[i].version(), or[j].version());
                let (ivi, ivj) = (im[i].version(), im[j].version());
                let d_impl = (
                    ivi.partial_cmp(ivj),
                    im[i].party().is_disjoint(im[j].party()),
                );
                let d_oracle = (
                    ovi.partial_cmp(&ovj),
                    or[i].party().is_disjoint(or[j].party()),
                );
                let d_sem = (
                    ev_order(&se[i].ev, &se[j].ev, g),
                    disjoint(&se[i].id, &se[j].id, g),
                );
                prop_assert_eq!(d_impl, d_oracle, "impl vs oracle at ({}, {})", i, j);
                prop_assert_eq!(d_oracle, d_sem, "function-space vs oracle at ({}, {})", i, j);
            }
        }
    }
}

// ───────────────────────────── operation cross-checks ─────────────────────────────

// ───────────────────────────── law suite (the function-space model is a sound ITC) ─────────────────────────────

proptest! {
    /// The event order is a partial order: reflexive and transitive over
    /// arbitrary events (antisymmetry is the `Equal` case of the replay's order
    /// agreement).
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

    /// `join` is the least upper bound: idempotent, commutative, associative,
    /// and an upper bound (`a ≤ a∨b`).
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

    /// `meet` is the greatest lower bound: idempotent, commutative,
    /// associative, and a lower bound (`a∧b ≤ a`).
    ///
    /// The order-theoretic dual of [`join_is_the_lub`], and the function-space
    /// witness that the pointwise minimum is a sound GLB.
    #[test]
    fn meet_is_the_glb(a in arb_oracle_version(), b in arb_oracle_version(), c in arb_oracle_version()) {
        let g = grid_for(&[ev_depth(&a), ev_depth(&b), ev_depth(&c)]);
        let (fa, fb, fc) = (lift_ev(a), lift_ev(b), lift_ev(c));
        prop_assert!(ev_eq(&meet(fa.clone(), fa.clone()), &fa, g));                            // idempotent
        prop_assert!(ev_eq(&meet(fa.clone(), fb.clone()), &meet(fb.clone(), fa.clone()), g));  // commutative
        let l = meet(meet(fa.clone(), fb.clone()), fc.clone());
        let r = meet(fa.clone(), meet(fb.clone(), fc.clone()));
        prop_assert!(ev_eq(&l, &r, g));                                                        // associative
        let ab = meet(fa.clone(), fb.clone());
        prop_assert!(
            matches!(ev_order(&ab, &fa, g), Some(Ordering::Less | Ordering::Equal)),
            "a&b is not <= a",
        );
    }

    /// `fork` partitions, whatever the random split: the two halves are
    /// disjoint, both nonempty, and recombine (`sum`) to the original (the §4
    /// fork law).
    ///
    /// It need *not* match the paper's split — only obey the law — which is the
    /// whole point of randomizing it.
    #[test]
    fn fork_partitions(p in arb_oracle_party_nonempty(), seed in any::<u64>()) {
        // Children refine ≤ 2 levels below the id's depth; scan deep enough to resolve them.
        let g = (id_depth(&p) + 3).min(GRID_N);
        let i = lift_id(p);
        let (l, r) = super::fork(&i, &mut StdRng::seed_from_u64(seed));
        prop_assert!(disjoint(&l, &r, g), "fork halves overlap");
        prop_assert!(id_eq(&sum(l.clone(), r.clone()), &i, g), "fork halves do not recombine");
        prop_assert_ne!(id_res(&l), 0, "left half is empty or all"); // both halves nonempty:
        prop_assert_ne!(id_res(&r), 0, "right half is empty or all"); // a strict sub-region splits
    }

    /// `event` dominates, is local to the owned region, and strictly advances
    /// on a nonempty id (the §4 event condition).
    #[test]
    fn event_dominates_local_and_advances(
        p in arb_oracle_party_nonempty(), e in arb_oracle_version(), seed in any::<u64>(),
    ) {
        let g = grid_for(&[id_depth(&p), ev_depth(&e) + 1]);
        let id = lift_id(p);
        let before = lift_ev(e);
        let after = event(&id, before.clone(), &mut StdRng::seed_from_u64(seed));
        let mut advanced = false;
        for k in 0..(1u64 << g) {
            let x = Dyadic::grid(k, g);
            let (vb, va) = (before.at(x), after.at(x));
            prop_assert!(va >= vb, "event decreased a sample");
            if !id.at(x) {
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
    fn sum_of_disjoint_is_union(p in arb_oracle_party_nonempty(), seed in any::<u64>()) {
        // Fork a nonempty id into two disjoint halves, then sum them back two ways.
        let g = (id_depth(&p) + 3).min(GRID_N);
        let i = lift_id(p);
        let (l, r) = super::fork(&i, &mut StdRng::seed_from_u64(seed));
        prop_assert!(id_eq(&sum(l.clone(), r.clone()), &sum(r.clone(), l.clone()), g)); // commutative
        prop_assert!(id_eq(&sum(l, r), &i, g));                                          // union == original
    }
}

// ───────────────────────────── unit anchors ─────────────────────────────

/// The embedding reproduces the paper's worked function value: `⟦(1, 2, (0, (1,
/// 0, 2), 0))⟧` (§4, event tree graphical-notation example) samples to `[3, 3,
/// 3, 3, 2, 4, 1, 1]` at depth 3.
///
/// Pins the embedding + sampling against a value the paper states, with no tree
/// recursion in the comparison.
#[test]
fn embedding_matches_paper_worked_value() {
    use oracle::Version as V;
    let e = V::Node(
        1u64.into(),
        Arc::new(V::Leaf(2u64.into())),
        Arc::new(V::Node(
            0u64.into(),
            Arc::new(V::Node(
                1u64.into(),
                Arc::new(V::Leaf(0u64.into())),
                Arc::new(V::Leaf(2u64.into())),
            )),
            Arc::new(V::Leaf(0u64.into())),
        )),
    );
    let f = lift_ev(e);
    let got: Vec<Base> = (0..8).map(|k| f.at(Dyadic::grid(k, 3))).collect();
    let want: Vec<Base> = [3u64, 3, 3, 3, 2, 4, 1, 1]
        .into_iter()
        .map(Base::from)
        .collect();
    assert_eq!(got, want);
}

/// The id and event are *genuine functions*: a lifted event is constant across
/// distinct non-grid dyadic points inside the same leaf interval (the
/// step-function property, shown directly rather than argued).
#[test]
fn lifted_event_is_constant_within_a_leaf_interval() {
    use oracle::Version as V;
    // Left leaf 7 over [0,1/2), right leaf 9 over [1/2,1).
    let f = lift_ev(V::Node(
        0u64.into(),
        Arc::new(V::Leaf(7u64.into())),
        Arc::new(V::Leaf(9u64.into())),
    ));
    // Two finer-than-needed points inside [0,1/2) agree; two inside [1/2,1) agree.
    assert_eq!(f.at(Dyadic::grid(1, 4)), Base::from(7u64)); // 1/16
    assert_eq!(f.at(Dyadic::grid(7, 4)), Base::from(7u64)); // 7/16
    assert_eq!(f.at(Dyadic::grid(9, 4)), Base::from(9u64)); // 9/16
    assert_eq!(f.at(Dyadic::grid(15, 4)), Base::from(9u64)); // 15/16
}

// ───────────────────────── known-bad references, held convicted ─────────────────────────

/// A right comb of depth `d` with one base per node and a ticked tip: the
/// asymmetric family the known-bad convictions sweep.
///
/// Every level's left child is a zero leaf, so the function climbs
/// toward `1` and the value at the last grid cell is the full path sum —
/// nonzero at every depth, and never mirror-symmetric.
fn right_comb(d: u32) -> oracle::Version {
    use oracle::Version as V;
    let mut v = V::Leaf(1u64.into());
    for _ in 0..d {
        v = V::Node(1u64.into(), Arc::new(V::Leaf(0u64.into())), Arc::new(v));
    }
    v
}

/// The Riemann sum with one deliberate defect: it drops the final grid
/// cell.
///
/// The committed known-bad reference for the prod↔fs leg
/// (`crate::testing::surface_coverage`'s tripwire roster):
/// [`rank_differential_convicts_the_cell_dropping_riemann_sum`] holds it
/// convicted by the same comparison the rank descriptor's fs leg
/// performs.
fn riemann_sum_dropping_the_last_cell(e: &Event, g: u32) -> crate::Rank {
    let mut total = Base::ZERO;
    for k in 0..(1u64 << g) - 1 {
        total += &e.at(Dyadic::grid(k, g));
    }
    crate::Rank::from_raw(total, u64::from(g))
}

/// The prod↔fs leg's criterion can fail: the rank differential convicts
/// the cell-dropping Riemann sum at every depth of an asymmetric family,
/// while the genuine sum agrees with the impl everywhere.
///
/// Per depth, the right-comb family stacks its bases down the rightmost
/// path, so the dropped cell's height is the full path sum and the
/// truncated total falls short exactly there. The genuine Riemann sum
/// realizes the impl's rank at every depth (the comparison's liveness);
/// the known-bad variant reads different at every depth, so the leg's
/// criterion is proven able to reject a wrong reference.
#[test]
fn rank_differential_convicts_the_cell_dropping_riemann_sum() {
    for d in 1..=6 {
        let v = right_comb(d);
        let g = grid_for(&[ev_depth(&v)]);
        let want = from_oracle_version(&v).rank();
        assert_eq!(
            rank(&lift_ev(v.clone()), g),
            want,
            "the genuine Riemann sum must realize the impl rank at depth {d}"
        );
        assert_ne!(
            riemann_sum_dropping_the_last_cell(&lift_ev(v), g),
            want,
            "the rank differential must convict the cell-dropping sum at depth {d}"
        );
    }
}

/// The embedding with one deliberate defect: every descent step walks
/// into the wrong child (the mirror image of the §4 recursion).
///
/// The committed known-bad reference for the tree↔fs leg
/// (`crate::testing::surface_coverage`'s tripwire roster):
/// [`worked_value_anchor_convicts_the_mirrored_embedding`] holds it
/// convicted by the paper worked-value anchor's comparison — and
/// documents that the leg's pointwise-operation differentials alone
/// cannot convict it.
fn mirrored_lift_ev(t: oracle::Version) -> Event {
    fn eval(t: &oracle::Version, mut x: Dyadic) -> Base {
        use oracle::Version as V;
        let mut node = t;
        let mut acc = Base::ZERO;
        loop {
            match node {
                V::Leaf(n) => return acc + n,
                V::Node(n, l, r) => {
                    acc += n;
                    let (right, nx) = descend(x);
                    // The defect: the honest embedding descends right
                    // exactly when the point lies in [1/2, 1).
                    node = if right { l } else { r };
                    x = nx;
                }
            }
        }
    }
    Event::new(ev_depth(&t), move |x| eval(&t, x))
}

/// The tree↔fs leg's criterion can fail: the paper worked-value anchor
/// convicts the mirrored embedding, which the leg's pointwise-operation
/// differentials alone can never reject.
///
/// The mirror is a measure-preserving relabeling of `[0,1)`, so it
/// commutes with every pointwise combinator: substituted on *both* sides
/// of the meet differential it stays green at every depth (asserted
/// below — the twin-spelling blindness that makes an absolute-geometry
/// anchor the leg's necessary tripwire, not a redundancy). The anchor
/// and the honest embedding convict it: the mirrored lift of the paper's
/// §4 worked example disagrees with the samples the paper states, and it
/// disagrees with the honest lift at every depth of the asymmetric
/// right-comb family.
#[test]
fn worked_value_anchor_convicts_the_mirrored_embedding() {
    use oracle::Version as V;
    // The worked tree the committed anchor pins
    // ([`embedding_matches_paper_worked_value`]'s fixture).
    let e = V::Node(
        1u64.into(),
        Arc::new(V::Leaf(2u64.into())),
        Arc::new(V::Node(
            0u64.into(),
            Arc::new(V::Node(
                1u64.into(),
                Arc::new(V::Leaf(0u64.into())),
                Arc::new(V::Leaf(2u64.into())),
            )),
            Arc::new(V::Leaf(0u64.into())),
        )),
    );
    let f = mirrored_lift_ev(e.clone());
    let got: Vec<Base> = (0..8).map(|k| f.at(Dyadic::grid(k, 3))).collect();
    let want: Vec<Base> = [3u64, 3, 3, 3, 2, 4, 1, 1]
        .into_iter()
        .map(Base::from)
        .collect();
    assert_ne!(
        got, want,
        "the worked-value anchor must convict the mirrored embedding"
    );
    for d in 1..=6 {
        let v = right_comb(d);
        let g = grid_for(&[ev_depth(&v), ev_depth(&e)]);
        assert!(
            !ev_eq(&lift_ev(v.clone()), &mirrored_lift_ev(v.clone()), g),
            "the honest embedding must convict the mirror at depth {d}"
        );
        // The blindness this artifact documents: with the mirror on both
        // sides, the meet differential's comparison stays green.
        assert!(
            ev_eq(
                &mirrored_lift_ev(v.clone() & e.clone()),
                &meet(mirrored_lift_ev(v), mirrored_lift_ev(e.clone())),
                g,
            ),
            "the pointwise differential is blind to the twin-substituted mirror at depth {d}"
        );
    }
}

/// Guard the soundness premise: the chosen grid must fully resolve every
/// function the keystone scans, i.e. the finest boundary observed over a wide
/// op-trace sweep must stay below [`GRID_N`] (else sampling could alias).
///
/// This covers *both* the oracle's tree depth and the function space's probed
/// resolution — the random `fork` refines up to two levels per call (vs. the
/// paper's one), so its resolution can run ahead of the oracle's, and it is the
/// binding constraint.
///
/// The structural guarantee lives in [`GRID_N`]'s derivation from the op cap
/// (one level per op at most, so every in-support trace stays under); this
/// sweep is the distributional canary over sampled traces, and
/// [`fork_chain_raises_resolution_one_level_per_fork`] pins the derivation's
/// growth-rate premise deterministically on the worst in-support schedule.
///
/// Single seed, matching the keystone: forking one lineage repeatedly is the
/// worst case for per-lineage resolution growth, so a single seed bounds it.
/// (Multiple seeds reuse `[0,1)`, an improper configuration the keystone
/// deliberately avoids; under the random policy their structural-vs-geometric
/// disjointness even disagrees, so they can't be replayed in lockstep — see the
/// keystone doc.)
#[test]
fn grid_cap_is_never_reached() {
    use proptest::test_runner::{Config, TestRunner};
    use std::sync::atomic::{AtomicU32, Ordering as AOrd};
    // Fewer cases than a typical canary sweep: each case probes the function
    // space's grid to read its resolution, and the bound it guards is
    // structural (`fork` only deepens by bisecting an indivisible piece, the
    // paper's rate), so a modest sweep is an ample canary.
    let mut runner = TestRunner::new(Config {
        cases: 400,
        ..Config::default()
    });
    let max_d = AtomicU32::new(0);
    runner
        .run(&(world_strategy(), any::<u64>()), |(ops, seed)| {
            let (_, or, se) = replay(1, &ops, &mut StdRng::seed_from_u64(seed));
            for c in &or {
                max_d.fetch_max(ev_depth(&c.version()), AOrd::Relaxed);
                max_d.fetch_max(id_depth(c.party()), AOrd::Relaxed);
            }
            for c in &se {
                max_d.fetch_max(id_res(&c.id), AOrd::Relaxed);
                max_d.fetch_max(ev_res(&c.ev), AOrd::Relaxed);
            }
            Ok(())
        })
        .unwrap();
    let observed = max_d.load(AOrd::Relaxed);
    assert!(
        observed < GRID_N,
        "op-trace reached resolution {observed} ≥ GRID_N {GRID_N}; raise GRID_N so the scans \
         stay fully faithful",
    );
}

/// The grid derivation's premise, met with equality on its worst schedule:
/// a same-lineage fork chain raises resolution exactly one level per fork,
/// with both halves nonempty, strictly below the derived [`GRID_N`].
///
/// A run of consecutive `Fork(0)`s is inside `world_strategy`'s support but
/// beyond any sampled sweep's reach (a 10-long chain has ~1e-10 mass per
/// trace), so the fastest in-support resolution growth is pinned here
/// deterministically: each fork of a single-piece lineage bisects at its
/// midpoint, one level finer. If the rate ever exceeded one level per fork,
/// or a ceiling clamp resurfaced and handed one child an empty half, the
/// equality or the nonemptiness read here goes red long before the canary
/// sweep could sample it. The chain stops at 16 — past any resolution the
/// sampled sweeps reach, still cheap to probe (the probe scans
/// `2^resolution` centers) — while the full in-support schedule is covered
/// by construction: [`GRID_N`] is derived from the op cap, and this test
/// asserts the derivation clears the deepest in-support bisection level.
#[test]
fn fork_chain_raises_resolution_one_level_per_fork() {
    // The derivation's arithmetic: the deepest in-support bisection level is
    // the op cap itself (resolution strictly below the cap, bisection one
    // finer), and the ceiling must clear it.
    assert!(
        u32::try_from(MAX_TRACE_OPS).expect("small cap") <= GRID_N,
        "the grid ceiling no longer clears the deepest in-support bisection"
    );
    let mut rng = StdRng::seed_from_u64(0);
    let mut kept = FunctionClock::seed();
    for k in 1..=16u32 {
        let child = kept.fork(&mut rng);
        assert!(k < GRID_N, "the chain itself must stay inside the grid");
        assert_eq!(
            id_res(&kept.id),
            k,
            "kept half's resolution after fork {k}: exactly one level per fork"
        );
        assert_eq!(
            id_res(&child.id),
            k,
            "split-off half's resolution after fork {k}: exactly one level per fork"
        );
        // Nonempty halves: an empty id is disjoint from the seed id (which
        // owns everything); a live one is not.
        assert!(
            !disjoint(&kept.id, &seed_id(), k + 1),
            "the kept half went empty at fork {k}"
        );
        assert!(
            !disjoint(&child.id, &seed_id(), k + 1),
            "the split-off half went empty at fork {k}"
        );
    }
}
