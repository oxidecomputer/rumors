//! Party tests: fork/join round-trip, disjointness, split/sum, and overlap
//! behavior, all differential against the oracle.

use proptest::prelude::*;

use super::ops::IdIndex;
use super::Party;
use crate::idbits::IdReader;
use crate::testing::bridge::{from_oracle_party, to_oracle_party};
use crate::testing::complexity::{assert_linear_scaling, steps_of, MIN_SCALE};
use crate::testing::fold_oracle;
use crate::testing::generators::{
    arb_oracle_party, arb_oracle_party_nonempty, arb_shape, covers_stress_pair, shape_party,
    skip_stress_pair, Shape,
};
use crate::testing::optrace::{run, world_strategy};

// ───────────────────────────── the join fold ─────────────────────────────

/// Build one organic history's party population, deterministically
/// reproducible from the same ops.
fn world_parties(ops: &[crate::testing::optrace::Op]) -> Vec<Party> {
    run(ops)
        .iter()
        .map(|c| from_oracle_party(c.party()))
        .collect()
}

proptest! {
    /// The balanced `join_all` is the sequential fold on parties.
    ///
    /// Over one organic history's pairwise-disjoint parties, folding the
    /// rest into any member returns `Ok` with exactly the party the
    /// sequential `join`-per-input reference produces, in both input
    /// orders.
    #[test]
    fn join_all_matches_the_sequential_fold(ops in world_strategy(), i in 0usize..64, reverse in any::<bool>()) {
        let mut reference_pool = world_parties(&ops);
        let n = reference_pool.len();
        let mut reference = reference_pool.remove(i % n);
        if reverse {
            reference_pool.reverse();
        }
        for p in reference_pool {
            reference.join(p).expect("one world's parties are pairwise disjoint");
        }

        let mut pool = world_parties(&ops);
        let mut acc = pool.remove(i % n);
        if reverse {
            pool.reverse();
        }
        acc.join_all(pool).expect("one world's parties are pairwise disjoint");
        prop_assert_eq!(acc, reference);
    }
}

/// Aliased inputs stay best-effort: a duplicated share collides on its
/// way in and is handed back whole (nothing panics, nothing is dropped),
/// while the honest copy of every share still reunites the seed region.
///
/// The duplicate rides directly behind its original, so the collision
/// happens original-against-duplicate; which parties come back for other
/// interleavings is deliberately unspecified (the contract's
/// order-dependence for aliased input).
#[test]
fn join_all_hands_back_aliased_inputs() {
    let mut acc = Party::seed();
    let shares: Vec<Party> = acc.forks(3).collect();
    let mut dup_seed = Party::seed();
    let mut dups = dup_seed.forks(3);
    let duplicate = dups.next().expect("three shares were requested");
    drop(dups);
    drop(dup_seed);
    let mut again = Party::seed();
    let mut again_shares = again.forks(3);
    let expected_back = again_shares.next().expect("three shares were requested");
    drop(again_shares);
    drop(again);
    let mut inputs = shares;
    inputs.insert(1, duplicate);
    let back = acc
        .join_all(inputs)
        .expect_err("the duplicated share must collide");
    assert_eq!(back.len(), 1, "exactly the duplicate comes back");
    assert_eq!(back[0], expected_back, "the duplicate comes back whole");
    assert!(
        acc.is_seed(),
        "the honest copy of every share reunites the seed region"
    );
}

// ───────────────────── the fold's up-front index, differentially ─────────────────────
//
// `join_all`'s up-front overlap test runs against a per-call `IdIndex` of
// the fixed accumulator; the index is a performance mechanism only, so
// every observable outcome — the hand-back vector (contents *and* order)
// and the accumulator's final bytes — must be exactly what the same
// discipline decides with the up-front test spelled as a per-input cursor
// walk (`testing::fold_oracle`). These differentials pin that, across
// arbitrary mixes and the named adversarial ones.

/// Run the production fold and the cursor-walk oracle over one input
/// population and assert identical outcomes.
///
/// `build` constructs the population twice and must be deterministic.
/// Identical outcomes: the same `Ok`/`Err` with the same hand-back
/// vector in the same order, and byte-identical accumulators.
fn assert_join_all_matches_oracle(build: impl Fn() -> (Party, Vec<Party>)) {
    let (mut acc_new, inputs_new) = build();
    let (mut acc_ref, inputs_ref) = build();
    let new = acc_new.join_all(inputs_new);
    let reference = fold_oracle::party_join_all(&mut acc_ref, inputs_ref);
    assert_eq!(
        new, reference,
        "the indexed fold and the cursor-walk oracle must hand back the same inputs in the \
         same order"
    );
    assert_eq!(
        acc_new.as_bytes(),
        acc_ref.as_bytes(),
        "the indexed fold and the cursor-walk oracle must leave byte-identical accumulators"
    );
}

proptest! {
    /// The indexed fold decides exactly as the cursor-walk oracle over
    /// arbitrary normal-form mixes.
    ///
    /// An arbitrary accumulator against inputs drawn with repetition
    /// from an arbitrary pool — mixed sizes, duplicates, and every
    /// overlap disposition (against the accumulator, against each
    /// other, or none) arise from the draws — with identical hand-backs
    /// and byte-identical accumulators.
    #[test]
    fn join_all_matches_the_cursor_walk_oracle(
        oacc in arb_oracle_party_nonempty(),
        (pool, picks) in proptest::collection::vec(arb_oracle_party_nonempty(), 1..5)
            .prop_flat_map(|pool| {
                let len = pool.len();
                (Just(pool), proptest::collection::vec(0..len, 0..12))
            }),
    ) {
        let build = || {
            let acc = from_oracle_party(&oacc);
            let inputs: Vec<Party> =
                picks.iter().map(|&i| from_oracle_party(&pool[i])).collect();
            (acc, inputs)
        };
        assert_join_all_matches_oracle(build);
    }
}

/// With no overlap anywhere, the indexed fold and the cursor-walk
/// oracle agree.
///
/// A forked population reuniting: both return `Ok` and rebuild the
/// same accumulator, byte for byte.
#[test]
fn join_all_agrees_with_oracle_when_none_overlap() {
    assert_join_all_matches_oracle(|| {
        let mut acc = Party::seed();
        let shares: Vec<Party> = acc.forks(5).collect();
        (acc, shares)
    });
}

/// The hand-back outcome is invariant to where the overlapping input
/// sits in the sequence — first, interior, or last.
///
/// The indexed fold and the cursor-walk oracle hand back exactly the
/// aliased input at every position, with the honest shares still
/// reuniting.
#[test]
fn join_all_agrees_with_oracle_at_every_overlap_position() {
    for position in [0usize, 2, 4] {
        assert_join_all_matches_oracle(|| {
            let mut acc = Party::seed();
            let mut inputs: Vec<Party> = acc.forks(4).collect();
            // The residual `acc` region duplicated: overlaps `acc` and
            // nothing else, so exactly it comes back.
            inputs.insert(position, acc.dangerously_alias());
            (acc, inputs)
        });
    }
}

/// On the maximally-deferred witness, the indexed fold and the
/// cursor-walk oracle hand every input back in order and leave the
/// accumulator untouched.
///
/// Every input aliases a deep spine accumulator whose single owned
/// region is its preorder-last tip, so each overlap test resolves only
/// at the stream's end.
#[test]
fn join_all_agrees_with_oracle_on_all_overlapping_deferred_witness() {
    assert_join_all_matches_oracle(|| {
        let acc = shape_party(Shape::RightSpine, 64);
        let inputs: Vec<Party> = (0..8).map(|_| acc.dangerously_alias()).collect();
        (acc, inputs)
    });
}

// ───────────────────────────── differential vs oracle ─────────────────────────────

proptest! {
    /// `fork` yields two disjoint halves, both matching the oracle's split;
    /// `join` of the two recovers the parent.
    #[test]
    fn d_fork_join_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let mut oracle_party = cs[i % n].party().clone();
        let snapshot = oracle_party.clone();

        let mut keep = from_oracle_party(&snapshot);
        let parent = from_oracle_party(&snapshot);
        let oracle_child = oracle_party.fork();
        let child = keep.fork();

        // Both halves match the oracle's split.
        prop_assert!(keep == from_oracle_party(&oracle_party));
        prop_assert!(child == from_oracle_party(&oracle_child));

        // Forks are disjoint, and join recovers the parent.
        prop_assert!(keep.is_disjoint(&child));
        prop_assert!(keep.join(child).is_ok());
        prop_assert!(keep == parent);
    }
}

// ───────────────────────────── complexity (linear scaling) ─────────────────────────────

proptest! {
    /// Complexity. `split` is `O(n)`: over a random deep id shape, its
    /// traversal steps grow linearly (not quadratically) from `scale` to `4 *
    /// scale` — proving no re-scan to find a right child.
    #[test]
    fn split_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let p = shape_party(shape, s);
            steps_of(|| {
                IdReader::root(p.as_bits()).split();
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `sum` is `O(n + m)`: on a deep disjoint pair (the halves of
    /// a forked spine) its steps grow linearly with shape size.
    #[test]
    fn sum_is_linear(shape in arb_shape(), scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let mut keep = shape_party(shape, s);
            let give = keep.fork(); // a deep disjoint pair; this build is not measured
            steps_of(|| {
                IdReader::root(keep.as_bits()).sum(IdReader::root(give.as_bits()));
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `is_disjoint` is `O(n + m)`: a *misaligned* disjoint pair (a
    /// shallow `0`-leaf on one side aligned with the other's whole deep
    /// subtree) drives the bounded lazy-skip at scale.
    ///
    /// The pair is disjoint, so the walk runs to completion (no early `false`)
    /// and the skip dominates; steps stay linear from `scale` to `4 * scale`,
    /// proving each node is skipped at most once (no per-node re-scan).
    #[test]
    fn is_disjoint_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let (a, b) = skip_stress_pair(s);
            steps_of(|| {
                IdReader::root(a.as_bits()).is_disjoint(IdReader::root(b.as_bits()));
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `covers` is `O(n + m)`: a *covering* misaligned pair (`a`'s
    /// full `1` leaf at each level aligned against `b`'s small owned subtree)
    /// drives the bounded lazy-skip at scale.
    ///
    /// `a` covers `b`, so the walk runs to completion (no early `false`) and the
    /// skip dominates; steps stay linear from `scale` to `4 * scale`, proving
    /// each node is skipped at most once (no per-node re-scan).
    #[test]
    fn covers_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let (a, b) = covers_stress_pair(s);
            steps_of(|| {
                IdReader::root(a.as_bits()).covers(IdReader::root(b.as_bits()));
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

proptest! {
    /// Complexity. `diff` is `O(n + m)`: on the misaligned disjoint pair, a
    /// shallow unowned plateau on `a` overlays `b`'s whole deep subtree, so
    /// the sweep consumes every `b` plateau against single `a` plateaus.
    ///
    /// The pair is disjoint, so the walk runs to completion (nothing empties
    /// early); steps stay linear from `scale` to `4 * scale`, proving each
    /// tag is read at most once (no per-node re-scan).
    #[test]
    fn diff_is_linear(scale in MIN_SCALE..256) {
        let measure = |s: usize| {
            let (a, b) = skip_stress_pair(s);
            steps_of(|| {
                IdReader::root(a.as_bits()).diff(IdReader::root(b.as_bits()));
            })
        };
        assert_linear_scaling(measure(scale), measure(scale * 4));
    }
}

// ───────────────────────────── covering (containment) ─────────────────────────────

proptest! {
    /// `covers` on arbitrary id pairs — typically *unrelated* and frequently
    /// *overlapping* — agrees with the oracle, including the partial-overlap
    /// case (neither covers the other) that the seed pipeline never produces.
    ///
    /// Covering is *antisymmetric*: two regions cover each other exactly when
    /// they are equal.
    #[test]
    fn covers_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        prop_assert_eq!(ia.covers(&ib), oa.covers(&ob));
        prop_assert_eq!(ib.covers(&ia), ob.covers(&oa));
        prop_assert_eq!(ia.covers(&ib) && ib.covers(&ia), ia == ib);
    }
}

proptest! {
    /// On seed-derived parties, covering tracks the fork/join lattice.
    ///
    /// The whole [`Party::seed`] covers every live party, a party covers itself
    /// (and any alias), a fork's parent covers both resulting halves, and the
    /// rejoin of two halves covers each part. Disjoint live halves cover neither
    /// other — the partial-overlap-free shadow of [`Party::is_disjoint`].
    #[test]
    fn covers_tracks_fork_join(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let snapshot = cs[i % n].party().clone();

        // The whole covers any live party; a party covers an alias of itself.
        let live = from_oracle_party(&snapshot);
        prop_assert!(Party::seed().covers(&live));
        prop_assert!(live.covers(&live.dangerously_alias()));

        // A fork's parent covers both halves; the halves cover neither other.
        let mut keep = from_oracle_party(&snapshot);
        let parent = from_oracle_party(&snapshot);
        let give = keep.fork();
        prop_assert!(parent.covers(&keep));
        prop_assert!(parent.covers(&give));
        prop_assert!(!keep.covers(&give));
        prop_assert!(!give.covers(&keep));

        // The rejoin of the two halves covers each part it absorbed.
        let keep_half = keep.dangerously_alias();
        let give_half = give.dangerously_alias();
        keep.join(give).expect("disjoint halves rejoin");
        prop_assert!(keep.covers(&keep_half));
        prop_assert!(keep.covers(&give_half));
    }
}

// ───────────────────────────── join overlap ─────────────────────────────

proptest! {
    /// Joining overlapping parties errors and hands the party back unchanged.
    #[test]
    fn d_join_overlap_hands_back(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let snapshot = cs[i % n].party().clone();

        let mut sub = from_oracle_party(&snapshot);
        let _ = sub.fork(); // sub is now a sub-region of the snapshot
        let whole = from_oracle_party(&snapshot);
        let whole_copy = from_oracle_party(&snapshot);

        prop_assert!(!sub.is_disjoint(&whole));
        match sub.join(whole) {
            Err(handed_back) => prop_assert!(handed_back == whole_copy),
            Ok(()) => prop_assert!(false, "expected an overlap error"),
        }
    }
}

// ───────────────────────── dangerously_alias ─────────────────────────

proptest! {
    /// `dangerously_alias` yields a byte-identical, `Eq` copy that aliases the
    /// original's entire region: the two are therefore *not* disjoint — the
    /// deliberate linearity violation the method documents.
    ///
    /// (The caller alone is responsible for keeping at most one of them live.)
    #[test]
    fn dangerously_alias_aliases_region(op in arb_oracle_party_nonempty()) {
        let p = from_oracle_party(&op);
        let dup = p.dangerously_alias();
        prop_assert!(dup == p);
        prop_assert_eq!(dup.as_bytes(), p.as_bytes());
        prop_assert!(!p.is_disjoint(&dup), "a duplicate aliases the whole region");
    }
}

// ───────────────────────── paper-notation TryFrom ─────────────────────────

/// `TryFrom` numeric/tuple literals build parties via the same paper notation
/// as the string parser.
///
/// The seed `1`, a flat `(1, 0)`, and a nested `((0, 1), (1, (1, 0)))` all
/// construct, while the anonymous bare `0` is rejected (a standalone id must
/// own some region).
#[test]
fn parse_bare_notation() {
    let _party: Party = 1.try_into().unwrap();
    assert!(Party::try_from(0).is_err());
    let _party: Party = (1, 0).try_into().unwrap();
    let _party: Party = ((0, 1), (1, (1, 0))).try_into().unwrap();
}

// ───────────── arbitrary normal-form ids (decoupled from the op pipeline) ─────────────
//
// The op-trace differentials above only ever compare ids that descend from one
// seed (so every pair is causally related and pairwise disjoint by
// construction). These feed *arbitrary* normal-form ids — random shape, random
// ownership, including genuinely *overlapping* and *unrelated* pairs — to every
// id op and diff against the oracle. They reach the overlap/incomparable arms
// (`is_disjoint == false`, `compare == None`, `sum == None`) that the
// seed-derived pipeline cannot produce.

proptest! {
    /// `is_disjoint` on arbitrary id pairs — typically *unrelated* and
    /// frequently *overlapping* — agrees with the oracle, including the
    /// not-disjoint verdict the op pipeline never produces.
    #[test]
    fn disjoint_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        prop_assert_eq!(ia.is_disjoint(&ib), oa.is_disjoint(&ob));
        // Disjointness is symmetric on the impl directly.
        prop_assert_eq!(ia.is_disjoint(&ib), ib.is_disjoint(&ia));
    }
}

proptest! {
    /// The per-call [`IdIndex`] answers disjointness with the identical
    /// verdict as the cursor walk, over arbitrary normal-form pairs —
    /// typically unrelated, frequently overlapping — in both roles
    /// (either operand indexed).
    ///
    /// This is the fold's semantic seam: `join_all`'s up-front test may
    /// differ from `is_disjoint` in mechanism only.
    #[test]
    fn indexed_disjointness_matches_the_cursor_walk(
        oa in arb_oracle_party_nonempty(),
        ob in arb_oracle_party_nonempty(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let walk = ia.is_disjoint(&ib);
        prop_assert_eq!(IdIndex::build(ia.as_bits()).is_disjoint(ib.view()), walk);
        prop_assert_eq!(IdIndex::build(ib.as_bits()).is_disjoint(ia.view()), walk);
    }
}

proptest! {
    /// The per-call [`IdIndex`] matches the cursor walk on *deep*
    /// operand pairs, where the arbitrary generator stays shallow.
    ///
    /// Spines, zigzags, and bushy shapes at scale, in both roles —
    /// driving the index's table search and its skip-free descent
    /// through real depth, on disjoint pairs (both single-tip spine
    /// halves and the misaligned skip-stress pair) and overlapping
    /// ones (a shape against itself).
    #[test]
    fn indexed_disjointness_matches_the_cursor_walk_deep(
        shape_a in arb_shape(),
        shape_b in arb_shape(),
        scale in MIN_SCALE..256,
    ) {
        let a = shape_party(shape_a, scale);
        let b = shape_party(shape_b, scale);
        let (sa, sb) = skip_stress_pair(scale);
        for (x, y) in [(&a, &b), (&a, &a), (&sa, &sb)] {
            let walk = x.is_disjoint(y);
            prop_assert_eq!(IdIndex::build(x.as_bits()).is_disjoint(y.view()), walk);
            prop_assert_eq!(IdIndex::build(y.as_bits()).is_disjoint(x.view()), walk);
        }
    }
}

proptest! {
    /// `split` (the structural op behind `fork`) on an arbitrary non-empty id
    /// matches the oracle's `split`, structurally — on shapes the seed pipeline
    /// never forks.
    ///
    /// The two halves are read straight off the impl's packed `IdReader::split`
    /// output and lowered for comparison.
    #[test]
    fn split_arbitrary(op in arb_oracle_party_nonempty()) {
        let mut oracle_self = op.clone();
        let oracle_give = oracle_self.fork(); // fork = split; mutates `oracle_self` to the kept half

        let p = from_oracle_party(&op);
        let (keep_bits, give_bits) = IdReader::root(p.as_bits()).split();
        let keep = Party::from_bits(keep_bits);
        let give = Party::from_bits(give_bits);

        prop_assert!(keep == from_oracle_party(&oracle_self));
        prop_assert!(give == from_oracle_party(&oracle_give));
    }
}

proptest! {
    /// `sum` on arbitrary id pairs agrees with the oracle: it returns the
    /// merged id exactly when the pair is disjoint (matching
    /// `oracle::Party::join`), and `None` on overlap.
    ///
    /// The op pipeline only ever sums disjoint halves, so the overlap `None` arm
    /// is otherwise untested at arbitrary shapes.
    #[test]
    fn sum_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let summed = IdReader::root(ia.as_bits()).sum(IdReader::root(ib.as_bits()));

        if oa.is_disjoint(&ob) {
            let mut oracle_sum = oa.clone();
            oracle_sum.join(ob.clone()).expect("disjoint, just checked");
            let bits = summed.expect("disjoint pair sums");
            prop_assert!(Party::from_bits(bits) == from_oracle_party(&oracle_sum));
        } else {
            prop_assert!(summed.is_none(), "overlapping ids must not sum");
        }
    }
}

proptest! {
    /// `without` on arbitrary id pairs — typically *unrelated* and frequently
    /// *overlapping* — agrees with the oracle's `without`, mapping the oracle's
    /// empty result to `None`.
    ///
    /// Reaches the partial-overlap shapes the seed-derived pipeline never
    /// produces.
    ///
    /// It also pins the two characterizations the type encodes: the result is
    /// `None` exactly when `other` covers `self`, and whenever a remainder
    /// survives it is a subregion of `self` that is disjoint from `other`
    /// (`self \ other ⊆ self` and `(self \ other) ∩ other = ∅`).
    #[test]
    fn without_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let self_copy = from_oracle_party(&oa);
        let oracle_diff = oa.without(&ob);

        match ia.without(&ib) {
            None => {
                prop_assert!(oracle_diff.is_empty(), "impl emptied but oracle did not");
                prop_assert!(ob.covers(&oa), "None iff `other` covers `self`");
            }
            Some(remainder) => {
                prop_assert!(remainder == from_oracle_party(&oracle_diff));
                prop_assert!(!ob.covers(&oa), "Some iff `other` does not cover `self`");
                prop_assert!(self_copy.covers(&remainder), "the remainder is a subregion of `self`");
                prop_assert!(remainder.is_disjoint(&ib), "the remainder shares nothing with `other`");
            }
        }
    }
}

proptest! {
    /// `without` is the partial inverse of `join` on the fork/join lattice:
    /// carving a forked-off share back out of the parent recovers the kept
    /// half, and removing a *disjoint* share is a no-op.
    ///
    /// Removing a covering share (a party from itself) empties it to `None`.
    #[test]
    fn without_inverts_fork(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let snapshot = cs[i % n].party().clone();

        // Fork splits `parent` into disjoint halves `keep ⊔ give`.
        let mut keep = from_oracle_party(&snapshot);
        let parent = from_oracle_party(&snapshot);
        let give = keep.fork();
        let kept = keep.dangerously_alias(); // a stable reference to the kept half

        // Carving the give-half back out of the parent recovers the kept half:
        // `parent \ give = keep`.
        let carved = parent.without(&give).expect("parent is not covered by its give-half");
        prop_assert!(carved == kept.dangerously_alias());

        // Removing a disjoint share is a no-op: `keep \ give = keep`.
        prop_assert!(keep.is_disjoint(&give));
        let no_op = keep.without(&give).expect("disjoint removal keeps everything");
        prop_assert!(no_op == kept.dangerously_alias());

        // A party covers itself, so removing itself leaves nothing.
        prop_assert!(no_op.without(&kept).is_none());
    }
}

proptest! {
    /// `decode ∘ encode == identity` over arbitrary non-empty normal-form ids,
    /// and the decoded value lowers to the same oracle tree.
    ///
    /// (The anonymous tree is excluded: a standalone `Party` must own a region,
    /// and `decode` rejects it.)
    #[test]
    fn decode_encode_arbitrary(op in arb_oracle_party_nonempty()) {
        let p = from_oracle_party(&op);
        let bytes = p.encode();
        let decoded = Party::decode(&bytes[..]).expect("canonical encoding decodes");
        prop_assert!(decoded == p);
        prop_assert_eq!(to_oracle_party(&decoded), op);
    }
}

proptest! {
    /// `as_bytes` returns exactly the canonical `encode` bytes (zero-padded
    /// tail), over arbitrary non-empty ids — the `id_node`/`extend` build path.
    #[test]
    fn as_bytes_matches_encode(op in arb_oracle_party_nonempty()) {
        let p = from_oracle_party(&op);
        let encoded = p.encode();
        prop_assert_eq!(p.as_bytes(), encoded.as_slice());
    }

    /// The invariant holds for both halves produced by `fork` (the split path),
    /// not just for rebuilt parties.
    #[test]
    fn as_bytes_matches_encode_after_fork(op in arb_oracle_party_nonempty()) {
        let mut p = from_oracle_party(&op);
        let q = p.fork();
        let (pe, qe) = (p.encode(), q.encode());
        prop_assert_eq!(p.as_bytes(), pe.as_slice());
        prop_assert_eq!(q.as_bytes(), qe.as_slice());
    }
}

proptest! {
    /// Byte-level equality (`codec::canonical_eq`) agrees with a plain
    /// bit-level compare of the live id streams, in both operand orders.
    ///
    /// The cross-check that the canonical-raw-slice invariant (dead bits
    /// zeroed at every storage seam) really licenses the byte shortcut
    /// over raw bytes plus live length. Equal values must also hash
    /// equally (`Eq`/`Hash` consistency).
    #[test]
    fn byte_equality_matches_bit_equality(
        oa in arb_oracle_party_nonempty(),
        ob in arb_oracle_party_nonempty(),
    ) {
        let a = from_oracle_party(&oa);
        let b = from_oracle_party(&ob);
        let bit_eq = a.as_bits() == b.as_bits();
        prop_assert_eq!(a == b, bit_eq);
        prop_assert_eq!(b == a, bit_eq);
        if a == b {
            let hash = |p: &Party| {
                use core::hash::{Hash, Hasher};
                let mut h = std::hash::DefaultHasher::new();
                p.hash(&mut h);
                h.finish()
            };
            prop_assert_eq!(hash(&a), hash(&b));
        }
    }
}

// ───────────────────── fork orbits: iterated size trajectories ─────────────────────
//
// A per-call cost bound does not preclude compounding: an operation can
// be cheap per call while its output grows so that iterated application
// is quadratic in total work. These pins fix the size trajectory of
// iterated fork — deterministic, exact at every step, so the whole
// shape is asserted and tuning any one point cannot pass. A future
// change that makes repeated forking mint more than its one tree level
// per split trips a committed diff here.

/// An iterated fork chain's id sizes are exactly affine.
///
/// Following the forked-off child each round (the mover lineage
/// descends one level per split), both halves read exactly `2 + 2·k`
/// encoded bits after the k-th fork, for every k — one two-bit tree
/// level per fork, nothing compounding [measured: exact at all 512
/// steps].
///
/// Liveness floor: the trajectory's equality at `k = 512` is itself the
/// floor — a chain that stopped splitting would read short of 1026
/// bits. Budget: 512 forks of O(depth) each, milliseconds.
#[test]
fn fork_chain_orbit_sizes_are_exactly_affine() {
    let mut p = Party::seed();
    assert_eq!(p.encoded_bits(), 2, "the seed is the 2-bit whole region");
    for k in 1usize..=512 {
        let q = p.fork();
        assert_eq!(p.encoded_bits(), 2 + 2 * k, "keeper id bits after fork {k}");
        assert_eq!(q.encoded_bits(), 2 + 2 * k, "mover id bits after fork {k}");
        p = q;
    }
}

/// An iterated fork fan grows exactly affine and unwinds exactly.
///
/// Each round forks a fresh child off the root lineage (the keeper
/// deepens one level per split), both halves reading exactly `2 + 2·k`
/// encoded bits at the k-th fork; rejoining the children in reverse
/// order then walks the root back down the same trajectory, ending
/// byte-identical to the seed — sizes return, never ratchet [measured:
/// exact at all 512 steps, both directions].
///
/// Liveness floor: the root must visit 1026 bits at the fan's rim and
/// end `is_seed` — an unwind that dropped or double-counted a share
/// would miss one or the other. Budget: 512 forks + 512 joins,
/// milliseconds.
#[test]
fn fork_fan_orbit_grows_affine_and_unwinds_to_seed() {
    let mut root = Party::seed();
    let mut children = Vec::new();
    for k in 1usize..=512 {
        let q = root.fork();
        assert_eq!(
            root.encoded_bits(),
            2 + 2 * k,
            "root id bits after fork {k}"
        );
        assert_eq!(q.encoded_bits(), 2 + 2 * k, "child id bits after fork {k}");
        children.push(q);
    }
    for (i, q) in children.into_iter().rev().enumerate() {
        root.join(q)
            .expect("fan children are disjoint from the root");
        assert_eq!(
            root.encoded_bits(),
            2 + 2 * (511 - i),
            "root id bits after unwind join {i}"
        );
    }
    assert!(root.is_seed(), "the fully unwound fan is the seed again");
}
