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
use crate::{metrics, oracle};
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

/// Apply one op to an impl population, mirroring [`run`] for the oracle (same index
/// arithmetic, so traces line up). Used by tests that drive the impl alone.
pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &Op) {
    let n = imp.len();
    match *op {
        Op::Tick(i) => imp[i % n].tick(),
        Op::Fork(i) => {
            let child = imp[i % n].fork();
            imp.push(child);
        }
        Op::Send(i, j) => {
            let (i, j) = (i % n, j % n);
            let msg = imp[i].send();
            imp[j].receive(msg);
        }
        Op::Sync(i, j) => {
            let (i, j) = (i % n, j % n);
            if i != j {
                let (lo, hi) = (i.min(j), i.max(j));
                let (a, b) = imp.split_at_mut(hi);
                a[lo]
                    .sync(&mut b[0])
                    .expect("seed-derived parties are disjoint");
            }
        }
        Op::Join(i, j) => {
            if n > 1 {
                let (i, j) = (i % n, j % n);
                if i != j {
                    let victim = imp.remove(j);
                    let i2 = if j < i { i - 1 } else { i };
                    imp[i2]
                        .join(victim)
                        .expect("seed-derived parties are disjoint");
                }
            }
        }
    }
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
            codec::encode_int(out, n);
        }
        oracle::Version::Node(n, l, r) => {
            out.push(true);
            codec::encode_int(out, n);
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

/// Build the impl `Version` whose canonical bits encode `t`. Recursive over a bounded
/// oracle tree (test-only; the impl's own traversals are iterative).
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

// ───────────────────────────── impl → oracle bridge ─────────────────────────────
//
// Structural lowering for differential agreement (§8): rebuild the oracle's tree shape
// from the impl's *internal* packed representation, then compare with `==`. This is the
// inverse of `from_oracle_*`. It walks the packed bits directly — the impl's at-rest
// storage — rather than round-tripping the public `encode`/`decode`, so the master
// harness checks algorithm correctness without sharing a failure mode with the byte
// codec (which is exercised separately). Recursive over a bounded tree
// (test-only; the impl's own traversals are iterative). Both forms are normalized, so
// structural `==` ⇔ semantic equality.

fn read_id(bits: &codec::BitsSlice, pos: usize) -> (oracle::Party, usize) {
    if bits[pos] {
        let (l, after_l) = read_id(bits, pos + 1);
        let (r, after_r) = read_id(bits, after_l);
        (oracle::Party::Node(Box::new(l), Box::new(r)), after_r)
    } else {
        (oracle::Party::Leaf(bits[pos + 1]), pos + 2)
    }
}

fn read_ev(bits: &codec::BitsSlice, pos: usize) -> (oracle::Version, usize) {
    let internal = bits[pos];
    // The oracle base is the arbitrary-precision `Base` (matching the impl), so lowering is
    // lossless for any magnitude: no `u64` truncation point.
    let (n, after_n) = codec::decode_int(bits, pos + 1).expect("canonical impl bits decode");
    if internal {
        let (l, after_l) = read_ev(bits, after_n);
        let (r, after_r) = read_ev(bits, after_l);
        (oracle::Version::Node(n, Box::new(l), Box::new(r)), after_r)
    } else {
        (oracle::Version::Leaf(n), after_n)
    }
}

/// Lower an impl `Party` to the oracle's structural tree by reading its packed bits.
pub(crate) fn to_oracle_party(p: &Party) -> oracle::Party {
    read_id(p.as_bits(), 0).0
}

/// Lower an impl `Version` to the oracle's structural tree by reading its packed bits.
pub(crate) fn to_oracle_version(v: &Version) -> oracle::Version {
    read_ev(v.as_bits(), 0).0
}

/// Lower an impl `Clock` to the oracle's `(Party, Version)` structural form.
pub(crate) fn to_oracle_clock(c: &Clock) -> (oracle::Party, oracle::Version) {
    (to_oracle_party(c.party()), to_oracle_version(&c.version()))
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

/// A deep tree shape. The spines (depth linear in `scale`) stress right-child location;
/// the bushy shape stresses multi-region cost comparisons (a node whose two children are
/// both feasible), which the spines — with a single owned leaf — never produce.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shape {
    /// Every node leans left: `(…((·,·),·)…,·)`.
    LeftSpine,
    /// Every node leans right.
    RightSpine,
    /// Alternating left/right lean.
    Zigzag,
    /// A balanced-ish bushy tree: many leaves at varying depths, so an id built from it
    /// has multiple genuinely feasible owned regions (see [`shape_party`]).
    Bushy,
}

/// A random deep shape for the complexity proptests.
pub(crate) fn arb_shape() -> impl Strategy<Value = Shape> {
    prop_oneof![
        Just(Shape::LeftSpine),
        Just(Shape::RightSpine),
        Just(Shape::Zigzag),
        Just(Shape::Bushy),
    ]
}

/// Build a balanced-ish bushy event tree over `leaves` distinct-based leaves, numbered
/// from `lo` (so no two siblings collapse). Splitting an odd count unevenly gives leaves
/// at varying depths. Recursive over a `O(log)` depth (test-only; the impl is iterative).
fn bushy_version(lo: u64, leaves: usize) -> oracle::Version {
    use oracle::Version as V;
    if leaves <= 1 {
        return V::leaf(lo);
    }
    let half = leaves / 2;
    V::node(
        0u64,
        bushy_version(lo, half),
        bushy_version(lo + half as u64, leaves - half),
    )
}

/// Build a balanced-ish bushy id over `leaves` leaves with bases alternating `1`/`0`, so
/// adjacent leaves never collapse and multiple owned (`1`) regions sit at varying depths.
/// Recursive over a `O(log)` depth (test-only; the impl is iterative).
fn bushy_party(lo: usize, leaves: usize) -> oracle::Party {
    use oracle::Party as P;
    if leaves <= 1 {
        return P::Leaf(lo.is_multiple_of(2)); // even index owned, odd empty
    }
    let half = leaves / 2;
    P::node(bushy_party(lo, half), bushy_party(lo + half, leaves - half))
}

/// Build a normal-form event tree of `shape` sized linearly in `scale`. The spines have
/// `scale` internal nodes (`2*scale + 1` nodes total); the bushy shape has `~scale`
/// leaves. Distinct leaf bases prevent collapse, preserving the shape and size.
pub(crate) fn shape_version(shape: Shape, scale: usize) -> Version {
    use oracle::Version as V;
    if let Shape::Bushy = shape {
        return from_oracle_version(&bushy_version(0, scale + 1));
    }
    let mut t = V::leaf(0u64);
    for k in 1..=scale as u64 {
        let leaf = V::leaf(k);
        t = match shape {
            Shape::LeftSpine => V::node(0u64, t, leaf),
            Shape::RightSpine => V::node(0u64, leaf, t),
            Shape::Zigzag if k % 2 == 0 => V::node(0u64, t, leaf),
            Shape::Zigzag => V::node(0u64, leaf, t),
            Shape::Bushy => unreachable!("handled above"),
        };
    }
    from_oracle_version(&t)
}

/// Build a disjoint "staircase" id pair `(a, b)` that drives the bounded lazy-skip in
/// `is_disjoint` to its worst case: `Θ(scale)` distinct skips, each over a small
/// subtree. `b` is a right-spine whose every left child is a 2-leaf subtree `(1, 0)`;
/// `a` is a right-spine of `0`-leaf left children. In lockstep, at every one of the
/// `scale` levels `a`'s left `0`-leaf aligns against `b`'s left *subtree*, so that
/// subtree is skipped once. The pair is disjoint (`a` owns only its deepest-right tip,
/// `b` owns its left subtrees and deepest-left tip), so the walk runs to completion (no
/// early `false`) and the cumulative skip cost is measured. Both ids are linear in
/// `scale`. With a *bounded* skip the total is `O(scale)`; an *unbounded* (rescanning)
/// skip would be `O(scale²)` — which the linear-scaling assertion would catch.
pub(crate) fn skip_stress_pair(scale: usize) -> (Party, Party) {
    use oracle::Party as P;
    // A 2-leaf subtree `(1, 0)`: a small node that owns its left half.
    let owned_left = || P::node(P::seed(), P::Leaf(false));
    // `b`: right-spine, each left child a small owned subtree, deepest-right tip empty.
    let mut b = P::Leaf(false);
    for _ in 0..scale {
        b = P::node(owned_left(), b);
    }
    // `a`: right-spine of `0`-leaf left children; owns only its deepest-right `1` tip,
    // which lands in `b`'s empty deepest-right region — so the pair is disjoint and the
    // walk runs to completion, skipping `b`'s left subtree once at every level.
    let mut a = P::seed();
    for _ in 0..scale {
        a = P::node(P::Leaf(false), a);
    }
    (from_oracle_party(&a), from_oracle_party(&b))
}

/// Build a containment "staircase" pair `(big, small)` that drives the bounded lazy-skip
/// in `compare` to its worst case: `Θ(scale)` distinct skips. `big` is a right-spine
/// whose every left child is a `1`-leaf (owns that whole left region); `small` is a
/// right-spine whose every left child is a 2-leaf subtree `(1, 0)`. In lockstep, at each
/// level `big`'s left `1`-leaf dominates `small`'s left *subtree*, so that subtree is
/// skipped once. `big ⊇ small`, so `compare` reports `Less` (ancestor) and runs to
/// completion; the
/// cumulative skip cost is measured. Both ids are linear in `scale`; a bounded skip is
/// `O(scale)`, an unbounded one `O(scale²)`.
pub(crate) fn contain_stress_pair(scale: usize) -> (Party, Party) {
    use oracle::Party as P;
    // `big`: right-spine, every left child fully owned (`1`); deepest-right empty so the
    // spine does not collapse (`(1, 1)` would). Owns every left region `small` touches.
    let mut big = P::Leaf(false);
    for _ in 0..scale {
        big = P::node(P::seed(), big);
    }
    // A 2-leaf subtree `(1, 0)`: a sub-region of `big`'s corresponding `1`.
    let owned_left = || P::node(P::seed(), P::Leaf(false));
    // `small`: right-spine, every left child a sub-region of `big`'s corresponding `1`.
    let mut small = P::Leaf(false);
    for _ in 0..scale {
        small = P::node(owned_left(), small);
    }
    (from_oracle_party(&big), from_oracle_party(&small))
}

/// Build a non-empty normal-form id of `shape` sized linearly in `scale`. The spines
/// carry a single owned region (a `1` leaf at the tip) with `0` off-spine; the bushy
/// shape carries many owned regions at varying depths (so a `grow` over it has nodes
/// whose two children are both feasible, exercising the multi-region cost comparison).
pub(crate) fn shape_party(shape: Shape, scale: usize) -> Party {
    use oracle::Party as P;
    if let Shape::Bushy = shape {
        return from_oracle_party(&bushy_party(0, scale + 1));
    }
    let mut t = P::seed(); // the `1` leaf
    for k in 0..scale {
        let zero = P::Leaf(false);
        t = match shape {
            Shape::LeftSpine => P::node(t, zero),
            Shape::RightSpine => P::node(zero, t),
            Shape::Zigzag if k % 2 == 0 => P::node(t, zero),
            Shape::Zigzag => P::node(zero, t),
            Shape::Bushy => unreachable!("handled above"),
        };
    }
    from_oracle_party(&t)
}

/// Build a depth-`depth` left-spine [`Party`] directly as canonical packed bits, with a
/// single owned region at the deep-left tip. Used by the stack-safety test,
/// which needs structures far deeper than the recursive oracle bridge (`emit_id`) or the
/// oracle's own recursive `Drop` could build or tear down. The preorder `enc_id` stream
/// is `depth` node flags, then `Leaf(true)` (`01`), then `depth` `Leaf(false)` right
/// children (`00`) — a normal-form id (every node mixes a `true`/node child with a
/// `false` child, so nothing collapses). Built with a flat loop: no recursion at any
/// depth, in the builder or in `Drop` (the packed form is a flat `BitVec`).
pub(crate) fn deep_left_spine_party(depth: usize) -> Party {
    let mut bits = codec::Bits::with_capacity(3 * depth + 2);
    for _ in 0..depth {
        bits.push(true); // node flag
    }
    bits.push(false); // Leaf
    bits.push(true); //   value 1 (the deep-left owned tip)
    for _ in 0..depth {
        bits.push(false); // Leaf
        bits.push(false); //   value 0 (each node's right child)
    }
    Party::from_bits(bits)
}

/// Smallest spine scale a complexity proptest measures at; below this the step count is
/// too noisy for the ratio to be meaningful. The big input is always `4×` this (see
/// [`assert_linear_scaling`]).
pub(crate) const MIN_SCALE: usize = 64;

/// Steps taken by `f`, measured on a fresh traversal-step counter. The complexity
/// proptests wrap each measured traversal in this and feed the two results to
/// [`assert_linear_scaling`].
pub(crate) fn steps_of(f: impl FnOnce()) -> u64 {
    metrics::reset();
    f();
    metrics::taken()
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

// ───────────────────────── arbitrary normal-form generators (PROG-1) ─────────────────────────
//
// The op-trace generator above only ever produces the tree *shapes operations produce*,
// and only ever produces causally *related* pairs (every member descends from one seed).
// These strategies break that coupling: they build *arbitrary* recursive id and event
// trees with random shape and random base magnitudes — pushed through the oracle's
// normalizing constructors (`Party::node`/`Version::node`), so whatever random shape
// comes out is always valid normal form. Fed to every operation and diffed against the
// oracle, they exercise the `Kind`-arm selection, cost folding, and tie-breaks on shapes
// the op pipeline never reaches, and (crucially) generate genuinely *unrelated* pairs.
//
// Base magnitudes deliberately span small values AND values near/beyond `u64::MAX`: this
// is the natural home for the BUG-1 regression class (path sums that would overflow a
// `u64`). With arbitrary-precision `Base` values the impl threads them losslessly, so the
// large-base differentials must agree with the oracle exactly.

/// Recursion-depth cap for the arbitrary generators. Kept small so the default proptest
/// run stays CI-cheap while still covering every arm; deeper coverage is the job of the
/// (ignored) exhaustive variant and the deep-tree stack-safety test.
const ARB_DEPTH: u32 = 4;

/// Branching budget for the arbitrary generators: the expected interior-node count, which
/// bounds how bushy a generated tree gets.
const ARB_NODES: u32 = 16;

/// An arbitrary event base magnitude. Mixes a dense small range (where collapses and
/// `one_zero` corners live) with values straddling `u64::MAX`, so a generated event tree
/// can have root-to-leaf path sums that would overflow `u64` — the BUG-1 class. The
/// big-value arms are built from `u128`/shifted `BigUint` literals, well beyond `u64`.
pub(crate) fn arb_base() -> impl Strategy<Value = codec::Base> {
    prop_oneof![
        6 => (0u64..6).prop_map(codec::Base::from),
        2 => any::<u64>().prop_map(codec::Base::from),
        1 => (u64::MAX - 4..=u64::MAX).prop_map(codec::Base::from),
        1 => any::<u128>().prop_map(|n| codec::Base::from(n) + codec::Base::from(u64::MAX)),
        1 => (0u32..96).prop_map(|k| (codec::Base::from(1u8) << k) + codec::Base::from(1u8)),
    ]
}

/// An arbitrary normal-form id tree (may be the anonymous `Leaf(false)`). Random recursive
/// shape; every interior node goes through the oracle's normalizing `Party::node`, so the
/// result is always in normal form (no collapsible `(b, b)` node survives).
pub(crate) fn arb_oracle_party() -> impl Strategy<Value = oracle::Party> {
    let leaf = any::<bool>().prop_map(oracle::Party::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| oracle::Party::node(l, r))
    })
}

/// An arbitrary *non-empty* normal-form id tree — a valid standalone [`Party`] (owns at
/// least one region). Filters out the anonymous tree so the impl bridge and ops that
/// require a real share (fork/join) get a meaningful input.
pub(crate) fn arb_oracle_party_nonempty() -> impl Strategy<Value = oracle::Party> {
    arb_oracle_party().prop_filter("non-anonymous id", |p| !p.is_empty())
}

/// An arbitrary normal-form event tree. Random recursive shape with random base
/// magnitudes from [`arb_base`] (including values near/beyond `u64::MAX`); every interior
/// node goes through the oracle's normalizing `Version::node`, so the result is always in
/// normal form (a zero-base child at every node, no collapsible `(n, m, m)`).
pub(crate) fn arb_oracle_version() -> impl Strategy<Value = oracle::Version> {
    let leaf = arb_base().prop_map(oracle::Version::Leaf);
    leaf.prop_recursive(ARB_DEPTH, ARB_NODES, 2, |inner| {
        (arb_base(), inner.clone(), inner).prop_map(|(n, l, r)| oracle::Version::node(n, l, r))
    })
}

// ───────────────── brute-force grow-optimality oracle (PAP-1 / PROG-4) ─────────────────
//
// The paper's event condition (§3 L94-99, §5.3.4) requires `event` register a *minimal*
// inflation: `e < e'` and `e'` dominates no more than needed. `grow` delivers this by a
// dynamic-programming search that, at every branch node, greedily descends the cheaper
// child. Both the recursive oracle and the packed impl realize that *same* DP — so the
// op-trace and PROG-1 differentials (impl == oracle) can only confirm the two agree, never
// that the shared DP is actually optimal. That is this module's job, and it is independent
// of the DP: it enumerates the *entire* feasible single-region inflation space by brute
// force (descending BOTH children at every node, with no pruning), computes each
// candidate's true `(expansions, depth)` cost from first principles, and takes the global
// minimum. If `grow`'s greedy local choice ever disagrees with the global brute-force
// minimum, the DP is wrong — a major finding.
//
// A "single-region inflation" of `(id, e)` is exactly what the paper's `grow` may produce:
// pick one owned leaf-region of `e` (a region the id holds with a `1`), then either
// increment its integer (a free inflation, cost `0` expansions) or, where the id is a node
// but the event is a leaf, expand that leaf into `(n, 0, 0)` (one expansion) and descend.
// The cost is `(expansions, depth)`, lexicographic; ties favor the *right* (root-ward)
// child. This mirrors the paper's recursion structurally, but where `grow` keeps only the
// cheaper child at each node, [`all_inflations`] keeps *every* feasible child, so its
// global minimum is computed over the full search space rather than the pruned one.

/// The inflation cost the paper assigns: `(expansions, depth)`, lexicographic. Matches the
/// oracle's `Cost` and the impl's `grow::Cost`.
pub(crate) type GrowCost = (u32, u32);

/// Every feasible single-region inflation of `(id, e)`, each paired with its true
/// `(expansions, depth)` cost — the full search space `grow` optimizes over, enumerated
/// without pruning. Empty iff the id owns nothing here (an empty region can never be
/// inflated). Trees are raw (un-normalized), exactly as the paper's `grow` builds them;
/// callers normalize before comparing to `event`'s output. Recursive over a bounded test
/// tree (the impl's own traversals are iterative).
pub(crate) fn all_inflations(
    id: &oracle::Party,
    e: &oracle::Version,
) -> Vec<(oracle::Version, GrowCost)> {
    use oracle::Party as P;
    use oracle::Version as V;
    match (id, e) {
        // id full over a leaf: the one free inflation — increment in place.
        (P::Leaf(true), V::Leaf(n)) => vec![(V::Leaf(n + 1u32), (0, 0))],
        // id full over a node: descend either child; the id stays full (`1`) under it.
        (P::Leaf(true), V::Node(n, el, er)) => {
            let mut out = Vec::new();
            for (el2, c) in all_inflations(&P::Leaf(true), el) {
                out.push((
                    V::Node(n.clone(), Box::new(el2), er.clone()),
                    (c.0, c.1 + 1),
                ));
            }
            for (er2, c) in all_inflations(&P::Leaf(true), er) {
                out.push((
                    V::Node(n.clone(), el.clone(), Box::new(er2)),
                    (c.0, c.1 + 1),
                ));
            }
            out
        }
        // empty id: nothing owned here, so no inflation is feasible.
        (P::Leaf(false), _) => Vec::new(),
        // id node over a leaf: expand the leaf into `(n, 0, 0)` (one expansion), descend.
        (P::Node(..), V::Leaf(n)) => {
            let expanded = V::Node(
                n.clone(),
                Box::new(V::Leaf(0u32.into())),
                Box::new(V::Leaf(0u32.into())),
            );
            all_inflations(id, &expanded)
                .into_iter()
                .map(|(e2, c)| (e2, (c.0 + 1, c.1)))
                .collect()
        }
        // id node over an event node: descend either child under the matching id child.
        (P::Node(il, ir), V::Node(n, el, er)) => {
            let mut out = Vec::new();
            for (el2, c) in all_inflations(il, el) {
                out.push((
                    V::Node(n.clone(), Box::new(el2), er.clone()),
                    (c.0, c.1 + 1),
                ));
            }
            for (er2, c) in all_inflations(ir, er) {
                out.push((
                    V::Node(n.clone(), el.clone(), Box::new(er2)),
                    (c.0, c.1 + 1),
                ));
            }
            out
        }
    }
}

/// The globally minimal inflation cost over the full search space, or `None` if the id
/// owns nothing. Independent of `grow`'s DP: a flat minimum over [`all_inflations`].
pub(crate) fn min_inflation_cost(id: &oracle::Party, e: &oracle::Version) -> Option<GrowCost> {
    all_inflations(id, e).into_iter().map(|(_, c)| c).min()
}

/// The single inflation the paper's `grow` must choose: globally cost-minimal, with the
/// root-ward (right-favoring) tie-break applied *locally* at each branch node. Returns the
/// raw (un-normalized) tree and its cost, or `None` if the id owns nothing.
///
/// Independent of `grow`'s greedy DP in the way that matters: each child's weight is its
/// *full-enumeration* minimum cost ([`min_inflation_cost`]), not a value carried up a
/// pruned recursion. So if `grow`'s local pruning ever diverges from the global optimum,
/// `grow`'s output will differ from this. The right-favoring rule is the paper's: descend
/// left iff the left child's minimum is strictly cheaper than the right's (`cl < cr`),
/// else descend right.
pub(crate) fn best_inflation(
    id: &oracle::Party,
    e: &oracle::Version,
) -> Option<(oracle::Version, GrowCost)> {
    use oracle::Party as P;
    use oracle::Version as V;
    match (id, e) {
        (P::Leaf(false), _) => None,
        (P::Leaf(true), V::Leaf(n)) => Some((V::Leaf(n + 1u32), (0, 0))),
        (P::Node(..), V::Leaf(n)) => {
            let expanded = V::Node(
                n.clone(),
                Box::new(V::Leaf(0u32.into())),
                Box::new(V::Leaf(0u32.into())),
            );
            best_inflation(id, &expanded).map(|(e2, c)| (e2, (c.0 + 1, c.1)))
        }
        // Both node cases share the right-favoring child selection; only the id children
        // differ (`(1, 1)` for a full id over a node, `(il, ir)` for an id node).
        (P::Leaf(true) | P::Node(..), V::Node(n, el, er)) => {
            let (idl, idr): (&P, &P) = match id {
                P::Node(il, ir) => (il, ir),
                _ => (&P::Leaf(true), &P::Leaf(true)),
            };
            let cl = min_inflation_cost(idl, el);
            let cr = min_inflation_cost(idr, er);
            // Descend left only when it is strictly cheaper and feasible; the root-ward
            // tie-break (and any infeasible left) sends us right.
            let go_left = match (cl, cr) {
                (Some(cl), Some(cr)) => cl < cr,
                (Some(_), None) => true,
                (None, _) => false,
            };
            if go_left {
                let (el2, c) = best_inflation(idl, el)?;
                Some((
                    V::Node(n.clone(), Box::new(el2), er.clone()),
                    (c.0, c.1 + 1),
                ))
            } else {
                let (er2, c) = best_inflation(idr, er)?;
                Some((
                    V::Node(n.clone(), el.clone(), Box::new(er2)),
                    (c.0, c.1 + 1),
                ))
            }
        }
    }
}
