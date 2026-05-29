//! Shared test scaffolding: the seed-derived op-trace generator (used by both the
//! oracle property suite and the impl property tests) and the oracle↔impl bridge
//! used for differential structural agreement.
//!
//! Values are always generated via operations from a seed, so they are valid,
//! normal-form, and — for populations — pairwise party-disjoint. Impl values are
//! built from oracle trees with [`from_oracle_party`]/[`from_oracle_version`], which
//! emit the canonical packed bits directly (NOT via the public codec), keeping
//! algorithm correctness decoupled from codec correctness.

use std::cmp::Ordering;

use proptest::prelude::*;

use crate::codec::{self, Bits};
use crate::oracle;
use crate::{Clock, Party, Version};

// ───────────────────────────── op-trace generator ─────────────────────────────

/// One step of a seed-derived execution. Indices are reduced modulo the live
/// population, so any index is valid and every member descends from one seed via
/// fork/join/sync — keeping all parties pairwise disjoint.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// Advance member `i`.
    Tick(usize),
    /// Split member `i`, appending the child.
    Fork(usize),
    /// `i` sends (ticks, emits its version); `j` receives it.
    Send(usize, usize),
    /// Reconcile `i` and `j` (join then re-split).
    Sync(usize, usize),
    /// Join `j` into `i`, removing `j`.
    Join(usize, usize),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0usize..8).prop_map(Op::Tick),
        (0usize..8).prop_map(Op::Fork),
        (0usize..8, 0usize..8).prop_map(|(a, b)| Op::Send(a, b)),
        (0usize..8, 0usize..8).prop_map(|(a, b)| Op::Sync(a, b)),
        (0usize..8, 0usize..8).prop_map(|(a, b)| Op::Join(a, b)),
    ]
}

/// A trace of up to 30 ops over a population that starts as a single seed clock.
pub(crate) fn world_strategy() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(op_strategy(), 0..30)
}

/// Apply a trace to a fresh oracle population.
pub(crate) fn run(ops: &[Op]) -> Vec<oracle::Clock> {
    let mut cs = vec![oracle::Clock::seed()];
    for op in ops {
        let n = cs.len();
        match *op {
            Op::Tick(i) => cs[i % n].tick(),
            Op::Fork(i) => {
                let child = cs[i % n].fork();
                cs.push(child);
            }
            Op::Send(i, j) => {
                let (i, j) = (i % n, j % n);
                let msg = cs[i].send();
                cs[j].receive(msg);
            }
            Op::Sync(i, j) => {
                let (i, j) = (i % n, j % n);
                if i != j {
                    let (lo, hi) = (i.min(j), i.max(j));
                    let (a, b) = cs.split_at_mut(hi);
                    a[lo]
                        .sync(&mut b[0])
                        .expect("seed-derived parties are disjoint");
                }
            }
            Op::Join(i, j) => {
                if n > 1 {
                    let (i, j) = (i % n, j % n);
                    if i != j {
                        let victim = cs.remove(j);
                        let i2 = if j < i { i - 1 } else { i };
                        cs[i2]
                            .join(victim)
                            .expect("seed-derived parties are disjoint");
                    }
                }
            }
        }
    }
    cs
}

/// Every live clock's current version.
pub(crate) fn versions(cs: &[oracle::Clock]) -> Vec<oracle::Version> {
    cs.iter().map(|c| c.version()).collect()
}

/// `a <= b` under the oracle causal order (treating concurrency as not-`<=`).
pub(crate) fn leq(a: &oracle::Version, b: &oracle::Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

// ───────────────────────────── oracle → impl bridge ─────────────────────────────

fn emit_id(out: &mut Bits, t: &oracle::Party) {
    match t {
        oracle::Party::Leaf(b) => {
            out.push(false);
            out.push(*b);
        }
        oracle::Party::Node(l, r) => {
            out.push(true);
            emit_id(out, l);
            emit_id(out, r);
        }
    }
}

fn emit_ev(out: &mut Bits, t: &oracle::Version) {
    match t {
        oracle::Version::Leaf(n) => {
            out.push(false);
            codec::encode_int(out, *n);
        }
        oracle::Version::Node(n, l, r) => {
            out.push(true);
            codec::encode_int(out, *n);
            emit_ev(out, l);
            emit_ev(out, r);
        }
    }
}

/// Build the impl `Party` whose canonical bits encode `t`. Recursive over a bounded
/// oracle tree (test-only; the impl's own traversals are iterative).
pub(crate) fn from_oracle_party(t: &oracle::Party) -> Party {
    let mut bits = Bits::new();
    emit_id(&mut bits, t);
    Party::from_bits(bits)
}

/// Build the impl `Version` whose canonical bits encode `t`.
pub(crate) fn from_oracle_version(t: &oracle::Version) -> Version {
    let mut bits = Bits::new();
    emit_ev(&mut bits, t);
    Version::from_bits(bits)
}

/// Build the impl `Clock` mirroring an oracle clock.
pub(crate) fn from_oracle_clock(c: &oracle::Clock) -> Clock {
    let (party, version) = c.trees();
    Clock::from_parts(from_oracle_party(party), from_oracle_version(version))
}

// ───────────────────────────── adversarial deep inputs ─────────────────────────────
//
// The complexity proptests assert that a traversal's step count grows linearly, not
// quadratically, with input size. They drive that over random *spine-family* shapes —
// the deep, unbalanced trees that are the worst case for any traversal locating a right
// child by re-scanning its left subtree. Each shape is parameterized by a `scale` knob
// whose node count is linear in `scale`, so building at `scale` and `4 * scale` gives a
// 4x-larger input of the *same* shape, and the step ratio should stay near 4x (not
// 16x). Trees are built via the oracle's normalizing constructors, so they are always
// in normal form; the constructors are `O(1)` per node (no deep recursion at build).

/// A deep, unbalanced tree shape. All are spine-like (depth linear in `scale`) — the
/// shapes that stress right-child location.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    /// Every node leans left: `(…((·,·),·)…,·)`.
    LeftSpine,
    /// Every node leans right.
    RightSpine,
    /// Alternating left/right lean.
    Zigzag,
}

/// A random deep shape for the complexity proptests.
pub(crate) fn arb_shape() -> impl Strategy<Value = Shape> {
    prop_oneof![
        Just(Shape::LeftSpine),
        Just(Shape::RightSpine),
        Just(Shape::Zigzag),
    ]
}

/// Build a normal-form event tree of `shape` with `scale` internal nodes (so
/// `2*scale + 1` nodes total). Distinct leaf bases prevent collapse, preserving the
/// shape and size.
pub(crate) fn shape_version(shape: Shape, scale: usize) -> Version {
    use oracle::Version as V;
    let mut t = V::Leaf(0);
    for k in 1..=scale as u64 {
        let leaf = V::Leaf(k);
        t = match shape {
            Shape::LeftSpine => V::node(0, t, leaf),
            Shape::RightSpine => V::node(0, leaf, t),
            Shape::Zigzag if k % 2 == 0 => V::node(0, t, leaf),
            Shape::Zigzag => V::node(0, leaf, t),
        };
    }
    from_oracle_version(&t)
}

/// Build a non-empty normal-form id of `shape` with `scale` interior nodes. The spine
/// carries the owned region (a `1` leaf at its tip); the off-spine children are `0`.
pub(crate) fn shape_party(shape: Shape, scale: usize) -> Party {
    use oracle::Party as P;
    let mut t = P::seed(); // the `1` leaf
    for k in 0..scale {
        let zero = P::Leaf(false);
        t = match shape {
            Shape::LeftSpine => P::node(t, zero),
            Shape::RightSpine => P::node(zero, t),
            Shape::Zigzag if k % 2 == 0 => P::node(t, zero),
            Shape::Zigzag => P::node(zero, t),
        };
    }
    from_oracle_party(&t)
}

/// Assert that `steps`, measured at two input sizes whose node counts differ by `4×`,
/// grows roughly linearly rather than quadratically. Linear predicts `~4×` more steps;
/// quadratic predicts `~16×`. The `6×` threshold sits comfortably between, independent
/// of any constant factor.
pub(crate) fn assert_linear_scaling(small_steps: u64, big_steps: u64) {
    assert!(
        big_steps <= 6 * small_steps,
        "steps grew super-linearly: {small_steps} -> {big_steps} for a 4x larger input \
         (linear would be ~4x; this is {:.1}x)",
        big_steps as f64 / small_steps.max(1) as f64,
    );
}
