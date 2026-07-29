//! Version tests.
//!
//! The causal order and its comparison matrix, the join/meet operator
//! matrices and lattice laws, complexity (linear-scaling) checks, grow
//! optimality against the brute-force reference, `min_ticks`, and
//! projection (`/`).

use crate::meter::registry::Shape;
use std::cmp::Ordering;

use proptest::prelude::*;

use super::{skyline, Ranked, Version};
use crate::testing::bridge::{from_oracle_party, from_oracle_version, to_oracle_version};
use crate::testing::complexity::{assert_linear_scaling, steps_of, MIN_SCALE};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, arb_shape, bushy_expand_party, shape_party,
    shape_version,
};
use crate::testing::grow_brute_force::{all_inflations, best_inflation};
use crate::testing::optrace::{leq as oracle_leq, run, step_impl, versions, world_strategy, Op};
use crate::{Clock, Party, Ticks};

/// `a <= b` under the impl causal order.
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

// ───────────────────────────── causal order ─────────────────────────────

proptest! {
    /// Complexity. The causal order is `O(n + m)`.
    ///
    /// Comparing `a` against `b = a | extra` drives the bounded lazy-skip at
    /// scale. `a <= b` always holds (so the walk traverses fully, no early
    /// `false`), and where `extra` added structure that `a` lacks, `a`'s leaf
    /// aligns with `b`'s subtree, so `b`'s dominated subtree is skipped once
    /// under that leaf. Building `a` and `extra` from independent shapes
    /// maximizes such misalignments. Steps stay linear from `scale` to
    /// `4 * scale`.
    #[test]
    fn leq_is_linear(
        shape_a in arb_shape(),
        shape_b in arb_shape(),
        scale in MIN_SCALE..256,
    ) {
        let measure = |s: usize| {
            let a = shape_version(shape_a, s);
            let extra = shape_version(shape_b, s);
            let b = a.clone() | extra; // a <= b always; b has subtrees where a has leaves
            steps_of(|| {
                let _ = a.partial_cmp(&b);
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Differential. The impl causal order agrees with the oracle's on every
    /// generated pair; this subsumes the order laws since the oracle satisfies
    /// them (its `version_partial_order` property) and the impl matches it
    /// exactly.
    #[test]
    fn compare_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (oa, ob) = (&vs[i % n], &vs[j % n]);
        let (ia, ib) = (from_oracle_version(oa), from_oracle_version(ob));
        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(ob));
    }
}

proptest! {
    /// The order laws on impl versions directly: reflexive, antisymmetric,
    /// transitive; `==` ⇔ `Some(Equal)`; concurrency ⇔ `None`.
    #[test]
    fn order_laws(ops in world_strategy(), i in 0usize..64, j in 0usize..64, k in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let (a, b, c) = (
            from_oracle_version(&vs[i % n]),
            from_oracle_version(&vs[j % n]),
            from_oracle_version(&vs[k % n]),
        );

        prop_assert_eq!(a.partial_cmp(&a), Some(Ordering::Equal)); // reflexive
        if le(&a, &b) && le(&b, &a) {
            prop_assert!(a == b); // antisymmetric
        }
        if le(&a, &b) && le(&b, &c) {
            prop_assert!(le(&a, &c)); // transitive
        }
        prop_assert_eq!(a == b, a.partial_cmp(&b) == Some(Ordering::Equal));
        let concurrent = !le(&a, &b) && !le(&b, &a);
        prop_assert_eq!(concurrent, a.partial_cmp(&b).is_none());
    }
}

/// Assert one comparison-matrix cell agrees with `expected`.
///
/// Checks its `partial_cmp` (`PartialOrd`) and `==`/`!=` (`PartialEq`), plus
/// the four ordering operators `partial_cmp` derives. Generic over the operand
/// types, so each call resolves to exactly the impl for `(L, R)` —
/// `assert_cmp_cell(&a, b, ..)` exercises the `&Lhs`/`Rhs` cell,
/// `assert_cmp_cell(a, &b, ..)` the `Lhs`/`&Rhs` cell, `&`/`&` the std blanket
/// — with no method-resolution ambiguity to mask which cell ran. A cell wired
/// into a delegation cycle overflows the stack here rather than diverging
/// silently in production.
fn assert_cmp_cell<L, R>(lhs: L, rhs: R, expected: Option<Ordering>) -> Result<(), TestCaseError>
where
    L: PartialEq<R> + PartialOrd<R>,
{
    prop_assert_eq!(lhs.partial_cmp(&rhs), expected);
    prop_assert_eq!(lhs == rhs, expected == Some(Ordering::Equal));
    prop_assert_eq!(lhs != rhs, expected != Some(Ordering::Equal));
    prop_assert_eq!(lhs < rhs, expected == Some(Ordering::Less));
    prop_assert_eq!(lhs > rhs, expected == Some(Ordering::Greater));
    prop_assert_eq!(
        lhs <= rhs,
        matches!(expected, Some(Ordering::Less | Ordering::Equal))
    );
    prop_assert_eq!(
        lhs >= rhs,
        matches!(expected, Some(Ordering::Greater | Ordering::Equal))
    );
    Ok(())
}

proptest! {
    /// The full comparison matrix over owned and borrowed `Version`
    /// operands agrees with the oracle's verdict on the same pair.
    ///
    /// Every owned and borrowed form of each operand, covering all six
    /// generated `PartialEq`/`PartialOrd` impls plus the `&Lhs`/`&Rhs` std
    /// blanket forms. Pinning every cell to one source of truth is the "the
    /// cells can't drift out of sync" guarantee; invoking every cell is the "no
    /// cell recurses forever" guarantee.
    #[test]
    fn compare_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = vs[i % n].partial_cmp(&vs[j % n]); // oracle: the source of truth
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version × Version (owned/owned, &/owned, owned/&, &/& blanket).
        assert_cmp_cell(a.clone(), b.clone(), expected)?;
        assert_cmp_cell(&a, b.clone(), expected)?;
        assert_cmp_cell(a.clone(), &b, expected)?;
        assert_cmp_cell(&a, &b, expected)?;
    }
}

// ───────────────────────────── event mutation ─────────────────────────────

/// `Version::new()` is the empty history and the two-sided identity for `|`.
#[test]
fn new_is_join_identity() {
    use crate::oracle::Version as V;
    let empty = Version::new();
    assert!(empty == from_oracle_version(&V::leaf(0u64))); // empty history is Leaf(0)
    assert!(Version::default() == empty); // Default delegates to new()
    for v in [
        V::leaf(0u64),
        V::leaf(7u64),
        V::node(1u64, V::leaf(0u64), V::leaf(2u64)),
    ] {
        let iv = from_oracle_version(&v);
        assert!(empty.clone() | iv.clone() == iv);
        assert!(iv.clone() | empty.clone() == iv);
    }
}

proptest! {
    /// The impl `tick` matches the oracle's `event` for every
    /// clock's own `(party, version)` (the party owns the regions tick may inflate).
    #[test]
    fn tick_matches_oracle(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let (party, version) = cs[i % n].trees();

        let mut oracle_after = version.clone();
        oracle_after.tick(party);

        let mut iv = from_oracle_version(version);
        iv.tick(&from_oracle_party(party));

        prop_assert!(iv == from_oracle_version(&oracle_after));
    }

    /// The impl's fused `ticks(n)` matches the oracle's literally iterated
    /// `ticks(n)` for every clock's own `(party, version)`.
    ///
    /// The counts stay within what the oracle's `O(n · tree)` loop
    /// affords (the oracle module doc's operating envelope caps `n`
    /// here; the wide counts ride the composition law and the
    /// closed-form witnesses).
    #[test]
    fn ticks_matches_oracle(ops in world_strategy(), i in 0usize..64, n in 0u64..24) {
        let cs = run(&ops);
        let len = cs.len();
        let (party, version) = cs[i % len].trees();

        let mut oracle_after = version.clone();
        oracle_after.ticks(party, n);

        let mut iv = from_oracle_version(version);
        iv.ticks(&from_oracle_party(party), n);

        prop_assert!(iv == from_oracle_version(&oracle_after));
    }
}

proptest! {
    /// Differential. The impl version join (`|`) matches the oracle's `join`.
    #[test]
    fn merge_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let oracle_join = vs[i % n].clone() | vs[j % n].clone();
        let merged = from_oracle_version(&vs[i % n]) | from_oracle_version(&vs[j % n]);
        prop_assert!(merged == from_oracle_version(&oracle_join));
    }
}

proptest! {
    /// The join is representationally subadditive:
    /// `encode(a | b).len() <= encode(a).len() + encode(b).len()`.
    ///
    /// The join's event tree branches only where an input branches and its
    /// bases come from pointwise combination, so joining can restructure but
    /// never invent structure beyond both inputs together. Callers that
    /// track version-size maxima rely on this to charge a join of two
    /// bounded versions the sum of their bounds. Probed here over churned
    /// (fork/send/sync/retire) populations — the causally related pairs live
    /// replicas hold, including the normalization corners churn produces.
    #[test]
    fn join_encoding_is_subadditive(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let join = a.clone() | b.clone();
        prop_assert!(
            join.encode().len() <= a.encode().len() + b.encode().len(),
            "join encoding outgrew its inputs: {} > {} + {}",
            join.encode().len(), a.encode().len(), b.encode().len(),
        );
    }
}

proptest! {
    /// The meet is representationally subadditive:
    /// `encode(a & b).len() <= encode(a).len() + encode(b).len()`.
    ///
    /// Dual to [`join_encoding_is_subadditive`]: the meet's event tree
    /// branches only where an input branches and its bases come from
    /// pointwise combination, so meeting can restructure but never invent
    /// structure beyond both inputs together. Callers that track
    /// version-size maxima rely on this to charge a meet of two bounded
    /// versions (an assembled floor) the sum of their bounds. Probed over
    /// the same churned populations as the join lemma.
    #[test]
    fn meet_encoding_is_subadditive(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let meet = a.clone() & b.clone();
        prop_assert!(
            meet.encode().len() <= a.encode().len() + b.encode().len(),
            "meet encoding outgrew its inputs: {} > {} + {}",
            meet.encode().len(), a.encode().len(), b.encode().len(),
        );
    }
}

proptest! {
    /// Differential. The impl version meet (`&`) matches the oracle's `meet`,
    /// dual to [`merge_matches_oracle`].
    #[test]
    fn meet_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let oracle_meet = vs[i % n].clone() & vs[j % n].clone();
        let met = from_oracle_version(&vs[i % n]) & from_oracle_version(&vs[j % n]);
        prop_assert!(met == from_oracle_version(&oracle_meet));
    }
}

proptest! {
    /// Differential. The impl projection's materialization
    /// (`(&v / &p).to_version()`) matches the oracle's projection (mask `v`
    /// to `p`'s region), over a shared population: arbitrary versions and
    /// parties drawn from the clocks.
    #[test]
    fn div_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let oracle_proj = vs[i % n].clone() / cs[j % n].party();
        let v = from_oracle_version(&vs[i % n]);
        let p = from_oracle_party(cs[j % n].party());
        let proj = (&v / &p).to_version();
        prop_assert!(proj == from_oracle_version(&oracle_proj));
    }
}

proptest! {
    /// Differential. The impl's `min_ticks` matches the oracle's base sum on
    /// every generated version.
    #[test]
    fn min_ticks_matches_oracle(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        prop_assert_eq!(from_oracle_version(&vs[i % n]).min_ticks(), vs[i % n].min_ticks());
    }
}

proptest! {
    /// Every assigning join surface on `Version` yields the same result
    /// as `a | b`, which `merge_matches_oracle` already pins to the oracle's
    /// `join`.
    ///
    /// Covers `Version |= Version` and `Version |= &Version` — neither of
    /// which the by-value `|` differential reaches.
    #[test]
    fn version_assign_join_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // `Version |= Version`.
        let mut assign = a.clone();
        assign |= b.clone();
        prop_assert!(assign == expected);

        // `Version |= &Version`.
        let mut assign_ref = a.clone();
        assign_ref |= &b;
        prop_assert!(assign_ref == expected);
    }
}

proptest! {
    /// The full `|` (BitOr) matrix over owned and borrowed `Version`
    /// operands equals the oracle's `join`.
    ///
    /// `merge_matches_oracle` already pins the bare owned/owned case; each
    /// of the four reference cells must agree with it.
    #[test]
    fn join_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version × Version (four reference forms).
        prop_assert!(a.clone() | b.clone() == expected);
        prop_assert!(&a | b.clone() == expected);
        prop_assert!(a.clone() | &b == expected);
        prop_assert!(&a | &b == expected);
    }
}

proptest! {
    /// The full `|=` (BitOrAssign) matrix — owned and borrowed right
    /// operands — lands on the oracle's `join`.
    #[test]
    fn join_assign_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() | vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version |= Version / &Version.
        { let mut x = a.clone(); x |= b.clone(); prop_assert!(x == expected); }
        { let mut x = a.clone(); x |= &b; prop_assert!(x == expected); }
    }
}

proptest! {
    /// The full `&` (BitAnd) matrix over owned and borrowed `Version`
    /// operands equals the oracle's `meet`, dual to
    /// [`join_matrix_matches_oracle`].
    ///
    /// `meet_matches_oracle` pins the bare owned/owned cell; each of the
    /// four reference cells must agree with it.
    #[test]
    fn meet_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() & vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version × Version (four reference forms).
        prop_assert!(a.clone() & b.clone() == expected);
        prop_assert!(&a & b.clone() == expected);
        prop_assert!(a.clone() & &b == expected);
        prop_assert!(&a & &b == expected);
    }
}

proptest! {
    /// The full `&=` (BitAndAssign) matrix — owned and borrowed right
    /// operands — lands on the oracle's `meet`, dual to
    /// [`join_assign_matrix_matches_oracle`].
    #[test]
    fn meet_assign_matrix_matches_oracle(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let expected = from_oracle_version(&(vs[i % n].clone() & vs[j % n].clone()));
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);

        // Version &= Version / &Version.
        { let mut x = a.clone(); x &= b.clone(); prop_assert!(x == expected); }
        { let mut x = a.clone(); x &= &b; prop_assert!(x == expected); }
    }
}

proptest! {
    /// Lattice identity for join, byte-identical: `0 | v == v == v | 0`.
    ///
    /// The encoded bytes equal `v`'s own — both through the `join_view`
    /// short-circuit (empty on either side) and through the general merge
    /// kernel called directly (`skyline::emit::join`), which the
    /// short-circuit must match bit for bit.
    #[test]
    fn join_identity_byte_parity(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let v = from_oracle_version(&vs[i % n]);
        let empty = Version::new();

        // The general path, bypassing the short-circuit: the merge kernel on
        // the identity cases lands on `v`'s canonical bytes.
        let general_left =
            Version::from_bits(skyline::emit::join(empty.as_bits(), v.as_bits()));
        let general_right =
            Version::from_bits(skyline::emit::join(v.as_bits(), empty.as_bits()));
        prop_assert_eq!(general_left.encode(), v.encode());
        prop_assert_eq!(general_right.encode(), v.encode());

        // The empty version on either side of the operator (the short-circuit).
        prop_assert_eq!((&empty | &v).encode(), v.encode());
        prop_assert_eq!((&v | &empty).encode(), v.encode());
    }
}

proptest! {
    /// The empty version absorbs the meet, byte-identical: `0 & v == 0 == v & 0`.
    ///
    /// The encoded bytes equal `Version::new()`'s — both through the
    /// `meet_view` short-circuit (empty on either side) and through the
    /// general merge kernel called directly (`skyline::emit::meet`), which
    /// the short-circuit must match bit for bit. Dual to
    /// [`join_identity_byte_parity`].
    #[test]
    fn meet_absorbing_byte_parity(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let v = from_oracle_version(&vs[i % n]);
        let empty = Version::new();

        // The general path, bypassing the short-circuit: the merge kernel on
        // the absorbing cases lands on the canonical empty bytes.
        let general_left =
            Version::from_bits(skyline::emit::meet(empty.as_bits(), v.as_bits()));
        let general_right =
            Version::from_bits(skyline::emit::meet(v.as_bits(), empty.as_bits()));
        prop_assert_eq!(general_left.encode(), empty.encode());
        prop_assert_eq!(general_right.encode(), empty.encode());

        // The empty version on either side of the operator (the short-circuit).
        prop_assert_eq!((&empty & &v).encode(), empty.encode());
        prop_assert_eq!((&v & &empty).encode(), empty.encode());
    }
}

// The lattice, order, tick, and projection laws on impl values live in
// `crate::laws` and are driven by the algebraic-laws suite over both
// arbitrary normal forms and these same op-trace populations; this file
// keeps the differential and mechanism-level tests.

// ───────────────────────── complexity (linear scaling) ─────────────────────────

proptest! {
    /// Complexity. `tick` is `O(n + m)`: ticking a deep event tree against a deep id of
    /// the same shape (walking both at once) grows linearly with size.
    #[test]
    fn tick_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let mut v = shape_version(shape, s);
            let p = shape_party(shape, s);
            steps_of(|| {
                v.tick(&p);
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. The inflation's multi-region cost fold is `O(n + m)`.
    ///
    /// Ticking the empty history (`Leaf(0)`) against a deep *bushy* id takes
    /// the grow branch (`fill` is a no-op: the id is a node over an event
    /// leaf), and the bushy subtree's many owned regions at varying depths
    /// make the walk's route fold genuinely weigh two feasible children at
    /// each branch (`cl < cr` with neither a `COST_MAX` loser). The id roots
    /// the bushy subtree beside one owned terminal
    /// ([`bushy_expand_party`]), pinning the cheapest inflation — hence the
    /// splice's one skip of the whole off-path bushy subtree — to the same
    /// route at every scale: the splice walks only the chosen path, so a
    /// scale-dependent route would swing a two-point step ratio by up to
    /// the input's own size. Steps stay linear from `scale` to `4 * scale`.
    #[test]
    fn grow_bushy_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let p = bushy_expand_party(s);
            let mut v = Version::new(); // Leaf(0): fill is a no-op, so grow runs
            steps_of(|| {
                v.tick(&p);
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `merge` (`|`) is `O(n + m)`: joining two deep event trees of the same
    /// shape stays linear.
    #[test]
    fn merge_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let a = shape_version(shape, s);
            steps_of(|| {
                let _ = a.clone() | a.clone();
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `meet` (`&`) is `O(n + m)`.
    ///
    /// Meeting two deep event trees of the same shape stays linear, dual to
    /// [`merge_is_linear`]. The operands are independent shapes so the walk
    /// genuinely descends both sides (`a & a` would short-circuit on
    /// `trivially_eq`).
    #[test]
    fn meet_is_linear(shape_a in arb_shape(), shape_b in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let a = shape_version(shape_a, s);
            let b = shape_version(shape_b, s);
            steps_of(|| {
                let _ = a.clone() & b.clone();
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

// ───────────────────────────── path-sum overflow regression ─────────────────────────────

/// A normal-form tree whose root-to-leaf path sum exceeds `u64::MAX` compares
/// correctly.
///
/// With arbitrary-precision leaf heights there is no overflow class, so the
/// answer is `Greater` in every build profile (no debug panic, no release
/// wrap that would invert the causal order). `decode`/`try_from` admit such
/// trees, so the comparison must thread the heights at full precision.
#[test]
fn path_sum_beyond_u64_compares_greater() {
    let big = 1u64 << 63;
    // Normal form: the outer min(big, 0) child is the right `0` leaf; the inner node's
    // min(0, 1) child is its left `0` leaf. The left half's true value is
    // big + big + 1 = 2^64 + 1, past `u64::MAX`.
    let a = Version::try_from((big, (big, 0u64, 1u64), 0u64)).unwrap();
    let b = Version::try_from(big).unwrap(); // constant 2^63
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater));
}

/// A stored leaf height above `u64::MAX` stays exact across mutation and
/// merge. This pins the arbitrary-width payload path at the machine-word
/// spill boundary, not only path sums made from individually-small nodes.
#[test]
fn stored_base_beyond_u64_ticks_and_merges() {
    let big: Version = "18446744073709551616".parse().unwrap();
    let mut ticked = big.clone();
    ticked.tick(&Party::seed());

    assert_eq!(ticked.to_string(), "18446744073709551617");
    assert_eq!(big.clone() | ticked.clone(), ticked);
    assert_eq!(Version::decode(&ticked.encode()[..]).unwrap(), ticked);
}

// ───────────── arbitrary normal-form trees (decoupled from the op pipeline) ─────────────
//
// The op-trace differentials above only ever compare causally *related*
// versions (every member descends from one seed) on the *shapes operations
// produce*. These feed *arbitrary* normal-form event trees — random shape,
// random base magnitudes including values near/beyond `u64::MAX` — to every
// event op and diff structurally against the oracle. They are the natural home
// for the large-base (path-sum-overflow) regression class.

proptest! {
    /// `partial_cmp` on arbitrary, typically *unrelated* event-tree pairs
    /// agrees with the oracle.
    ///
    /// Including the concurrent (`None`) verdict the op pipeline rarely
    /// produces, and large-base pairs whose root-to-leaf path sums exceed
    /// `u64::MAX`: with arbitrary-precision bases the answer must still match.
    #[test]
    fn causal_cmp_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let (ia, ib) = (from_oracle_version(&oa), from_oracle_version(&ob));
        prop_assert_eq!(ia.partial_cmp(&ib), oa.partial_cmp(&ob));
        // Symmetry of the verdict under swap, on the impl directly.
        prop_assert_eq!(
            ib.partial_cmp(&ia),
            ia.partial_cmp(&ib).map(Ordering::reverse)
        );
    }
}

proptest! {
    /// `==` agrees with the full causal-compare walk.
    ///
    /// The equality cells decide by a byte compare of the two stored streams
    /// (canonical unique representation: byte equality ⟺ equality); this pins
    /// that shortcut to the comparison sweep's verdict on arbitrary,
    /// typically *unequal* pairs (the inequality direction the shortcut
    /// decides without walking) and on equal pairs (the equality direction).
    #[test]
    fn eq_matches_causal_walk(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        // The walk's verdict, taken from the comparison sweep directly.
        let walk_eq =
            skyline::sweep::causal_cmp(a.as_bits(), b.as_bits()) == Some(Ordering::Equal);

        prop_assert_eq!(a == b, walk_eq);
        // The equality direction: a version equals its own clone.
        prop_assert!(a == a.clone());
    }
}

proptest! {
    /// The join-size lemma of [`join_encoding_is_subadditive`], on
    /// arbitrary, typically *unrelated* normal-form pairs.
    ///
    /// The churned generator only produces causally related versions from
    /// one seed; these pairs add independent shapes and large-base leaves
    /// (values near/beyond `u64::MAX`), where a join must restructure most —
    /// the corner where subadditivity would break if normalization could
    /// ever inflate a combined tree past its inputs.
    #[test]
    fn join_encoding_is_subadditive_arbitrary(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let join = a.clone() | b.clone();
        prop_assert!(
            join.encode().len() <= a.encode().len() + b.encode().len(),
            "join encoding outgrew its inputs: {} > {} + {}",
            join.encode().len(), a.encode().len(), b.encode().len(),
        );
    }
}

proptest! {
    /// The meet-size lemma of [`meet_encoding_is_subadditive`], on
    /// arbitrary, typically *unrelated* normal-form pairs.
    ///
    /// Dual to [`join_encoding_is_subadditive_arbitrary`], and for the same
    /// reason: independent shapes and large-base leaves are the corner
    /// where a meet must restructure most, so this is where subadditivity
    /// would break if normalization could ever inflate a combined tree
    /// past its inputs.
    #[test]
    fn meet_encoding_is_subadditive_arbitrary(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let meet = a.clone() & b.clone();
        prop_assert!(
            meet.encode().len() <= a.encode().len() + b.encode().len(),
            "meet encoding outgrew its inputs: {} > {} + {}",
            meet.encode().len(), a.encode().len(), b.encode().len(),
        );
    }
}

proptest! {
    /// `is_empty` ⟺ `v == Version::new()`, over arbitrary normal-form trees.
    ///
    /// Pins the O(1) two-bit emptiness test to the definitional comparison
    /// `v == Version::new()`; `arb_oracle_version` generates the empty leaf
    /// too, so both arms are exercised.
    #[test]
    fn is_empty_iff_new(ov in arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        prop_assert_eq!(v.is_empty(), v == Version::new());
    }
}

proptest! {
    /// `|` (merge / LUB) on arbitrary unrelated event trees agrees with the
    /// oracle's `join`, structurally. Exercises the join's arm selection on
    /// shapes the op pipeline never builds, with large bases threaded
    /// losslessly.
    #[test]
    fn merge_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let merged = from_oracle_version(&oa) | from_oracle_version(&ob);
        let oracle_join = oa.clone() | ob.clone();
        prop_assert!(merged == from_oracle_version(&oracle_join));
        // The result is a normal-form tree that lowers back to the same oracle value.
        prop_assert_eq!(to_oracle_version(&merged), oracle_join);
    }
}

proptest! {
    /// `&` (meet / GLB) on arbitrary unrelated event trees agrees with the
    /// oracle's `meet`, structurally — dual to [`merge_arbitrary`].
    ///
    /// Exercises the meet's arm selection and `close_node` sink/collapse on
    /// shapes the op pipeline never builds, with large bases threaded
    /// losslessly.
    #[test]
    fn meet_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let met = from_oracle_version(&oa) & from_oracle_version(&ob);
        let oracle_meet = oa.clone() & ob.clone();
        prop_assert!(met == from_oracle_version(&oracle_meet));
        // The result is a normal-form tree that lowers back to the same oracle value.
        prop_assert_eq!(to_oracle_version(&met), oracle_meet);
    }
}

proptest! {
    /// `tick` (= `fill` then, if no fill, `grow`) on an arbitrary `(id, event)`
    /// pair with *unrelated* shapes matches the oracle's `event`.
    ///
    /// This is where the `Kind` arm selection, the cost folding, and the
    /// root-ward tie-break live; feeding unrelated id/event shapes drives the
    /// `fill` full-subtree arms and the multi-region `grow` cost comparison
    /// that same-clock `(party, version)` pairs under-hit.
    #[test]
    fn tick_arbitrary(
        op in arb_oracle_party_nonempty(),
        ov in arb_oracle_version(),
    ) {
        let mut oracle_after = ov.clone();
        oracle_after.tick(&op);

        let mut iv = from_oracle_version(&ov);
        iv.tick(&from_oracle_party(&op));

        prop_assert!(iv == from_oracle_version(&oracle_after));
    }
}

// ───────────── grow optimality, impl side ─────────────
//
// The defining causality property (§3, §5.3.4): an event registers a *minimal*
// inflation. The oracle's `grow` is pinned to a brute-force search over the
// entire feasible inflation space in `oracle::tests`; these hold the packed
// impl to the same standard. `tick = fill else grow`, so when `fill` already
// simplifies the tree the grow path is not taken — `grow_matches_brute_force`
// filters to the grow case (fill a no-op) and asserts the impl's inflation
// equals the brute-force right-favoring minimum; `grow_minimal` checks the
// paper's metamorphic condition on every `tick`.

proptest! {
    /// When `tick` takes the `grow` branch (`fill` leaves the tree unchanged),
    /// the impl inflates exactly the brute-force cost-minimal, right-favoring
    /// region: `tick` lowered to the oracle equals `best_inflation` normalized.
    ///
    /// This holds the packed `grow`'s dynamic program to the full-enumeration
    /// global optimum directly — not merely to the recursive oracle (which
    /// realizes the same DP). Large bases are threaded losslessly, so the cost
    /// comparison is exact regardless of magnitude.
    #[test]
    fn grow_matches_brute_force(
        op in arb_oracle_party_nonempty(),
        ov in arb_oracle_version(),
    ) {
        // Only the grow path is under test: skip inputs where `fill` already
        // simplifies (those are covered by the tick/fill differentials). `fill`
        // is a no-op iff it returns the input unchanged. About a quarter of
        // arbitrary inputs reach grow, comfortably within proptest's reject
        // budget.
        prop_assume!(ov.fill_for_test(&op) == ov);

        let (best_tree, _cost) = best_inflation(&op, &ov).expect("non-empty id inflates");
        let expected = best_tree.normalized_for_test();

        let mut iv = from_oracle_version(&ov);
        iv.tick(&from_oracle_party(&op));

        prop_assert_eq!(to_oracle_version(&iv), expected);
    }
}

proptest! {
    /// §3 (the event condition), metamorphic form, on the impl.
    ///
    /// When `tick` takes the `grow` branch, the inflated `e'` "dominates no
    /// more than needed": no feasible single-region inflation candidate `x` of
    /// `(id, e)` satisfies `e ≤ x < e'`. This is the correctly scoped reading
    /// of the paper's `x < e' ⇒ x ≤ e` (the literal form over the dense
    /// pointwise lattice is false even for a single increment — see the oracle
    /// twin `grow_dominates_no_more_than_needed`). Run on the impl's own causal
    /// order, with the candidate set enumerated by the brute-force oracle.
    /// Cross-checked against the oracle order on the same values.
    #[test]
    fn grow_minimal(
        op in arb_oracle_party_nonempty(),
        ov in arb_oracle_version(),
    ) {
        prop_assume!(ov.fill_for_test(&op) == ov);

        let e = from_oracle_version(&ov);
        let mut eprime = e.clone();
        eprime.tick(&from_oracle_party(&op)); // grow path: tick == grow

        for (cand, _) in all_inflations(&op, &ov) {
            let cand_norm = cand.normalized_for_test();
            let cand_v = from_oracle_version(&cand_norm);
            let above_e = le(&e, &cand_v);
            let strictly_below = cand_v.partial_cmp(&eprime) == Some(Ordering::Less);
            prop_assert!(
                !(above_e && strictly_below),
                "an inflation candidate sits strictly between e and e' on the impl",
            );
            // The impl and oracle agree on `e ≤ cand` for each candidate.
            prop_assert_eq!(above_e, oracle_leq(&ov, &cand_norm));
        }
    }
}

proptest! {
    /// `decode ∘ encode == identity` over arbitrary normal-form event trees,
    /// including large-base ones.
    ///
    /// The widened Elias-gamma code round-trips every magnitude a leaf can
    /// hold, and the decoded value lowers to the same oracle tree.
    #[test]
    fn decode_encode_arbitrary(ov in arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        let bytes = v.encode();
        let decoded = Version::decode(&bytes[..]).expect("canonical encoding decodes");
        prop_assert!(decoded == v);
        prop_assert_eq!(to_oracle_version(&decoded), ov);
    }
}

proptest! {
    /// `as_bytes` returns exactly the canonical `encode` bytes.
    ///
    /// The stored form keeps its final partial byte zero-padded, so the raw
    /// storage slice is byte-identical to the packed encoding. Exercises the
    /// literal/`extend` construction path over arbitrary normal-form trees.
    #[test]
    fn as_bytes_matches_encode(ov in arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        let encoded = v.encode();
        prop_assert_eq!(v.as_bytes(), encoded.as_slice());
    }

    /// The invariant survives mutation too: ticking re-emits the stored
    /// stream through the fill splice, which must also leave a zero-padded
    /// tail.
    #[test]
    fn as_bytes_matches_encode_after_ticks(n in 0u32..256) {
        let party = Party::seed();
        let mut v = Version::new();
        for _ in 0..n {
            v.tick(&party);
        }
        let encoded = v.encode();
        prop_assert_eq!(v.as_bytes(), encoded.as_slice());
    }
}

// ─────────────────────────────── min_ticks ───────────────────────────────

/// The number of `tick`s a trace performs against the impl population, derived
/// straight from the op list.
///
/// `Tick` advances once; `Send` advances twice (the sender `tick`s, the
/// receiver `recv`s = join-then-`tick`); `Fork`, `Sync`, and `Join` never
/// `tick`. Each `Tick`/`Send` always executes fully (no index guard can skip
/// it), so this count is exact — it mirrors `step_impl`.
fn trace_ticks(ops: &[Op]) -> u64 {
    ops.iter()
        .map(|op| match op {
            Op::Tick(_) => 1,
            Op::Ticks(_, k) => u64::from(*k),
            Op::Send(..) => 2,
            Op::Fork(_) | Op::Sync(..) | Op::Join(..) => 0,
        })
        .sum()
}

/// `min_ticks` known values: the empty version, a single-party line (= the leaf
/// value), and two concurrent peaks (forced above their tallest path of `1`).
#[test]
fn min_ticks_known_values() {
    assert_eq!(Version::new().min_ticks(), Ticks::ZERO);
    assert_eq!(Version::try_from(5).unwrap().min_ticks(), Ticks::from(5u64));
    let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    assert_eq!(peaks.min_ticks(), Ticks::from(2u64));
}

proptest! {
    /// `min_ticks` is a true floor: for *every* live clock in *any* causal
    /// history of fork/tick/send/sync/join, its version's `min_ticks` never
    /// exceeds the ticks actually performed.
    ///
    /// Cross-checks the fold itself against the recursive oracle's
    /// sum-of-bases (`oracle::Version::min_ticks`); the function-space
    /// leg (`min_ticks_realizes_base_sum`) supplies the independent
    /// second computation.
    #[test]
    fn min_ticks_floors_every_history(ops in world_strategy()) {
        let total = trace_ticks(&ops);
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        for c in &imp {
            let v = c.version();
            // The fold computes exactly the sum-of-bases.
            prop_assert_eq!(v.min_ticks(), to_oracle_version(v).min_ticks());
            // And that minimum never exceeds the ticks the history performed.
            prop_assert!(
                v.min_ticks() <= Ticks::from(total),
                "min_ticks {} exceeded the {} ticks performed",
                v.min_ticks(),
                total,
            );
        }
    }
}

/// There is no *maximum* tick count: leaf `1` can be built by arbitrarily many
/// ticks — `n` disjoint forks each ticking once, then all joined — yet
/// `min_ticks` stays `1`.
///
/// This witnesses the unboundedness of the dual quantity while pinning the
/// floor.
#[test]
fn no_maximum_tick_count() {
    for n in 1usize..=16 {
        // Fork a seed into `n` disjoint clocks tiling the whole id space.
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < n {
            let i = clocks.len() - 1;
            let child = clocks[i].fork();
            clocks.push(child);
        }
        // Each ticks exactly once: `n` ticks in total.
        for c in &mut clocks {
            c.tick();
        }
        // Join them all back into one. Joins move no events.
        let mut whole = clocks.remove(0);
        for c in clocks {
            whole.join(c).expect("seed-derived parties are disjoint");
        }
        let v = whole.version();
        assert_eq!(
            v,
            &Version::try_from(1).unwrap(),
            "n={n}: rejoins to leaf 1"
        );
        assert_eq!(
            v.min_ticks(),
            Ticks::from(1u64),
            "n={n}: {n} ticks collapse to the floor 1"
        );
    }
}

// ─────────────────────────────── rank ───────────────────────────────

/// `rank` known values.
///
/// The empty version is zero; a leaf is its integer base; the pair `min_ticks`
/// cannot separate — `(0, 1, 0) < 1`, both one tick — gets strictly ordered
/// ranks; and two *concurrent* versions may share a rank (the two-peak tree
/// also covers half the interval), which is exactly what the
/// strict-monotonicity contract permits.
#[test]
fn rank_known_values() {
    assert_eq!(Version::new().rank().to_string(), "0");
    assert_eq!(Version::try_from(5).unwrap().rank().to_string(), "5");

    let half: Version = "(0, 1, 0)".parse().unwrap();
    let one = Version::try_from(1).unwrap();
    assert!(half < one, "strict containment in the causal order");
    assert!(half.rank() < one.rank(), "so strictly smaller rank");
    assert_eq!(half.min_ticks(), one.min_ticks(), "the floor ties them");
    assert_eq!(half.rank().to_string(), "1/2^1");

    let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    assert!(half.concurrent(&peaks), "different halves of the interval");
    assert_eq!(
        half.rank(),
        peaks.rank(),
        "equal rank is fine when concurrent"
    );
}

proptest! {
    /// Differential. The impl's cursor-threaded `rank` fold matches the
    /// recursive oracle's area fold (`oracle::Version::rank`) on every
    /// version any causal history produces.
    ///
    /// The function-space leg (`rank_realizes_riemann_sum`) supplies the
    /// independent second computation.
    #[test]
    fn rank_matches_oracle(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        prop_assert_eq!(from_oracle_version(&vs[i % n]).rank(), vs[i % n].rank());
    }

    /// The contract that makes `rank` a causal rank, on causally *related*
    /// versions (an op-trace world is full of comparable pairs).
    ///
    /// Strictly ordered versions have strictly ordered ranks, in the same
    /// direction, and equal versions have equal ranks. Concurrent pairs are
    /// unconstrained.
    #[test]
    fn rank_strictly_monotone_in_histories(ops in world_strategy()) {
        let vs: Vec<Version> = versions(&run(&ops)).iter().map(from_oracle_version).collect();
        for a in &vs {
            for b in &vs {
                match a.partial_cmp(b) {
                    Some(Ordering::Less) => prop_assert!(
                        a.rank() < b.rank(),
                        "{a} < {b} but rank {} >= {}", a.rank(), b.rank(),
                    ),
                    Some(Ordering::Greater) => prop_assert!(
                        a.rank() > b.rank(),
                        "{a} > {b} but rank {} <= {}", a.rank(), b.rank(),
                    ),
                    Some(Ordering::Equal) => prop_assert_eq!(a.rank(), b.rank()),
                    None => {} // concurrent: no constraint
                }
            }
        }
    }

    /// The same contract on arbitrary normal-form pairs (uncoupled from the
    /// op-trace generator's causally related shapes), plus the join probe.
    ///
    /// `a | b` dominates each side, so its rank dominates each side's, with
    /// equality exactly when that side already contained the other.
    #[test]
    fn rank_strictly_monotone_arbitrary(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        match a.partial_cmp(&b) {
            Some(Ordering::Less) => prop_assert!(a.rank() < b.rank()),
            Some(Ordering::Greater) => prop_assert!(a.rank() > b.rank()),
            Some(Ordering::Equal) => prop_assert_eq!(a.rank(), b.rank()),
            None => {}
        }
        let joined = &a | &b;
        for side in [&a, &b] {
            if joined == *side {
                prop_assert_eq!(joined.rank(), side.rank());
            } else {
                prop_assert!(joined.rank() > side.rank(), "the join strictly grew");
            }
        }
    }

    /// Every `tick`, on any live clock in any causal history, strictly
    /// increases the rank: a tick adds an event the version did not contain.
    #[test]
    fn tick_strictly_increases_rank(ops in world_strategy(), i in 0usize..64) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        let n = imp.len();
        let c = &mut imp[i % n];
        let before = c.version().rank();
        c.tick();
        prop_assert!(before < c.version().rank());
    }
}

/// The alignment oracle for `Rank` order: shift both numerators to the
/// common exponent and compare — the definitionally correct order the
/// class-first streamed comparison must reproduce.
fn alignment_cmp(a: &super::Rank, b: &super::Rank) -> core::cmp::Ordering {
    let (an, ae) = rank_parts(a);
    let (bn, be) = rank_parts(b);
    let e = ae.max(be);
    (an << ((e - ae) as usize)).cmp(&(bn << ((e - be) as usize)))
}

/// A rank's raw parts for the oracle, as plain `UBig` arithmetic
/// operands.
fn rank_parts(r: &super::Rank) -> (dashu_int::UBig, u32) {
    let (num, exp) = r.raw_parts();
    (dashu_int::UBig::from_le_bytes(&num.to_bytes_le()), exp)
}

/// One adversarial `Rank` for the order-agreement sweeps, from a
/// deterministic word stream.
///
/// Odd numerators from one to a few hundred limbs wide (with all-ones
/// runs so shared prefixes go deep), exponents from zero to well past
/// any numerator width.
fn stream_rank(next: &mut impl FnMut() -> u64) -> super::Rank {
    let limbs = match next() % 8 {
        0..=3 => 1,
        4..=5 => 1 + (next() % 4) as usize,
        6 => 8 + (next() % 8) as usize,
        _ => 64 + (next() % 200) as usize,
    };
    let mut words: Vec<u64> = (0..limbs)
        .map(|_| match next() % 4 {
            0 => u64::MAX,
            1 => 0,
            _ => next(),
        })
        .collect();
    if let Some(top) = words.last_mut() {
        if *top == 0 {
            *top = 1;
        }
    }
    words[0] |= 1; // odd: the stored normalization invariant
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let num = crate::codec::Base::from(dashu_int::UBig::from_le_bytes(&bytes));
    let exp = (next() % 100_000) as u32;
    super::Rank::from_raw(num, exp)
}

/// The order-agreement sweep's fixed PRNG seed: every run replays the
/// same 25,000-pair corpus.
const RANK_CMP_SWEEP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The class-first streamed `Rank` order agrees with the alignment
/// oracle on 25,000 adversarial pairs.
///
/// The pairs cross random wide/narrow numerators with far-apart
/// exponents (the mismatched-class fast path), forced class ties with
/// deep shared prefixes (the streamed-window path), and exact
/// duplicates (the equality path). `checked_sub`'s pre-check is
/// asserted consistent on every pair: `Some` exactly when
/// `rhs <= self`.
#[test]
fn rank_cmp_agrees_with_the_alignment_oracle_on_25k_pairs() {
    let mut next = crate::testing::rng::word_stream(RANK_CMP_SWEEP_SEED);
    for case in 0..25_000u32 {
        let a = stream_rank(&mut next);
        let b = match next() % 4 {
            // An unrelated rank: usually a mismatched class.
            0 => stream_rank(&mut next),
            // An exact duplicate: the equality path.
            1 => a.clone(),
            // The same value at a perturbed exponent: a guaranteed class
            // mismatch with an identical mantissa.
            2 => {
                let (num, exp) = rank_parts(&a);
                super::Rank::from_raw(
                    crate::codec::Base::from(num),
                    exp.saturating_add((next() % 64) as u32 + 1),
                )
            }
            // A forced class tie: same exponent, same width, a low bit
            // perturbed, so the streamed windows share a deep prefix.
            _ => {
                let (num, exp) = rank_parts(&a);
                use dashu_int::ops::BitTest;
                let flipped = num ^ (dashu_int::UBig::from(2u8) << ((next() % 16) as usize));
                let bits_kept = flipped.bit_len() as u64 == a_bits(&a);
                let candidate = super::Rank::from_raw(crate::codec::Base::from(flipped), exp);
                if bits_kept {
                    candidate
                } else {
                    a.clone()
                }
            }
        };
        let want = alignment_cmp(&a, &b);
        assert_eq!(a.cmp(&b), want, "case {case}: order disagrees: {a} vs {b}");
        assert_eq!(
            b.cmp(&a),
            want.reverse(),
            "case {case}: antisymmetry breaks: {b} vs {a}"
        );
        assert_eq!(
            a.checked_sub(&b).is_some(),
            want != core::cmp::Ordering::Less,
            "case {case}: checked_sub pre-check disagrees with the order"
        );
    }
}

proptest! {
    /// `Sum` is the pairwise fold.
    ///
    /// Over an arbitrary multiset of ranks in arbitrary order, both
    /// `Sum` impls return exactly the value the reference `fold(ZERO, +)`
    /// produces — one raw accumulation with a final normalization changes
    /// the cost, never the result.
    #[test]
    fn rank_sum_equals_the_pairwise_fold(seeds in proptest::collection::vec(any::<u64>(), 0..24)) {
        let ranks: Vec<super::Rank> = seeds.iter().map(|&seed| seeded_rank(seed)).collect();
        let reference = ranks
            .iter()
            .fold(super::Rank::ZERO, |acc, r| acc + r);
        prop_assert_eq!(&ranks.iter().sum::<super::Rank>(), &reference);
        prop_assert_eq!(&ranks.into_iter().sum::<super::Rank>(), &reference);
    }
}

/// A rank's numerator width for the tie construction above.
fn a_bits(r: &super::Rank) -> u64 {
    use dashu_int::ops::BitTest;
    rank_parts(r).0.bit_len() as u64
}

proptest! {
    /// `distance` and `lag` realize both reference oracles.
    ///
    /// The recursive tree fold's rank differences pin the arithmetic, and
    /// the semantic oracle's Riemann sums over join/meet events pin the
    /// meaning — the three computations share no walk, no accumulator,
    /// and no normalization sink.
    #[test]
    fn distance_and_lag_realize_both_oracles(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        use crate::testing::semantic_oracle;

        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let d = a.distance(&b);
        let l = a.lag(&b);

        // The tree oracle: rank differences over its own join and meet.
        let tree_join = (oa.clone() | ob.clone()).rank();
        let tree_meet = (oa.clone() & ob.clone()).rank();
        prop_assert_eq!(
            tree_join.checked_sub(&tree_meet).expect("join dominates meet"),
            d.clone(),
            "tree-fold distance disagrees: {} vs {}", a, b
        );
        prop_assert_eq!(
            tree_join.checked_sub(&oa.rank()).expect("join dominates self"),
            l.clone(),
            "tree-fold lag disagrees: {} vs {}", a, b
        );

        // The semantic oracle: Riemann sums over the function space.
        let ea = semantic_oracle::lift_ev(oa);
        let eb = semantic_oracle::lift_ev(ob);
        let joined = semantic_oracle::join(ea.clone(), eb.clone());
        let met = semantic_oracle::meet(ea.clone(), eb);
        let gj = semantic_oracle::ev_res(&joined);
        let gm = semantic_oracle::ev_res(&met);
        let ga = semantic_oracle::ev_res(&ea);
        prop_assert_eq!(
            semantic_oracle::rank(&joined, gj)
                .checked_sub(&semantic_oracle::rank(&met, gm))
                .expect("join dominates meet"),
            d,
            "Riemann-sum distance disagrees: {} vs {}", a, b
        );
        prop_assert_eq!(
            semantic_oracle::rank(&joined, gj)
                .checked_sub(&semantic_oracle::rank(&ea, ga))
                .expect("join dominates self"),
            l,
            "Riemann-sum lag disagrees: {} vs {}", a, b
        );
    }

    /// Every `laws::RANK_TRIPLE` law (the monoid, order, and cross-path
    /// normalization laws) holds on adversarial seeded ranks.
    ///
    /// Mixed magnitude classes, spilled numerators, and perturbed exponents:
    /// the regime the version-derived driver in the algebraic-laws suite
    /// cannot reach.
    #[test]
    fn rank_triple_laws_on_seeded_ranks(seeds in proptest::collection::vec(any::<u64>(), 3)) {
        let ranks: Vec<super::Rank> = seeds.iter().map(|&seed| seeded_rank(seed)).collect();
        let (a, b, c) = (&ranks[0], &ranks[1], &ranks[2]);
        for (name, law) in crate::laws::RANK_TRIPLE {
            prop_assert!(law(a, b, c), "law violated: {}", name);
        }
    }
}

/// One deterministic adversarial rank from a word seed, via the shared
/// test word stream.
fn seeded_rank(seed: u64) -> super::Rank {
    stream_rank(&mut crate::testing::rng::word_stream(seed))
}

// ─────────────────────────────── the join fold ───────────────────────────────

/// Build one organic history's version population.
fn world_versions(ops: &[Op]) -> Vec<Version> {
    let mut clocks = vec![crate::Clock::seed()];
    for op in ops {
        step_impl(&mut clocks, op);
    }
    clocks.iter().map(|c| c.version().clone()).collect()
}

proptest! {
    /// The balanced `join_all` is the sequential fold on versions.
    ///
    /// Over organic version populations in both orders, `join_all`, both
    /// `Sum` forms, and both `FromIterator` forms all return exactly the
    /// left fold's join — the reduction changes the grouping, never the
    /// value.
    #[test]
    fn join_all_equals_the_sequential_fold(ops in world_strategy()) {
        let pool = world_versions(&ops);
        let reference = pool
            .iter()
            .fold(Version::new(), |acc, v| acc | v);
        prop_assert_eq!(&Version::join_all(pool.clone()), &reference);
        let mut reversed = pool.clone();
        reversed.reverse();
        prop_assert_eq!(&Version::join_all(reversed), &reference);
        prop_assert_eq!(&pool.clone().into_iter().sum::<Version>(), &reference);
        prop_assert_eq!(&pool.iter().sum::<Version>(), &reference);
        prop_assert_eq!(&pool.clone().into_iter().collect::<Version>(), &reference);
        prop_assert_eq!(&pool.iter().collect::<Version>(), &reference);
    }
}

proptest! {
    /// The balanced `meet_all` is the sequential fold on versions.
    ///
    /// Over organic version populations in both orders, `meet_all` returns
    /// exactly the left fold of `&` — the reduction changes the grouping,
    /// never the value — and `None` for the empty iterator, which has no
    /// meet (the lattice has no top element).
    #[test]
    fn meet_all_equals_the_sequential_fold(ops in world_strategy()) {
        let pool = world_versions(&ops);
        let reference = pool.iter().cloned().reduce(|acc, v| acc & v);
        prop_assert_eq!(Version::meet_all(pool.clone()), reference.clone());
        let mut reversed = pool;
        reversed.reverse();
        prop_assert_eq!(Version::meet_all(reversed), reference);
        prop_assert_eq!(Version::meet_all(Vec::new()), None);
    }
}

proptest! {
    /// `meet_all` matches the recursive oracle's fold over arbitrary
    /// normal-form pools.
    ///
    /// `Some` exactly when the pool is nonempty, the production meet of
    /// every pool lowering to the oracle's; independent arbitrary shapes
    /// (not just op-trace populations) are the corner where meets
    /// restructure most.
    #[test]
    fn meet_all_matches_oracle(
        pool in proptest::collection::vec(arb_oracle_version(), 0..8),
    ) {
        let prod = Version::meet_all(pool.iter().map(from_oracle_version));
        let reference = crate::oracle::Version::meet_all(pool.iter().cloned());
        prop_assert_eq!(prod.map(|v| to_oracle_version(&v)), reference);
    }
}

/// `meet_all` on the meet-shade population returns exactly the carrier,
/// in every feed order, agreeing with the sequential fold and the
/// recursive oracle.
///
/// The meter family doubles as a differential shape (`meter::meet_shade`:
/// one dense carrier among dominating single-leaf shades, the population
/// whose running meet never shrinks — the shape the fold's flatness band
/// prices). Organic populations rarely hold one operand strictly below
/// all others, so this pins the value exactly where the reduction's
/// grouping differs most from the left fold's: every combine against the
/// carrier returns the carrier byte-for-byte, and shade ∧ shade answers
/// by canonical equality.
#[test]
fn meet_all_returns_the_carrier_on_the_shade_population() {
    for (d, k) in [(1, 2), (3, 5), (8, 16), (16, 9), (33, 64)] {
        let population = Shape::MeetShade.versions(d, k);
        let carrier = population[0].clone();
        let sequential = population
            .iter()
            .cloned()
            .reduce(|acc, v| acc & v)
            .expect("the population is nonempty");
        assert_eq!(sequential, carrier, "the shades dominate the carrier");
        assert_eq!(
            Version::meet_all(population.clone()),
            Some(carrier.clone()),
            "meet_all must return the carrier on MS({d}, {k})"
        );
        let mut reversed = population.clone();
        reversed.reverse();
        assert_eq!(
            Version::meet_all(reversed),
            Some(carrier.clone()),
            "feed order must not change the meet on MS({d}, {k})"
        );
        let oracle = crate::oracle::Version::meet_all(population.iter().map(to_oracle_version))
            .expect("the population is nonempty");
        assert_eq!(
            to_oracle_version(&carrier),
            oracle,
            "the oracle's meet must be the carrier on MS({d}, {k})"
        );
    }
}

// ─────────────────────────────── ranked ───────────────────────────────

/// `Ranked` known values: a concurrent pair sharing a rank (half vs. the
/// two-peak tree) is tiebroken by canonical bytes — unequal, ordered in
/// exactly one direction — and `into_parts` returns the version with its
/// own rank.
#[test]
fn ranked_tiebreaks_equal_ranks_by_bytes() {
    let half: Version = "(0, 1, 0)".parse().unwrap();
    let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    assert!(half.concurrent(&peaks), "the tie under test is concurrent");

    let (h, p) = (Ranked::from(half), Ranked::from(peaks));
    assert_eq!(h.rank(), p.rank(), "the pair shares a rank");
    assert_ne!(h, p, "equality is version equality, not rank equality");
    assert!((h < p) ^ (p < h), "the byte tiebreak picks one direction");

    let (version, rank) = h.into_parts();
    assert_eq!(version.rank(), rank, "the carried rank is the version's");
}

proptest! {
    /// A plain sort of `Ranked` keys delivers causes before effects: in
    /// the sorted sequence, no version is causally dominated by an earlier
    /// one.
    // `Version` is a partial order: `!(later < earlier)` also admits
    // concurrent pairs, which `later >= earlier` would reject.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    #[test]
    fn ranked_sort_respects_causality(ops in world_strategy()) {
        let mut keys: Vec<Ranked> = versions(&run(&ops))
            .iter()
            .map(|v| Ranked::from(from_oracle_version(v)))
            .collect();
        keys.sort();
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                prop_assert!(
                    !(keys[j].version() < keys[i].version()),
                    "causal inversion survived the sort",
                );
            }
        }
    }
}

// ─────────────────────── projection onto a party (`/`) ───────────────────────

/// Projection decomposes a version along a fork.
///
/// Each half's contribution is a sub-version, the two rejoin to the whole, and
/// their supports are disjoint (so their meet is empty). The whole-interval
/// seed party is the identity, and projecting onto a *disjoint* party keeps
/// nothing.
#[test]
fn div_decomposes_along_fork() {
    let mut a = Clock::seed();
    let mut b = a.fork();
    a.tick();
    a.tick();
    b.tick();
    a.sync(&mut b).unwrap(); // both learn the full history
    let v = a.version().clone();

    let from_a = (&v / a.party()).to_version();
    let from_b = (&v / b.party()).to_version();

    assert!(from_a <= v && from_b <= v); // each contribution is a sub-version
    assert_eq!(&from_a | &from_b, v); // and they rejoin to the whole
    assert_eq!(&from_a & &from_b, Version::new()); // over disjoint supports
    assert_eq!(&v / &Party::seed(), v); // the whole-interval party is the identity
}

/// The view and its materialization agree, and projecting onto a party
/// disjoint from where the events happened keeps nothing — lazily and
/// materialized alike.
#[test]
fn div_view_matches_materialization() {
    let mut a = Clock::seed();
    let b = a.fork(); // a: one half, b: the disjoint other
    a.tick();
    let v = a.version().clone();

    let w = (&v / a.party()).to_version();
    assert_eq!(&v / a.party(), w);
    assert_eq!(w, v); // a's whole version lives in a's region

    assert_eq!(&v / b.party(), Version::new()); // none of a's tick lies in b's region
    assert_eq!((&v / b.party()).to_version(), Version::new());
}

/// Projection can *raise* `min_ticks`: it is not monotone under `<=`.
///
/// A single whole-interval tick (`leaf 1`, `min_ticks` 1) projected onto a
/// "comb" region — two quarters in different halves — becomes two concurrent
/// peaks, which no single tick can produce, forcing `min_ticks` to 2 even
/// though the projection is a sub-version.
#[test]
fn div_can_fragment_and_raise_min_ticks() {
    // Fork a seed into quarters, then rejoin two that lie in different halves.
    let mut q0 = Clock::seed();
    let mut q2 = q0.fork(); // q0: one half, q2: the other
    let _q1 = q0.fork(); // q0: a quarter of its half
    let _q3 = q2.fork(); // q2: a quarter of the other half
    q0.join(q2).unwrap(); // q0 now owns two quarters, one per half
    let comb = q0.party();

    let v = Version::try_from(1).unwrap();
    assert_eq!(v.min_ticks(), Ticks::from(1u64)); // one tick covers the whole interval

    let frag = (&v / comb).to_version();
    assert!(frag <= v); // still a sub-version
    assert_eq!(frag.min_ticks(), Ticks::from(2u64)); // but now two concurrent peaks
}

/// The at-rest form is the wire bytes in a length-carrying container.
///
/// A [`Version`] is exactly one `codec::Bits` (pointer, live bit length,
/// capacity — 24 bytes on 64-bit), and a [`Clock`](crate::Clock) is a
/// `Party` plus a `Version` (48). A regression here means the storage
/// grew a field beside the container — the cached live length must ride
/// inside it, since the wire legitimately omits it.
#[test]
fn at_rest_size_is_one_container_per_stream() {
    assert_eq!(core::mem::size_of::<Version>(), 24);
    assert_eq!(core::mem::size_of::<crate::Clock>(), 48);
}

proptest! {
    /// Byte-level equality (`codec::canonical_eq`) agrees with a plain
    /// bit-level compare of the live streams, in both operand orders.
    ///
    /// The cross-check that the canonical-raw-slice invariant (dead bits
    /// zeroed at every storage seam) really licenses the byte shortcut
    /// over raw bytes plus live length. Equal values must also hash
    /// equally (`Eq`/`Hash` consistency).
    #[test]
    fn byte_equality_matches_bit_equality(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let bit_eq = a.as_bits() == b.as_bits();
        prop_assert_eq!(a == b, bit_eq);
        prop_assert_eq!(b == a, bit_eq);
        if a == b {
            let hash = |v: &Version| {
                use core::hash::{Hash, Hasher};
                let mut h = std::hash::DefaultHasher::new();
                v.hash(&mut h);
                h.finish()
            };
            prop_assert_eq!(hash(&a), hash(&b));
        }
    }
}
