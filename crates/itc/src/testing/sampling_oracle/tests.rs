//! Differential tests: the packed impl vs. the function-space sampling oracle.
//!
//! Each property lowers the impl's result to the recursive [`oracle`] tree (via the
//! existing `to_oracle_*` bridge), samples it on a dyadic grid that resolves the trees
//! under test, and checks the function-space relation *pointwise* — never against the
//! recursive oracle's own verdict. The sampling reference shares no representation and no
//! recursion with the tree machinery, so a disagreement here is a genuine independent
//! finding (it catches the impl and tree-oracle being wrong together).
//!
//! Inputs come from *both* the seed-derived op-trace (`world_strategy`/`run`) and the
//! decoupled arbitrary-normal-form generators (`arb_oracle_*`) — so the checks cover the
//! shapes operations produce *and* the full space of valid normal-form trees, including the
//! large-base events (path sums that would overflow `u64`) that the sampling values
//! represent exactly.
//!
//! The grid depth is chosen per case as `min(actual tree depth, MAX_GRID_DEPTH)`, decoupling
//! grid density (exponential in depth) from the generators' own depth knob; see the parent
//! module doc. The arbitrary generators cap depth at 4 and the op-trace at 7, both under the
//! cap, so the gate grid always fully resolves the trees (no aliasing) — `grid_cap_is_never_reached`
//! pins that. A dense, deeper variant lives behind `#[ignore]` (`dense_deep_arbitrary`).

use std::cmp::Ordering;

use proptest::prelude::*;

use super::{
    ev_grid_depth, ev_join, ev_leq, id_contains, id_disjoint, id_grid_depth, sample_event,
    sample_id,
};
use crate::oracle;
use crate::testing::bridge::{from_oracle_party, from_oracle_version, to_oracle_version};
use crate::testing::generators::{arb_oracle_party, arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{run, versions, world_strategy};
use crate::Version;

/// Map a `(le, ge)` pair of pointwise comparisons to the partial order the impl reports.
fn order_of(le: bool, ge: bool) -> Option<Ordering> {
    match (le, ge) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// The impl's event `partial_cmp` agrees with pointwise comparison of the sampled step
/// functions. This is the keystone check: it pins the causal order to the paper's
/// function-space `≤` (§4) on a representation independent of the tree recursion.
fn check_ev_cmp(a: &oracle::Version, b: &oracle::Version) {
    let d = ev_grid_depth(a, b);
    let sa = sample_event(a, d);
    let sb = sample_event(b, d);
    let expected = order_of(ev_leq(&sa, &sb), ev_leq(&sb, &sa));

    let ia = from_oracle_version(a);
    let ib = from_oracle_version(b);
    assert_eq!(
        ia.partial_cmp(&ib),
        expected,
        "impl partial_cmp disagrees with the sampled function-space order for {a:?} vs {b:?}",
    );
}

/// The impl's event merge `|` equals the pointwise `max` of the two sampled functions
/// (§4). The merge result is lowered and re-sampled, then compared to the
/// pointwise-max sample vector.
fn check_ev_join(a: &oracle::Version, b: &oracle::Version) {
    let ia = from_oracle_version(a);
    let ib = from_oracle_version(b);
    let merged = ia | ib;
    let merged_oracle = to_oracle_version(&merged);

    // Resolve all three trees: the inputs and the result.
    let d = super::ev_depth(a)
        .max(super::ev_depth(b))
        .max(super::ev_depth(&merged_oracle))
        .min(super::MAX_GRID_DEPTH);
    let sa = sample_event(a, d);
    let sb = sample_event(b, d);
    let expected = ev_join(&sa, &sb);
    let got = sample_event(&merged_oracle, d);
    assert_eq!(
        got, expected,
        "impl merge is not the pointwise max of the sampled functions for {a:?} | {b:?}",
    );
}

/// The impl's id `partial_cmp` and `is_disjoint` agree with the function-space containment
/// and disjointness of the sampled characteristic functions (§4). An ancestor
/// (larger owned region) reads as `Less`, so containment `a ⊇ b ⇔ a ≤ b`.
fn check_id_pair(a: &oracle::Party, b: &oracle::Party) {
    let d = id_grid_depth(a, b);
    let sa = sample_id(a, d);
    let sb = sample_id(b, d);

    let a_contains_b = id_contains(&sa, &sb);
    let b_contains_a = id_contains(&sb, &sa);
    let expected_cmp = order_of(a_contains_b, b_contains_a);

    let ia = from_oracle_party(a);
    let ib = from_oracle_party(b);
    assert_eq!(
        ia.partial_cmp(&ib),
        expected_cmp,
        "impl id partial_cmp disagrees with sampled containment for {a:?} vs {b:?}",
    );
    assert_eq!(
        ia.is_disjoint(&ib),
        id_disjoint(&sa, &sb),
        "impl is_disjoint disagrees with sampled `i1 · i2 = 0` for {a:?} vs {b:?}",
    );
}

/// `tick` realizes the paper's `event` condition (§3, §5.3.4) as read by the sampling
/// oracle, independent of the tree recursion:
/// - **strict advance**: the sampled function strictly increases — `e ≤ e'` pointwise and
///   `e < e'` at some point (so `e'` is a real successor).
/// - **inflation only on owned intervals** (§4, `event((i,e)) = (i, e + f·i)`): every
///   grid point where the value grew is a point the id owns. No unowned interval moves.
fn check_tick(id: &oracle::Party, e: &oracle::Version) {
    let ip = from_oracle_party(id);
    let mut ev = from_oracle_version(e);
    ev.tick(&ip);
    let after = to_oracle_version(&ev);

    let d = super::ev_depth(e)
        .max(super::ev_depth(&after))
        .max(super::id_depth(id))
        .min(super::MAX_GRID_DEPTH);
    let before_s = sample_event(e, d);
    let after_s = sample_event(&after, d);
    let owned = sample_id(id, d);

    let mut grew_somewhere = false;
    for k in 0..before_s.len() {
        assert!(
            before_s[k] <= after_s[k],
            "tick decreased the function at point {k} for id {id:?} on {e:?}",
        );
        if after_s[k] > before_s[k] {
            grew_somewhere = true;
            assert!(
                owned[k],
                "tick inflated an unowned interval at point {k} for id {id:?} on {e:?}",
            );
        }
    }
    assert!(
        grew_somewhere,
        "tick did not advance the function for id {id:?} on {e:?}",
    );
}

// ───────────────────────────── op-trace differentials ─────────────────────────────

proptest! {
    /// Over the op-trace: every live clock's version, compared pairwise, has the same
    /// causal order under the impl as under the sampled function space.
    #[test]
    fn optrace_event_cmp(ops in world_strategy()) {
        let cs = run(&ops);
        let vs = versions(&cs);
        for a in &vs {
            for b in &vs {
                check_ev_cmp(a, b);
            }
        }
    }

    /// Over the op-trace: the impl's merge of any two live versions is the pointwise
    /// max of their sampled functions.
    #[test]
    fn optrace_event_join(ops in world_strategy()) {
        let cs = run(&ops);
        let vs = versions(&cs);
        for a in &vs {
            for b in &vs {
                check_ev_join(a, b);
            }
        }
    }

    /// Over the op-trace: the live parties (always pairwise disjoint by construction)
    /// have the impl id order and disjointness the sampled characteristic functions predict.
    #[test]
    fn optrace_id_pair(ops in world_strategy()) {
        let cs = run(&ops);
        let parties: Vec<oracle::Party> = cs.iter().map(|c| c.party().clone()).collect();
        for a in &parties {
            for b in &parties {
                check_id_pair(a, b);
            }
        }
    }
}

// ──────────────────────── decoupled-generator differentials ────────────────────────

proptest! {
    /// Over arbitrary normal-form events, including large-base values the sampler represents
    /// exactly: the impl's `partial_cmp` matches pointwise `≤` of the sampled step
    /// functions. Unrelated pairs (the op-trace never builds these) reach the concurrent
    /// (`None`) verdict.
    #[test]
    fn arbitrary_event_cmp(a in arb_oracle_version(), b in arb_oracle_version()) {
        check_ev_cmp(&a, &b);
    }

    /// Over arbitrary normal-form events: the impl's `|` is the pointwise max of the
    /// sampled functions.
    #[test]
    fn arbitrary_event_join(a in arb_oracle_version(), b in arb_oracle_version()) {
        check_ev_join(&a, &b);
    }

    /// Over arbitrary normal-form ids (may overlap — the op-trace only builds
    /// disjoint parties): the impl's id order and disjointness match the sampled
    /// characteristic functions, reaching the overlap and incomparable arms.
    #[test]
    fn arbitrary_id_pair(a in arb_oracle_party(), b in arb_oracle_party()) {
        check_id_pair(&a, &b);
    }

    /// Over arbitrary (non-anonymous id, event) pairs: `tick` strictly advances the
    /// sampled function and inflates only owned intervals — the paper's `event` condition
    /// checked on a representation independent of the tree recursion.
    #[test]
    fn arbitrary_tick(id in arb_oracle_party_nonempty(), e in arb_oracle_version()) {
        check_tick(&id, &e);
    }
}

// ──────────────────────── dense / deep variant (ignored) ────────────────────────

/// A deeper arbitrary normal-form event tree than the gate generator builds: recursion
/// depth up to 8, so the resolving grid reaches `2^8 = 256` points. Bases still span the
/// large range (path sums that would overflow `u64`). Used only by the `#[ignore]`d dense
/// variant.
fn deep_arb_oracle_version() -> impl Strategy<Value = oracle::Version> {
    let leaf = crate::testing::generators::arb_base().prop_map(oracle::Version::Leaf);
    leaf.prop_recursive(8, 256, 2, |inner| {
        (crate::testing::generators::arb_base(), inner.clone(), inner)
            .prop_map(|(n, l, r)| oracle::Version::node(n, l, r))
    })
}

/// A deeper arbitrary normal-form id tree (recursion depth up to 8), mirroring
/// [`deep_arb_oracle_version`].
fn deep_arb_oracle_party() -> impl Strategy<Value = oracle::Party> {
    let leaf = any::<bool>().prop_map(oracle::Party::Leaf);
    leaf.prop_recursive(8, 256, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| oracle::Party::node(l, r))
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Dense/deep: the same function-space differentials at a far higher grid
    /// resolution and tree depth (up to depth 8 → `2^8 = 256` grid points). Decoupled from
    /// the gate variant precisely so the exponential grid does not slow the gate; runs in
    /// tens of seconds to minutes, so it is `#[ignore]`d. Run with:
    ///
    /// ```text
    /// cargo nextest run -p itc --release --all-features \
    ///     dense_deep_arbitrary --run-ignored ignored-only
    /// ```
    #[test]
    #[ignore = "dense deep sampling grid (up to 2^8 points): tens of seconds; see doc comment"]
    fn dense_deep_arbitrary(
        ea in deep_arb_oracle_version(),
        eb in deep_arb_oracle_version(),
        ida in deep_arb_oracle_party(),
        idb in deep_arb_oracle_party(),
    ) {
        check_ev_cmp(&ea, &eb);
        check_ev_join(&ea, &eb);
        check_id_pair(&ida, &idb);
        if !ida.is_empty() {
            check_tick(&ida, &ea);
        }
    }
}

// ───────────────────────────── unit anchors ─────────────────────────────

/// The sampler reproduces the paper's worked function value: `J(1, 2, (0, (1, 0, 2), 0))K`
/// (§4, event tree graphical-notation example). Base offsets accumulate down the path: root
/// `1`; left leaf `1+2 = 3`; the
/// right subtree `(0, (1,0,2), 0)` adds `1+0 = 1` to the root, its left grandchild
/// `(1,0,2)` adds `+1` then its own children, etc. This pins the sampler against a value
/// the paper states, with no tree-recursion reference involved.
#[test]
fn sampler_matches_paper_worked_value() {
    use oracle::Version as V;
    // (1, 2, (0, (1, 0, 2), 0)) — built raw (not re-normalized) to match the paper figure.
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
    // Depth 3 → 8 grid points over [0,1). The right subtree `(0,(1,0,2),0)` covers
    // [1/2,1): its left child `(1,0,2)` (offset 1+0+1 = 2) covers [1/2,3/4), splitting into
    // [1/2,5/8) leaf 0 → 2 and [5/8,3/4) leaf 2 → 4; its right child leaf 0 (offset 1) covers
    // [3/4,1) → 1.
    //   [0,1/2)   left leaf:        1 + 2          = 3   (points 0..4)
    //   [1/2,5/8) right-left-left:  1 + 0 + 1 + 0  = 2   (point 4)
    //   [5/8,3/4) right-left-right: 1 + 0 + 1 + 2  = 4   (point 5)
    //   [3/4,1)   right-right:      1 + 0 + 0      = 1   (points 6..8)
    let s = sample_event(&e, 3);
    let want: Vec<crate::codec::Base> = [3u64, 3, 3, 3, 2, 4, 1, 1]
        .into_iter()
        .map(Into::into)
        .collect();
    assert_eq!(s, want);
}

/// A constant event tick on the seed id raises every sample by exactly one (the seed owns
/// all of `[0, 1)`), and `Version::new() < ticked`. A minimal end-to-end anchor that the
/// `tick`-advance machinery and the sampler agree on the simplest stamp.
#[test]
fn seed_tick_raises_every_sample() {
    let id = oracle::Party::seed();
    let before = Version::default();
    let mut after = Version::default();
    after.tick(&from_oracle_party(&id));

    let d = 0; // both are leaves
    let bs = sample_event(&to_oracle_version(&before), d);
    let as_ = sample_event(&to_oracle_version(&after), d);
    assert_eq!(bs.len(), 1);
    assert!(
        as_[0] > bs[0],
        "seed tick did not advance the constant function"
    );
    assert!(before.partial_cmp(&after) == Some(Ordering::Less));
}

/// Guard the soundness premise of every check above: the chosen grid must *fully resolve*
/// the trees under test, i.e. [`MAX_GRID_DEPTH`] must never bite. If it did, sampling would
/// alias and could report a spurious disagreement. Empirically the op-trace (≤30 ops, ≤8
/// parties) tops out at depth 7 and the arbitrary generators at depth 4 (+1 for a `tick`), so
/// [`MAX_GRID_DEPTH`] sits comfortably above both. This pins that headroom: over a wide
/// op-trace sweep, no version or party ever reaches the cap.
#[test]
fn grid_cap_is_never_reached() {
    use proptest::test_runner::{Config, TestRunner};
    use std::sync::atomic::{AtomicU32, Ordering as AOrd};
    let mut runner = TestRunner::new(Config {
        cases: 4000,
        ..Config::default()
    });
    let max_d = AtomicU32::new(0);
    runner
        .run(&world_strategy(), |ops| {
            let cs = run(&ops);
            for v in versions(&cs) {
                max_d.fetch_max(super::ev_depth(&v), AOrd::Relaxed);
            }
            for c in &cs {
                max_d.fetch_max(super::id_depth(c.party()), AOrd::Relaxed);
            }
            Ok(())
        })
        .unwrap();
    let observed = max_d.load(AOrd::Relaxed);
    assert!(
        observed < super::MAX_GRID_DEPTH,
        "op-trace reached depth {observed} ≥ MAX_GRID_DEPTH {}; raise the cap so sampling \
         stays fully faithful",
        super::MAX_GRID_DEPTH,
    );
}
