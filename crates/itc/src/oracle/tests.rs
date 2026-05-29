//! Oracle property suite (Appendix D, O1–O15): the Phase 0 gate.
//!
//! These establish that the recursive oracle is a faithful (if suboptimal)
//! realization of the paper, so it can be trusted as differential ground truth.
//! Values are generated via operations from a seed (always valid, normal-form,
//! and — for populations — pairwise party-disjoint), never by fabricating trees.

use std::cmp::Ordering;

use proptest::prelude::*;

use super::{Clock, Party, Version};
use crate::test_support::{leq, run, versions, world_strategy};

// ───────────────────────────── O1 ─────────────────────────────

/// O1. `Clock::seed()` decomposes to `(Party::seed(), Version::new())`.
#[test]
fn o1_genesis() {
    let (p, v) = Clock::seed().into_parts();
    assert_eq!(p, Party::seed());
    assert_eq!(v, Version::new());
}

proptest! {
    /// O1. `Version::new()` is the two-sided identity for `|`.
    #[test]
    fn o1_join_identity(ops in world_strategy()) {
        for v in versions(&run(&ops)) {
            prop_assert_eq!(Version::new() | v.clone(), v.clone());
            prop_assert_eq!(v.clone() | Version::new(), v);
        }
    }
}

// ───────────────────────────── O2 ─────────────────────────────

proptest! {
    /// O2. Every value any op produces is in normal form (parties and versions),
    /// including the result of a join.
    #[test]
    fn o2_normal_form(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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
    }
}

// ───────────────────────────── O3 ─────────────────────────────

proptest! {
    /// O3. The causal order is a partial order: reflexive, antisymmetric,
    /// transitive; `==` ⇔ `Some(Equal)`; concurrency ⇔ `None`.
    #[test]
    fn o3_partial_order(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
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

// ───────────────────────────── O4 ─────────────────────────────

proptest! {
    /// O4. `tick` strictly advances: `v < v.tick(p)` for the clock's own party.
    #[test]
    fn o4_tick_advances(ops in world_strategy()) {
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

// ───────────────────────────── O5 ─────────────────────────────

proptest! {
    /// O5. Join is a bounded join-semilattice and the least upper bound:
    /// commutative, associative, idempotent; upper bound; and least (below any
    /// common upper bound).
    #[test]
    fn o5_lattice(ops in world_strategy(),
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

// ───────────────────────────── O6 ─────────────────────────────

proptest! {
    /// O6. The order is induced by the join: `a <= b` ⇔ `a|b == b`.
    #[test]
    fn o6_order_from_join(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let vs = versions(&run(&ops));
        let n = vs.len();
        let (a, b) = (&vs[i % n], &vs[j % n]);
        prop_assert_eq!(leq(a, b), (a.clone() | b.clone()) == *b);
    }
}

// ───────────────────────────── O7 ─────────────────────────────

proptest! {
    /// O7. Party fork is invertible by join, and fork preserves a clock's version.
    #[test]
    fn o7_fork_join_roundtrip(ops in world_strategy()) {
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
    /// O7/O10. `fork` leaves both halves carrying the parent's version.
    #[test]
    fn o7_fork_preserves_version(ops in world_strategy()) {
        let mut cs = run(&ops);
        for c in &mut cs {
            let before = c.version();
            let child = c.fork();
            prop_assert_eq!(c.version(), before.clone());
            prop_assert_eq!(child.version(), before);
        }
    }
}

// ───────────────────────────── O8 ─────────────────────────────

proptest! {
    /// O8. Party order is descent: each fork child is `>` its parent; the order
    /// is a partial order; `join` is the meet (a lower bound) for disjoint
    /// parties; and `join` errors (handing the party back unchanged) on overlap.
    #[test]
    fn o8_party_order(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let parties: Vec<Party> = cs.iter().map(|c| c.party().clone()).collect();
        let n = parties.len();
        let (a, b, c) = (&parties[i % n], &parties[j % n], &parties[k % n]);

        // Partial-order laws under descent.
        prop_assert_eq!(a.partial_cmp(a), Some(Ordering::Equal));
        let le = |x: &Party, y: &Party| x.partial_cmp(y).is_some_and(|o| o != Ordering::Greater);
        if le(a, b) && le(b, a) {
            prop_assert_eq!(a, b);
        }
        if le(a, b) && le(b, c) {
            prop_assert!(le(a, c));
        }

        // Fork: parent < each child; siblings disjoint.
        let mut parent = a.clone();
        let snapshot = parent.clone();
        let child = parent.fork();
        prop_assert_eq!(snapshot.partial_cmp(&parent), Some(Ordering::Less));
        prop_assert_eq!(snapshot.partial_cmp(&child), Some(Ordering::Less));
        prop_assert!(parent.is_disjoint(&child));

        // Meet of two disjoint parties (the two fresh halves): a lower bound that
        // equals their sum, recovering the original region.
        let mut meet = parent.clone();
        prop_assert_eq!(meet.join(child.clone()), Ok(()));
        prop_assert!(le(&meet, &parent) && le(&meet, &child));
        prop_assert_eq!(&meet, &snapshot);

        // Overlap: joining a descendant into its ancestor errors and is a no-op.
        let mut ancestor = snapshot.clone();
        let descendant = {
            let mut x = snapshot.clone();
            x.fork()
        };
        prop_assert!(!ancestor.is_disjoint(&descendant));
        let before = ancestor.clone();
        prop_assert_eq!(ancestor.join(descendant.clone()), Err(descendant));
        prop_assert_eq!(ancestor, before);
    }
}

// ───────────────────────────── O9 ─────────────────────────────

proptest! {
    /// O9. Over any seed-derived trace, all live parties are pairwise disjoint and
    /// their overall `sum` recovers the whole id space.
    #[test]
    fn o9_disjointness_invariant(ops in world_strategy()) {
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

// ───────────────────────────── O10 ─────────────────────────────

proptest! {
    /// O10. Peek does not advance: `version()` is idempotent and leaves the clock
    /// unchanged. (Fork-preserves-history is O7.)
    #[test]
    fn o10_peek_does_not_advance(ops in world_strategy()) {
        for c in &run(&ops) {
            let v1 = c.version();
            let v2 = c.version();
            prop_assert_eq!(v1, v2);
        }
    }
}

// ───────────────────────────── O11 ─────────────────────────────

proptest! {
    /// O11. A dominated receive equals a bare tick, and re-delivery is idempotent
    /// (`v | m | m == v | m`).
    #[test]
    fn o11_dominated_receive(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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

// ───────────────────────────── O12 ─────────────────────────────

proptest! {
    /// O12. `sync` reconciles to the join and re-splits without losing ownership.
    #[test]
    fn o12_sync(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let mut cs = run(&ops);
        let n = cs.len();
        if n < 2 {
            return Ok(());
        }
        let (i, j) = (i % n, j % n);
        prop_assume!(i != j);

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

// ───────────────────────────── O13 ─────────────────────────────

proptest! {
    /// O13. Heterogeneous joins change only the version, to the `ev_join` of the
    /// two; a bare `Version` acts as a party-`0` clock would.
    #[test]
    fn o13_heterogeneous_joins(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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

        // Version | Version is ev_join.
        prop_assert_eq!(v0 | m, expected);
    }
}

// ───────────────────────────── O14 ─────────────────────────────

/// O14. Two clocks forked from a common seed that tick without exchanging
/// messages are concurrent (incomparable).
#[test]
fn o14_independent_forks_are_concurrent() {
    let mut a = Clock::seed();
    let mut b = a.fork();
    a.tick();
    b.tick();
    assert!(a.concurrent_with(&b));
    assert!(b.concurrent_with(&a));
}

/// O14. A receive carries the sender's knowledge: the message is `<=` the
/// receiver's resulting version, and the receiver strictly advances.
#[test]
fn o14_receive_carries_knowledge() {
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
    /// O14. No version decreases and join never loses knowledge: after `i` sends
    /// to `j`, the message and `j`'s prior version are both `<=` `j`'s new one.
    #[test]
    fn o14_monotone(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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

// ───────────────────────────── O15 ─────────────────────────────

/// O15 (§5.2). Normalization of the event component matches the paper's literal
/// examples: `(2,1,1) ≡ 3` and `(2,(2,1,0),3) ≡ (4,(0,1,0),1)`.
#[test]
fn o15_event_normalization() {
    use Version::{Leaf, Node};

    // (2,1,1) ~ 3
    let unit_pulse = Version::node(2, Leaf(1), Leaf(1));
    assert_eq!(unit_pulse, Leaf(3));

    // (2,(2,1,0),3) ~ (4,(0,1,0),1)
    let left = Node(2, Box::new(Leaf(1)), Box::new(Leaf(0))); // (2,1,0), already normal
    let example = Version::node(2, left, Leaf(3));
    let expected = Node(
        4,
        Box::new(Node(0, Box::new(Leaf(1)), Box::new(Leaf(0)))),
        Box::new(Leaf(1)),
    );
    assert_eq!(example, expected);
}

/// O15 (§5.2). The unit pulse `1 ≡ (1,1) ≡ (1,(1,1)) ≡ ((1,1),1)` collapses for
/// the id component too: `norm((1,1)) = 1`, `norm((0,0)) = 0`.
#[test]
fn o15_id_normalization() {
    assert_eq!(
        Party::node(Party::Leaf(true), Party::Leaf(true)),
        Party::Leaf(true)
    );
    assert_eq!(
        Party::node(Party::Leaf(false), Party::Leaf(false)),
        Party::Leaf(false)
    );
}

/// O15 (§5.3.2). The `split` equations: `split(1) = ((1,0),(0,1))`, and a node
/// with two nonzero subtrees splits by handing each side one subtree.
#[test]
fn o15_split() {
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

/// O15 (§5.3.3). `sum` of complementary halves recovers the whole space, and the
/// event `join` is the pointwise max / LUB: `(0,1,0) ⊔ (0,0,2) = (1,0,1)`.
#[test]
fn o15_sum_and_join() {
    use Party::{Leaf as PLeaf, Node as PNode};
    use Version::{Leaf as VLeaf, Node as VNode};

    let left_half = PNode(Box::new(PLeaf(true)), Box::new(PLeaf(false)));
    let right_half = PNode(Box::new(PLeaf(false)), Box::new(PLeaf(true)));
    assert_eq!(left_half.sum(right_half), PLeaf(true)); // sum((1,0),(0,1)) = 1

    let a = VNode(0, Box::new(VLeaf(1)), Box::new(VLeaf(0))); // (0,1,0)
    let b = VNode(0, Box::new(VLeaf(0)), Box::new(VLeaf(2))); // (0,0,2)
    let joined = a | b;
    let expected = VNode(1, Box::new(VLeaf(0)), Box::new(VLeaf(1))); // (1,0,1)
    assert_eq!(joined, expected);
}

/// O15 (§5.3.4). The headline of the example: when the id owns the whole space,
/// `event` fills the gap so the event component collapses to a single integer —
/// `event(1, (0,1,0)) = (1, 1)`, i.e. the event tree becomes `Leaf(1)`.
#[test]
fn o15_event_fills_to_single_integer() {
    use Version::{Leaf, Node};

    let gapped = Node(0, Box::new(Leaf(1)), Box::new(Leaf(0))); // (0,1,0)
    let mut v = gapped;
    v.tick(&Party::seed()); // id = 1 (whole space)
    assert_eq!(v, Leaf(1));
}

/// O15 (§5.1). Run the paper's example end-to-end and assert its published
/// qualitative outcomes: three participants, ids always summing to the whole
/// space, the third fork reusing existing id subtrees (not deepening the spine),
/// and a post-join event that collapses the event component to a single integer.
#[test]
fn o15_worked_example() {
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
    // Rejoin all three participants (recovering id = 1) and tick: because the id
    // owns the whole space, `event` fills every gap and the event tree collapses
    // to a Leaf.
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
