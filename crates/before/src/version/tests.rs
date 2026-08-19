//! Version tests.
//!
//! The causal order and its comparison matrix, the join/meet operator matrices
//! and lattice laws, grow optimality against the brute-force reference,
//! `min_ticks`, and projection (`/`).

use crate::meter::registry::Shape;
use std::cmp::Ordering;

use proptest::prelude::*;

use super::{skyline, Ranked, Version};
use crate::testing::bridge::{from_oracle_party, from_oracle_version, to_oracle_version};
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::grow_brute_force::{all_inflations, best_inflation};
use crate::testing::optrace::{leq as oracle_leq, run, step_impl, versions, world_strategy, Op};
use crate::{Clock, Party, Ticks};

/// `a <= b` under the impl causal order.
fn le(a: &Version, b: &Version) -> bool {
    a.partial_cmp(b).is_some_and(|o| o != Ordering::Greater)
}

// ───────────────────────────── causal order ─────────────────────────────

// The order laws (reflexivity, antisymmetry, transitivity, `==` ⟺
// `Some(Equal)`, concurrency ⟺ `None`) are `laws::VERSION_SOLO` /
// `VERSION_PAIR` / `VERSION_TRIPLE` entries (order_reflexive,
// order_antisymmetric, order_transitive_incidental and _constructed,
// eq_iff_cmp_equal, concurrent_iff_incomparable, partial_cmp_is_dual),
// driven over these op-trace populations and two more.

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
    /// The full comparison matrix over owned and borrowed `Version` operands
    /// agrees with the oracle's verdict on the same pair.
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
    /// The join is representationally subadditive: `encode(a | b).len() <=
    /// encode(a).len() + encode(b).len()`.
    ///
    /// The join's event tree branches only where an input branches and its
    /// bases come from pointwise combination, so joining can restructure but
    /// never invent structure beyond both inputs together. Callers that track
    /// version-size maxima rely on this to charge a join of two bounded
    /// versions the sum of their bounds. Probed here over churned
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
    /// The meet is representationally subadditive: `encode(a & b).len() <=
    /// encode(a).len() + encode(b).len()`.
    ///
    /// Dual to [`join_encoding_is_subadditive`]: the meet's event tree branches
    /// only where an input branches and its bases come from pointwise
    /// combination, so meeting can restructure but never invent structure
    /// beyond both inputs together. Callers that track version-size maxima rely
    /// on this to charge a meet of two bounded versions (an assembled floor)
    /// the sum of their bounds. Probed over the same churned populations as the
    /// join lemma.
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
    /// Every assigning join surface on `Version` yields the same result as `a |
    /// b`, which the `version_join_matches_the_oracle` descriptor already pins
    /// to the oracle's `join`.
    ///
    /// Covers `Version |= Version` and `Version |= &Version` — neither of which
    /// the by-value `|` differential reaches.
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
    /// The full `|` (BitOr) matrix over owned and borrowed `Version` operands
    /// equals the oracle's `join`.
    ///
    /// The `version_join_matches_the_oracle` descriptor pins the semantics on
    /// both differential populations; each of the four reference cells must
    /// agree with it.
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
    /// The full `|=` (BitOrAssign) matrix — owned and borrowed right operands —
    /// lands on the oracle's `join`.
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
    /// The full `&` (BitAnd) matrix over owned and borrowed `Version` operands
    /// equals the oracle's `meet`, dual to [`join_matrix_matches_oracle`].
    ///
    /// The `version_meet_matches_the_oracle` descriptor pins the semantics on
    /// both differential populations; each of the four reference cells must
    /// agree with it.
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
    /// The full `&=` (BitAndAssign) matrix — owned and borrowed right operands
    /// — lands on the oracle's `meet`, dual to
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

// The method spellings of `|`, `&`, and `^` (`Version::join`, `Version::meet`,
// and the four-cell `^` matrix against `Version::span`) are laws
// (`join_method_is_the_operator`, `meet_method_is_the_operator`,
// `span_operator_matrix_is_the_method` in `crate::laws`), driven over
// arbitrary normal forms, these op-trace populations, and the fuzz target's
// decoded values.

proptest! {
    /// Lattice identity for join, byte-identical: `0 | v == v == v | 0`.
    ///
    /// The encoded bytes equal `v`'s own — both through the `join_view`
    /// short-circuit (empty on either side) and through the general merge
    /// kernel called directly (`skyline::emit::join`), which the short-circuit
    /// must match bit for bit.
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
    /// `meet_view` short-circuit (empty on either side) and through the general
    /// merge kernel called directly (`skyline::emit::meet`), which the
    /// short-circuit must match bit for bit. Dual to
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
// `crate::laws` and are driven by the algebraic-laws suite over both arbitrary
// normal forms and these same op-trace populations; this file keeps the
// differential and mechanism-level tests.

// ───────────────────────────── path-sum overflow regression ─────────────────────────────

/// A normal-form tree whose root-to-leaf path sum exceeds `u64::MAX` compares
/// correctly.
///
/// With arbitrary-precision leaf heights there is no overflow class, so the
/// answer is `Greater` in every build profile (no debug panic, no release wrap
/// that would invert the causal order). `decode`/`try_from` admit such trees,
/// so the comparison must thread the heights at full precision.
#[test]
fn path_sum_beyond_u64_compares_greater() {
    let big = 1u64 << 63;
    // Normal form: the outer min(big, 0) child is the right `0` leaf; the inner
    // node's min(0, 1) child is its left `0` leaf. The left half's true value
    // is big + big + 1 = 2^64 + 1, past `u64::MAX`.
    let a = Version::try_from((big, (big, 0u64, 1u64), 0u64)).unwrap();
    let b = Version::try_from(big).unwrap(); // constant 2^63
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater));
}

proptest! {
    /// A node literal over two equal-height leaves is refused at every base
    /// and every height.
    ///
    /// `(n, m, m)` collapses to the leaf `n + m`, so the leaf is the one
    /// canonical spelling and the `TryFrom` surface rejects the node form.
    /// The collapse check precedes min-lifting, so the collapse rejection
    /// owns every equal pair, whatever `m` — and the accepted neighbors on
    /// either side (the unequal pairs that stay min-lifted) pin that the
    /// rejection is the equality, not the shape.
    #[test]
    fn equal_leaf_literals_are_refused(
        n in 0u64..=u64::MAX / 2,
        m in 0u64..=1u64 << 40,
        z in 1u64..=1u64 << 40,
    ) {
        prop_assert_eq!(
            Version::try_from((n, m, m)),
            Err(crate::error::Parse::NotCanonical),
            "(n, m, m) must collapse-reject"
        );
        prop_assert!(Version::try_from((n, 0u64, z)).is_ok());
        prop_assert!(Version::try_from((n, z, 0u64)).is_ok());
    }

    /// Nested literals whose leaf heights descend build exactly the oracle's
    /// tree.
    ///
    /// The composer re-derives each child's absolute heights from the
    /// child's own stream, and a later-lower leaf rides the negative half of
    /// that scan's zigzag decode — swept over descending runs in the left
    /// child, the right child, and both at once.
    #[test]
    fn descending_literals_build_the_oracle_tree(
        w in 0u64..=6,
        k in 1u64..=1u64 << 40,
        z in 1u64..=1u64 << 40,
        z2 in 1u64..=1u64 << 40,
    ) {
        use crate::oracle::Version as V;
        let expect = |t: &V| from_oracle_version(t);
        // Descent inside the left child: leaves (k + z, k), strictly falling.
        let left: Version = Version::try_from((w, (k, z, 0u64), 0u64)).unwrap();
        prop_assert_eq!(
            &left,
            &expect(&V::node(w, V::node(k, V::leaf(z), V::leaf(0u64)), V::leaf(0u64)))
        );
        // Descent inside the right child.
        let right: Version = Version::try_from((w, 0u64, (k, z, 0u64))).unwrap();
        prop_assert_eq!(
            &right,
            &expect(&V::node(w, V::leaf(0u64), V::node(k, V::leaf(z), V::leaf(0u64))))
        );
        // Descents in both children, the right anchored at base zero so the
        // node stays min-lifted.
        let both: Version = Version::try_from((w, (k, z, 0u64), (0u64, z2, 0u64))).unwrap();
        prop_assert_eq!(
            &both,
            &expect(&V::node(
                w,
                V::node(k, V::leaf(z), V::leaf(0u64)),
                V::node(0u64, V::leaf(z2), V::leaf(0u64)),
            ))
        );
    }
}

/// A stored leaf height above `u64::MAX` stays exact across mutation and merge.
/// This pins the arbitrary-width payload path at the machine-word spill
/// boundary, not only path sums made from individually-small nodes.
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
    /// `==` agrees with the full causal-compare walk.
    ///
    /// The equality cells decide by a byte compare of the two stored streams
    /// (canonical unique representation: byte equality ⟺ equality); this pins
    /// that shortcut to the comparison sweep's verdict on arbitrary, typically
    /// *unequal* pairs (the inequality direction the shortcut decides without
    /// walking) and on equal pairs (the equality direction).
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
    /// The join-size lemma of [`join_encoding_is_subadditive`], on arbitrary,
    /// typically *unrelated* normal-form pairs.
    ///
    /// The churned generator only produces causally related versions from one
    /// seed; these pairs add independent shapes and large-base leaves (values
    /// near/beyond `u64::MAX`), where a join must restructure most — the corner
    /// where subadditivity would break if normalization could ever inflate a
    /// combined tree past its inputs.
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
    /// The meet-size lemma of [`meet_encoding_is_subadditive`], on arbitrary,
    /// typically *unrelated* normal-form pairs.
    ///
    /// Dual to [`join_encoding_is_subadditive_arbitrary`], and for the same
    /// reason: independent shapes and large-base leaves are the corner where a
    /// meet must restructure most, so this is where subadditivity would break
    /// if normalization could ever inflate a combined tree past its inputs.
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
    /// The stored form keeps its padding sealed (the marker, then zeros), so
    /// the raw storage slice is byte-identical to the packed encoding.
    /// Exercises the literal/`extend` construction path over arbitrary
    /// normal-form trees.
    #[test]
    fn as_bytes_matches_encode(ov in arb_oracle_version()) {
        let v = from_oracle_version(&ov);
        let encoded = v.encode();
        prop_assert_eq!(v.as_bytes(), encoded.as_slice());
    }

    /// The invariant survives mutation too: ticking re-emits the stored
    /// stream through the fill splice, which must also leave a sealed
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
    /// Cross-checks the fold itself against the recursive oracle's sum-of-bases
    /// (`oracle::Version::min_ticks`); the min_ticks descriptor's fs leg
    /// supplies the independent second computation.
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
    assert_eq!(half.rank().to_string(), "1/2");

    let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    assert!(half.concurrent(&peaks), "different halves of the interval");
    assert_eq!(
        half.rank(),
        peaks.rank(),
        "equal rank is fine when concurrent"
    );
}

// Strict rank monotonicity on the causal order is the
// `laws::VERSION_PAIR::rank_strictly_monotone` law on all three law
// populations; the join probe (the join's rank strictly grows past a side
// unless that side already contained the other) is its composition with
// `merge_is_upper_bound` and `order_absorbing`, and tick-increases-rank the
// composition with `laws::VERSION_PARTY::tick_strictly_advances` — each leg
// executed by the law drivers on the same populations.

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
fn rank_parts(r: &super::Rank) -> (dashu_int::UBig, u64) {
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
    let exp = next() % 100_000;
    super::Rank::from_raw(num, exp)
}

/// The order-agreement sweep's fixed PRNG seed: every run replays the
/// same 25,000-pair corpus.
const RANK_CMP_SWEEP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The class-first streamed `Rank` order agrees with the alignment oracle on
/// 25,000 adversarial pairs.
///
/// The pairs cross random wide/narrow numerators with far-apart exponents (the
/// mismatched-class fast path), forced class ties with deep shared prefixes
/// (the streamed-window path), and exact duplicates (the equality path).
/// `checked_sub`'s pre-check is asserted consistent on every pair: `Some`
/// exactly when `rhs <= self`.
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
                    exp.saturating_add(next() % 64 + 1),
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
    /// Over an arbitrary multiset of ranks in arbitrary order, both `Sum` impls
    /// return exactly the value the reference `fold(ZERO, +)` produces — one
    /// raw accumulation with a final normalization changes the cost, never the
    /// result.
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
    /// Every `laws::RANK_TRIPLE` law (the monoid, order, and cross-path
    /// normalization laws) holds on adversarial seeded ranks.
    ///
    /// Mixed magnitude classes, spilled numerators, and perturbed exponents:
    /// the regime the version-derived driver in the algebraic-laws suite cannot
    /// reach.
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

// ─────────────────────── the rank wire form ───────────────────────

/// Committed witnesses for the lexicographic law's boundary genres.
///
/// Zero, the smallest fractions, integral-only vs fractional at a shared
/// integral part, every step of the integral header (the mantissa-width steps
/// at `I + 1` crossing a power of two, and the width-of-width steps where the
/// unary run itself lengthens), and equal integral parts separated only deep in
/// the fraction — across a group seam, where the deeper rank's extra expansion
/// bits ride a further continuation-framed group. Each byte string is pinned
/// exactly — the wire form is canonical, so these are format goldens — and the
/// whole battery must be strictly ascending in byte order exactly as it is
/// ascending in rank order.
#[test]
fn rank_encoding_known_values() {
    let rank_of = |text: &str| text.parse::<Version>().unwrap().rank();
    let int = |n: u64| Version::try_from(n).unwrap().rank();
    // (value, its pinned canonical bytes), in strictly ascending order.
    let battery: Vec<(super::Rank, Vec<u8>)> = vec![
        // Zero = "0" ++ "0": the smallest header, an empty fraction's
        // immediate close.
        (super::Rank::ZERO, vec![0x00]),
        // 1/4 = "0" ++ "1 01000000 0": one group framing the
        // expansion ".01", zero-padded past its last set bit.
        (rank_of("(0, (0, 1, 0), 0)"), vec![0x50, 0x00]),
        // 1/2 = "0" ++ "1 10000000 0".
        (rank_of("(0, 1, 0)"), vec![0x60, 0x00]),
        // 3/4 = "0" ++ "1 11000000 0": splits from 1/2 inside the
        // shared group, at the second expansion bit.
        (rank_of("(0, 1, (0, 1, 0))"), vec![0x70, 0x00]),
        // 1 = "1000" ++ "0": the first integral header step.
        (int(1), vec![0x80]),
        // 3/2 = 1's integral code, then one group framing ".1": integral-only
        // vs fractional at a shared integral part is decided at the
        // continuation-vs-close bit.
        (
            super::Rank::from_raw(crate::codec::Base::from(3u8), 1),
            vec![0x8C, 0x00],
        ),
        (int(2), vec![0x90]),
        (int(3), vec![0xA0]),
        (int(4), vec![0xA8]),
        // 5, then 5 + 2⁻⁴⁰ and 5 + 2⁻⁴⁰ + 2⁻⁴¹: equal integral parts
        // separated only deep in the fraction — the deepest pair only
        // past a group seam (the 41st expansion bit opens a sixth
        // group), and the integral-only rank separated from both at
        // its close bit, never by a byte-prefix relation.
        (int(5), vec![0xB0]),
        (
            super::Rank::from_raw(crate::codec::Base::from(5u128 << 40 | 1), 40),
            vec![0xB4, 0x02, 0x01, 0x00, 0x80, 0x40, 0x40],
        ),
        (
            super::Rank::from_raw(crate::codec::Base::from(5u128 << 41 | 3), 41),
            vec![0xB4, 0x02, 0x01, 0x00, 0x80, 0x40, 0x70, 0x00],
        ),
        // 6 and 7: the last mantissa of width 2 against the first of
        // width 3 — the header's width-of-width (unary run) step.
        (int(6), vec![0xB8]),
        (int(7), vec![0xC0, 0x00]),
        (int(8), vec![0xC1, 0x00]),
        // 15 and 16: the next mantissa-width step inside one run
        // width, separated by the mantissa's final bit at the byte
        // seam.
        (int(15), vec![0xC8, 0x00]),
        (int(16), vec![0xC8, 0x80]),
    ];
    for (i, (rank, bytes)) in battery.iter().enumerate() {
        assert_eq!(&rank.encode(), bytes, "case {i}: pinned bytes for {rank}");
        assert_eq!(
            &super::Rank::decode(&bytes[..]).unwrap(),
            rank,
            "case {i}: round-trip for {rank}"
        );
    }
    for pair in battery.windows(2) {
        assert!(pair[0].0 < pair[1].0, "battery is ascending in rank order");
        assert!(
            pair[0].1 < pair[1].1,
            "byte order agrees: {} vs {}",
            pair[0].0,
            pair[1].0
        );
    }
}

// THE LAW of the rank wire form (byte order == `Ord`, the codec round-trip,
// and prefix-freedom) lives in `laws::RANK_TRIPLE` (`rank_lex_order`,
// `rank_codec_roundtrip`, `rank_encoding_prefix_free`): the roster drivers
// run it on version-derived ranks over the organic, arbitrary, and
// fuzz-decoded populations, and `rank_triple_laws_on_seeded_ranks` above
// keeps the adversarial spilled-magnitude regime driving the same group.

/// The exhaustive small-scope sweep over **every** byte string of zero, one,
/// and two bytes.
///
/// Decode is total (accepts or rejects, never panics); every accepted string
/// re-encodes byte-identically (the format is bijective on accepted strings, so
/// no value has a second spelling — the strict-canonicality statement that
/// subsumes the per-genre rejections); the accepted strings in byte order carry
/// strictly ascending ranks (the lexicographic law, total at this scope); every
/// decoded rank's numeric size is linear in its input bytes (no decompression
/// bomb); and both live rejection genres (truncation, non-minimal packing)
/// actually fire.
#[test]
fn rank_encoding_exhaustive_small_scope() {
    let mut accepted: Vec<(Vec<u8>, super::Rank)> = Vec::new();
    let (mut truncated, mut trailing) = (0u32, 0u32);
    let mut sweep = |bytes: Vec<u8>| match super::Rank::decode(&bytes[..]) {
        Ok(rank) => {
            assert_eq!(rank.encode(), bytes, "canonical: re-encodes to itself");
            assert!(
                rank.content_bits() <= 16 * bytes.len() as u64,
                "decoded size is input-linear"
            );
            accepted.push((bytes, rank));
        }
        Err(crate::error::Decode::Truncated) => truncated += 1,
        Err(crate::error::Decode::TrailingBits) => trailing += 1,
        Err(e) => panic!("unexpected rejection genre at this scope: {e}"),
    };
    sweep(vec![]);
    for b0 in 0..=255u8 {
        sweep(vec![b0]);
        for b1 in 0..=255u8 {
            sweep(vec![b0, b1]);
        }
    }
    accepted.sort();
    for pair in accepted.windows(2) {
        assert!(
            pair[0].1 < pair[1].1,
            "byte order must be rank order: {:02x?} ({}) vs {:02x?} ({})",
            pair[0].0,
            pair[0].1,
            pair[1].0,
            pair[1].1
        );
    }
    // Liveness: the scope actually exercises acceptance and both
    // reachable rejection genres.
    assert!(
        accepted.len() > 1_000,
        "acceptance is live: {}",
        accepted.len()
    );
    assert!(truncated > 0, "the truncation genre fires");
    assert!(trailing > 0, "the non-minimal-packing genre fires");
}

/// Committed witnesses, one per rejection genre the decoder can reach.
///
/// Empty input, an unterminated unary run, a truncated header payload, a
/// truncated integral mantissa, a truncated fraction group, a trailing zero
/// byte, a set padding bit, the one spelling a trailing-zero fraction can take
/// (an all-zero final group — inside the final group trailing zeros *are* the
/// padding, so the only non-canonical spelling spills them into a group of
/// their own), and the integral representation bound (a unary run of 64,
/// declaring a mantissa width beyond `2⁶⁴` bits — the one format bound a small
/// input can reach; the fraction bound needs over half a GiB of real groups,
/// since the fraction has no length header to forge). The remaining documented
/// genre — a non-minimal integral header — is structurally unrepresentable
/// (every `(run, payload)` pair decodes to a width whose own width matches the
/// run exactly), which the exhaustive sweep witnesses mechanically at small
/// scope.
#[test]
#[allow(clippy::type_complexity)]
fn rank_decoding_rejects_each_genre() {
    use crate::error::Decode;
    let genres: [(&[u8], fn(&Decode) -> bool, &str); 9] = [
        (&[], |e| matches!(e, Decode::Truncated), "empty input"),
        (
            &[0xFF],
            |e| matches!(e, Decode::Truncated),
            "unary run to the end",
        ),
        // 11111101: run 6, payload needs 6 bits, 1 remains.
        (
            &[0xFD],
            |e| matches!(e, Decode::Truncated),
            "truncated header payload",
        ),
        // 11011111: run 2, w = 7, mantissa needs 6 bits, 3 remain.
        (
            &[0xDF],
            |e| matches!(e, Decode::Truncated),
            "truncated mantissa",
        ),
        // 01000000: a set continuation bit with 6 bits left, no room
        // for its 8-bit group.
        (
            &[0x40],
            |e| matches!(e, Decode::Truncated),
            "truncated fraction group",
        ),
        // The encoding of 1, then a whole padding byte.
        (
            &[0x80, 0x00],
            |e| matches!(e, Decode::TrailingBits),
            "trailing zero byte",
        ),
        // 10000100: the encoding of 1 (stream "10000") with a set bit
        // in its three padding positions.
        (
            &[0x84],
            |e| matches!(e, Decode::TrailingBits),
            "set padding bit",
        ),
        // "0" ++ "1 10000000" ++ "1 00000000" ++ "0": the fraction
        // ".1" spelled with a second, all-zero group — the
        // trailing-zero-fraction spelling, non-minimal packing.
        (
            &[0x60, 0x20, 0x00],
            |e| matches!(e, Decode::TrailingBits),
            "all-zero final group",
        ),
        // 64 ones, then the run's terminating zero: an integral
        // mantissa width of 2⁶⁴ or more bits — past the format bound,
        // whatever follows.
        (
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00],
            |e| matches!(e, Decode::NotCanonical),
            "integral width past the format bound",
        ),
    ];
    for (bytes, is_genre, what) in genres {
        let err = super::Rank::decode(bytes).expect_err(what);
        assert!(is_genre(&err), "{what}: wrong genre: {err}");
    }
}

/// The provenance size bound, measured and pinned per committed family: every
/// rank reachable through a version fold encodes linearly in the version's
/// packed bytes.
///
/// The families white-box the encoder's two axes — numerator width (wide
/// counters, answer-embedding products) and exponent depth (spines), plus the
/// dense-fraction staircase that maximizes set bits per level — and the pin
/// holds each family's encoded size at or under 1.0 bit per packed input bit.
/// Measured \[by this test's own instrumentation\]: wide counter 0.56 (the
/// worst — a lone counter's version pays gamma's doubled width where the
/// encoding pays the width once), deep spine 0.38, dense staircase 0.38, deep
/// wide counter 0.27, plateau puncture 0.18; the 1.0 pin leaves headroom for
/// packing drift while sitting an order under the exponential blowup arbitrary
/// in-memory ranks can reach.
#[test]
fn rank_encoding_size_is_provenance_linear() {
    // A deep spine holding one unit leaf: rank 2⁻ᵏ, the exponent axis.
    fn spine(depth: usize) -> Version {
        let mut text = String::from("1");
        for _ in 0..depth {
            text = format!("(0, {text}, 0)");
        }
        text.parse().unwrap()
    }
    // The dense staircase: one new unit plateau per level, so every level
    // contributes a set fraction bit — the set-bits-per-level maximum the
    // white-box attack found.
    fn staircase(depth: usize) -> Version {
        let mut text = String::from("(0, 1, 0)");
        for _ in 0..depth {
            text = format!("(0, {text}, 1)");
        }
        text.parse().unwrap()
    }
    // A wide counter behind a spine: both axes at once.
    fn deep_counter(depth: usize, counter: &str) -> Version {
        let mut text = String::from(counter);
        for _ in 0..depth {
            text = format!("(0, {text}, 0)");
        }
        text.parse().unwrap()
    }
    let wide = "340282366920938463463374607431768211455"; // 2¹²⁸ − 1
    let families: [(&str, Version); 5] = [
        ("wide counter", wide.parse().unwrap()),
        ("deep spine", spine(800)),
        ("dense staircase", staircase(800)),
        ("deep wide counter", deep_counter(400, wide)),
        // The answer-embedding shape's essence at test scale: a wide plateau
        // over dense puncturing turns, keeping the numerator wide *and* dense.
        ("plateau puncture", {
            let mut text = String::from(wide);
            for _ in 0..100 {
                text = format!("(0, (1, {text}, 0), 0)");
            }
            text.parse().unwrap()
        }),
    ];
    for (name, version) in families {
        let input_bits = version.encoded_bits() as f64;
        let encoded_bits = (version.rank().encode().len() * 8) as f64;
        assert!(
            encoded_bits <= input_bits,
            "{name}: encoded rank ({encoded_bits} bits) exceeds the pinned \
             1.0 ratio over packed input ({input_bits} bits)"
        );
    }
}

/// Committed witnesses for suffix safety at the padding seam: the shapes where
/// a naive expansion spelling would make one encoding a byte prefix of
/// another's.
///
/// Each pair is two distinct ranks whose streams agree bit-for-bit up to where
/// the smaller one's content ends — an integral-only rank against a
/// deep-fraction extension, zero against a small deep fraction, and a one-bit
/// fraction against an extension whose extra bits begin with a byte's worth of
/// zeros. For each pair the law's full strength is asserted directly: the
/// encodings are not byte prefixes of one another, so the smaller rank's key
/// sorts first under *any* tiebreak suffix — including the worst one, `0xFF`
/// against the larger key's continuation.
#[test]
fn rank_encoding_is_suffix_safe_at_the_padding_seam() {
    let pairs: [(super::Rank, super::Rank); 3] = [
        // 5 against 5 + 2⁻⁴⁰: equal integral parts, one fraction empty.
        (
            Version::try_from(5).unwrap().rank(),
            super::Rank::from_raw(crate::codec::Base::from(5u128 << 40 | 1), 40),
        ),
        // Zero against 2⁻⁹: the empty stream tail against a fraction
        // whose first byte's worth of expansion bits is all zero.
        (
            super::Rank::ZERO,
            super::Rank::from_raw(crate::codec::Base::from(1u8), 9),
        ),
        // 1/2 against 1/2 + 2⁻⁸: the extension's extra expansion bits
        // are exactly the shorter stream's padding, then a set bit.
        (
            super::Rank::from_raw(crate::codec::Base::from(1u8), 1),
            super::Rank::from_raw(crate::codec::Base::from(129u8), 8),
        ),
    ];
    for (small, large) in &pairs {
        assert!(small < large, "the witness pair is ordered");
        let (es, el) = (small.encode(), large.encode());
        assert!(
            !el.starts_with(&es) && !es.starts_with(&el),
            "prefix-free: {small} vs {large}"
        );
        assert!(es < el, "byte order is rank order: {small} vs {large}");
        // The worst suffix: the smaller key padded high, the larger low.
        let key_small = [es, vec![0xFF; 4]].concat();
        let key_large = [el, vec![0x00; 4]].concat();
        assert!(
            key_small < key_large,
            "no suffix flips the order: {small} vs {large}"
        );
    }
}

proptest! {
    /// THE LAW's suffix-safety half, adversarially: distinct ranks' encodings
    /// are never byte prefixes of one another.
    ///
    /// So a key built as `encoding ++ tiebreak` orders by rank first under
    /// every choice of tiebreak — the KV-key contract `Rank::encode` documents.
    ///
    /// Over pairs mixing far-apart magnitude classes, forced class ties, and
    /// near-miss extensions (the second rank re-derived from the first with a
    /// deepened fraction), with arbitrary suffix bytes on both keys.
    #[test]
    fn rank_lex_encoding_is_suffix_safe(
        sa in any::<u64>(),
        sb in any::<u64>(),
        extend in any::<bool>(),
        deepen in 1u32..64,
        suffix_a in proptest::collection::vec(any::<u8>(), 0..5),
        suffix_b in proptest::collection::vec(any::<u8>(), 0..5),
    ) {
        let a = seeded_rank(sa);
        let b = if extend {
            // A strict extension of `a`'s expansion: the genre where
            // one stream continues past the other's content.
            let (num, exp) = a.raw_parts();
            super::Rank::from_raw(
                (num.clone() << deepen) + 1u32,
                exp.saturating_add(u64::from(deepen)),
            )
        } else {
            seeded_rank(sb)
        };
        let (ea, eb) = (a.encode(), b.encode());
        if a != b {
            prop_assert!(
                !eb.starts_with(&ea) && !ea.starts_with(&eb),
                "prefix-free: {} vs {}", a, b
            );
        }
        let key_a = [ea, suffix_a].concat();
        let key_b = [eb, suffix_b].concat();
        match a.cmp(&b) {
            Ordering::Less => prop_assert!(key_a < key_b, "{} vs {}", a, b),
            Ordering::Greater => prop_assert!(key_a > key_b, "{} vs {}", a, b),
            Ordering::Equal => {}
        }
    }
}

// ─────────────────────────────── the join fold ───────────────────────────────

// The n-ary fold doors against the sequential pair fold — `join_all`,
// `meet_all`, both `Sum` forms, both `FromIterator` forms, in every feed
// order — are the `laws::VERSION_LIST` / `VERSION_AND_LIST` fold laws
// (version_sum_is_the_sequential_pair_fold, version_sum_is_order_invariant,
// join_all_is_the_sequential_pair_fold, meet_all_is_the_sequential_pair_fold,
// fold_all_is_rotation_invariant), driven at boundary-band arities over the
// organic, arbitrary, and fuzz-decoded populations.

proptest! {
    /// `meet_all` matches the recursive oracle's fold over arbitrary
    /// normal-form pools.
    ///
    /// The production door folds the receiver and its items; the oracle folds
    /// the same family as one list. Independent arbitrary shapes (not just
    /// op-trace populations) are the corner where meets restructure most.
    #[test]
    fn meet_all_matches_oracle(
        pool in proptest::collection::vec(arb_oracle_version(), 1..8),
    ) {
        let versions: Vec<Version> = pool.iter().map(from_oracle_version).collect();
        let (first, rest) = versions.split_first().expect("the pool is nonempty");
        let prod = first.meet_all(rest);
        let reference = crate::oracle::Version::meet_all(pool.iter().cloned())
            .expect("the pool is nonempty");
        prop_assert_eq!(to_oracle_version(&prod), reference);
    }
}

/// `meet_all` on the meet-shade population returns exactly the carrier, in
/// every feed order, agreeing with the sequential fold and the recursive
/// oracle.
///
/// The meter family doubles as a differential shape (`meter::meet_shade`: one
/// dense carrier among dominating single-leaf shades, the population whose
/// running meet never shrinks — the shape the fold's flatness band prices).
/// Organic populations rarely hold one operand strictly below all others, so
/// this pins the value exactly where the reduction's grouping differs most from
/// the left fold's: every combine against the carrier returns the carrier
/// byte-for-byte, and shade ∧ shade answers by canonical equality.
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
            population[0].meet_all(&population[1..]),
            carrier,
            "meet_all must return the carrier on MS({d}, {k})"
        );
        let mut reversed = population.clone();
        reversed.reverse();
        assert_eq!(
            reversed[0].meet_all(&reversed[1..]),
            carrier,
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

/// `Ranked` known values: a rank-equal concurrent pair is separated by the
/// version-byte tiebreak, never conflated.
///
/// A concurrent pair sharing a rank (half vs. the two-peak tree) drives the
/// fused walk's hardest arm (the exact total must cancel to zero) into the
/// tiebreak: the views compare non-`Equal`, ordered exactly as the versions'
/// canonical bytes, while the rank question — asked explicitly through
/// `rank` — still answers a tie.
#[test]
fn ranked_orders_equal_rank_concurrent_pairs_by_bytes() {
    let half: Version = "(0, 1, 0)".parse().unwrap();
    let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    assert!(half.concurrent(&peaks), "the tie under test is concurrent");
    assert_eq!(half.rank(), peaks.rank(), "the pair shares a rank");

    let (h, p) = (Ranked::from(&half), Ranked::from(&peaks));
    assert_ne!(h, p, "equality is version identity: the views differ");
    let byte_order = half.as_bytes().cmp(peaks.as_bytes());
    assert_ne!(byte_order, Ordering::Equal, "distinct canonical bytes");
    assert_eq!(h.cmp(&p), byte_order, "rank ties order by version bytes");
    assert_eq!(p.cmp(&h), byte_order.reverse());
    assert_eq!(h.rank(), p.rank(), "the ranks themselves still tie");
}

proptest! {
    /// A plain sort of `Ranked` keys delivers causes before effects.
    ///
    /// In the sorted sequence, no version is causally dominated by an earlier
    /// one (rank order refines causality; equal-rank keys are concurrent or
    /// identical, so any tie order is causally safe).
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

// The fused Ranked comparison against the materialized rank-then-bytes order
// (both argument orders) is the `laws::VERSION_PAIR::
// ranked_orders_by_rank_then_bytes` law, and the fused rank-only encode
// against `Rank::encode` over the materialized rank is a clause of
// `laws::VERSION_SOLO::ranked_carries_own_rank` — both driven on all three
// law populations, a strict superset of the arbitrary pairs alone.

/// The staircase: one new unit plateau per level over the given core, mass
/// leaning left (`(0, t, 1)`) or right (`(0, 1, t)`).
///
/// The two leans are mirror images — their areas agree level for level, so they
/// share a rank by symmetry. The extreme-depth genre no generator reaches,
/// shared by the deep-cancellation and composite-key suites.
fn stairs(depth: usize, lean_left: bool, core: &str) -> Version {
    let mut text = String::from(core);
    for _ in 0..depth {
        text = if lean_left {
            format!("(0, {text}, 1)")
        } else {
            format!("(0, 1, {text})")
        };
    }
    text.parse().unwrap()
}

/// The fused comparison's hard genres, constructed: deep total cancellation,
/// and verdicts decided only at the walk's last contribution.
///
/// A staircase and its mirror image share a rank by symmetry while their mass
/// sits at opposite ends of the interval, so the signed co-sweep's running
/// difference swings through every level's magnitude before the exact total
/// cancels — the widest cancellation an 800-level walk can force through the
/// freeze and promotion machinery, handing the mirror pair's verdict to the
/// version-byte tiebreak. Splitting one mirror's core step then moves the total
/// by `2⁻⁸⁰³` alone: every level above still cancels, and the verdict's sign
/// rests entirely on the deepest contribution. Each verdict is checked against
/// the materialized rank-then-bytes order in both argument orders, and the
/// fused rank-only encode against the materialized encode on the same deep
/// shapes.
#[test]
fn ranked_fused_walk_survives_deep_cancellation() {
    let left = stairs(800, true, "(0, 1, 0)");
    let right = stairs(800, false, "(0, 1, 0)");
    // The same mirror with its deepest step split: a rank-3/8 core instead of
    // 1/2, so the total drops by exactly 2⁻⁸⁰³ after 800 levels of
    // cancellation.
    let shallower = stairs(800, false, "(0, (0, 1, (0, 1, 0)), 0)");
    assert_ne!(left, right, "the mirrors are distinct versions");
    assert!(left.concurrent(&right), "and concurrent");
    for (a, b) in [(&left, &right), (&left, &shallower), (&right, &shallower)] {
        let want = a
            .rank()
            .cmp(&b.rank())
            .then_with(|| a.as_bytes().cmp(b.as_bytes()));
        assert_eq!(Ranked::from(a).cmp(&Ranked::from(b)), want);
        assert_eq!(Ranked::from(b).cmp(&Ranked::from(a)), want.reverse());
        assert_eq!(Ranked::from(a).encode_rank(), a.rank().encode());
    }
    assert_eq!(
        Ranked::from(&left).cmp(&Ranked::from(&right)),
        left.as_bytes().cmp(right.as_bytes()),
        "the mirror pair's rank tie falls to the byte tiebreak"
    );
    assert_eq!(left.rank(), right.rank(), "the mirrors tie by symmetry");
    assert!(
        shallower.rank() < right.rank(),
        "the split step decides alone"
    );
}

// ─────────────────────── the composite ranked key ───────────────────────

// Version-encoding prefix-freedom is the `version_encoding_is_prefix_free`
// law in `laws::VERSION_PAIR`, driven over arbitrary normal forms, organic
// op-trace populations, and the fuzz target's decoded values — exactly the
// population where a prefix-aliasing bug would live. The growth-seam
// witnesses below stay: extreme depth past any generator's reach.

/// Committed witnesses for version-encoding prefix-freedom at the growth seam:
/// chains where one version's stream extends another's structure — the shapes
/// most likely to share a long byte prefix.
///
/// A tick chain (each version one event past the last), a spine tower (each one
/// level deeper, out to 800 levels — extreme depth past the arb generator's
/// reach), the 800-level mirror staircases, and the equal-rank concurrent pair
/// are checked pairwise: no encoding is a byte prefix of any other's.
#[test]
fn version_encoding_is_prefix_free_on_growth_chains() {
    let mut battery: Vec<Version> = Vec::new();
    let mut clock = Clock::seed();
    let mut b = clock.fork();
    for _ in 0..4 {
        clock.tick();
        battery.push(clock.version().clone());
        b.tick();
        clock.join(b).unwrap();
        battery.push(clock.version().clone());
        b = clock.fork();
    }
    for depth in [0usize, 1, 2, 3, 8, 200, 201, 800] {
        let mut text = String::from("1");
        for _ in 0..depth {
            text = format!("(0, {text}, 0)");
        }
        battery.push(text.parse().unwrap());
    }
    // The 800-level staircase and its mirror: extreme depth past any
    // generator's reach, sharing a rank by symmetry — the deep genre whose
    // streams extend structure level by level.
    battery.push(stairs(800, true, "(0, 1, 0)"));
    battery.push(stairs(800, false, "(0, 1, 0)"));
    battery.push("(0, 1, 0)".parse().unwrap());
    battery.push("(0, (0, 1, 0), (0, 0, 1))".parse().unwrap());
    battery.push(Version::new());
    for (i, a) in battery.iter().enumerate() {
        for b in &battery[i + 1..] {
            if a == b {
                continue;
            }
            assert!(
                !a.as_bytes().starts_with(b.as_bytes()) && !b.as_bytes().starts_with(a.as_bytes()),
                "prefix-free: {a} vs {b}"
            );
        }
    }
}

proptest! {
    /// The composite `Ranked` key is suffix-safe: distinct versions' keys are
    /// never byte prefixes of one another.
    ///
    /// So a key built as `Ranked::encode ++ payload tag` orders by the view's
    /// total order under every choice of appended bytes.
    ///
    /// Rank-unequal pairs are decided inside the rank component (its own
    /// committed prefix-freedom), and rank-equal pairs fall through
    /// byte-identical rank prefixes to the version component (prefix-free by
    /// the `version_encoding_is_prefix_free` law) — this pins the composition
    /// of the two arguments over arbitrary normal-form pairs with arbitrary
    /// suffix bytes on both keys.
    #[test]
    fn ranked_composite_encoding_is_suffix_safe(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        suffix_a in proptest::collection::vec(any::<u8>(), 0..5),
        suffix_b in proptest::collection::vec(any::<u8>(), 0..5),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let (ra, rb) = (Ranked::from(&a), Ranked::from(&b));
        let (ea, eb) = (ra.encode(), rb.encode());
        if a != b {
            prop_assert!(
                !eb.starts_with(&ea) && !ea.starts_with(&eb),
                "prefix-free: {} vs {}", a, b
            );
        }
        let key_a = [ea, suffix_a].concat();
        let key_b = [eb, suffix_b].concat();
        match ra.cmp(&rb) {
            Ordering::Less => prop_assert!(key_a < key_b, "{} vs {}", a, b),
            Ordering::Greater => prop_assert!(key_a > key_b, "{} vs {}", a, b),
            Ordering::Equal => {}
        }
    }
}

/// Committed witnesses for composite-key suffix safety at the tiebreak seam:
/// pairs whose keys agree byte-for-byte through the whole rank component, so
/// the order is decided inside the version tail.
///
/// The equal-rank concurrent pair (half vs. the two-peak tree), the empty
/// version against half (the zero rank's one-byte stream against a fractional
/// one — decided in the rank component, with the version tail present on both),
/// a tick chain pair, and the 800-level mirror staircases (rank-equal at
/// extreme depth, so the keys agree through a rank prefix hundreds of bytes
/// long before the version tail decides). For each pair the full strength is
/// asserted directly: neither key is a byte prefix of the other, byte order
/// equals the views' total order, and the worst suffixes (`0xFF` on the smaller
/// key, `0x00` on the larger) cannot flip it.
#[test]
fn ranked_composite_key_is_suffix_safe_at_the_tiebreak_seam() {
    let half: Version = "(0, 1, 0)".parse().unwrap();
    let peaks: Version = "(0, (0, 1, 0), (0, 0, 1))".parse().unwrap();
    assert_eq!(half.rank(), peaks.rank(), "the seam pair shares a rank");
    let mut clock = Clock::seed();
    let one = clock.tick().clone();
    let two = clock.tick().clone();
    let deep_left = stairs(800, true, "(0, 1, 0)");
    let deep_right = stairs(800, false, "(0, 1, 0)");
    assert_eq!(
        deep_left.rank(),
        deep_right.rank(),
        "the deep mirrors tie by symmetry"
    );
    let pairs: [(&Version, &Version); 4] = [
        (&half, &peaks),
        (&Version::new(), &half),
        (&one, &two),
        (&deep_left, &deep_right),
    ];
    for (a, b) in pairs {
        let want = Ranked::from(a).cmp(&Ranked::from(b));
        assert_ne!(want, Ordering::Equal, "the witness pair is ordered");
        let (small, large) = match want {
            Ordering::Less => (a, b),
            _ => (b, a),
        };
        let (es, el) = (Ranked::from(small).encode(), Ranked::from(large).encode());
        assert!(
            !el.starts_with(&es) && !es.starts_with(&el),
            "prefix-free: {small} vs {large}"
        );
        assert!(es < el, "byte order is the total order: {small} vs {large}");
        let key_small = [es, vec![0xFF; 4]].concat();
        let key_large = [el, vec![0x00; 4]].concat();
        assert!(
            key_small < key_large,
            "no suffix flips the order: {small} vs {large}"
        );
    }
}

/// Committed witnesses, one per rejection genre `Ranked::decode` adds over its
/// components' own.
///
/// Empty input; truncation at every byte boundary of a composite (cuts land in
/// the rank stream, at the component seam, and inside the version); a trailing
/// zero byte (the version component's whole-input strictness); a set bit in the
/// version's padding; and a well-formed rank prefix paired with a version it
/// does not measure, witnessed from both sides of the true rank (the
/// composite's redundancy check — `NotCanonical`, the genre for well-formed
/// structure that is the canonical spelling of no value). The components'
/// interior genres are their own suites' business
/// (`rank_decoding_rejects_each_genre` and the codec suite's rejection
/// battery); the cuts here prove each component's rejection surfaces through
/// the composite entry.
#[test]
fn ranked_decode_rejects_each_genre() {
    use crate::error::Decode;
    let half: Version = "(0, 1, 0)".parse().unwrap();
    let key = Ranked::from(&half).encode();
    assert!(
        matches!(Ranked::decode(&[][..]), Err(Decode::Truncated)),
        "empty input"
    );
    for cut in 0..key.len() {
        assert!(
            matches!(Ranked::decode(&key[..cut]), Err(Decode::Truncated)),
            "cut at byte {cut}"
        );
    }
    let padded = [key.clone(), vec![0]].concat();
    assert!(
        matches!(Ranked::decode(&padded[..]), Err(Decode::TrailingBits)),
        "trailing zero byte"
    );
    assert_ne!(
        half.encoded_bits() % 8,
        0,
        "the witness's version tail must end mid-byte, so its final \
         byte carries padding"
    );
    let mut set_padding = key.clone();
    *set_padding.last_mut().unwrap() |= 0x01;
    assert!(
        matches!(Ranked::decode(&set_padding[..]), Err(Decode::TrailingBits)),
        "set bit in the version padding"
    );
    // A rank the version does not measure, from both sides of the true rank
    // (the verification is an equality, not an ordering): rank(5) over half's
    // bytes, and half's rank (1/2) over five's bytes.
    let five = Version::try_from(5).unwrap();
    let above = [five.rank().encode(), half.as_bytes().to_vec()].concat();
    assert!(
        matches!(Ranked::decode(&above[..]), Err(Decode::NotCanonical)),
        "rank prefix above the true rank"
    );
    let below = [half.rank().encode(), five.as_bytes().to_vec()].concat();
    assert!(
        matches!(Ranked::decode(&below[..]), Err(Decode::NotCanonical)),
        "rank prefix below the true rank"
    );
}

proptest! {
    /// Flipping any single bit of a canonical composite key yields a byte
    /// string `Ranked::decode` either rejects or accepts canonically (the
    /// accepted view re-encodes to exactly the mutated input).
    ///
    /// Acceptance-canonicity is what keeps decode injective on bytes and byte
    /// equality on keys exactly [`Eq`] on views.
    ///
    /// The codec suite's mutation genre, aimed at the composite's own seam: a
    /// flip in the self-delimiting rank prefix can move where the version parse
    /// begins, and the accepted language must still contain only canonical
    /// keys. The rank-against-version verification makes any accept a needle's
    /// eye (the flipped rank must be exactly what the reparsed version
    /// measures), so in practice every flip rejects; the disjunction is the
    /// contract, and it also holds if a flip ever lands on another view's key.
    #[test]
    fn ranked_composite_bit_flip_rejects_or_decodes_canonically(oa in arb_oracle_version()) {
        let v = from_oracle_version(&oa);
        let key = Ranked::from(&v).encode();
        for byte in 0..key.len() {
            for bit in 0..8u8 {
                let mut mutated = key.clone();
                mutated[byte] ^= 0x80 >> bit;
                if let Ok(view) = Ranked::decode(&mutated[..]) {
                    prop_assert_eq!(
                        view.encode(),
                        mutated,
                        "accepted mutation must re-encode to itself: {} byte {} bit {}",
                        v, byte, bit
                    );
                }
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

/// The view and its materialization agree, and projecting onto a party disjoint
/// from where the events happened keeps nothing — lazily and materialized
/// alike.
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

/// The at-rest form is exactly the wire bytes' refcounted handle.
///
/// A [`Version`] is exactly one `codec::Bits` — the refcounted buffer handle
/// alone: pointer, byte length, shared-state pointer, vtable — 32 bytes on
/// 64-bit, and a [`Clock`](crate::Clock) is a `Party` plus a `Version` (64). A
/// regression here means the storage grew a field beside the container: the
/// live bit length must stay recoverable from the padding marker inside the
/// bytes, never cached beside them.
#[test]
fn at_rest_size_is_one_container_per_stream() {
    assert_eq!(
        core::mem::size_of::<Version>(),
        core::mem::size_of::<bytes::Bytes>()
    );
    assert_eq!(core::mem::size_of::<Version>(), 32);
    assert_eq!(core::mem::size_of::<crate::Clock>(), 64);
}

proptest! {
    /// Byte-level equality (`codec::canonical_eq`) agrees with a plain
    /// bit-level compare of the live streams, in both operand orders.
    ///
    /// The cross-check that the canonical-padding invariant (the marker sealed
    /// at every storage seam) really makes the raw bytes injective, licensing
    /// the byte-compare shortcut. Equal values must also hash equally
    /// (`Eq`/`Hash` consistency).
    #[test]
    fn byte_equality_matches_bit_equality(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let bit_eq = a.as_bits().to_buf() == b.as_bits().to_buf();
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

proptest! {
    /// The identity-law fast paths agree with the walked paths across
    /// buffer identity.
    ///
    /// Every equal-operand shortcut answers identically on a clone (shared
    /// buffer, the clone-identity rung) and on a byte-equal re-build in a
    /// distinct buffer (the byte compare or the full walk) — comparison
    /// `Equal`, join and meet the value itself, distance and lag zero, and the
    /// hull coincident.
    #[test]
    fn identity_fast_paths_agree_across_buffer_identity(oa in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let clone = a.clone();
        let distinct = from_oracle_version(&oa);
        for b in [&clone, &distinct] {
            prop_assert_eq!(a.partial_cmp(b), Some(Ordering::Equal));
            prop_assert_eq!(&(&a | b), &a);
            prop_assert_eq!(&(&a & b), &a);
            prop_assert_eq!(a.distance(b), crate::Rank::ZERO);
            prop_assert_eq!(a.lag(b), crate::Rank::ZERO);
            let hull = a.span(b);
            prop_assert_eq!(hull.lo(), &a);
            prop_assert_eq!(hull.hi(), &a);
        }
    }
}

proptest! {
    /// The n-ary folds' adjacent clone collapse is value-invisible.
    ///
    /// Folding a population with each element expanded into an adjacent run of
    /// clones equals folding the population itself, for `join_all`, `meet_all`,
    /// and `span_all` (idempotence makes a run one operand; the collapse must
    /// change no verdict).
    #[test]
    fn fold_clone_collapse_is_value_invisible(
        ovs in proptest::collection::vec(arb_oracle_version(), 1..5),
        reps in proptest::collection::vec(1usize..4, 5),
    ) {
        let vs: Vec<Version> = ovs.iter().map(from_oracle_version).collect();
        let dup: Vec<Version> = vs
            .iter()
            .zip(reps.iter().cycle())
            .flat_map(|(v, &r)| std::iter::repeat_with(|| v.clone()).take(r))
            .collect();
        prop_assert_eq!(vs[0].join_all(&dup), vs[0].join_all(&vs));
        prop_assert_eq!(vs[0].meet_all(&dup), vs[0].meet_all(&vs));
        prop_assert_eq!(vs[0].span_all(&dup), vs[0].span_all(&vs));
    }
}

proptest! {
    /// The composite row-key shape — a rank's canonical stream, then a
    /// fixed-width opaque key — orders by first difference exactly as `(rank,
    /// suffix)` orders lexicographically.
    ///
    /// The KV-key use `Rank::encode` documents, exercised in context: distinct
    /// ranks decide the composite inside the rank prefix (no 32-byte suffix can
    /// flip it, in either assignment), and equal ranks — byte-identical
    /// prefixes, by canonical uniqueness — hand the verdict to the suffix's own
    /// first differing byte.
    #[test]
    fn rank_prefix_orders_fixed_suffix_row_keys(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        sa in proptest::array::uniform32(any::<u8>()),
        sb in proptest::array::uniform32(any::<u8>()),
    ) {
        let a = from_oracle_version(&oa).rank();
        let b = from_oracle_version(&ob).rank();
        let key_a = [a.encode(), sa.to_vec()].concat();
        let key_b = [b.encode(), sb.to_vec()].concat();
        let expect = a.cmp(&b).then_with(|| sa.cmp(&sb));
        prop_assert_eq!(key_a.cmp(&key_b), expect, "{} vs {}", a, b);
        // Both assignments: the swap must invert exactly.
        let swapped_a = [a.encode(), sb.to_vec()].concat();
        let swapped_b = [b.encode(), sa.to_vec()].concat();
        let expect = a.cmp(&b).then_with(|| sb.cmp(&sa));
        prop_assert_eq!(swapped_a.cmp(&swapped_b), expect, "{} vs {}", a, b);
    }
}

/// Fan-shaped operand sets at counter-boundary arities fold to the sequential
/// pair fold, with adjacent clones and empties interleaved.
///
/// Three separately-tested mechanisms meet in one deterministic construction:
/// the balanced binary counter (whose grouping diverges most from the
/// sequential fold at arities that fill or straddle a counter level — k = 4 and
/// k = 6), the run-dedup adapter (driven by an adjacent clone run), and the
/// empty-operand identity rungs. Each operand is one tick on its own fork of
/// one seed, so every pair is concurrent and no combine short-circuits; on the
/// same input list the sequential fold reads verbatim, and `span_all`'s two
/// legs agree.
#[test]
fn boundary_arity_fan_folds_match_the_sequential_fold() {
    for k in [4usize, 6] {
        // k concurrent single-tick versions on k disjoint forks.
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < k {
            let next = clocks.last_mut().expect("nonempty").fork();
            clocks.push(next);
        }
        let fan: Vec<Version> = clocks
            .iter_mut()
            .map(|c| {
                c.tick();
                c.version().clone()
            })
            .collect();

        // The raw fan, and the fan salted with an adjacent clone run and an
        // empty version (idempotence and identity make both value-invisible;
        // the machinery they exercise differs).
        let mut salted = fan.clone();
        salted.insert(1, fan[0].clone()); // adjacent clone: dedup fires
        salted.insert(1, fan[0].clone()); // a run of three total
        salted.push(Version::new()); // identity rung on the drain side
        for pool in [&fan, &salted] {
            let (first, rest) = pool.split_first().expect("nonempty pool");
            let join_seq = pool.iter().fold(Version::new(), |acc, v| acc | v);
            assert_eq!(
                first.join_all(rest),
                join_seq,
                "join_all diverged from the sequential fold at k={k}",
            );
            let meet_seq = pool
                .iter()
                .cloned()
                .reduce(|acc, v| acc & v)
                .expect("nonempty pool");
            assert_eq!(
                first.meet_all(rest),
                meet_seq,
                "meet_all diverged from the sequential fold at k={k}",
            );
            let hull = pool[0].span_all(pool[1..].iter());
            assert_eq!(hull.lo(), &meet_seq, "span_all meet leg at k={k}");
            assert_eq!(hull.hi(), &join_seq, "span_all join leg at k={k}");
        }
    }
}

/// The cheapest canonical deep spine costs exactly 3 stored bits per
/// marginal level.
///
/// This pins the grammar's depth-to-size exchange rate: every level a stream
/// reaches is paid for by stored bits, so depth-derived quantities (the rank
/// exponent among them) stay linear in the input the caller already holds. If
/// the grammar ever admits a cheaper per-level spelling, this pin moves and any
/// prose pricing depth in input bytes must be re-derived with it.
#[test]
fn deep_spine_marginal_cost_is_three_bits_per_level() {
    let spine = |depth: usize| -> usize {
        let mut text = String::new();
        for _ in 0..depth {
            text.push_str("(0, 1, ");
        }
        text.push('0');
        text.push_str(&")".repeat(depth));
        let v: Version = text.parse().expect("the unit-leaf deep spine is canonical");
        v.encode().len() * 8
    };
    let (small, large) = (spine(1_000), spine(2_000));
    assert_eq!(
        large - small,
        3 * 1_000,
        "the deep spine's marginal level cost moved off 3 bits: re-derive \
         the query depth-guard size prose from the new grammar"
    );
}
