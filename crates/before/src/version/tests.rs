//! Version tests.
//!
//! The causal order and its comparison matrix, the join/meet operator matrices
//! and lattice laws, grow optimality against the brute-force reference,
//! `min_ticks`, and projection (`/`).

use crate::testing::meter::registry::Shape;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::rc::Rc;

use num_bigint::BigUint;
use proptest::prelude::*;
use proptest::sample::Index;

use super::{Ranked, Version};
use crate::bits::BitsWriter;
use crate::error::Decode;
use crate::rank::DECODE_CHUNK_BYTES;
use crate::testing::bridge::{from_oracle_party, from_oracle_version, to_oracle_version};
use crate::testing::canonical::Canonical;
use crate::testing::generators::co_generated::arb_spine_case;
use crate::testing::generators::{
    arb_dyadic, arb_oracle_party_nonempty, arb_oracle_version, arb_related_dyadic,
};
use crate::testing::grow_brute_force::{all_inflations, best_inflation};
use crate::testing::optrace::{
    leq as oracle_leq, production_strategy, run, step_production, versions, world_strategy, Arity,
    Production, ProductionModel, TraceModel,
};
use crate::testing::oracles::dyadic::Dyadic;
use crate::testing::oracles::function;
use crate::testing::oracles::tree;
use crate::testing::scripted_io::{arb_io_schedule, IoSchedule, ScriptedReader, ScriptedWriter};
use crate::{Clock, Count, Party, Rank};

/// Build a uniform version through the public tick operation.
fn uniform(ticks: impl Into<Count>) -> Version {
    let mut version = Version::new();
    Party::seed().ticks(&mut version, ticks);
    version
}

/// Builds a version whose left half is one tick ahead.
fn half() -> Version {
    use crate::testing::oracles::tree::Version as V;
    from_oracle_version(&V::node(0u8, V::leaf(1u8), V::leaf(0u8)))
}

/// Builds two opposite quarter-height peaks with the same rank as [`half`].
fn peaks() -> Version {
    use crate::testing::oracles::tree::Version as V;
    from_oracle_version(&V::node(
        0u8,
        V::node(0u8, V::leaf(1u8), V::leaf(0u8)),
        V::node(0u8, V::leaf(0u8), V::leaf(1u8)),
    ))
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
/// the four ordering operators supplied by the same [`PartialOrd`] impl.
/// Generic over the operand
/// types, so each call resolves to exactly the impl for `(L, R)`
/// (`assert_cmp_cell(&a, b, ..)` exercises the `&Lhs`/`Rhs` cell,
/// `assert_cmp_cell(a, &b, ..)` the `Lhs`/`&Rhs` cell, `&`/`&` the std blanket),
/// with no method-resolution ambiguity to mask which cell ran. A cell wired
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
    use crate::testing::oracles::tree::Version as V;
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
    /// The join's stored size does not exceed the operands' combined stored
    /// size.
    ///
    /// Every output boundary belongs to an input, and its delta lies between
    /// the two input deltas there. Its code is therefore no wider than the
    /// wider input code; the opening height comes from one input and collapse
    /// only removes data. Probed here over churned
    /// op-trace populations: the causally related pairs live
    /// replicas hold, including the normalization corners churn produces.
    #[test]
    fn join_stored_size_is_subadditive(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let join = &a | &b;
        prop_assert!(
            join.as_bytes().len() <= a.as_bytes().len() + b.as_bytes().len(),
            "join stored size outgrew its inputs: {} > {} + {}",
            join.as_bytes().len(), a.as_bytes().len(), b.as_bytes().len(),
        );
    }
}

proptest! {
    /// The meet's stored size does not exceed the operands' combined stored
    /// size.
    ///
    /// Dual to [`join_stored_size_is_subadditive`]: the same boundary-delta
    /// argument applies to pointwise minimum. Probed over the same churned
    /// populations as the join lemma.
    #[test]
    fn meet_stored_size_is_subadditive(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        let meet = &a & &b;
        prop_assert!(
            meet.as_bytes().len() <= a.as_bytes().len() + b.as_bytes().len(),
            "meet stored size outgrew its inputs: {} > {} + {}",
            meet.as_bytes().len(), a.as_bytes().len(), b.as_bytes().len(),
        );
    }
}

proptest! {
    /// Every assigning join surface on `Version` yields the same result as `a |
    /// b`, which the `version_join_matches_the_oracle` descriptor already pins
    /// to the oracle's `join`.
    ///
    /// Covers `Version |= Version` and `Version |= &Version`, neither of which
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
    /// The full `|=` (BitOrAssign) matrix, owned and borrowed right operands,
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
    /// The full `&=` (BitAndAssign) matrix, owned and borrowed right operands,
    /// lands on the oracle's `meet`, dual to
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
// `span_operator_matrix_is_the_method` in `crate::testing::laws`), driven over
// arbitrary normal forms, these op-trace populations, and the fuzz target's
// decoded values.

proptest! {
    /// Lattice identity for join, byte-identical: `0 | v == v == v | 0`.
    ///
    /// The encoded bytes equal `v`'s own through both the public empty-operand
    /// shortcut and the general merge walk.
    #[test]
    fn join_identity_byte_parity(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let vs = versions(&cs);
        let n = vs.len();
        let v = from_oracle_version(&vs[i % n]);
        let empty = Version::new();

        // The general path, bypassing the short-circuit: the merge kernel on
        // the identity cases lands on `v`'s canonical bytes.
        let general_left = empty.join(&v);
        let general_right = v.join(&empty);
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
    /// The encoded bytes equal `Version::new()`'s, both through the
    /// public empty-operand shortcut and through the general merge walk. Dual to
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
        let general_left = empty.meet(&v);
        let general_right = v.meet(&empty);
        prop_assert_eq!(general_left.encode(), empty.encode());
        prop_assert_eq!(general_right.encode(), empty.encode());

        // The empty version on either side of the operator (the short-circuit).
        prop_assert_eq!((&empty & &v).encode(), empty.encode());
        prop_assert_eq!((&v & &empty).encode(), empty.encode());
    }
}

// The lattice, order, tick, and projection laws on impl values live in
// `crate::testing::laws` and are driven by the algebraic-laws suite over both arbitrary
// normal forms and these same op-trace populations; this file keeps the
// differential and mechanism-level tests.

// ───────────────────────────── path-sum overflow regression ─────────────────────────────

/// A normal-form tree whose root-to-leaf path sum exceeds `u64::MAX` compares
/// correctly.
///
/// With arbitrary-precision leaf heights there is no overflow class, so the
/// answer is `Greater` in every build profile, so comparison must thread the
/// heights at full precision.
#[test]
fn path_sum_beyond_u64_compares_greater() {
    use crate::testing::oracles::tree::Version as V;
    let big = 1u64 << 63;
    // Normal form: the outer min(big, 0) child is the right `0` leaf; the inner
    // node's min(0, 1) child is its left `0` leaf. The left half's true value
    // is big + big + 1 = 2^64 + 1, past `u64::MAX`.
    let a = from_oracle_version(&V::node(
        big,
        V::node(big, V::leaf(0u64), V::leaf(1u64)),
        V::leaf(0u64),
    ));
    let b = uniform(big);
    assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater));
}

/// A stored leaf height above `u64::MAX` stays exact across mutation and merge.
/// This pins the arbitrary-width payload path at the machine-word spill
/// boundary, not only path sums made from individually-small nodes.
#[test]
fn stored_base_beyond_u64_ticks_and_merges() {
    let height = BigUint::from(1u8) << 64u32;
    let big = from_oracle_version(&crate::testing::oracles::tree::Version::leaf(
        height.clone(),
    ));
    let mut ticked = big.clone();
    ticked.tick(&Party::seed());

    assert_eq!(
        ticked,
        from_oracle_version(&crate::testing::oracles::tree::Version::leaf(
            height + BigUint::from(1u8),
        ))
    );
    assert_eq!(big.clone() | ticked.clone(), ticked);
    assert_eq!(Version::decode(&ticked.encode()[..]).unwrap(), ticked);
}

// ───────────── arbitrary normal-form trees (decoupled from the op pipeline) ─────────────
//
// The op-trace differentials above only ever compare causally *related*
// versions (every member descends from one seed) on the *shapes operations
// produce*. These feed *arbitrary* normal-form event trees (random shape,
// random base magnitudes including values near/beyond `u64::MAX`) to every
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
        let walk_eq = a.partial_cmp(&b) == Some(Ordering::Equal);

        prop_assert_eq!(a == b, walk_eq);
        // The equality direction: a version equals its own clone.
        prop_assert!(a == a.clone());
    }
}

proptest! {
    /// The join-size lemma of [`join_stored_size_is_subadditive`], on arbitrary,
    /// typically *unrelated* normal-form pairs.
    ///
    /// The churned generator only produces causally related versions from one
    /// seed; these pairs add independent shapes and large-base leaves (values
    /// near/beyond `u64::MAX`), where a join must restructure most, the corner
    /// where subadditivity would break if normalization could ever inflate a
    /// combined tree past its inputs.
    #[test]
    fn join_stored_size_is_subadditive_arbitrary(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let join = &a | &b;
        prop_assert!(
            join.as_bytes().len() <= a.as_bytes().len() + b.as_bytes().len(),
            "join stored size outgrew its inputs: {} > {} + {}",
            join.as_bytes().len(), a.as_bytes().len(), b.as_bytes().len(),
        );
    }
}

proptest! {
    /// The meet-size lemma of [`meet_stored_size_is_subadditive`], on arbitrary,
    /// typically *unrelated* normal-form pairs.
    ///
    /// Dual to [`join_stored_size_is_subadditive_arbitrary`], and for the same
    /// reason: independent shapes and large-base leaves are the corner where a
    /// meet must restructure most, so this is where subadditivity would break
    /// if normalization could ever inflate a combined tree past its inputs.
    #[test]
    fn meet_stored_size_is_subadditive_arbitrary(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let meet = &a & &b;
        prop_assert!(
            meet.as_bytes().len() <= a.as_bytes().len() + b.as_bytes().len(),
            "meet stored size outgrew its inputs: {} > {} + {}",
            meet.as_bytes().len(), a.as_bytes().len(), b.as_bytes().len(),
        );
    }
}

// ──────────────────────────── exact result sizing ────────────────────────────

/// The deepest event tree the sizing families build.
const MAX_SIZING_DEPTH: usize = 2_048;

/// The rightmost `2^-depth` cell of party space.
fn rightmost_cell(depth: usize) -> Party {
    let mut bits = BitsWriter::new();
    for _ in 0..depth {
        bits.push(false);
        bits.push(true);
    }
    bits.push(false);
    bits.push(false);
    Party::from_test_bits(bits)
}

/// `base` events everywhere, and two more on the rightmost `2^-depth` cell:
/// an event tree as deep as the cell.
fn deep_bump(base: u64, depth: usize) -> Version {
    let mut version = uniform(base);
    version.ticks(&rightmost_cell(depth), 2u8);
    version
}

/// Describe `version`'s spare capacity, if its buffer holds more than its
/// encoded bytes.
///
/// The empty version's static storage has no allocation to inspect. The
/// caller must already have dropped every other value sharing the result's
/// buffer. The families collect every case before asserting, so one failure
/// reports each operation that keeps spare capacity.
fn spare_capacity(case: &str, version: Version) -> Option<String> {
    if version.ptr_eq(&Version::new()) {
        return None;
    }
    let encoded = version.as_bytes().len();
    let capacity = version.0.allocation_capacity();
    (capacity != encoded)
        .then(|| format!("{case}: a {encoded}-byte version keeps a {capacity}-byte buffer"))
}

proptest! {
    /// Each operation that collapses a deep event tree to one leaf returns
    /// that leaf in a buffer of exactly its encoded length.
    ///
    /// The tree collapses when joined with a uniform version above it, when
    /// met with a uniform version below it, and when ticked by the seed.
    #[test]
    fn collapsing_version_results_are_sized_exactly(depth in 1..=MAX_SIZING_DEPTH) {
        let mut spare = Vec::new();

        let join = &deep_bump(0, depth) | &uniform(3u8);
        prop_assert_eq!(&join, &uniform(3u8));
        spare.extend(spare_capacity("join", join));

        let join_all = deep_bump(0, depth).join_all([uniform(3u8)]);
        prop_assert_eq!(&join_all, &uniform(3u8));
        spare.extend(spare_capacity("join_all", join_all));

        let meet = &deep_bump(1, depth) & &uniform(1u8);
        prop_assert_eq!(&meet, &uniform(1u8));
        spare.extend(spare_capacity("meet", meet));

        let meet_all = deep_bump(1, depth).meet_all([uniform(1u8)]);
        prop_assert_eq!(&meet_all, &uniform(1u8));
        spare.extend(spare_capacity("meet_all", meet_all));

        let mut ticked = deep_bump(0, depth);
        ticked.tick(&Party::seed());
        prop_assert_eq!(&ticked, &uniform(2u8));
        spare.extend(spare_capacity("tick", ticked));

        prop_assert!(spare.is_empty(), "{}", spare.join("; "));
    }

    /// Every version that the lattice operations, `tick`, `ticks`, and
    /// projection build is held in a buffer of exactly its encoded length,
    /// over arbitrary operands.
    #[test]
    fn built_versions_are_sized_exactly(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        party in arb_oracle_party_nonempty(),
        count in 0u64..24,
    ) {
        // Each result is bound in its own statement, so the operand
        // temporaries are gone before the result's buffer is inspected.
        let version = from_oracle_version;
        let party = from_oracle_party(&party);
        let mut spare = Vec::new();

        let join = &version(&oa) | &version(&ob);
        spare.extend(spare_capacity("join", join));
        let meet = &version(&oa) & &version(&ob);
        spare.extend(spare_capacity("meet", meet));
        let join_all = version(&oa).join_all([version(&ob)]);
        spare.extend(spare_capacity("join_all", join_all));
        let meet_all = version(&oa).meet_all([version(&ob)]);
        spare.extend(spare_capacity("meet_all", meet_all));
        let mut joined = version(&oa);
        joined |= &version(&ob);
        spare.extend(spare_capacity("join in place", joined));
        let mut met = version(&oa);
        met &= &version(&ob);
        spare.extend(spare_capacity("meet in place", met));

        let mut ticked = version(&oa);
        ticked.tick(&party);
        spare.extend(spare_capacity("tick", ticked));
        let mut ticked = version(&oa);
        ticked.ticks(&party, count);
        spare.extend(spare_capacity("ticks", ticked));

        let projection = (&version(&oa) / &party).to_version();
        spare.extend(spare_capacity("projection", projection));

        prop_assert!(spare.is_empty(), "{}", spare.join("; "));
    }
}

// ───────────── grow optimality, impl side ─────────────
//
// The defining causality property (§3, §5.3.4): an event registers a *minimal*
// inflation. The oracle's `grow` is pinned to a brute-force search over the
// entire feasible inflation space in `tree::tests`; these hold the encoded
// impl to the same standard. `tick = fill else grow`, so when `fill` already
// simplifies the tree the grow path is not taken; both tests draw their inputs
// from `arb_grow_case`, which constructs only grow-branch inputs.
// `grow_matches_brute_force` asserts the impl's inflation equals the
// brute-force right-favoring minimum; `grow_minimal` checks the paper's
// metamorphic condition.

/// Generates a party and a version on which `tick` takes the `grow` branch,
/// that is, a version `fill` leaves unchanged.
///
/// The version is an arbitrary one filled once by the party. `fill` is
/// idempotent, so its image is exactly its fixed points: every grow-branch
/// input is reachable (as its own image), and every generated input reaches
/// `grow`. Constructing the domain this way, rather than discarding
/// non-fixed points with `prop_assume!`, keeps the tests within proptest's
/// global reject limit at every case count. The tests assert the fixed-point
/// property, so a non-idempotent `fill` fails them rather than silently
/// testing the `fill` branch.
fn arb_grow_case() -> impl Strategy<Value = (tree::Party, tree::Version)> {
    (arb_oracle_party_nonempty(), arb_oracle_version()).prop_map(|(op, ov)| {
        let filled = ov.fill_for_test(&op);
        (op, filled)
    })
}

proptest! {
    /// `tick` and fused `ticks` choose exactly the recursive oracle's
    /// inflation over arbitrary canonical parties and versions.
    ///
    /// The function-space model may choose any valid inflation, so the shared
    /// three-model trace compares causal observations instead of requiring
    /// identical representations. This focused two-model property pins the
    /// stronger implementation contract: production uses the recursive
    /// oracle's minimal, right-favoring policy. The master clock differential
    /// exercises the same equality after every step of organic traces.
    #[test]
    fn tick_and_ticks_match_the_recursive_oracle(
        party in arb_oracle_party_nonempty(),
        version in arb_oracle_version(),
        count in 0u64..24,
    ) {
        let production_party = from_oracle_party(&party);

        let mut production_tick = from_oracle_version(&version);
        production_tick.tick(&production_party);
        let mut recursive_tick = version.clone();
        recursive_tick.tick(&party);
        prop_assert_eq!(to_oracle_version(&production_tick), recursive_tick);

        let mut production_ticks = from_oracle_version(&version);
        production_ticks.ticks(&production_party, count);
        let mut recursive_ticks = version;
        recursive_ticks.ticks(&party, Count::from(count));
        prop_assert_eq!(to_oracle_version(&production_ticks), recursive_ticks);
    }
}

proptest! {
    /// When `tick` takes the `grow` branch (`fill` leaves the tree unchanged),
    /// the impl inflates exactly the brute-force cost-minimal, right-favoring
    /// region: `tick` lowered to the oracle equals `best_inflation` normalized.
    ///
    /// This holds the encoded `grow`'s dynamic program to the full-enumeration
    /// global optimum directly, not merely to the recursive oracle (which
    /// realizes the same DP). Large bases are threaded losslessly, so the cost
    /// comparison is exact regardless of magnitude.
    #[test]
    fn grow_matches_brute_force((op, ov) in arb_grow_case()) {
        // Only the grow path is under test (the `fill` path is covered by the
        // tick/fill differentials); `tick` takes it iff `fill` is a no-op.
        prop_assert_eq!(ov.fill_for_test(&op), ov.clone(), "input must reach grow");

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
    /// pointwise lattice is false even for a single increment; see the oracle
    /// twin `grow_dominates_no_more_than_needed`). Run on the impl's own causal
    /// order, with the candidate set enumerated by the brute-force oracle.
    /// Cross-checked against the oracle order on the same values.
    #[test]
    fn grow_minimal((op, ov) in arb_grow_case()) {
        prop_assert_eq!(ov.fill_for_test(&op), ov.clone(), "input must reach grow");

        let e = from_oracle_version(&ov);
        let mut eprime = e.clone();
        eprime.tick(&from_oracle_party(&op)); // grow path: tick == grow

        for (cand, _) in all_inflations(&op, &ov) {
            let cand_norm = cand.normalized_for_test();
            let cand_v = from_oracle_version(&cand_norm);
            let above_e = e <= cand_v;
            let strictly_below = cand_v < eprime;
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
    /// The stored form includes canonical padding, so its bytes are identical
    /// to the encoder's output. Exercises the literal/`extend` construction
    /// path over arbitrary normal-form trees.
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

/// The tick batches in one clock's causal past: each batch's tick count, keyed
/// by a batch number unique within one history.
///
/// A clock's causal past holds its own ticks and every tick it has heard of
/// through a message, a sync, or a join, which together form one history that
/// yields its version.
#[derive(Clone)]
struct CausalPast {
    batches: BTreeMap<u64, Count>,
    /// The history's next unused batch number, shared by every clock split
    /// from its one seed.
    next_batch: Rc<Cell<u64>>,
    /// The ticks the whole history has recorded, shared the same way.
    history_ticks: Rc<RefCell<Count>>,
}

impl CausalPast {
    /// Records one batch of `ticks` ticks under a fresh batch number.
    fn record(&mut self, ticks: Count) {
        let batch = self.next_batch.get();
        self.next_batch.set(batch + 1);
        *self.history_ticks.borrow_mut() += &ticks;
        self.batches.insert(batch, ticks);
    }

    /// Adds every batch in `other` that this past lacks.
    fn absorb(&mut self, other: &CausalPast) {
        self.batches.extend(
            other
                .batches
                .iter()
                .map(|(batch, ticks)| (*batch, ticks.clone())),
        );
    }

    /// The number of ticks in this past.
    fn ticks(&self) -> Count {
        self.batches.values().sum()
    }
}

/// Production clocks paired with their causal pasts, driven through the trace
/// schedule as one model.
///
/// Each operation calls [`Production`]'s and then updates the past:
///
/// - a tick, `ticks(k)`, or a wide tick count records a batch of 1 or `k`;
/// - a fork, `forks`, or array split gives each child or share a copy of the
///   past;
/// - a send records the sender's tick and carries its past, and a receive
///   absorbs that past and records its own tick;
/// - an absorb or `absorb_all` absorbs every source's past and records no
///   tick, since it registers no event;
/// - a sync or `sync_all` leaves every participant holding the union of their
///   pasts, and a join or `join_all` leaves the receiver holding it;
/// - a byte round trip leaves the past unchanged.
struct WithCausalPast;

impl TraceModel for WithCausalPast {
    type Clock = (Clock, CausalPast);
    type Message = (Version, CausalPast);
    type Rng = ();

    fn seed() -> Self::Clock {
        let past = CausalPast {
            batches: BTreeMap::new(),
            next_batch: Rc::new(Cell::new(0)),
            history_ticks: Rc::new(RefCell::new(Count::ZERO)),
        };
        (Production::seed(), past)
    }

    fn tick((clock, past): &mut Self::Clock, rng: &mut ()) {
        Production::tick(clock, rng);
        past.record(Count::from(1u64));
    }

    fn ticks((clock, past): &mut Self::Clock, count: u8, rng: &mut ()) {
        Production::ticks(clock, count, rng);
        past.record(Count::from(u64::from(count)));
    }

    fn fork((clock, past): &mut Self::Clock, rng: &mut ()) -> Self::Clock {
        (Production::fork(clock, rng), past.clone())
    }

    fn forks(
        (clock, past): &mut Self::Clock,
        count: u8,
        taken: usize,
        rng: &mut (),
    ) -> Vec<Self::Clock> {
        Production::forks(clock, count, taken, rng)
            .into_iter()
            .map(|child| (child, past.clone()))
            .collect()
    }

    fn split((clock, past): Self::Clock, arity: Arity, rng: &mut ()) -> Vec<Self::Clock> {
        Production::split(clock, arity, rng)
            .into_iter()
            .map(|share| (share, past.clone()))
            .collect()
    }

    fn send((clock, past): &mut Self::Clock, rng: &mut ()) -> Self::Message {
        let message = Production::send(clock, rng);
        past.record(Count::from(1u64));
        (message, past.clone())
    }

    fn receive((clock, past): &mut Self::Clock, (message, sender): Self::Message, rng: &mut ()) {
        Production::receive(clock, message, rng);
        past.absorb(&sender);
        past.record(Count::from(1u64));
    }

    fn snapshot((clock, past): &Self::Clock) -> Self::Message {
        (Production::snapshot(clock), past.clone())
    }

    fn absorb((clock, past): &mut Self::Clock, (message, source): Self::Message) {
        Production::absorb(clock, message);
        past.absorb(&source);
    }

    fn absorb_all((clock, past): &mut Self::Clock, messages: Vec<Self::Message>) {
        let (messages, sources): (Vec<Version>, Vec<CausalPast>) = messages.into_iter().unzip();
        Production::absorb_all(clock, messages);
        for source in &sources {
            past.absorb(source);
        }
    }

    fn sync(
        (left, left_past): &mut Self::Clock,
        (right, right_past): &mut Self::Clock,
        rng: &mut (),
    ) -> bool {
        let synced = Production::sync(left, right, rng);
        if synced {
            left_past.absorb(right_past);
            right_past.clone_from(left_past);
        }
        synced
    }

    fn sync_all(
        (clock, past): &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        rng: &mut (),
    ) -> bool {
        let (others, other_pasts): (Vec<&mut Clock>, Vec<&mut CausalPast>) = others
            .into_iter()
            .map(|(other, other_past)| (other, other_past))
            .unzip();
        let synced = Production::sync_all(clock, others, rng);
        if synced {
            for other_past in &other_pasts {
                past.absorb(other_past);
            }
            for other_past in other_pasts {
                other_past.clone_from(past);
            }
        }
        synced
    }

    fn join(
        (clock, past): &mut Self::Clock,
        (other, other_past): Self::Clock,
        rng: &mut (),
    ) -> Result<(), Self::Clock> {
        match Production::join(clock, other, rng) {
            Ok(()) => {
                past.absorb(&other_past);
                Ok(())
            }
            Err(other) => Err((other, other_past)),
        }
    }

    /// Absorbs every input's past into the receiver's.
    ///
    /// A failed `join_all` may hand back unions of its inputs and may have
    /// absorbed some of them, so no input's own past can be paired with what
    /// it returns. Each clock it touches then takes the union of every
    /// input's past, which holds each one's true past; the trace's `step`
    /// fails on the error before any past is read.
    fn join_all(
        (clock, past): &mut Self::Clock,
        others: Vec<Self::Clock>,
        rng: &mut (),
    ) -> Result<(), Vec<Self::Clock>> {
        let (others, other_pasts): (Vec<Clock>, Vec<CausalPast>) = others.into_iter().unzip();
        let joined = Production::join_all(clock, others, rng);
        for other_past in &other_pasts {
            past.absorb(other_past);
        }
        joined.map_err(|returned| {
            returned
                .into_iter()
                .map(|other| (other, past.clone()))
                .collect()
        })
    }

    fn transmit((clock, _): &mut Self::Clock) -> bool {
        Production::transmit(clock)
    }

    fn transmit_parts((clock, _): &mut Self::Clock) -> bool {
        Production::transmit_parts(clock)
    }
}

impl ProductionModel for WithCausalPast {
    fn wide_ticks((clock, past): &mut Self::Clock, count: &Count) {
        Production::wide_ticks(clock, count);
        past.record(count.clone());
    }
}

/// `min_ticks` known values: the empty version, a single-party line (= the leaf
/// value), and two concurrent peaks (forced above their tallest path of `1`).
#[test]
fn min_ticks_known_values() {
    assert_eq!(Version::new().min_ticks(), Count::ZERO);
    assert_eq!(uniform(5u8).min_ticks(), Count::from(5u64));
    use crate::testing::oracles::tree::Version as V;
    let peaks = from_oracle_version(&V::node(
        0u8,
        V::node(0u8, V::leaf(1u8), V::leaf(0u8)),
        V::node(0u8, V::leaf(0u8), V::leaf(1u8)),
    ));
    assert_eq!(peaks.min_ticks(), Count::from(2u64));
}

proptest! {
    /// `min_ticks` floors each clock's own causal past: after every step of any
    /// production-side op-trace history, every live clock's `min_ticks` is at
    /// most the number of ticks in that clock's causal past.
    ///
    /// The histories include tick counts wider than `u64`.
    ///
    /// `min_ticks` is a floor over every history that yields a version, and a
    /// clock's causal past is one such history. Checking each clock against its
    /// own past after every step, rather than the final clocks against the
    /// whole history's total, meets the bound where it is tight many times
    /// per history. Each clock's past must also lie within the ticks the whole
    /// history recorded, so the per-clock bound still implies the
    /// whole-history one. Each step also checks the fold against the
    /// recursive oracle's sum of bases; the min_ticks descriptor's
    /// function-space leg supplies an independent second computation.
    #[test]
    fn min_ticks_floors_every_history(ops in production_strategy()) {
        let mut clocks = vec![WithCausalPast::seed()];
        for op in &ops {
            step_production::<WithCausalPast>(&mut clocks, op, &mut ());
            for (clock, past) in &clocks {
                let v = clock.version();
                let ticks = past.ticks();
                let history_ticks = past.history_ticks.borrow();
                prop_assert!(
                    ticks <= *history_ticks,
                    "a clock's causal past holds {} ticks, more than the history's {}",
                    ticks,
                    *history_ticks,
                );
                prop_assert_eq!(v.min_ticks(), to_oracle_version(v).min_ticks());
                prop_assert!(
                    v.min_ticks() <= ticks,
                    "min_ticks {} exceeded the {} ticks in its clock's causal past after {:?}",
                    v.min_ticks(),
                    ticks,
                    op,
                );
            }
        }
    }
}

/// Reaches `target` by a single-seed history, returning the version reached
/// and the ticks spent.
///
/// The history visits the normal-form tree in preorder, carrying a party that
/// owns exactly the current node's interval. It spends the node's base `b` as
/// `ticks(b)` by that party, then forks it and hands each child the half over
/// that child's interval, rejoining the halves once both subtrees are done.
/// `fork` keeps the left half, as the paper's split does; were that to change,
/// the history would reach another version and the attainment properties
/// would fail.
///
/// The version built so far is constant over the current node's interval,
/// because only the node's ancestors have ticked there, so each tick's fill
/// changes nothing and its grow raises exactly the party's interval. The ticks
/// spent are therefore the sum of the bases.
///
/// The traversal recurses once per level of `target`, an oracle tree whose
/// strategy bounds its depth by construction.
fn realize(target: &crate::testing::oracles::tree::Version) -> (Version, BigUint) {
    use crate::testing::oracles::tree::Version as V;

    fn visit(node: &V, mut party: Party, built: &mut Version, spent: &mut BigUint) -> Party {
        let (V::Leaf(base) | V::Node(base, ..)) = node;
        if *base != BigUint::ZERO {
            built.ticks(&party, Count(base.clone()));
            *spent += base;
        }
        if let V::Node(_, left, right) = node {
            let right_half = party.fork();
            party = visit(left, party, built, spent);
            let right_half = visit(right, right_half, built, spent);
            party
                .join(right_half)
                .expect("the two halves of one interval are disjoint");
        }
        party
    }

    let mut built = Version::new();
    let mut spent = BigUint::ZERO;
    let seed = visit(target, Party::seed(), &mut built, &mut spent);
    assert!(seed.is_seed(), "the history's parties rejoin into the seed");
    (built, spent)
}

/// Checks that [`realize`] reaches `target` and that `min_ticks` equals the
/// ticks it spent.
fn assert_min_ticks_attained(
    target: &crate::testing::oracles::tree::Version,
) -> Result<(), TestCaseError> {
    let version = from_oracle_version(target);
    let (built, spent) = realize(target);
    prop_assert_eq!(
        &built,
        &version,
        "the constructed history reaches another version"
    );
    prop_assert_eq!(
        version.min_ticks(),
        Count(spent),
        "the constructed history spends another count"
    );
    Ok(())
}

proptest! {
    /// `min_ticks` is attained on arbitrary canonical versions: a single-seed
    /// history of forks, wide `ticks`, and joins reaches each one spending
    /// exactly `min_ticks` ticks.
    ///
    /// `min_ticks` is a minimum over the histories that yield a version, and a
    /// minimum is attained. `min_ticks_floors_every_history` checks the
    /// floor half; this checks the attained half, on versions with many
    /// regions, and shows that a rule-respecting history reaches every drawn
    /// canonical version.
    #[test]
    fn min_ticks_is_attained_on_arbitrary_versions(target in arb_oracle_version()) {
        assert_min_ticks_attained(&target)?;
    }

    /// `min_ticks` is attained on the co-generated spine versions: a
    /// single-seed history reaches each one spending exactly `min_ticks` ticks.
    ///
    /// These versions nest regions far deeper than arbitrary versions and draw
    /// heights just past `2^64` and `2^130`.
    #[test]
    fn min_ticks_is_attained_on_spine_versions(case in arb_spine_case()) {
        assert_min_ticks_attained(&case.version)?;
    }
}

/// There is no *maximum* tick count: leaf `1` can be built by arbitrarily many
/// ticks (`n` disjoint forks each ticking once, then all joined), yet
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
        assert_eq!(v, &uniform(1u8), "n={n}: rejoins to leaf 1");
        assert_eq!(
            v.min_ticks(),
            Count::from(1u64),
            "n={n}: {n} ticks collapse to the floor 1"
        );
    }
}

// ─────────────────────────────── rank ───────────────────────────────

/// Canonical binary rank strings with varied integer and fractional widths.
fn canonical_rank_text() -> impl Strategy<Value = String> {
    let integer = prop_oneof![
        Just(String::from("0")),
        prop::collection::vec(any::<bool>(), 0..128).prop_map(|tail| {
            let mut text = String::from("1");
            text.extend(tail.into_iter().map(|bit| if bit { '1' } else { '0' }));
            text
        }),
    ];
    let fraction = prop::option::of(prop::collection::vec(any::<bool>(), 0..128).prop_map(
        |middle| {
            let mut text = String::new();
            text.extend(middle.into_iter().map(|bit| if bit { '1' } else { '0' }));
            text.push('1');
            text
        },
    ));
    (integer, fraction).prop_map(|(mut integer, fraction)| {
        if let Some(fraction) = fraction {
            integer.push('.');
            integer.push_str(&fraction);
        }
        integer
    })
}

/// `rank` known values.
///
/// The empty version is zero; a leaf is its integer base; the pair `min_ticks`
/// cannot separate, `(0, 1, 0) < 1` with both one tick, gets strictly ordered
/// ranks; and two *concurrent* versions may share a rank (the two-peak tree
/// also covers half the interval), which is exactly what the
/// strict-monotonicity contract permits.
#[test]
fn rank_known_values() {
    assert_eq!(Version::new().rank().to_string(), "0");
    assert_eq!(uniform(5u8).rank().to_string(), "101");

    use crate::testing::oracles::tree::Version as V;
    let half = from_oracle_version(&V::node(0u8, V::leaf(1u8), V::leaf(0u8)));
    let one = uniform(1u8);
    assert!(half < one, "strict containment in the causal order");
    assert!(half.rank() < one.rank(), "so strictly smaller rank");
    assert_eq!(half.min_ticks(), one.min_ticks(), "the floor ties them");
    assert_eq!(half.rank().to_string(), "0.1");

    let peaks = from_oracle_version(&V::node(
        0u8,
        V::node(0u8, V::leaf(1u8), V::leaf(0u8)),
        V::node(0u8, V::leaf(0u8), V::leaf(1u8)),
    ));
    assert!(half.concurrent(&peaks), "different halves of the interval");
    assert_eq!(
        half.rank(),
        peaks.rank(),
        "equal rank is fine when concurrent"
    );
}

/// Rank formatting applies width, fill, alignment, and precision to the whole
/// binary value while ignoring numeric sign and zero-padding flags.
#[test]
fn rank_formatting_behaves_as_text() {
    let rank: Rank = "101.01".parse().expect("canonical rank text parses");
    for (actual, expected) in [
        (format!("{rank:10}"), "101.01    "),
        (format!("{rank:>10}"), "    101.01"),
        (format!("{rank:^10}"), "  101.01  "),
        (format!("{rank:*^10}"), "**101.01**"),
        (format!("{rank:.4}"), "101."),
        (format!("{rank:>10.4}"), "      101."),
        (format!("{rank:*^10.4}"), "***101.***"),
    ] {
        assert_eq!(actual, expected);
    }
    assert_eq!(format!("{rank:+}"), "101.01");
    assert_eq!(format!("{rank:010}"), "101.01    ");
}

/// Parsing preserves the binary value and rejects every malformed component
/// of the text grammar.
#[test]
fn rank_text_known_values_and_boundaries() {
    for (text, numerator, exponent) in [
        ("0", 0u8, 0u64),
        ("101", 5, 0),
        ("0.01", 1, 2),
        ("101.01", 21, 2),
    ] {
        assert_eq!(
            text.parse::<Rank>(),
            Ok(Rank::from_raw(BigUint::from(numerator), exponent)),
        );
    }

    for text in ["", ".", ".1", "1.", "00", "1.0", "2", "1.2", "1..1"] {
        assert!(text.parse::<Rank>().is_err(), "accepted {text:?}");
    }
}

proptest! {
    /// Canonical binary rank text parses to its exact value, and every displayed
    /// rank parses back to the same value.
    #[test]
    fn rank_text_roundtrips(
        text in canonical_rank_text(),
        version in arb_oracle_version(),
    ) {
        let parsed: Rank = text.parse().expect("generated canonical text parses");
        prop_assert_eq!(parsed.to_string(), text);

        let rank = from_oracle_version(&version).rank();
        let rendered = rank.to_string();
        prop_assert_eq!(rendered.parse::<Rank>(), Ok(rank));
    }

    /// Redundant digits and non-grammar characters are rejected for every
    /// canonical rank form.
    #[test]
    fn rank_text_rejects_noncanonical_forms(text in canonical_rank_text()) {
        let leading_zero = format!("0{text}");
        prop_assert!(leading_zero.parse::<Rank>().is_err());
        let trailing_zero = if text.contains('.') {
            format!("{text}0")
        } else {
            format!("{text}.0")
        };
        prop_assert!(trailing_zero.parse::<Rank>().is_err());
        for invalid in [
            format!("+{text}"),
            format!("-{text}"),
            format!(" {text}"),
            format!("{text} "),
            format!("{text}..1"),
        ] {
            prop_assert!(invalid.parse::<Rank>().is_err(), "accepted {:?}", invalid);
        }
    }
}

/// The alignment oracle for `Rank` order: shift both numerators to the
/// common exponent and compare: the definitionally correct order the
/// class-first streamed comparison must reproduce.
fn alignment_cmp(a: &super::Rank, b: &super::Rank) -> core::cmp::Ordering {
    let (an, ae) = rank_parts(a);
    let (bn, be) = rank_parts(b);
    let e = ae.max(be);
    (an << ((e - ae) as usize)).cmp(&(bn << ((e - be) as usize)))
}

/// A rank's raw parts for the oracle, as plain `BigUint` arithmetic
/// operands.
fn rank_parts(r: &super::Rank) -> (BigUint, u64) {
    let (num, exp) = r.raw_parts();
    (BigUint::from_bytes_le(&num.to_bytes_le()), exp)
}

/// Draws one worst-case `Rank`.
///
/// Odd numerators from one to a few hundred limbs wide, whose limbs are
/// all ones, zero, or arbitrary, so shared prefixes between two draws go
/// deep, at exponents from zero to well past any numerator width.
fn arb_wide_rank() -> impl Strategy<Value = super::Rank> {
    let limbs = prop_oneof![
        4 => Just(1usize),
        2 => 1..=4usize,
        1 => 8..=15usize,
        1 => 64..=263usize,
    ];
    let limb = prop_oneof![
        1 => Just(u64::MAX),
        1 => Just(0u64),
        2 => any::<u64>(),
    ];
    let words = limbs.prop_flat_map(move |limbs| proptest::collection::vec(limb.clone(), limbs));
    (words, 0..100_000u64).prop_map(|(mut words, exp)| {
        if let Some(top) = words.last_mut() {
            if *top == 0 {
                *top = 1;
            }
        }
        words[0] |= 1; // odd: the stored normalization invariant
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        super::Rank::from_raw(BigUint::from_bytes_le(&bytes), exp)
    })
}

/// Draws a wide rank and a second rank related to it in one of four ways,
/// each reaching one path of the class-first streamed comparison.
fn arb_rank_cmp_pair() -> impl Strategy<Value = (super::Rank, super::Rank)> {
    arb_wide_rank().prop_flat_map(|a| {
        let (num, exp) = rank_parts(&a);
        let perturbed = {
            let num = num.clone();
            (1..=64u64).prop_map(move |shift| {
                super::Rank::from_raw(num.clone(), exp.saturating_add(shift))
            })
        };
        let tied = {
            let a = a.clone();
            (0..16usize).prop_map(move |bit| {
                let flipped = &num ^ (BigUint::from(2u8) << bit);
                if flipped.bits() == a_bits(&a) {
                    super::Rank::from_raw(flipped, exp)
                } else {
                    a.clone()
                }
            })
        };
        let second = prop_oneof![
            // An unrelated rank: usually a mismatched class.
            arb_wide_rank(),
            // An exact duplicate: the equality path.
            Just(a.clone()),
            // The same value at a perturbed exponent: a guaranteed class
            // mismatch with an identical mantissa.
            perturbed,
            // A forced class tie: same exponent, same width, a low bit
            // perturbed, so the streamed windows share a deep prefix.
            tied,
        ];
        (Just(a), second)
    })
}

proptest! {
    /// The class-first streamed `Rank` order agrees with the alignment oracle
    /// on wide and differently scaled pairs.
    ///
    /// The pairs cross random wide/narrow numerators with far-apart exponents
    /// (the mismatched-class fast path), forced class ties with deep shared
    /// prefixes (the streamed-window path), and exact duplicates (the equality
    /// path). `checked_sub`'s pre-check is asserted consistent on every pair:
    /// `Some` exactly when `rhs <= self`.
    #[test]
    fn rank_cmp_agrees_with_the_alignment_oracle((a, b) in arb_rank_cmp_pair()) {
        let want = alignment_cmp(&a, &b);
        prop_assert_eq!(a.cmp(&b), want, "order disagrees: {} vs {}", a, b);
        prop_assert_eq!(b.cmp(&a), want.reverse(), "antisymmetry breaks: {} vs {}", b, a);
        prop_assert_eq!(
            a.checked_sub(&b).is_some(),
            want != core::cmp::Ordering::Less,
            "checked_sub pre-check disagrees with the order"
        );
    }
}

proptest! {
    /// `Sum` is the pairwise fold.
    ///
    /// Over an arbitrary multiset of ranks in arbitrary order, both `Sum` impls
    /// return exactly the value the reference `fold(ZERO, +)` produces: one
    /// raw accumulation with a final normalization changes the cost, never the
    /// result.
    #[test]
    fn rank_sum_equals_the_pairwise_fold(ranks in proptest::collection::vec(arb_wide_rank(), 0..24)) {
        assert_sum_is_the_pairwise_fold(ranks, None)?;
    }

    /// `Sum` is the pairwise fold and the exact rational sum, over multisets
    /// of dyadic ranks.
    ///
    /// [`arb_dyadic`] draws integral ranks, zero, and equal values, which the
    /// wide ranks of `rank_sum_equals_the_pairwise_fold` almost never are.
    /// The reference is [`Dyadic::add`]'s sum, rendered to text and parsed, so
    /// no production arithmetic computes the reference.
    #[test]
    fn rank_sum_equals_the_dyadic_sum(values in proptest::collection::vec(arb_dyadic(), 0..10)) {
        let total = values.iter().fold(Dyadic::zero(), |sum, value| sum.add(value));
        let ranks: Vec<super::Rank> = values.iter().map(Dyadic::rank).collect();
        assert_sum_is_the_pairwise_fold(ranks, Some(&total.rank()))?;
    }
}

/// Checks that both `Sum` impls return the value of the pairwise
/// `fold(ZERO, +)`, and that this value is `expected` when one is given.
fn assert_sum_is_the_pairwise_fold(
    ranks: Vec<super::Rank>,
    expected: Option<&super::Rank>,
) -> Result<(), TestCaseError> {
    let reference = ranks.iter().fold(super::Rank::ZERO, |acc, r| acc + r);
    if let Some(expected) = expected {
        prop_assert_eq!(&reference, expected);
    }
    prop_assert_eq!(&ranks.iter().sum::<super::Rank>(), &reference);
    prop_assert_eq!(&ranks.into_iter().sum::<super::Rank>(), &reference);
    Ok(())
}

/// `Sum` over no owned ranks is [`Rank::ZERO`], as the `Sum<Rank>` impl
/// documents.
#[test]
fn empty_owned_rank_sum_is_zero() {
    assert_eq!(core::iter::empty::<Rank>().sum::<Rank>(), Rank::ZERO);
}

/// `Sum` over no borrowed ranks is [`Rank::ZERO`], as the `Sum<&Rank>` impl
/// documents.
#[test]
fn empty_borrowed_rank_sum_is_zero() {
    assert_eq!(core::iter::empty::<&Rank>().sum::<Rank>(), Rank::ZERO);
}

/// A rank's numerator width for the tie construction above.
fn a_bits(r: &super::Rank) -> u64 {
    rank_parts(r).0.bits()
}

proptest! {
    /// Every `laws::RANK_TRIPLE` law (the monoid, order, and cross-path
    /// normalization laws) holds on generated wide ranks.
    ///
    /// Mixed magnitude classes, spilled numerators, and perturbed exponents:
    /// the regime the version-derived driver in the algebraic-laws suite cannot
    /// reach.
    #[test]
    fn rank_triple_laws_on_wide_ranks(
        (a, b, c) in (arb_wide_rank(), arb_wide_rank(), arb_wide_rank()),
    ) {
        let (a, b, c) = (&a, &b, &c);
        for (name, law) in crate::testing::laws::RANK_TRIPLE {
            prop_assert!(law(a, b, c), "law violated: {}", name);
        }
    }
}

proptest! {
    /// Every `laws::RANK_TRIPLE` law holds on ranks of dyadic rationals, and
    /// each rank agrees with its rational in value, text, and order.
    ///
    /// `rank_triple_laws_on_wide_ranks` draws odd numerators at exponents
    /// below 100,000, so it almost never builds an integral rank, and never
    /// zero or one rank reached from two spellings of a value. [`arb_dyadic`]
    /// and [`arb_related_dyadic`] build all three often. Each rank comes from
    /// parsing [`Dyadic::render`]'s text, which no production code writes. It
    /// must equal `Rank::from_raw` of the same pair, production's `Display`
    /// must print exactly that text, and any two ranks must order as their
    /// rationals do by cross-multiplication.
    #[test]
    fn rank_triple_laws_on_dyadic_ranks(
        (qa, qb) in arb_dyadic().prop_flat_map(arb_related_dyadic),
        qc in arb_dyadic(),
    ) {
        let values = [&qa, &qb, &qc];
        let ranks: Vec<super::Rank> = values.iter().map(|value| value.rank()).collect();
        for (value, rank) in values.iter().zip(&ranks) {
            prop_assert_eq!(
                &super::Rank::from_raw(value.num.clone(), value.exp),
                rank,
                "the parsed and raw routes disagree on {:?}",
                value,
            );
            prop_assert_eq!(rank.to_string(), value.render(), "the printer on {:?}", value);
        }
        for (i, j) in [(0, 1), (1, 0), (0, 2), (1, 2)] {
            prop_assert_eq!(
                ranks[i].cmp(&ranks[j]),
                values[i].cmp_value(values[j]),
                "order of {:?} against {:?}",
                values[i],
                values[j],
            );
        }
        let (a, b, c) = (&ranks[0], &ranks[1], &ranks[2]);
        for (name, law) in crate::testing::laws::RANK_TRIPLE {
            prop_assert!(law(a, b, c), "law violated: {}", name);
        }
    }
}

// ─────────────────────── the rank wire form ───────────────────────

/// Known rank encodings pin the lexicographic format at each boundary.
///
/// Zero, the smallest fractions, integral-only vs fractional at a shared
/// integral part, every step of the integral header (the mantissa-width steps
/// at `I + 1` crossing a power of two, and the width-of-width steps where the
/// unary run itself lengthens), and equal integral parts separated only deep in
/// the fraction, across a group boundary, where the deeper rank's extra expansion
/// bits ride a further continuation-framed group. Each byte string is pinned
/// exactly (the wire form is canonical, so these are format goldens), and the
/// whole battery must be strictly ascending in byte order exactly as it is
/// ascending in rank order.
#[test]
fn rank_encoding_known_values() {
    let fraction =
        |numerator: u8, exponent| super::Rank::from_raw(BigUint::from(numerator), exponent);
    let int = |n: u64| uniform(n).rank();
    // (value, its pinned canonical bytes), in strictly ascending order.
    let battery: Vec<(super::Rank, Vec<u8>)> = vec![
        // Zero = "0" ++ "0": the smallest header, an empty fraction's
        // immediate close.
        (super::Rank::ZERO, vec![0x00]),
        // 1/4 = "0" ++ "1 01000000 0": one group framing the
        // expansion ".01", zero-padded past its last set bit.
        (fraction(1, 2), vec![0x50, 0x00]),
        // 1/2 = "0" ++ "1 10000000 0".
        (fraction(1, 1), vec![0x60, 0x00]),
        // 3/4 = "0" ++ "1 11000000 0": splits from 1/2 inside the
        // shared group, at the second expansion bit.
        (fraction(3, 2), vec![0x70, 0x00]),
        // 1 = "1000" ++ "0": the first integral header step.
        (int(1), vec![0x80]),
        // 3/2 = 1's integral code, then one group framing ".1": integral-only
        // vs fractional at a shared integral part is decided at the
        // continuation-vs-close bit.
        (
            super::Rank::from_raw(BigUint::from(3u8), 1),
            vec![0x8C, 0x00],
        ),
        (int(2), vec![0x90]),
        (int(3), vec![0xA0]),
        (int(4), vec![0xA8]),
        // 5, then 5 + 2⁻⁴⁰ and 5 + 2⁻⁴⁰ + 2⁻⁴¹: equal integral parts
        // separated only deep in the fraction, the deepest pair only
        // past a group boundary (the 41st expansion bit opens a sixth
        // group), and the integral-only rank separated from both at
        // its close bit, never by a byte-prefix relation.
        (int(5), vec![0xB0]),
        (
            super::Rank::from_raw(BigUint::from(5u128 << 40 | 1), 40),
            vec![0xB4, 0x02, 0x01, 0x00, 0x80, 0x40, 0x40],
        ),
        (
            super::Rank::from_raw(BigUint::from(5u128 << 41 | 3), 41),
            vec![0xB4, 0x02, 0x01, 0x00, 0x80, 0x40, 0x70, 0x00],
        ),
        // 6 and 7: the last mantissa of width 2 against the first of
        // width 3: the header's width-of-width (unary run) step.
        (int(6), vec![0xB8]),
        (int(7), vec![0xC0, 0x00]),
        (int(8), vec![0xC1, 0x00]),
        // 15 and 16: the next mantissa-width step inside one run
        // width, separated by the mantissa's final bit at the byte
        // boundary.
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

/// A writer that exercises `Write::write_all`'s partial-write path.
#[derive(Default)]
struct OneByteWriter(Vec<u8>);

impl Write for OneByteWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(&byte) = bytes.first() else {
            return Ok(0);
        };
        self.0.push(byte);
        Ok(1)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Streaming rank encoding matches the canonical bytes across buffer flushes
/// and partial writes.
///
/// The fractional and integral cases both exceed the encoder's fixed staging
/// buffer. The writer accepts only one byte per call, so `encode_to` must
/// correctly resume every partial write as well as every internal flush.
#[test]
fn rank_encode_to_streams_the_canonical_encoding() {
    let ranks = [
        super::Rank::from_raw(BigUint::ONE, 4_096),
        super::Rank::from_raw((BigUint::ONE << 4_096usize) - 1u8, 0),
    ];
    for rank in ranks {
        let expected = rank.encode();
        let mut writer = OneByteWriter::default();
        rank.encode_to(&mut writer).unwrap();
        assert_eq!(writer.0, expected);
    }
}

proptest! {
    /// Every rank-bearing and span streaming encoder emits exactly its
    /// canonical buffered form, even when the sink accepts one byte at a time.
    ///
    /// These encoders compose independently verified version and rank streams,
    /// but composition still has observable boundaries: a missing component,
    /// reversed component order, or mishandled partial write changes the bytes.
    #[test]
    fn composite_streaming_encoders_match_their_buffered_forms(
        a in arb_oracle_version(),
        b in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let rank = a.rank();
        let ranked = a.ranked();
        let span = a.span(&b);

        let mut writer = OneByteWriter::default();
        rank.encode_to(&mut writer).unwrap();
        prop_assert_eq!(writer.0, rank.encode());

        let mut writer = OneByteWriter::default();
        a.encode_rank_to(&mut writer).unwrap();
        prop_assert_eq!(writer.0, a.encode_rank());

        let mut writer = OneByteWriter::default();
        ranked.encode_rank_to(&mut writer).unwrap();
        prop_assert_eq!(writer.0, ranked.encode_rank());

        let mut writer = OneByteWriter::default();
        ranked.encode_to(&mut writer).unwrap();
        prop_assert_eq!(writer.0, ranked.encode());

        let mut writer = OneByteWriter::default();
        span.encode_to(&mut writer).unwrap();
        prop_assert_eq!(writer.0, span.encode());
    }
}

/// Runs one streaming encoder whose canonical bytes are `expected` against a
/// [`ScriptedWriter`] on `schedule` with no limit, with a limit at the
/// encoding's length, and with a limit inside the encoding at `cut`.
///
/// With room for the whole encoding, the encoder must return `Ok` having
/// written exactly `expected`, so a limit at the length also catches an extra
/// byte. With less room, it must meet the writer's failure and return it with
/// the writer's kind, as every `encode_to` promises to return any error the
/// writer reports. What the writer holds after a failure is left unchecked,
/// because no encoder promises it.
fn check_encoder(
    expected: &[u8],
    schedule: &IoSchedule,
    cut: Index,
    encode: impl Fn(&mut ScriptedWriter<'_>) -> io::Result<()>,
) -> Result<(), TestCaseError> {
    let len = expected.len();
    let mut limits = vec![None, Some(len)];
    if len > 0 {
        limits.push(Some(cut.index(len)));
    }
    for limit in limits {
        let mut writer = ScriptedWriter::new(schedule, limit);
        let result = encode(&mut writer).map_err(|error| error.kind());
        if limit.is_some_and(|limit| limit < len) {
            prop_assert!(
                writer.has_failed(),
                "the encoder stopped before its writer's limit {:?}",
                limit
            );
            prop_assert_eq!(result, Err(io::ErrorKind::Other), "limit {:?}", limit);
        } else {
            prop_assert!(
                !writer.has_failed(),
                "the encoder wrote past its {} bytes",
                len
            );
            prop_assert_eq!(result, Ok(()), "limit {:?}", limit);
            prop_assert_eq!(writer.written(), expected, "limit {:?}", limit);
        }
    }
    Ok(())
}

/// Runs one [`Canonical`] type's streaming `encode_to` on `value` through
/// [`check_encoder`], with its buffered `encode` as the reference bytes.
fn check_streaming_encoder<T: Canonical>(
    value: &T,
    schedule: &IoSchedule,
    cut: Index,
) -> Result<(), TestCaseError> {
    check_encoder(&value.encode_canonical(), schedule, cut, |writer| {
        value.write_canonical(writer)
    })
}

/// Runs one borsh serializer on `value` through [`check_encoder`], with
/// `borsh::to_vec` as the reference bytes.
#[cfg(feature = "borsh")]
fn check_borsh_encoder(
    value: &impl borsh::BorshSerialize,
    schedule: &IoSchedule,
    cut: Index,
) -> Result<(), TestCaseError> {
    let expected = borsh::to_vec(value).expect("serializing into memory succeeds");
    check_encoder(&expected, schedule, cut, |writer| value.serialize(writer))
}

proptest! {
    /// Every streaming encoder writes exactly its canonical bytes through a
    /// writer that accepts short writes and returns `Interrupted`, and returns
    /// the error a failing writer reports.
    ///
    /// Each [`Canonical`] type's `encode_to`, and `encode_rank_to` for `Version`
    /// and `Ranked`, run against a scripted writer that accepts from one byte
    /// per call to twice `Rank::decode`'s buffer of `DECODE_CHUNK_BYTES`,
    /// returning `Interrupted` on a generated schedule, with no limit, a limit
    /// at the encoding's length, and a limit inside it.
    #[test]
    fn streaming_encoders_survive_short_interrupted_and_failing_writers(
        party in arb_oracle_party_nonempty(),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        schedule in arb_io_schedule(),
        cut in any::<Index>(),
    ) {
        use crate::Ranked;

        let clock = Clock::from_parts(from_oracle_party(&party), from_oracle_version(&a));
        let party = from_oracle_party(&party);
        let (a, b) = (from_oracle_version(&a), from_oracle_version(&b));
        let ranked = Ranked::from(a.clone());
        check_streaming_encoder(&party, &schedule, cut)?;
        check_streaming_encoder(&a, &schedule, cut)?;
        check_streaming_encoder(&clock, &schedule, cut)?;
        check_streaming_encoder(&a.span(&b), &schedule, cut)?;
        check_streaming_encoder(&a.rank(), &schedule, cut)?;
        check_streaming_encoder(&ranked, &schedule, cut)?;
        check_encoder(&a.encode_rank(), &schedule, cut, |w| a.encode_rank_to(w))?;
        check_encoder(&ranked.encode_rank(), &schedule, cut, |w| ranked.encode_rank_to(w))?;
    }
}

#[cfg(feature = "borsh")]
proptest! {
    /// Every borsh serializer writes exactly its bytes through a writer that
    /// accepts short writes and returns `Interrupted`, and returns the error a
    /// failing writer reports.
    ///
    /// Each [`Canonical`] type, and `Count`, serializes against the scripted writer
    /// of `streaming_encoders_survive_short_interrupted_and_failing_writers`,
    /// under the same three limits, with `borsh::to_vec` as the reference
    /// bytes.
    #[test]
    fn borsh_serializers_survive_short_interrupted_and_failing_writers(
        party in arb_oracle_party_nonempty(),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        count in crate::testing::generators::arb_magnitude(),
        schedule in arb_io_schedule(),
        cut in any::<Index>(),
    ) {
        use crate::Ranked;

        let clock = Clock::from_parts(from_oracle_party(&party), from_oracle_version(&a));
        let party = from_oracle_party(&party);
        let (a, b) = (from_oracle_version(&a), from_oracle_version(&b));
        check_borsh_encoder(&party, &schedule, cut)?;
        check_borsh_encoder(&a, &schedule, cut)?;
        check_borsh_encoder(&clock, &schedule, cut)?;
        check_borsh_encoder(&a.span(&b), &schedule, cut)?;
        check_borsh_encoder(&a.rank(), &schedule, cut)?;
        check_borsh_encoder(&Ranked::from(a.clone()), &schedule, cut)?;
        check_borsh_encoder(&Count(count), &schedule, cut)?;
    }
}

/// The exhaustive small-scope sweep over **every** byte string of zero, one,
/// and two bytes.
///
/// Decode is total (accepts or rejects, never panics); every accepted string
/// re-encodes byte-identically (the format is bijective on accepted strings, so
/// no value has a second spelling: the strict-canonicality statement that
/// subsumes the individual rejection cases); the accepted strings in byte order carry
/// strictly ascending ranks (the lexicographic law, total at this scope); every
/// decoded rank's numeric size is linear in its input bytes (no decompression
/// bomb); and both reachable errors (truncation and non-minimal packing)
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
        Err(e) => panic!("unexpected rejection at this scope: {e}"),
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
    // reachable rejection classes.
    assert!(
        accepted.len() > 1_000,
        "acceptance is live: {}",
        accepted.len()
    );
    assert!(truncated > 0, "the sweep reaches truncation");
    assert!(trailing > 0, "the sweep reaches non-minimal packing");
}

/// Direct cases cover every rejection class the rank decoder can reach.
///
/// Empty input, an unterminated unary run, a truncated header payload, a
/// truncated integral mantissa, a truncated fraction group, a trailing zero
/// byte, a set padding bit, the one spelling a trailing-zero fraction can take
/// (an all-zero final group; inside the final group trailing zeros *are* the
/// padding, so the only non-canonical spelling spills them into a group of
/// their own), and the integral representation bound (a unary run of 64,
/// declaring a mantissa width beyond `2⁶⁴` bits, the one format bound a small
/// input can reach; the fraction bound needs over half a GiB of real groups,
/// since the fraction has no length header to forge). The remaining documented
/// error, a non-minimal integral header, is structurally unrepresentable
/// (every `(run, payload)` pair decodes to a width whose own width matches the
/// run exactly), which the exhaustive sweep witnesses mechanically at small
/// scope.
#[test]
#[allow(clippy::type_complexity)]
fn rank_decoding_rejects_each_malformed_input_class() {
    let cases: [(&[u8], fn(&Decode) -> bool, &str); 9] = [
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
        // ".1" spelled with a second, all-zero group: the
        // trailing-zero-fraction spelling, non-minimal packing.
        (
            &[0x60, 0x20, 0x00],
            |e| matches!(e, Decode::TrailingBits),
            "all-zero final group",
        ),
        // 64 ones, then the run's terminating zero: an integral
        // mantissa width of 2⁶⁴ or more bits, past the format bound,
        // whatever follows.
        (
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00],
            |e| matches!(e, Decode::NotCanonical),
            "integral width past the format bound",
        ),
    ];
    for (bytes, matches_error, description) in cases {
        let err = super::Rank::decode(bytes).expect_err(description);
        assert!(matches_error(&err), "{description}: wrong error: {err}");
    }
}

/// A decode result reduced to what the reader properties compare: the decoded
/// value, or the error's variant, with the kind of an I/O error.
#[derive(Debug, PartialEq)]
enum Verdict<T> {
    Decoded(T),
    Truncated,
    TrailingBits,
    NotCanonical,
    TooLarge,
    Io(io::ErrorKind),
}

/// Reduces a decode result to its variant, keeping only an I/O error's kind.
impl<T> From<Result<T, Decode>> for Verdict<T> {
    fn from(result: Result<T, Decode>) -> Self {
        match result {
            Ok(value) => Verdict::Decoded(value),
            Err(Decode::Truncated) => Verdict::Truncated,
            Err(Decode::TrailingBits) => Verdict::TrailingBits,
            Err(Decode::NotCanonical) => Verdict::NotCanonical,
            Err(Decode::TooLarge) => Verdict::TooLarge,
            Err(Decode::Io(error)) => Verdict::Io(error.kind()),
        }
    }
}

/// A decode input with the verdict it must receive, known before decoding.
#[derive(Debug)]
struct DecodeCase<T> {
    /// The bytes to decode.
    input: Vec<u8>,
    /// The verdict a reader that never fails must produce.
    verdict: Verdict<T>,
    /// Bounds from below how far into `input` a decoder must read before its
    /// verdict is settled, with the end of input counted as one position past
    /// the last byte.
    ///
    /// A reader failure before that many bytes leaves the verdict open, so
    /// the decoder must report it. An input whose verdict turns on where it
    /// ends has `input.len() + 1`.
    decided_by: usize,
}

/// The three inputs one rank yields: its encoding, the encoding followed by
/// `suffix`, and a strict prefix of the encoding.
///
/// `prefix_len` maps the encoding's length to the prefix's, which must be
/// shorter.
fn rank_cases(
    rank: &Rank,
    suffix: &[u8],
    prefix_len: impl FnOnce(usize) -> usize,
) -> Vec<DecodeCase<Rank>> {
    let encoding = rank.encode();
    let rank_len = encoding.len();
    let prefix = encoding[..prefix_len(rank_len)].to_vec();
    vec![
        DecodeCase {
            input: encoding.clone(),
            verdict: Verdict::Decoded(rank.clone()),
            decided_by: rank_len + 1,
        },
        DecodeCase {
            input: [encoding.as_slice(), suffix].concat(),
            verdict: Verdict::TrailingBits,
            // The first byte past the encoding settles the rejection.
            decided_by: rank_len + 1,
        },
        DecodeCase {
            verdict: Verdict::Truncated,
            decided_by: prefix.len() + 1,
            input: prefix,
        },
    ]
}

/// A byte string judged by `judge`, a decoder over the whole input in
/// memory, such as `Rank::decode_bytes` or a reader `decode` over a slice.
fn judged_case<T>(input: Vec<u8>, judge: impl FnOnce(&[u8]) -> Result<T, Decode>) -> DecodeCase<T> {
    let verdict = Verdict::from(judge(&input));
    let decided_by = match verdict {
        // Acceptance and truncation both turn on where the input ends.
        Verdict::Decoded(_) | Verdict::Truncated => input.len() + 1,
        // A rejection can be settled as early as the first byte.
        _ => 1,
    };
    DecodeCase {
        input,
        verdict,
        decided_by,
    }
}

/// Decodes `case` with `decode` through a [`ScriptedReader`] on `schedule`
/// that fails at `fail_at`, if given, and checks the verdict.
///
/// The verdict must be `Decode::Io` with the reader's kind exactly when the
/// reader returned its failure, and the case's verdict otherwise. A failure
/// before `case.decided_by` leaves the verdict open, so the decoder must meet
/// it.
fn check_decode<T: core::fmt::Debug + PartialEq>(
    case: &DecodeCase<T>,
    schedule: &IoSchedule,
    fail_at: Option<usize>,
    decode: impl FnOnce(&mut ScriptedReader<'_>) -> Result<T, Decode>,
) -> Result<(), TestCaseError> {
    let reader_failed = Verdict::Io(io::ErrorKind::Other);
    let mut reader = ScriptedReader::new(&case.input, schedule, fail_at);
    let verdict = Verdict::from(decode(&mut reader));
    let expected = if reader.has_failed() {
        &reader_failed
    } else {
        &case.verdict
    };
    prop_assert_eq!(
        &verdict,
        expected,
        "reader failure at {:?}, met: {}; input {:?}",
        fail_at,
        reader.has_failed(),
        case.input
    );
    if let Some(offset) = fail_at {
        prop_assert!(
            reader.has_failed() || offset >= case.decided_by,
            "a failure after {} bytes leaves the verdict open, yet the decoder returned {:?} \
             without meeting it; input {:?}",
            offset,
            verdict,
            case.input
        );
    }
    Ok(())
}

/// The longest fraction, in bits, that the reader property generates: three
/// decoder buffers' worth of bytes before framing, so the longest encodings
/// need more than two refills.
const MAX_FRACTION_BITS: usize = 3 * DECODE_CHUNK_BYTES * u8::BITS as usize;

/// The rank `0.b₁b₂…bₙ1`: `bits` followed by a closing set bit, which makes
/// the fraction canonical.
fn binary_fraction(bits: &[bool]) -> Rank {
    let digits: String = bits
        .iter()
        .map(|&bit| if bit { '1' } else { '0' })
        .collect();
    format!("0.{digits}1")
        .parse()
        .expect("a binary fraction ending in 1 is canonical rank text")
}

/// Generates a rank in `[0, 1)` whose fraction has exactly `len` bits, the
/// last set and the rest random; zero bits give zero.
fn arb_fraction_rank(len: usize) -> impl Strategy<Value = Rank> {
    prop::collection::vec(any::<bool>(), len.saturating_sub(1)).prop_map(move |bits| {
        if len == 0 {
            Rank::ZERO
        } else {
            binary_fraction(&bits)
        }
    })
}

/// Fraction lengths whose rank encodings end one byte before, at, or one
/// byte after the decoder's prefix length or twice it.
///
/// `Rank::decode` reads a prefix of `DECODE_CHUNK_BYTES` bytes and then
/// refills a buffer of that size, so these encodings end where it changes
/// read path. A fraction's encoded length depends only on its bit length,
/// so the fraction `2⁻ˡᵉⁿ` measures it for every fraction that long.
fn fraction_lengths_at_decode_boundaries() -> Vec<usize> {
    (1..=MAX_FRACTION_BITS)
        .filter(|&len| {
            let encoded = binary_fraction(&vec![false; len - 1]).encode().len();
            [1, 2]
                .iter()
                .any(|multiple| encoded.abs_diff(multiple * DECODE_CHUNK_BYTES) <= 1)
        })
        .collect()
}

/// Generates decode inputs with known verdicts: the three cases of a rank
/// ending at a read-path boundary, of a rank of any fraction length, or of a
/// wide rank, or else one arbitrary byte string.
fn arb_decode_cases() -> impl Strategy<Value = Vec<DecodeCase<Rank>>> {
    let rank = prop_oneof![
        2 => prop::sample::select(fraction_lengths_at_decode_boundaries())
            .prop_flat_map(arb_fraction_rank),
        1 => (0..=MAX_FRACTION_BITS).prop_flat_map(arb_fraction_rank),
        1 => arb_wide_rank(),
    ];
    // Zero bytes could pass for padding, so all-zero suffixes get their own arm.
    let suffix = prop_oneof![
        prop::collection::vec(any::<u8>(), 1..=70),
        prop::collection::vec(Just(0u8), 1..=70),
    ];
    prop_oneof![
        4 => (rank, suffix, any::<Index>())
            .prop_map(|(rank, suffix, cut)| rank_cases(&rank, &suffix, |len| cut.index(len))),
        1 => prop::collection::vec(any::<u8>(), 0..256)
            .prop_map(|input| vec![judged_case(input, Rank::decode_bytes)]),
    ]
}

proptest! {
    /// `Rank::decode`'s verdict depends only on the bytes its reader yields,
    /// and every reader failure it meets comes back as `Decode::Io`.
    ///
    /// A scripted reader serves each input in chunks from one byte to more
    /// than the decoder's buffer, returning `Interrupted` before its answers on
    /// a generated schedule, and the verdict must equal the one known for the
    /// input: the rank for its exact encoding, `TrailingBits` for the encoding
    /// with bytes appended, `Truncated` for a strict prefix, and the in-memory
    /// decoder's verdict for arbitrary bytes. Generated ranks are steered to
    /// end beside the decoder's prefix length and its first refill, where it
    /// changes read path.
    ///
    /// The same reader then fails, at the input's end and at a generated
    /// offset. The verdict must be `Decode::Io` with the reader's kind exactly
    /// when the decoder met the failure, and the input's verdict otherwise, and
    /// the decoder must meet a failure placed before the bytes that settle the
    /// verdict. The reader panics if called after failing, so a decoder that
    /// retries the failure fails this test rather than hanging it.
    #[test]
    fn rank_decode_is_independent_of_the_read_schedule(
        cases in arb_decode_cases(),
        schedule in arb_io_schedule(),
        failure in any::<Index>(),
    ) {
        for case in &cases {
            let end = case.input.len();
            for fail_at in [None, Some(end), Some(failure.index(end + 1))] {
                check_decode(case, &schedule, fail_at, |reader| Rank::decode(reader))?;
            }
        }
    }
}

/// `Rank::decode` retries `Interrupted` at each of its reads, on every run.
///
/// Each rank whose encoding ends one byte before, at, or one byte after the
/// decoder's prefix length or twice it is decoded whole, with one trailing
/// zero byte, and one byte short. A reader that returns `Interrupted` before
/// every answer serves each input one byte or a whole buffer per call, once
/// without a failure and once failing at the input's end. Together these
/// inputs reach the prefix reads, the probe after a rank that fills the prefix
/// exactly, the refills, and the probe after a longer rank; the property
/// reaches the probe after a full prefix only on some draws.
#[test]
fn rank_decode_retries_interrupts_at_every_read() -> Result<(), TestCaseError> {
    let schedules = [1, DECODE_CHUNK_BYTES].map(|chunk| IoSchedule {
        chunk,
        interrupts: vec![1],
    });
    for len in fraction_lengths_at_decode_boundaries() {
        let rank = binary_fraction(&vec![true; len - 1]);
        for case in rank_cases(&rank, &[0], |rank_len| rank_len - 1) {
            for schedule in &schedules {
                for fail_at in [None, Some(case.input.len())] {
                    check_decode(&case, schedule, fail_at, |reader| Rank::decode(reader))?;
                }
            }
        }
    }
    Ok(())
}

/// The ways the reader properties alter an encoding.
#[derive(Debug)]
struct Mangling {
    /// Bytes appended after the encoding.
    suffix: Vec<u8>,
    /// Where the strict prefix cuts the encoding.
    cut: Index,
    /// Which bit of the encoding to flip.
    flip: Index,
}

/// Generates manglings with suffixes of up to sixteen bytes.
fn arb_mangling() -> impl Strategy<Value = Mangling> {
    (
        prop::collection::vec(any::<u8>(), 1..=16),
        any::<Index>(),
        any::<Index>(),
    )
        .prop_map(|(suffix, cut, flip)| Mangling { suffix, cut, flip })
}

/// The four inputs one nonempty encoding yields: the encoding itself, the
/// encoding followed by the mangling's suffix, a strict prefix, and the
/// encoding with one bit flipped.
fn mangled_inputs(encoding: &[u8], mangling: &Mangling) -> [Vec<u8>; 4] {
    let mut flipped = encoding.to_vec();
    let bit = mangling.flip.index(8 * encoding.len());
    flipped[bit / 8] ^= 0x80 >> (bit % 8);
    [
        encoding.to_vec(),
        [encoding, &mangling.suffix].concat(),
        encoding[..mangling.cut.index(encoding.len())].to_vec(),
        flipped,
    ]
}

/// Checks one type's reader `decode` on every input its value's encoding
/// yields, judged by the slice decode over the input in memory, with the
/// reader failing nowhere, at the input's end, and at a generated offset.
fn check_reader_decoder<T: Canonical>(
    value: &T,
    mangling: &Mangling,
    schedule: &IoSchedule,
    failure: Index,
) -> Result<(), TestCaseError> {
    for input in mangled_inputs(&value.encode_canonical(), mangling) {
        let case = judged_case(input, |bytes| T::decode_reader(bytes));
        let end = case.input.len();
        for fail_at in [None, Some(end), Some(failure.index(end + 1))] {
            check_decode(&case, schedule, fail_at, |reader| T::decode_reader(reader))?;
        }
    }
    Ok(())
}

proptest! {
    /// Every reader `decode` reaches the slice decode's verdict through any
    /// read schedule, and every reader failure it meets comes back as
    /// `Decode::Io`.
    ///
    /// Each [`Canonical`] type decodes a generated value's encoding, the encoding
    /// with bytes appended, a strict prefix, and the encoding with one bit
    /// flipped, through the scripted reader of
    /// `rank_decode_is_independent_of_the_read_schedule`, with that property's
    /// exact failure oracle: the verdict is `Decode::Io` with the reader's
    /// kind exactly when the decoder met the failure, and the slice decode's
    /// verdict otherwise, and a failure placed before the bytes that settle
    /// the verdict must be met.
    #[test]
    fn decoders_are_independent_of_the_read_schedule(
        party in arb_oracle_party_nonempty(),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        mangling in arb_mangling(),
        schedule in arb_io_schedule(),
        failure in any::<Index>(),
    ) {
        use crate::Ranked;

        let (a, b) = (from_oracle_version(&a), from_oracle_version(&b));
        let clock = Clock::from_parts(from_oracle_party(&party), a.clone());
        let span = (&a & &b).span(&(&a | &b));
        check_reader_decoder(&from_oracle_party(&party), &mangling, &schedule, failure)?;
        check_reader_decoder(&a, &mangling, &schedule, failure)?;
        check_reader_decoder(&clock, &mangling, &schedule, failure)?;
        check_reader_decoder(&span, &mangling, &schedule, failure)?;
        check_reader_decoder(&a.rank(), &mangling, &schedule, failure)?;
        check_reader_decoder(&Ranked::from(a.clone()), &mangling, &schedule, failure)?;
    }
}

/// Checks one borsh decoder on `input` through a [`ScriptedReader`] on
/// `schedule` that fails at `fail_at`, if given, against the same decoder over
/// the whole input in memory.
///
/// Borsh values compose inside a larger stream, so a decoder reads no byte
/// past its value, and the bytes it requests do not depend on how they are
/// served. A probe reader that serves the whole input at once and fails at its
/// end measures them: the bytes it served, or every byte and one more when the
/// decoder met its failure. A failure is met exactly when it lies before the
/// requested bytes end, and must come back with the reader's kind. Otherwise
/// the verdict must equal the in-memory one, and an accepted value must have
/// consumed the same bytes; what a rejecting decode consumed is unspecified.
#[cfg(feature = "borsh")]
fn check_borsh<T: borsh::BorshDeserialize + core::fmt::Debug + PartialEq>(
    input: &[u8],
    schedule: &IoSchedule,
    fail_at: Option<usize>,
) -> Result<(), TestCaseError> {
    let mut whole = input;
    let expected = T::deserialize_reader(&mut whole).map_err(|error| error.kind());
    let consumed = input.len() - whole.len();
    let at_once = IoSchedule {
        chunk: input.len().max(1),
        interrupts: vec![0],
    };
    let mut probe = ScriptedReader::new(input, &at_once, Some(input.len()));
    let _ = T::deserialize_reader(&mut probe);
    let requested = if probe.has_failed() {
        input.len() + 1
    } else {
        probe.served()
    };

    let mut reader = ScriptedReader::new(input, schedule, fail_at);
    let verdict = T::deserialize_reader(&mut reader).map_err(|error| error.kind());
    if fail_at.is_some_and(|offset| offset < requested) {
        prop_assert!(
            reader.has_failed(),
            "a failure at {:?} went unmet; input {:?}",
            fail_at,
            input
        );
        prop_assert_eq!(verdict, Err(io::ErrorKind::Other), "input {:?}", input);
    } else {
        prop_assert!(
            !reader.has_failed(),
            "a read past the value met the failure at {:?}; input {:?}",
            fail_at,
            input
        );
        let accepted = expected.is_ok();
        prop_assert_eq!(verdict, expected, "input {:?}", input);
        if accepted {
            prop_assert_eq!(
                reader.served(),
                consumed,
                "bytes consumed; input {:?}",
                input
            );
        }
    }
    Ok(())
}

/// Checks one borsh decoder on every input its value's borsh bytes yield,
/// with the reader failing nowhere, at the input's end, and at a generated
/// offset.
#[cfg(feature = "borsh")]
fn check_borsh_decoder<T>(
    value: &T,
    mangling: &Mangling,
    schedule: &IoSchedule,
    failure: Index,
) -> Result<(), TestCaseError>
where
    T: borsh::BorshDeserialize + borsh::BorshSerialize + core::fmt::Debug + PartialEq,
{
    let encoding = borsh::to_vec(value).expect("serializing into memory succeeds");
    for input in mangled_inputs(&encoding, mangling) {
        let end = input.len();
        for fail_at in [None, Some(end), Some(failure.index(end + 1))] {
            check_borsh::<T>(&input, schedule, fail_at)?;
        }
    }
    Ok(())
}

#[cfg(feature = "borsh")]
proptest! {
    /// Every borsh decoder reaches the verdict it reaches over the whole input
    /// in memory through any read schedule, and returns every reader failure
    /// it meets.
    ///
    /// Each [`Canonical`] type, and `Count`, decodes a generated value's borsh
    /// bytes, those bytes with more appended, a strict prefix, and the bytes
    /// with one bit flipped, through the scripted reader of
    /// `rank_decode_is_independent_of_the_read_schedule`. A decoder reads no
    /// byte past its value, so a failure beyond the bytes it requests must go
    /// unmet, one within them must come back with the reader's kind, and an
    /// accepted value must consume exactly the bytes the in-memory decode did.
    #[test]
    fn borsh_decoders_are_independent_of_the_read_schedule(
        party in arb_oracle_party_nonempty(),
        a in arb_oracle_version(),
        b in arb_oracle_version(),
        count in crate::testing::generators::arb_magnitude(),
        mangling in arb_mangling(),
        schedule in arb_io_schedule(),
        failure in any::<Index>(),
    ) {
        use crate::Ranked;

        let (a, b) = (from_oracle_version(&a), from_oracle_version(&b));
        let clock = Clock::from_parts(from_oracle_party(&party), a.clone());
        let span = (&a & &b).span(&(&a | &b));
        check_borsh_decoder(&from_oracle_party(&party), &mangling, &schedule, failure)?;
        check_borsh_decoder(&a, &mangling, &schedule, failure)?;
        check_borsh_decoder(&clock, &mangling, &schedule, failure)?;
        check_borsh_decoder(&span, &mangling, &schedule, failure)?;
        check_borsh_decoder(&a.rank(), &mangling, &schedule, failure)?;
        check_borsh_decoder(&Ranked::from(a.clone()), &mangling, &schedule, failure)?;
        check_borsh_decoder(&Count(count), &mangling, &schedule, failure)?;
    }
}

/// Every version-derived rank encoding is no larger than its source version.
///
/// The constructed families vary numerator width, exponent depth, and dense
/// fractional bits. Each checks the claimed linear size bound directly.
#[test]
fn rank_encoding_size_is_provenance_linear() {
    use crate::testing::oracles::tree::Version as V;
    // A deep spine holding one unit leaf: rank 2⁻ᵏ, the exponent axis.
    fn spine(depth: usize) -> Version {
        use crate::testing::oracles::tree::Version as V;
        let mut tree = V::leaf(1u8);
        for _ in 0..depth {
            tree = V::node(0u8, tree, V::leaf(0u8));
        }
        from_oracle_version(&tree)
    }
    // The dense staircase: one new unit plateau per level, so every level
    // contributes a set fraction bit: the set-bits-per-level maximum the
    // white-box attack found.
    fn staircase(depth: usize) -> Version {
        use crate::testing::oracles::tree::Version as V;
        let mut tree = V::node(0u8, V::leaf(1u8), V::leaf(0u8));
        for _ in 0..depth {
            tree = V::node(0u8, tree, V::leaf(1u8));
        }
        from_oracle_version(&tree)
    }
    // A wide counter behind a spine: both axes at once.
    fn deep_counter(depth: usize, counter: &BigUint) -> Version {
        use crate::testing::oracles::tree::Version as V;
        let mut tree = V::leaf(counter.clone());
        for _ in 0..depth {
            tree = V::node(0u8, tree, V::leaf(0u8));
        }
        from_oracle_version(&tree)
    }
    let wide = (BigUint::ONE << 128usize) - 1u8;
    let families: [(&str, Version); 5] = [
        ("wide counter", from_oracle_version(&V::leaf(wide.clone()))),
        ("deep spine", spine(800)),
        ("dense staircase", staircase(800)),
        ("deep wide counter", deep_counter(400, &wide)),
        // The answer-embedding shape's essence at test scale: a wide plateau
        // over dense puncturing turns, keeping the numerator wide *and* dense.
        ("plateau puncture", {
            let mut tree = V::leaf(wide.clone());
            for _ in 0..100 {
                tree = V::node(0u8, V::node(1u8, tree, V::leaf(0u8)), V::leaf(0u8));
            }
            from_oracle_version(&tree)
        }),
    ];
    for (name, version) in families {
        let input_bits = version.encoded_bits() as f64;
        let encoded_bits = (version.rank().encode().len() * 8) as f64;
        assert!(
            encoded_bits <= input_bits,
            "{name}: encoded rank ({encoded_bits} bits) exceeds the pinned \
             1.0 ratio over input ({input_bits} bits)"
        );
    }
}

/// Direct cases pin suffix safety at the padding boundary, where
/// a naive expansion spelling would make one encoding a byte prefix of
/// another's.
///
/// Each pair is two distinct ranks whose streams agree bit-for-bit up to where
/// the smaller one's content ends: an integral-only rank against a
/// deep-fraction extension, zero against a small deep fraction, and a one-bit
/// fraction against an extension whose extra bits begin with a byte's worth of
/// zeros. For each pair the law's full strength is asserted directly: the
/// encodings are not byte prefixes of one another, so the smaller rank's key
/// sorts first under *any* tiebreak suffix, including the worst one, `0xFF`
/// against the larger key's continuation.
#[test]
fn rank_encoding_is_suffix_safe_at_the_padding_boundary() {
    let pairs: [(super::Rank, super::Rank); 3] = [
        // 5 against 5 + 2⁻⁴⁰: equal integral parts, one fraction empty.
        (
            uniform(5u8).rank(),
            super::Rank::from_raw(BigUint::from(5u128 << 40 | 1), 40),
        ),
        // Zero against 2⁻⁹: the empty stream tail against a fraction
        // whose first byte's worth of expansion bits is all zero.
        (
            super::Rank::ZERO,
            super::Rank::from_raw(BigUint::from(1u8), 9),
        ),
        // 1/2 against 1/2 + 2⁻⁸: the extension's extra expansion bits
        // are exactly the shorter stream's padding, then a set bit.
        (
            super::Rank::from_raw(BigUint::from(1u8), 1),
            super::Rank::from_raw(BigUint::from(129u8), 8),
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
    /// every choice of tiebreak: the KV-key contract `Rank::encode` documents.
    ///
    /// Over pairs mixing far-apart magnitude classes, forced class ties, and
    /// near-miss extensions (the second rank re-derived from the first with a
    /// deepened fraction), with arbitrary suffix bytes on both keys.
    #[test]
    fn rank_lex_encoding_is_suffix_safe(
        a in arb_wide_rank(),
        unrelated in arb_wide_rank(),
        extend in any::<bool>(),
        deepen in 1u32..64,
        suffix_a in proptest::collection::vec(any::<u8>(), 0..5),
        suffix_b in proptest::collection::vec(any::<u8>(), 0..5),
    ) {
        let b = if extend {
            // A strict extension of `a`'s expansion: the case where
            // one stream continues past the other's content.
            let (num, exp) = rank_parts(&a);
            super::Rank::from_raw(
                (num << (deepen as usize)) + 1u8,
                exp.saturating_add(u64::from(deepen)),
            )
        } else {
            unrelated
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

// The n-ary fold entry points against the sequential pair fold (`join_all`,
// `meet_all`, both `Sum` forms, both `FromIterator` forms, in every feed
// order) are the `laws::VERSION_LIST` / `VERSION_AND_LIST` fold laws
// (version_sum_is_the_sequential_pair_fold, version_sum_is_order_invariant,
// join_all_is_the_sequential_pair_fold, meet_all_is_the_sequential_pair_fold,
// fold_all_is_rotation_invariant), driven at boundary-band arities over the
// organic, arbitrary, and fuzz-decoded populations.

proptest! {
    /// `join_all` and `meet_all` match both independent semantic models over
    /// arbitrary normal-form pools.
    ///
    /// Each production entry point folds the receiver and its items. The
    /// recursive model performs the corresponding binary tree fold, while the
    /// function-space model takes pointwise maxima or minima. Independent
    /// shapes are the inputs on which the balanced production fold restructures
    /// most aggressively.
    #[test]
    fn version_collection_folds_match_all_models(
        pool in proptest::collection::vec(arb_oracle_version(), 1..8),
    ) {
        let versions: Vec<Version> = pool.iter().map(from_oracle_version).collect();
        let (first, rest) = versions.split_first().expect("the pool is nonempty");
        let grid = function::fs_grid(
            &pool.iter().map(function::ev_depth).collect::<Vec<_>>(),
        );

        let joined = first.join_all(rest);
        let joined_tree = pool
            .iter()
            .cloned()
            .reduce(|left, right| left | right)
            .expect("the pool is nonempty");
        let joined_function = function::join_all(pool.iter().cloned().map(function::lift_ev));
        prop_assert_eq!(to_oracle_version(&joined), joined_tree.clone());
        prop_assert_eq!(
            function::ev_order(&joined_function, &function::lift_ev(joined_tree), grid),
            Some(Ordering::Equal),
        );

        let met = first.meet_all(rest);
        let met_tree = crate::testing::oracles::tree::Version::meet_all(pool.iter().cloned())
            .expect("the pool is nonempty");
        let met_function = function::meet_all(pool.iter().cloned().map(function::lift_ev))
            .expect("the pool is nonempty");
        prop_assert_eq!(to_oracle_version(&met), met_tree.clone());
        prop_assert_eq!(
            function::ev_order(&met_function, &function::lift_ev(met_tree), grid),
            Some(Ordering::Equal),
        );
    }
}

/// `meet_all` on the meet-shade population returns exactly the carrier, in
/// every feed order, agreeing with the sequential fold and the recursive
/// oracle.
///
/// The registry family `meter::meet_shade` doubles as a differential shape:
/// one dense carrier among dominating single-leaf shades, the population
/// whose running meet never shrinks.
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
        let oracle = crate::testing::oracles::tree::Version::meet_all(
            population.iter().map(to_oracle_version),
        )
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
/// canonical bytes, while the rank question, asked explicitly through
/// `rank`, still answers a tie.
#[test]
fn ranked_orders_equal_rank_concurrent_pairs_by_bytes() {
    let half = half();
    let peaks = peaks();
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
// ranked_orders_by_rank_then_bytes` law. Rank-only encoding equivalence is a
// clause of `laws::VERSION_SOLO::ranked_carries_own_rank`. Both run over all
// three law populations, a strict superset of the arbitrary pairs alone.

/// Adds one unit plateau per level, leaning the previous tree left or right.
///
/// The two leans are mirror images: their areas agree level for level, so they
/// share a rank by symmetry. The depth extends beyond generated inputs.
fn stairs(depth: usize, lean_left: bool, core: &Version) -> Version {
    use crate::testing::oracles::tree::Version as V;
    let mut tree = to_oracle_version(core);
    for _ in 0..depth {
        tree = if lean_left {
            V::node(0u8, tree, V::leaf(1u8))
        } else {
            V::node(0u8, V::leaf(1u8), tree)
        };
    }
    from_oracle_version(&tree)
}

/// Constructed ranks exercise deep cancellation and last-contribution verdicts.
///
/// A staircase and its mirror image share a rank by symmetry while their mass
/// sits at opposite ends of the interval, so the signed co-sweep's running
/// difference swings through every level's magnitude before the exact total
/// cancels: the widest cancellation an 800-level walk can force through the
/// freeze and deferral machinery, handing the mirror pair's verdict to the
/// version-byte tiebreak. Splitting one mirror's core step then moves the total
/// by `2⁻⁸⁰³` alone: every level above still cancels, and the verdict's sign
/// rests entirely on the deepest contribution. Each verdict is checked against
/// the materialized rank-then-bytes order in both argument orders, and the
/// rank-only encoding against `Rank::encode` on the same deep shapes.
#[test]
fn ranked_fused_walk_survives_deep_cancellation() {
    let half = half();
    let left = stairs(800, true, &half);
    let right = stairs(800, false, &half);
    // The same mirror with its deepest step split: a rank-3/8 core instead of
    // 1/2, so the total drops by exactly 2⁻⁸⁰³ after 800 levels of
    // cancellation.
    use crate::testing::oracles::tree::Version as V;
    let shallower_core = from_oracle_version(&V::node(
        0u8,
        V::node(0u8, V::leaf(1u8), V::node(0u8, V::leaf(1u8), V::leaf(0u8))),
        V::leaf(0u8),
    ));
    let shallower = stairs(800, false, &shallower_core);
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
// op-trace populations, and the fuzz target's decoded values, exactly the
// population where a prefix-aliasing bug would live.

/// Version encodings remain prefix-free along deep growth chains.
///
/// A tick chain (each version one event past the last), a spine tower (each one
/// level deeper, out to 800 levels, extreme depth past the arb generator's
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
        use crate::testing::oracles::tree::Version as V;
        let mut tree = V::leaf(1u8);
        for _ in 0..depth {
            tree = V::node(0u8, tree, V::leaf(0u8));
        }
        battery.push(from_oracle_version(&tree));
    }
    // The 800-level staircase and its mirror: extreme depth past any
    // generator's reach, sharing a rank by symmetry: the case whose
    // streams extend structure level by level.
    battery.push(stairs(800, true, &half()));
    battery.push(stairs(800, false, &half()));
    battery.push(half());
    battery.push(peaks());
    battery.push(Version::new());
    for (i, a) in battery.iter().enumerate() {
        for b in &battery[i + 1..] {
            if a == b {
                continue;
            }
            assert!(
                !a.as_bytes().starts_with(b.as_bytes()) && !b.as_bytes().starts_with(a.as_bytes()),
                "prefix-free: {a:?} vs {b:?}"
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
    /// the `version_encoding_is_prefix_free` law); this pins the composition
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
                "prefix-free: {:?} vs {:?}", a, b
            );
        }
        let key_a = [ea, suffix_a].concat();
        let key_b = [eb, suffix_b].concat();
        match ra.cmp(&rb) {
            Ordering::Less => prop_assert!(key_a < key_b, "{:?} vs {:?}", a, b),
            Ordering::Greater => prop_assert!(key_a > key_b, "{:?} vs {:?}", a, b),
            Ordering::Equal => {}
        }
    }
}

/// Direct cases pin composite-key suffix safety at the tiebreak boundary:
/// pairs whose keys agree byte-for-byte through the whole rank component, so
/// the order is decided inside the version tail.
///
/// The equal-rank concurrent pair (half vs. the two-peak tree), the empty
/// version against half (the zero rank's one-byte stream against a fractional
/// one, decided in the rank component, with the version tail present on both),
/// a tick chain pair, and the 800-level mirror staircases (rank-equal at
/// extreme depth, so the keys agree through a rank prefix hundreds of bytes
/// long before the version tail decides). For each pair the full strength is
/// asserted directly: neither key is a byte prefix of the other, byte order
/// equals the views' total order, and the worst suffixes (`0xFF` on the smaller
/// key, `0x00` on the larger) cannot flip it.
#[test]
fn ranked_composite_key_is_suffix_safe_at_the_tiebreak_boundary() {
    let half = half();
    let peaks = peaks();
    assert_eq!(half.rank(), peaks.rank(), "the boundary pair shares a rank");
    let mut clock = Clock::seed();
    let one = clock.tick().clone();
    let two = clock.tick().clone();
    let deep_left = stairs(800, true, &half);
    let deep_right = stairs(800, false, &half);
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
            "prefix-free: {small:?} vs {large:?}"
        );
        assert!(
            es < el,
            "byte order is the total order: {small:?} vs {large:?}"
        );
        let key_small = [es, vec![0xFF; 4]].concat();
        let key_large = [el, vec![0x00; 4]].concat();
        assert!(
            key_small < key_large,
            "no suffix flips the order: {small:?} vs {large:?}"
        );
    }
}

/// Direct cases cover each composite-specific `Ranked::decode` rejection.
///
/// Empty input; truncation at every byte boundary of a composite (cuts land in
/// the rank stream, at the component boundary, and inside the version); a trailing
/// zero byte (the version component's whole-input strictness); a set bit in the
/// version's padding; and a well-formed rank prefix paired with a version it
/// does not measure, witnessed from both sides of the true rank (the
/// composite's redundancy check (`NotCanonical`). Component-specific errors
/// are covered by `rank_decoding_rejects_each_malformed_input_class` and the codec rejection
/// battery); the cuts here prove each component's rejection surfaces through
/// the composite entry.
#[test]
fn ranked_decode_rejects_each_composite_error() {
    let half = half();
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
    let five = uniform(5u8);
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
    /// The mutation targets the composite boundary: a
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
                        "accepted mutation must re-encode to itself: {:?} byte {} bit {}",
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
/// from where the events happened keeps nothing, lazily and materialized
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
/// "comb" region (two quarters in different halves) becomes two concurrent
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

    let v = uniform(1u8);
    assert_eq!(v.min_ticks(), Count::from(1u64)); // one tick covers the whole interval

    let frag = (&v / comb).to_version();
    assert!(frag <= v); // still a sub-version
    assert_eq!(frag.min_ticks(), Count::from(2u64)); // but now two concurrent peaks
}

/// A tick that fills can *lower* `min_ticks`: the floor is not monotone under
/// ticking.
///
/// Two concurrent peaks on the outer quarters (`min_ticks` 2) ticked by the
/// party owning the two inner quarters fill to the uniform version at height
/// 1, which one tick of the seed yields (`min_ticks` 1). The laws
/// `tick_raises_min_ticks_by_at_most_one` and
/// `ticks_raise_min_ticks_by_at_most_the_count` therefore bound only how far a
/// tick raises `min_ticks`.
#[test]
fn tick_can_lower_min_ticks() {
    use crate::testing::oracles::tree::{Party as P, Version as V};
    let mut peaks = from_oracle_version(&V::node(
        0u8,
        V::node(0u8, V::leaf(1u8), V::leaf(0u8)),
        V::node(0u8, V::leaf(0u8), V::leaf(1u8)),
    ));
    assert_eq!(peaks.min_ticks(), Count::from(2u64));
    let inner_quarters = from_oracle_party(&P::node(
        P::node(P::Leaf(false), P::Leaf(true)),
        P::node(P::Leaf(true), P::Leaf(false)),
    ));

    peaks.tick(&inner_quarters);

    assert_eq!(peaks, uniform(1u8)); // the fill raised both inner quarters to 1
    assert_eq!(peaks.min_ticks(), Count::from(1u64));
}

/// The at-rest form is exactly the wire bytes' refcounted handle.
///
/// A [`Version`] is exactly one immutable refcounted byte buffer
/// alone: pointer, byte length, shared-state pointer, vtable: 32 bytes on
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
    /// Byte-level equality (``==``) agrees with a plain
    /// bit-level compare of the live streams, in both operand orders.
    ///
    /// Canonical padding makes raw byte equality equivalent to live-bit
    /// equality. Equal values must also hash equally.
    #[test]
    fn byte_equality_matches_bit_equality(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let bit_eq =
            crate::version::instrument::bits(&a) == crate::version::instrument::bits(&b);
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
    /// distinct buffer (the byte compare or the full walk): comparison
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
    /// The composite row-key shape (a rank's canonical stream, then a
    /// fixed-width opaque key) orders by first difference exactly as `(rank,
    /// suffix)` orders lexicographically.
    ///
    /// The KV-key use `Rank::encode` documents, exercised in context: distinct
    /// ranks decide the composite inside the rank prefix (no 32-byte suffix can
    /// flip it, in either assignment), and equal ranks (byte-identical
    /// prefixes, by canonical uniqueness) hand the verdict to the suffix's own
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
/// sequential fold at arities that fill or straddle a counter level, k = 4 and
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
        let mut party = Party::seed();
        for _ in 0..depth {
            let _ = party.fork();
        }
        let v = (&uniform(1u8) / &party).to_version();
        v.encode().len() * 8
    };
    let (small, large) = (spine(1_000), spine(2_000));
    assert_eq!(
        large - small,
        3 * 1_000,
        "the deep spine's marginal level cost moved off 3 bits"
    );
}
