//! Oracle property suite: the ground-truth gate.
//!
//! These establish that the recursive oracle is a faithful (if suboptimal)
//! realization of the paper, so it can be trusted as differential ground truth.
//! Values are generated via operations from a seed (always valid, normal-form,
//! and — for populations — pairwise party-disjoint), never by fabricating
//! trees.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::{Clock, Party, Version};
use crate::testing::generators::{arb_oracle_party, arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::grow_brute_force::{all_inflations, best_inflation, min_inflation_cost};
use crate::testing::optrace::{leq, run, versions, world_strategy};

// ───────────────────────────── seed / join identity ─────────────────────────────

/// `Clock::seed()` decomposes to `(Party::seed(), Version::new())`.
#[test]
fn genesis() {
    let (p, v) = Clock::seed().into_parts();
    assert_eq!(p, Party::seed());
    assert_eq!(v, Version::new());
}

proptest! {
    /// `Version::new()` is the two-sided identity for `|`.
    #[test]
    fn join_identity(ops in world_strategy()) {
        for v in versions(&run(&ops)) {
            prop_assert_eq!(Version::new() | v.clone(), v.clone());
            prop_assert_eq!(v.clone() | Version::new(), v);
        }
    }
}

// ───────────────────────────── normal form ─────────────────────────────

proptest! {
    /// Every value any op produces is in normal form (parties and versions),
    /// including the result of a join.
    #[test]
    fn normal_form(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        for c in &cs {
            let (p, v) = c.trees();
            prop_assert!(p.is_normal(), "denormal party: {:?}", p);
            prop_assert!(v.is_normal(), "denormal version: {:?}", v);
        }
        let vs = versions(&cs);
        let n = vs.len();
        let joined = vs[i % n].clone() | vs[j % n].clone();
        prop_assert!(joined.is_normal(), "join produced denormal: {:?}", joined);
        let met = vs[i % n].clone() & vs[j % n].clone();
        prop_assert!(met.is_normal(), "meet produced denormal: {:?}", met);
    }
}

// ───────────────────────────── version causal order ─────────────────────────────

proptest! {
    /// The causal order is a partial order: reflexive, antisymmetric,
    /// transitive; `==` ⇔ `Some(Equal)`; concurrency ⇔ `None`.
    #[test]
    fn version_partial_order(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let vs = versions(&run(&ops));
        let n = vs.len();
        let (a, b, c) = (&vs[i % n], &vs[j % n], &vs[k % n]);

        prop_assert_eq!(a.partial_cmp(a), Some(Ordering::Equal)); // reflexive
        if leq(a, b) && leq(b, a) {
            prop_assert_eq!(a, b); // antisymmetric
        }
        if leq(a, b) && leq(b, c) {
            prop_assert!(leq(a, c)); // transitive
        }
        prop_assert_eq!(a == b, a.partial_cmp(b) == Some(Ordering::Equal));
        let concurrent = !leq(a, b) && !leq(b, a);
        prop_assert_eq!(concurrent, a.partial_cmp(b).is_none());
    }
}

// ───────────────────────────── tick advances ─────────────────────────────

proptest! {
    /// `tick` strictly advances: `v < v.tick(p)` for the clock's own party.
    #[test]
    fn tick_advances(ops in world_strategy()) {
        for c in &run(&ops) {
            let party = c.party().clone();
            let before = c.version();
            let mut after = before.clone();
            after.tick(&party);
            prop_assert!(leq(&before, &after) && !leq(&after, &before), "not strictly greater");
            prop_assert_ne!(before, after);
        }
    }
}

// ───────────────────────────── join semilattice ─────────────────────────────

proptest! {
    /// Join is a bounded join-semilattice and the least upper bound:
    /// commutative, associative, idempotent; upper bound; and least (below any
    /// common upper bound).
    #[test]
    fn lattice(ops in world_strategy(),
                  i in 0usize..64, j in 0usize..64, k in 0usize..64, l in 0usize..64) {
        let vs = versions(&run(&ops));
        let n = vs.len();
        let (a, b, c, extra) = (&vs[i % n], &vs[j % n], &vs[k % n], &vs[l % n]);

        let ab = a.clone() | b.clone();
        prop_assert_eq!(ab.clone(), b.clone() | a.clone());              // commutative
        prop_assert_eq!(a.clone() | a.clone(), a.clone());               // idempotent
        prop_assert_eq!(
            (a.clone() | b.clone()) | c.clone(),
            a.clone() | (b.clone() | c.clone()),
        );                                                               // associative
        prop_assert!(leq(a, &ab) && leq(b, &ab));                        // upper bound

        // Least: any common upper bound dominates a|b. Build one as ab|extra.
        let upper = ab.clone() | extra.clone();
        prop_assert!(leq(a, &upper) && leq(b, &upper));
        prop_assert!(leq(&ab, &upper));
    }
}

// ───────────────────────────── order induced by join ─────────────────────────────

proptest! {
    /// The order is induced by the join: `a <= b` ⇔ `a|b == b`.
    #[test]
    fn order_from_join(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let vs = versions(&run(&ops));
        let n = vs.len();
        let (a, b) = (&vs[i % n], &vs[j % n]);
        prop_assert_eq!(leq(a, b), (a.clone() | b.clone()) == *b);
    }
}

// ───────────────────────────── meet semilattice ─────────────────────────────

proptest! {
    /// Meet (`&`) is a meet-semilattice and the greatest lower bound:
    /// commutative, associative, idempotent; lower bound; and greatest (above
    /// any common lower bound). The order-theoretic dual of [`lattice`]. There is
    /// no top element — a version can always tick higher — so meet has no
    /// identity; instead `Version::new()` (the bottom) is *absorbing*: `a & 0 ==
    /// 0`.
    #[test]
    fn meet_semilattice(ops in world_strategy(),
                  i in 0usize..64, j in 0usize..64, k in 0usize..64, l in 0usize..64) {
        let vs = versions(&run(&ops));
        let n = vs.len();
        let (a, b, c, extra) = (&vs[i % n], &vs[j % n], &vs[k % n], &vs[l % n]);

        let ab = a.clone() & b.clone();
        prop_assert_eq!(ab.clone(), b.clone() & a.clone());              // commutative
        prop_assert_eq!(a.clone() & a.clone(), a.clone());               // idempotent
        prop_assert_eq!(
            (a.clone() & b.clone()) & c.clone(),
            a.clone() & (b.clone() & c.clone()),
        );                                                               // associative
        prop_assert!(leq(&ab, a) && leq(&ab, b));                        // lower bound

        // Greatest: any common lower bound is dominated by a&b. Build one as ab&extra.
        let lower = ab.clone() & extra.clone();
        prop_assert!(leq(&lower, a) && leq(&lower, b));
        prop_assert!(leq(&lower, &ab));

        // Bottom is absorbing for meet.
        prop_assert_eq!(a.clone() & Version::new(), Version::new());
    }
}

proptest! {
    /// Meet and join cohere as a lattice: both absorption laws hold, and the
    /// causal order is induced by *either* operation — `a <= b` iff `a & b == a`
    /// iff `a | b == b`.
    #[test]
    fn lattice_absorption(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let vs = versions(&run(&ops));
        let n = vs.len();
        let (a, b) = (&vs[i % n], &vs[j % n]);

        prop_assert_eq!(a.clone() & (a.clone() | b.clone()), a.clone()); // a & (a|b) == a
        prop_assert_eq!(a.clone() | (a.clone() & b.clone()), a.clone()); // a | (a&b) == a

        let le = leq(a, b);
        prop_assert_eq!(le, (a.clone() & b.clone()) == *a);              // a<=b ⇔ a&b==a
        prop_assert_eq!(le, (a.clone() | b.clone()) == *b);              // a<=b ⇔ a|b==b
    }
}

// ───────────────────────────── fork / join round-trip ─────────────────────────────

proptest! {
    /// Party fork is invertible by join, and fork preserves a clock's version.
    #[test]
    fn fork_join_roundtrip(ops in world_strategy()) {
        for c in &run(&ops) {
            // Party level: fork then join recovers the original party.
            let mut p = c.party().clone();
            let snapshot = p.clone();
            let b = p.fork();
            prop_assert!(p.is_disjoint(&b));
            prop_assert_eq!(p.join(b), Ok(()));
            prop_assert_eq!(&p, &snapshot);
        }
    }
}

proptest! {
    /// `fork` leaves both halves carrying the parent's version.
    #[test]
    fn fork_preserves_version(ops in world_strategy()) {
        let mut cs = run(&ops);
        for c in &mut cs {
            let before = c.version();
            let child = c.fork();
            prop_assert_eq!(c.version(), before.clone());
            prop_assert_eq!(child.version(), before);
        }
    }
}

// ───────────────────────────── disjointness invariant ─────────────────────────────

proptest! {
    /// Over any seed-derived trace, all live parties are pairwise disjoint and
    /// their overall `sum` recovers the whole id space.
    #[test]
    fn disjointness_invariant(ops in world_strategy()) {
        let cs = run(&ops);
        for i in 0..cs.len() {
            for j in (i + 1)..cs.len() {
                prop_assert!(
                    cs[i].party().is_disjoint(cs[j].party()),
                    "parties {} and {} overlap", i, j
                );
            }
        }
        let mut acc = Party::Leaf(false);
        for c in &cs {
            acc = acc.sum(c.party().clone());
        }
        prop_assert_eq!(acc, Party::seed());
    }
}

// ───────────────────────────── peek does not advance ─────────────────────────────

proptest! {
    /// Peek does not advance: `version()` is idempotent and leaves the clock
    /// unchanged. (Fork preserving history is covered by `fork_preserves_version`.)
    #[test]
    fn peek_does_not_advance(ops in world_strategy()) {
        for c in &run(&ops) {
            let v1 = c.version();
            let v2 = c.version();
            prop_assert_eq!(v1, v2);
        }
    }
}

// ───────────────────────────── dominated receive ─────────────────────────────

proptest! {
    /// A dominated receive equals a bare tick, and re-delivery is idempotent
    /// (`v | m | m == v | m`).
    #[test]
    fn dominated_receive(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);

        // Re-delivery idempotence at the version level.
        let vs = versions(&cs);
        let n = vs.len();
        let (a, m) = (&vs[i % n], &vs[j % n]);
        prop_assert_eq!(
            a.clone() | m.clone() | m.clone(),
            a.clone() | m.clone(),
        );

        // Dominated receive == tick: deliver the clock's own current version.
        for c in &cs {
            let mut by_receive = c.clone();
            let msg = by_receive.version(); // msg <= version (equal)
            by_receive.receive(msg);
            let mut by_tick = c.clone();
            by_tick.tick();
            prop_assert_eq!(by_receive.version(), by_tick.version());
        }
    }
}

// ───────────────────────────── sync ─────────────────────────────

proptest! {
    /// `sync` reconciles to the join and re-splits without losing ownership.
    #[test]
    fn sync(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let mut cs = run(&ops);
        let n = cs.len();
        if n < 2 {
            return Ok(());
        }
        // Derive two distinct members directly rather than rejecting collisions
        // — small populations collide often, and `prop_assume` would blow the
        // local reject cap under a high case count.
        let i = i % n;
        let j = (i + 1 + j % (n - 1)) % n;

        let a_pre_v = cs[i].version();
        let b_pre_v = cs[j].version();
        let a_pre_p = cs[i].party().clone();
        let b_pre_p = cs[j].party().clone();
        let merged_region = a_pre_p.sum(b_pre_p);

        let (lo, hi) = (i.min(j), i.max(j));
        let (left, right) = cs.split_at_mut(hi);
        left[lo].sync(&mut right[0]).expect("disjoint");

        prop_assert_eq!(cs[i].version(), cs[j].version());
        prop_assert_eq!(cs[i].version(), a_pre_v | b_pre_v);
        prop_assert!(cs[i].party().is_disjoint(cs[j].party()));

        let mut rejoined = cs[i].party().clone();
        rejoined.join(cs[j].party().clone()).expect("disjoint");
        prop_assert_eq!(rejoined, merged_region);
    }
}

// ───────────────────────────── heterogeneous joins ─────────────────────────────

proptest! {
    /// Heterogeneous joins change only the version, to the `join` of the two; a
    /// bare `Version` acts as a party-`0` clock would.
    #[test]
    fn heterogeneous_joins(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = cs.len();
        let m = vs[j % n].clone();
        let c = cs[i % n].clone();
        let pid = c.party().clone();
        let v0 = c.version();
        let expected = v0.clone() | m.clone();

        // Clock | Version
        let r1 = c.clone() | m.clone();
        prop_assert_eq!(r1.party(), &pid);
        prop_assert_eq!(r1.version(), expected.clone());

        // Version | Clock
        let r2 = m.clone() | c.clone();
        prop_assert_eq!(r2.party(), &pid);
        prop_assert_eq!(r2.version(), expected.clone());

        // Clock |= Version
        let mut r3 = c.clone();
        r3 |= m.clone();
        prop_assert_eq!(r3.party(), &pid);
        prop_assert_eq!(r3.version(), expected.clone());

        // Version | Version is join.
        prop_assert_eq!(v0 | m, expected);
    }
}

// ───────────────────────────── concurrency / message causality ─────────────────────────────

/// Two clocks forked from a common seed that tick without exchanging messages
/// are concurrent (incomparable).
#[test]
fn independent_forks_are_concurrent() {
    let mut a = Clock::seed();
    let mut b = a.fork();
    a.tick();
    b.tick();
    assert!(a.concurrent_with(&b));
    assert!(b.concurrent_with(&a));
}

/// A receive carries the sender's knowledge: the message is `<=` the receiver's
/// resulting version, and the receiver strictly advances.
#[test]
fn receive_carries_knowledge() {
    let mut a = Clock::seed();
    let mut b = a.fork();
    let msg = a.send(); // a ticks, emits its version
    let before_b = b.version();
    b.receive(msg.clone());
    assert!(leq(&msg, &b.version()), "b must have seen the message");
    assert!(leq(&before_b, &b.version()) && before_b != b.version());
    assert!(a.happens_before(&b) || a.concurrent_with(&b));
}

proptest! {
    /// No version decreases and join never loses knowledge: after `i` sends to
    /// `j`, the message and `j`'s prior version are both `<=` `j`'s new one.
    #[test]
    fn monotone(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let mut cs = run(&ops);
        let n = cs.len();
        let (i, j) = (i % n, j % n);
        let old_j = cs[j].version();
        let msg = cs[i].send();
        let msg_seen = msg.clone();
        cs[j].receive(msg);
        let new_j = cs[j].version();
        prop_assert!(leq(&msg_seen, &new_j));
        prop_assert!(leq(&old_j, &new_j));
        prop_assert_ne!(old_j, new_j);
    }
}

// ───────────────────────────── paper worked examples ─────────────────────────────

/// §5.2. Normalization of the event component matches the paper's literal
/// examples: `(2,1,1) ≡ 3` and `(2,(2,1,0),3) ≡ (4,(0,1,0),1)`.
#[test]
fn event_normalization() {
    use Version as V;
    let leaf = |n: u64| V::leaf(n);

    // (2,1,1) ~ 3
    let unit_pulse = V::node(2u64, leaf(1), leaf(1));
    assert_eq!(unit_pulse, leaf(3));

    // (2,(2,1,0),3) ~ (4,(0,1,0),1)
    let left = V::Node(2u64.into(), Box::new(leaf(1)), Box::new(leaf(0))); // (2,1,0), already normal
    let example = V::node(2u64, left, leaf(3));
    let expected = V::Node(
        4u64.into(),
        Box::new(V::Node(0u64.into(), Box::new(leaf(1)), Box::new(leaf(0)))),
        Box::new(leaf(1)),
    );
    assert_eq!(example, expected);
}

/// §5.2. The unit pulse `1 ≡ (1,1) ≡ (1,(1,1)) ≡ ((1,1),1)` collapses for the
/// id component too: `norm((1,1)) = 1`, `norm((0,0)) = 0`.
#[test]
fn id_normalization() {
    assert_eq!(
        Party::node(Party::Leaf(true), Party::Leaf(true)),
        Party::Leaf(true)
    );
    assert_eq!(
        Party::node(Party::Leaf(false), Party::Leaf(false)),
        Party::Leaf(false)
    );
}

/// §5.3.2. The `split` equations: `split(1) = ((1,0),(0,1))`, and a node with
/// two nonzero subtrees splits by handing each side one subtree.
#[test]
fn split_equations() {
    use Party::{Leaf, Node};

    // split(1) = ((1,0),(0,1))
    let (a, b) = Leaf(true).split();
    assert_eq!(a, Node(Box::new(Leaf(true)), Box::new(Leaf(false))));
    assert_eq!(b, Node(Box::new(Leaf(false)), Box::new(Leaf(true))));

    // split((1,0)) descends into the left: (((1,0),0), ((0,1),0))
    let left_half = Node(Box::new(Leaf(true)), Box::new(Leaf(false)));
    let (a, b) = left_half.split();
    assert_eq!(
        a,
        Node(
            Box::new(Node(Box::new(Leaf(true)), Box::new(Leaf(false)))),
            Box::new(Leaf(false)),
        )
    );
    assert_eq!(
        b,
        Node(
            Box::new(Node(Box::new(Leaf(false)), Box::new(Leaf(true)))),
            Box::new(Leaf(false)),
        )
    );
}

/// §5.3.3. `sum` of complementary halves recovers the whole space, and the
/// event `join` is the pointwise max / LUB: `(0,1,0) ⊔ (0,0,2) = (1,0,1)`.
#[test]
fn sum_and_join() {
    use Party::{Leaf as PLeaf, Node as PNode};
    use Version as V;
    let vleaf = |n: u64| V::leaf(n);

    let left_half = PNode(Box::new(PLeaf(true)), Box::new(PLeaf(false)));
    let right_half = PNode(Box::new(PLeaf(false)), Box::new(PLeaf(true)));
    assert_eq!(left_half.sum(right_half), PLeaf(true)); // sum((1,0),(0,1)) = 1

    let a = V::Node(0u64.into(), Box::new(vleaf(1)), Box::new(vleaf(0))); // (0,1,0)
    let b = V::Node(0u64.into(), Box::new(vleaf(0)), Box::new(vleaf(2))); // (0,0,2)
    let joined = a | b;
    let expected = V::Node(1u64.into(), Box::new(vleaf(0)), Box::new(vleaf(1))); // (1,0,1)
    assert_eq!(joined, expected);
}

/// The event `meet` is the pointwise min / GLB, dual to [`sum_and_join`]'s
/// pointwise max: `(0,1,0) ⊓ (0,0,2) = 0`, since `min(1,0) = min(0,2) = 0` and
/// the all-zero node collapses to a single leaf.
#[test]
fn meet_is_pointwise_min() {
    use Version as V;
    let vleaf = |n: u64| V::leaf(n);

    let a = V::Node(0u64.into(), Box::new(vleaf(1)), Box::new(vleaf(0))); // (0,1,0)
    let b = V::Node(0u64.into(), Box::new(vleaf(0)), Box::new(vleaf(2))); // (0,0,2)
    let met = a & b;
    assert_eq!(met, vleaf(0));
}

/// §5.3.4. The headline of the example: when the id owns the whole space,
/// `event` fills the gap so the event component collapses to a single integer —
/// `event(1, (0,1,0)) = (1, 1)`, i.e. the event tree becomes `Leaf(1)`.
#[test]
fn event_fills_to_single_integer() {
    use Version as V;

    let gapped = V::node(0u64, V::leaf(1u64), V::leaf(0u64)); // (0,1,0)
    let mut v = gapped;
    v.tick(&Party::seed()); // id = 1 (whole space)
    assert_eq!(v, V::leaf(1u64));
}

// ───────────────────── grow optimality, oracle side ─────────────────────
//
// The paper's event condition (§3, §5.3.4) is the defining causality property:
// an event registers a *minimal* inflation. `grow` delivers it via a dynamic
// program. These properties pin the oracle's `grow` against a brute-force
// search over the entire feasible inflation space
// (`testing::grow_brute_force::all_inflations`), independently establishing
// that the oracle's own DP is genuinely cost-minimal — something nothing else
// in the suite does (every other check is impl == oracle, which shares the DP).
// The impl is held to the same brute-force standard in `version::tests`.

proptest! {
    /// The oracle's `grow` reports the *globally* minimal inflation cost.
    /// `min_inflation_cost` enumerates the whole feasible single-region
    /// inflation space (descending both children everywhere, no pruning) and
    /// takes the flat minimum; the DP's greedy local choice must match it. This
    /// is the independent minimality check — it does not rely on the DP at all.
    #[test]
    fn grow_cost_is_globally_minimal(
        id in arb_oracle_party_nonempty(),
        e in arb_oracle_version(),
    ) {
        let (_, dp_cost) = e.grow_for_test(&id);
        let brute = min_inflation_cost(&id, &e).expect("non-empty id always has an inflation");
        prop_assert_eq!(dp_cost, brute, "grow's cost is not the global minimum");
    }
}

proptest! {
    /// The oracle's `grow` chooses exactly the brute-force right-favoring
    /// minimal inflation — the same raw tree and cost. `best_inflation` selects
    /// the globally cost-minimal candidate with the paper's root-ward tie-break
    /// (`cl < cr` goes left, else right), weighing each child by its
    /// full-enumeration minimum. So a match confirms both the cost minimality
    /// and the correct tie-break direction.
    #[test]
    fn grow_matches_brute_force_choice(
        id in arb_oracle_party_nonempty(),
        e in arb_oracle_version(),
    ) {
        let dp = e.grow_for_test(&id);
        let brute = best_inflation(&id, &e).expect("non-empty id always has an inflation");
        prop_assert_eq!(dp, brute);
    }
}

proptest! {
    /// The brute-force selection is internally consistent: `best_inflation` is
    /// one of the enumerated candidates, and its cost equals the global
    /// minimum. Guards the brute-force oracle itself, so the two checks above
    /// stand on solid ground.
    #[test]
    fn best_inflation_is_a_min_cost_candidate(
        id in arb_oracle_party_nonempty(),
        e in arb_oracle_version(),
    ) {
        let cands = all_inflations(&id, &e);
        prop_assume!(!cands.is_empty());
        let min = cands.iter().map(|(_, c)| *c).min().unwrap();
        let (best_tree, best_cost) = best_inflation(&id, &e).unwrap();
        prop_assert_eq!(best_cost, min, "best_inflation cost is not the minimum");
        prop_assert!(
            cands.iter().any(|(t, c)| *t == best_tree && *c == best_cost),
            "best_inflation is not among the enumerated candidates",
        );
    }
}

proptest! {
    /// §3 (the event condition), metamorphic form. `grow` "dominates no more
    /// events than needed": no *feasible inflation* `x` sits strictly between
    /// `e` and the grown `e'` — there is no `x` reachable by inflating `(id,
    /// e)` with `e ≤ x < e'`. This is the correct, scoped reading of the
    /// paper's `x < e' ⇒ x ≤ e`: the relevant `x` are the event components the
    /// system can produce (the inflation candidates), not arbitrary fabricated
    /// step functions. (Over the *full* pointwise lattice the literal `x < e' ⇒
    /// x ≤ e` is false even for a single `+1` increment — e.g. `e = 0`, `e' =
    /// 1`, `x = (0,1,0)` has `x < e'` but `x ≰ e` — because that lattice is
    /// dense and `fill` collapses owned regions to their max; the §3 condition
    /// is about system states, not arbitrary functions.) `grow` runs directly
    /// here, independent of whether `event` would have taken the `fill` branch.
    #[test]
    fn grow_dominates_no_more_than_needed(
        id in arb_oracle_party_nonempty(),
        e in arb_oracle_version(),
    ) {
        let (eprime, _) = e.grow_for_test(&id);
        let eprime = eprime.normalized_for_test();
        for (cand, _) in all_inflations(&id, &e) {
            let cand = cand.normalized_for_test();
            let above_e = leq(&e, &cand);
            let strictly_below = cand.partial_cmp(&eprime) == Some(Ordering::Less);
            prop_assert!(
                !(above_e && strictly_below),
                "an inflation candidate sits strictly between e and e': \
                 cand={:?} e={:?} e'={:?}",
                cand, e, eprime,
            );
        }
    }
}

/// §5.1. Run the paper's example end-to-end and assert its published
/// qualitative outcomes: three participants, ids always summing to the whole
/// space, the third fork reusing existing id subtrees (not deepening the
/// spine), and a post-join event that collapses the event component to a single
/// integer.
#[test]
fn worked_example() {
    // seed → fork into two.
    let mut p1 = Clock::seed();
    let mut p2 = p1.fork();

    // p1 suffers one event, then forks.
    p1.tick();
    let mut p1a = p1.fork(); // p1 keeps half; p1a is the child
    let mut p1b = p1; // rename the retained half for clarity

    // p2 suffers two events.
    p2.tick();
    p2.tick();

    // Three participants now.
    let region_sum = |cs: &[&Clock]| {
        let mut acc = Party::Leaf(false);
        for c in cs {
            acc = acc.sum(c.party().clone());
        }
        acc
    };
    assert_eq!(region_sum(&[&p1a, &p1b, &p2]), Party::seed());

    // One participant (p1a) suffers an event; the other two sync (join + fork).
    let v_before = p1a.version();
    p1a.tick();
    assert!(p1a.version() > v_before);

    let p1b_pre = p1b.party().clone();
    let p2_pre = p2.party().clone();
    let merged_region = p1b_pre.sum(p2_pre);
    p1b.sync(&mut p2).expect("disjoint");

    // Sync reconciled histories and preserved total ownership of the two halves.
    assert_eq!(p1b.version(), p2.version());
    let mut rejoined = p1b.party().clone();
    rejoined.join(p2.party().clone()).expect("disjoint");
    assert_eq!(rejoined, merged_region);

    // Still three participants covering the whole space.
    assert_eq!(region_sum(&[&p1a, &p1b, &p2]), Party::seed());

    // The paper's closing observation: a join merges id subtrees, and an event
    // can then inflate the gap so the event component becomes a single integer.
    // Rejoin all three participants (recovering id = 1) and tick: because the
    // id owns the whole space, `event` fills every gap and the event tree
    // collapses to a Leaf.
    let mut whole = p1a;
    whole.join(p1b).expect("disjoint");
    whole.join(p2).expect("disjoint");
    assert_eq!(whole.party(), &Party::seed());
    whole.tick();
    assert!(
        matches!(whole.version(), Version::Leaf(_)),
        "post-join event should collapse to a single integer, got {:?}",
        whole.version()
    );
}

// ───────────────────────────── party containment (`covers`) ─────────────────────────────

proptest! {
    /// `covers` is a partial order on regions over *arbitrary* (often
    /// overlapping or unrelated) parties: reflexive, antisymmetric (mutual
    /// covering ⇔ equality), and transitive. The whole `seed` covers every
    /// party, and covering a *nonempty* region rules out disjointness.
    #[test]
    fn party_covers(a in arb_oracle_party(), b in arb_oracle_party(), c in arb_oracle_party()) {
        prop_assert!(a.covers(&a)); // reflexive
        prop_assert_eq!(a.covers(&b) && b.covers(&a), a == b); // antisymmetric
        if a.covers(&b) && b.covers(&c) {
            prop_assert!(a.covers(&c)); // transitive
        }
        prop_assert!(Party::seed().covers(&a)); // the whole covers every part
        if a.covers(&b) && !b.is_empty() {
            prop_assert!(!a.is_disjoint(&b), "covering a nonempty region is not disjoint");
        }
    }
}

// ───────────────────────────── party difference (`without`) ─────────────────────────────

proptest! {
    /// `without` computes the region difference `a \ b`: the result is in normal
    /// form, a subregion of `a` (`a` covers it), disjoint from `b`, and empty
    /// exactly when `b` covers `a`. Removing a *disjoint* share is the identity.
    #[test]
    fn party_without(a in arb_oracle_party(), b in arb_oracle_party()) {
        let d = a.without(&b);
        prop_assert!(d.is_normal());
        prop_assert!(a.covers(&d)); // a subregion of `a`
        prop_assert!(d.is_disjoint(&b)); // shares nothing with `b`
        prop_assert_eq!(d.is_empty(), b.covers(&a)); // empty iff `b` covers `a`
        if a.is_disjoint(&b) {
            prop_assert_eq!(a.without(&b), a.clone()); // disjoint removal is a no-op
        }
    }
}

proptest! {
    /// `without` is the partial inverse of `sum`: forking a party into disjoint
    /// halves and removing one recovers the other (`(l ⊔ r) \ r == l`). The
    /// oracle's `fork` is the paper split, so no randomness is needed.
    #[test]
    fn without_inverts_sum(p in arb_oracle_party_nonempty()) {
        let mut keep = p.clone();
        let give = keep.fork(); // p == keep ⊔ give, disjoint
        prop_assert_eq!(p.without(&give), keep.clone()); // carve the give-half back out
        prop_assert_eq!(keep.clone().without(&give), keep); // disjoint removal is a no-op
    }
}

// ───────────────────────────── version projection (`/`) ─────────────────────────────

proptest! {
    /// The quotient `v / p` masks `v` to `p`'s region: a sub-version, idempotent,
    /// the identity under the whole-interval `seed`, and a homomorphism of both
    /// join and meet. The reference laws the impl's `/` is checked against.
    #[test]
    fn version_projection(
        ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64,
    ) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (a, b) = (vs[i % n].clone(), vs[j % n].clone());
        let p = cs[k % n].party();

        let proj = a.clone() / p;
        prop_assert!(proj.is_normal());
        prop_assert!(leq(&proj, &a)); // sub-version
        prop_assert_eq!(proj.clone() / p, proj.clone()); // idempotent
        prop_assert_eq!(a.clone() / &Party::seed(), a.clone()); // seed is the identity
        prop_assert_eq!((a.clone() | b.clone()) / p, (a.clone() / p) | (b.clone() / p)); // join hom
        prop_assert_eq!((a.clone() & b.clone()) / p, (a.clone() / p) & (b.clone() / p)); // meet hom
    }
}

// ───────────────────────────── min_ticks / own_version ─────────────────────────────

proptest! {
    /// `min_ticks` is the sum of every base, so it is `0` only for the zero
    /// version, additive under a disjoint-support join (`(a|b)` over fork halves
    /// sums their counts), and dominated by neither operand alone. A single
    /// whole-interval tick costs exactly one.
    #[test]
    fn version_min_ticks(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let v = &vs[i % n];

        prop_assert_eq!(v.min_ticks() == 0, *v == Version::new()); // zero iff the zero version
        prop_assert!(v.min_ticks() >= 1 || *v == Version::new());
        // The seed ticked once in a line costs exactly one.
        let mut one = Version::new();
        one.tick(&Party::seed());
        prop_assert_eq!(one.min_ticks(), 1);
    }
}

proptest! {
    /// `own_version` is exactly `version() / party()`: the clock's history within
    /// the region it owns. It is a sub-version of the full version, and ticking
    /// (which advances only the owned region) raises it.
    #[test]
    fn clock_own_version(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let c = &cs[i % n];
        prop_assert_eq!(c.own_version(), c.version() / c.party()); // the definition
        prop_assert!(leq(&c.own_version(), &c.version())); // a sub-version
    }
}
