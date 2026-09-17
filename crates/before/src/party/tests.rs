//! Independent checks of `Party` against the recursive oracle.

use proptest::prelude::*;

use super::Party;
use crate::idbits::IdReader;
use crate::oracle;
use crate::testing::bridge::{from_oracle_party, to_oracle_party};
use crate::testing::generators::{arb_oracle_party, arb_oracle_party_nonempty};
use crate::testing::optrace::{run, world_strategy};

// ───────────────────────────── the join fold ─────────────────────────────

/// `join_all` and the sequential oracle reunite the same disjoint forks.
#[test]
fn join_all_agrees_with_oracle_when_none_overlap() {
    let mut acc = Party::seed();
    let shares: Vec<Party> = acc.forks(5u64).collect();
    assert_join_all_matches_recursive_oracle(acc, shares);
}

/// An overlap among inputs is reported without losing any region.
#[test]
fn join_all_preserves_regions_on_overlap() {
    let mut acc = Party::seed();
    let mut shares: Vec<Party> = acc.forks(5u64).collect();
    let e = shares.pop().expect("five forks");
    let d = shares.pop().expect("five forks");
    let c = shares.pop().expect("five forks");
    let b = shares.pop().expect("five forks");
    let a = shares.pop().expect("five forks");
    let alias = a.dangerously_alias();
    assert_join_all_matches_recursive_oracle(acc, vec![a, b, alias, c, d, e]);
}

/// Return the union of some oracle parties.
fn oracle_union_all(parties: impl IntoIterator<Item = oracle::Party>) -> oracle::Party {
    parties
        .into_iter()
        .fold(oracle::Party::Leaf(false), oracle::Party::union)
}

/// Compare `join_all` with sequential oracle joins.
///
/// Both must agree whether every region can be absorbed. On success their
/// accumulators match; on error the production accumulator and returned groups
/// must together equal the union of every input region.
fn assert_join_all_matches_recursive_oracle(mut acc: Party, inputs: Vec<Party>) {
    let initial = to_oracle_party(&acc);
    let oracle_inputs: Vec<oracle::Party> = inputs.iter().map(to_oracle_party).collect();
    let expected =
        oracle_union_all(std::iter::once(initial.clone()).chain(oracle_inputs.iter().cloned()));
    let mut oracle_acc = initial;
    let reference = oracle_acc.join_all(oracle_inputs);
    let result = acc.join_all(inputs);

    assert_eq!(result.is_ok(), reference.is_ok(), "the verdicts differ");
    match result {
        Ok(()) => assert_eq!(to_oracle_party(&acc), oracle_acc),
        Err(back) => {
            let actual = oracle_union_all(
                std::iter::once(to_oracle_party(&acc)).chain(back.iter().map(to_oracle_party)),
            );
            assert_eq!(actual, expected, "join_all lost or invented a region");
        }
    }
}

proptest! {
    /// `join_all` matches the sequential oracle's verdict and successful
    /// result, and conserves every region when overlap prevents a full join.
    #[test]
    fn join_all_matches_the_recursive_oracle(
        oacc in arb_oracle_party_nonempty(),
        (pool, picks) in proptest::collection::vec(arb_oracle_party_nonempty(), 1..5)
            .prop_flat_map(|pool| {
                let len = pool.len();
                (Just(pool), proptest::collection::vec(0..len, 0..12))
            }),
    ) {
        let acc = from_oracle_party(&oacc);
        let inputs: Vec<Party> =
            picks.iter().map(|&i| from_oracle_party(&pool[i])).collect();
        assert_join_all_matches_recursive_oracle(acc, inputs);
    }
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

// Arbitrary normal-form parties exercise overlapping shapes that valid live
// populations cannot contain but rejection paths must handle.

proptest! {
    /// Splitting an arbitrary nonempty party matches the recursive oracle.
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
    /// [`Party::join`] on arbitrary pairs agrees with the oracle.
    ///
    /// A disjoint pair returns `Ok(())` and leaves `self` holding the
    /// oracle's join; an overlapping pair returns `Err(other)` with the
    /// refused party handed back unchanged and `self` unmodified.
    ///
    /// Arbitrary pairs exercise both successful disjoint joins and overlap.
    #[test]
    fn join_arbitrary(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let mut a = from_oracle_party(&oa);
        let b = from_oracle_party(&ob);

        if oa.is_disjoint(&ob) {
            let mut oracle_sum = oa.clone();
            oracle_sum.join(ob.clone()).expect("disjoint, just checked");
            prop_assert!(a.join(b).is_ok(), "disjoint parties must join");
            prop_assert!(a == from_oracle_party(&oracle_sum));
        } else {
            match a.join(b) {
                Ok(()) => prop_assert!(false, "overlapping parties must not join"),
                Err(returned) => {
                    prop_assert!(
                        returned == from_oracle_party(&ob),
                        "the refused party comes back unchanged"
                    );
                    prop_assert!(
                        a == from_oracle_party(&oa),
                        "`self` is unmodified on overlap"
                    );
                }
            }
        }
    }
}

proptest! {
    /// The fused `sum_split` equals its composition — `sum`, then `split` of
    /// the union — arm for arm on arbitrary id pairs.
    ///
    /// Byte-identical halves where the pair is disjoint, `None` exactly where
    /// `sum` refuses (overlap), the empty-operand identities included. This is
    /// the complete reference for the fused operation. Arbitrary pairs include
    /// overlap and a union whose two full children collapse.
    #[test]
    fn sum_split_is_sum_then_split(
        oa in arb_oracle_party(),
        ob in arb_oracle_party(),
    ) {
        let (ia, ib) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let fused = IdReader::root(ia.as_bits()).sum_split(IdReader::root(ib.as_bits()));
        let composed = IdReader::root(ia.as_bits())
            .sum(IdReader::root(ib.as_bits()))
            .map(|union| IdReader::root(crate::codec::built_view(&union)).split());
        prop_assert_eq!(fused, composed);
    }
}

/// `sum_split` handles a union whose full children collapse to the seed.
///
/// Summing the two halves of the seed makes both union children full, so the
/// built union collapses to the seed's terminal and `split` lands in its
/// terminal arm — the fused walk, which never builds the union, must emit those
/// exact bytes from its branch arm.
#[test]
fn sum_split_collapsed_union_matches_terminal_split() {
    let mut keep = Party::seed();
    let give = keep.fork();
    let fused = IdReader::root(keep.as_bits())
        .sum_split(IdReader::root(give.as_bits()))
        .expect("the seed's halves are disjoint");
    let union = IdReader::root(keep.as_bits())
        .sum(IdReader::root(give.as_bits()))
        .expect("the seed's halves are disjoint");
    let composed = IdReader::root(crate::codec::built_view(&union)).split();
    assert_eq!(fused, composed);
    assert_eq!(Party::from_bits(fused.0), keep, "the keep half is (1, 0)");
    assert_eq!(Party::from_bits(fused.1), give, "the give half is (0, 1)");
}

// ──────────────────────── constructed id encodings ────────────────────────

/// Hand-built canonical id encodings for deep tests.
///
/// Each encoding is emitted in one pass, so its depth does not increase the
/// number of allocations.
mod constructed {
    use crate::codec::BitsBuf;

    /// The full `1` leaf: terminal tag `00`.
    pub(super) fn full() -> BitsBuf {
        let mut b = BitsBuf::new();
        b.push(false);
        b.push(false);
        b
    }

    /// An internal node over the present children (normal form is the caller's
    /// obligation: at least one child, never two terminals).
    pub(super) fn node(left: Option<&BitsBuf>, right: Option<&BitsBuf>) -> BitsBuf {
        let mut b = BitsBuf::new();
        b.push(left.is_some());
        b.push(right.is_some());
        if let Some(l) = left {
            b.extend_from_buf(l);
        }
        if let Some(r) = right {
            b.extend_from_buf(r);
        }
        b
    }

    /// `levels` unary nodes toward `left_side` over `tail` (built tags-first,
    /// so a deep spine costs one pass, not one per level).
    pub(super) fn spine(levels: usize, left_side: bool, tail: BitsBuf) -> BitsBuf {
        let mut b = BitsBuf::with_capacity(2 * levels as u64 + tail.len());
        for _ in 0..levels {
            b.push(left_side);
            b.push(!left_side);
        }
        b.extend_from_buf(&tail);
        b
    }

    /// The leftmost `2^-k` cell: a `k`-level left-unary spine over `1`.
    pub(super) fn leftmost(k: usize) -> BitsBuf {
        spine(k, true, full())
    }

    /// The complement of [`leftmost`]`(k)`: the right half owned at every
    /// level.
    ///
    /// Built by one preorder pass — `k − 1` both-present nodes whose left child
    /// continues and whose right child is full, then the deepest right-only
    /// cell.
    pub(super) fn complement_leftmost(k: usize) -> BitsBuf {
        let mut b = BitsBuf::with_capacity(4 * k as u64);
        for _ in 1..k {
            b.push(true);
            b.push(true);
        }
        b.push(false);
        b.push(true);
        b.extend_from_buf(&full());
        for _ in 1..k {
            b.extend_from_buf(&full());
        }
        b
    }
}

/// Deep constructed parties keep `sum_split` equal to `sum` followed by `split`.
///
/// The cases cover deep collapse, whole-branch reuse, overlap detected at depth,
/// and empty or full operands. Kilolevel inputs also check iterative traversal.
mod sum_split_constructed {
    use super::constructed::{complement_leftmost, full, leftmost, node, spine};
    use super::*;
    use crate::codec::BitsBuf;

    /// The fused walk against its composition on one id pair, in both operand
    /// orders (byte equality, `None` arms included).
    fn assert_matches_composition(a: &BitsBuf, b: &BitsBuf) {
        for (x, y) in [(a, b), (b, a)] {
            let fused = IdReader::root(crate::codec::built_view(x))
                .sum_split(IdReader::root(crate::codec::built_view(y)));
            let composed = IdReader::root(crate::codec::built_view(x))
                .sum(IdReader::root(crate::codec::built_view(y)))
                .map(|union| IdReader::root(crate::codec::built_view(&union)).split());
            assert_eq!(fused, composed);
        }
    }

    /// Levels enough that no recursive generator plausibly reaches them and a
    /// per-level stack frame would overflow.
    const DEEP: usize = 10_000;

    /// Adjacent sibling cells at depth `DEEP`: the lockstep spine runs the
    /// whole way down and the union collapses at the deepest branch (both
    /// children full), followed by the terminal split far from the root.
    #[test]
    fn deep_adjacent_cells_collapse_at_the_branch() {
        let a = leftmost(DEEP);
        let b = spine(DEEP - 1, true, node(None, Some(&full())));
        assert_matches_composition(&a, &b);
    }

    /// A cell and its exact complement under a shared spine: the walk delegates
    /// the whole branch pair, and the delegated `sum` cascade-collapses every
    /// level to the terminal.
    #[test]
    fn deep_delegated_merge_cascade_collapses() {
        let a = spine(DEEP, false, leftmost(DEEP));
        let b = spine(DEEP, false, complement_leftmost(DEEP));
        assert_matches_composition(&a, &b);
    }

    /// One side owns the left half whole; the other owns a deep cell of the
    /// right half: both branch children splice verbatim, the deep subtree
    /// unread.
    #[test]
    fn deep_subtree_splices_verbatim() {
        let a = node(Some(&full()), None);
        let b = node(None, Some(&leftmost(DEEP)));
        assert_matches_composition(&a, &b);
    }

    /// A both-present operand against a right-only one at a deep branch: the
    /// kept child splices verbatim past the operand's paid skip, and the merged
    /// child collapses inside the delegated `sum`.
    #[test]
    fn deep_targeted_branch_with_collapsing_merged_child() {
        let quarter_left = node(Some(&full()), None);
        let quarter_right = node(None, Some(&full()));
        let a = spine(DEEP, true, node(Some(&quarter_left), Some(&quarter_left)));
        let b = spine(DEEP, true, node(None, Some(&quarter_right)));
        assert_matches_composition(&a, &b);
    }

    /// Overlap at depth is `None` exactly where the composition refuses: an
    /// identical deep pair (full meets nonempty on the spine's terminal), and
    /// an overlap buried inside a delegated merge.
    #[test]
    fn deep_overlap_is_refused() {
        let a = leftmost(DEEP);
        assert_matches_composition(&a, &a.clone());
        let inner = node(Some(&leftmost(2)), Some(&full()));
        let x = spine(DEEP, false, leftmost(2));
        let y = spine(DEEP, false, inner);
        assert_matches_composition(&x, &y);
    }

    /// The fused walk's scan never exceeds its composition's, and a
    /// splice-resolved pair reads `O(1)` bits however deep the spliced subtree.
    ///
    /// The method doc's cost claim, held by meter on the three constructed
    /// regimes at two scales each: whole-branch delegation (the composition's
    /// bytes minus the built union's spine), the pure splice (constant root
    /// reads, the sublinear case used by the `clock_sync` board floors), and
    /// the lockstep spine to a targeted branch. A fused
    /// walk that re-reads a skipped child or scans a spliced subtree moves the
    /// ratio above one.
    #[cfg(feature = "scan-meter")]
    #[test]
    fn sum_split_scan_never_exceeds_the_composition() {
        let scan = |f: &dyn Fn()| {
            crate::codec::scan::reset();
            f();
            crate::codec::scan::scan_bits()
        };
        let compare = |name: &str, a: &BitsBuf, b: &BitsBuf| -> u64 {
            let fused = scan(&|| {
                IdReader::root(crate::codec::built_view(a))
                    .sum_split(IdReader::root(crate::codec::built_view(b)));
            });
            let composed = scan(&|| {
                IdReader::root(crate::codec::built_view(a))
                    .sum(IdReader::root(crate::codec::built_view(b)))
                    .map(|u| IdReader::root(crate::codec::built_view(&u)).split());
            });
            assert!(
                0 < fused && fused <= composed,
                "{name}: fused walk scanned {fused} bits against the \
                 composition's {composed}",
            );
            fused
        };
        for k in [256usize, 4096] {
            let quarter_left = node(Some(&full()), None);
            let quarter_right = node(None, Some(&full()));
            let a = spine(k, true, node(Some(&leftmost(k)), Some(&quarter_left)));
            let b = spine(
                k,
                true,
                node(Some(&complement_leftmost(k)), Some(&quarter_right)),
            );
            compare(&format!("delegated k={k}"), &a, &b);
            let a = node(Some(&full()), None);
            let b = node(None, Some(&leftmost(k)));
            let spliced = compare(&format!("splice k={k}"), &a, &b);
            assert_eq!(
                spliced, 8,
                "a splice-resolved pair reads exactly its two root tags \
                 per operand (peeked, then read), at any depth",
            );
            let a = leftmost(k);
            let b = spine(k - 1, true, node(None, Some(&full())));
            compare(&format!("adjacent k={k}"), &a, &b);
        }
    }

    /// The root-owning and empty operands ride the same equalities: the full
    /// leaf overlaps every nonempty id, an empty side hands the split of the
    /// other, and two empties split to empties.
    #[test]
    fn root_leaf_and_empty_operands_match_composition() {
        let empty = BitsBuf::new();
        assert_matches_composition(&full(), &leftmost(3));
        assert_matches_composition(&full(), &empty);
        assert_matches_composition(&empty, &empty.clone());
        assert_matches_composition(&empty, &leftmost(DEEP));
    }
}

/// Deep constructed id pairs drive `diff`'s covered-block arms — the verbatim
/// splice and the owned-cover block scan — plus the complement walk, at depths
/// no committed generator reaches.
///
/// The deep `without` drivers elsewhere all route `self = seed`, which settles
/// at the root, so of `diff`'s four settle regimes only the complement descent
/// and the lockstep descent see real depth without these. Each family here is
/// size-generic over a scale ladder whose top no recursive walk survives: the
/// deep instances are asserted byte-for-byte against the constructed
/// expectation (doubling as stack-safety proof for the block scan and the
/// sweep), and the oracle-reachable scales are additionally held to the
/// recursive `oracle::Party::without`.
mod diff_constructed {
    use super::constructed::{complement_leftmost, full, leftmost, node};
    use super::*;
    use crate::codec::BitsBuf;

    /// The scale ladder: every family runs at each `k`, byte-checked.
    const SCALES: [usize; 3] = [256, 4096, 100_000];

    /// Scales the plain-recursive oracle (and the id-side bridge) can walk on
    /// the test stack; the ladder's top is deliberately beyond it.
    const ORACLE_SCALE_MAX: usize = 4096;

    /// `self \ other` on one constructed pair: byte-equal to `expected`, and
    /// at oracle-reachable scales (`k <= ORACLE_SCALE_MAX`) also equal to the
    /// recursive oracle's `without`, compared over lowered oracle trees.
    fn assert_diff(a: &BitsBuf, b: &BitsBuf, expected: &BitsBuf, k: usize) {
        let d = IdReader::root(crate::codec::built_view(a))
            .diff(IdReader::root(crate::codec::built_view(b)));
        assert_eq!(
            &d, expected,
            "diff diverged from the constructed expectation (k={k})"
        );
        if k <= ORACLE_SCALE_MAX {
            let oa = to_oracle_party(&Party::from_bits(a.clone()));
            let ob = to_oracle_party(&Party::from_bits(b.clone()));
            let oracle_diff = oa.without(&ob);
            if d.is_empty() {
                assert!(
                    oracle_diff.is_empty(),
                    "the oracle kept a remainder the sweep dropped (k={k})"
                );
            } else {
                assert_eq!(
                    to_oracle_party(&Party::from_bits(d)),
                    oracle_diff,
                    "diff diverged from the recursive oracle (k={k})"
                );
            }
        }
    }

    /// A deep `self` subtree under an unowned `other` cover survives whole:
    /// the remainder is `self` itself, byte for byte, at every scale.
    ///
    /// `self` is the leftmost `2^-k` cell and `other` owns only the right
    /// half, so the root descent settles the whole spine as one covered block
    /// — a single iterative scan and one verbatim splice, never a
    /// plateau-by-plateau walk.
    #[test]
    fn deep_subtree_under_unowned_cover_splices_verbatim() {
        for k in SCALES {
            let a = leftmost(k);
            let b = node(None, Some(&full()));
            assert_diff(&a, &b, &a, k);
        }
    }

    /// A deep `self` subtree under an owned `other` cover vanishes whole: the
    /// remainder is empty, at every scale.
    ///
    /// The same spine with the cover's polarity flipped — `other` owns the
    /// half the spine lives in — so the block scan consumes the subtree and
    /// nothing of it survives into the output.
    #[test]
    fn deep_subtree_under_owned_cover_vanishes() {
        for k in SCALES {
            let a = leftmost(k);
            let b = node(Some(&full()), None);
            assert_diff(&a, &b, &BitsBuf::new(), k);
        }
    }

    /// The complement dual: carving a deep cell out of the seed emits exactly
    /// the cell's complement, at every scale.
    ///
    /// A full `self` plateau over a deep `other` subtree is the one covered
    /// pairing that is *not* a block — the sweep walks the subtree plateau by
    /// plateau and the output is its complement, owned at every level down
    /// the spine.
    #[test]
    fn seed_without_deep_cell_is_its_complement() {
        for k in SCALES {
            assert_diff(&full(), &leftmost(k), &complement_leftmost(k), k);
        }
    }

    /// A covered block costs its own tags plus its verbatim output and no
    /// more, and never out-scans the complement walk over the same subtree.
    ///
    /// Each block regime's recorded scan sits between reading every operand
    /// tag once (the floor) and that plus writing the settled output once
    /// (the ceiling).
    ///
    /// The module doc's cost claim, held by meter at two scales. The scan
    /// currency counts builder writes as well as reads, and emitting plateau
    /// by plateau costs about triple the verbatim splice's bits per level
    /// (per-plateau tag reservations and patches against one block write), so
    /// a diff that re-walks or re-derives a block-settled subtree plateau by
    /// plateau lands far past the ceiling — a regression no other committed
    /// reading would notice. The complement walk — the owned-`self` dual
    /// driving the same spine plateau by plateau — rides as the relative
    /// yardstick the block regimes must stay under.
    #[cfg(feature = "scan-meter")]
    #[test]
    fn diff_block_scan_never_exceeds_the_complement_walk() {
        /// Constant scan overhead of a settled block beyond its operand reads
        /// and output write: the root-level tag reservations and patches.
        const BLOCK_SLACK: u64 = 8;
        let scan = |a: &BitsBuf, b: &BitsBuf| -> u64 {
            crate::codec::scan::reset();
            IdReader::root(crate::codec::built_view(a))
                .diff(IdReader::root(crate::codec::built_view(b)));
            crate::codec::scan::scan_bits()
        };
        for k in [256usize, 4096] {
            let spine = leftmost(k);
            let unowned_cover = node(None, Some(&full()));
            let owned_cover = node(Some(&full()), None);
            let walk = scan(&owned_cover, &spine);
            for (name, cover, output_len) in [
                ("splice", &unowned_cover, spine.len()),
                ("owned-cover block", &owned_cover, 0),
            ] {
                let blocked = scan(&spine, cover);
                let floor = spine.len() + cover.len();
                let ceiling = floor + output_len + BLOCK_SLACK;
                assert!(
                    floor <= blocked && blocked <= ceiling,
                    "{name} k={k}: scanned {blocked} bits outside \
                     [{floor}, {ceiling}] (every operand tag once, plus the \
                     settled output written once)",
                );
                assert!(
                    blocked <= walk,
                    "{name} k={k}: scanned {blocked} bits against the \
                     complement walk's {walk}",
                );
            }
        }
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
    /// `as_bytes` returns exactly the canonical `encode` bytes
    /// (marker-padded tail), over arbitrary non-empty ids — the
    /// `id_node`/`extend` build path.
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
    /// Canonical padding makes raw byte equality equivalent to live-bit
    /// equality. Equal parties must also hash equally.
    #[test]
    fn byte_equality_matches_bit_equality(
        oa in arb_oracle_party_nonempty(),
        ob in arb_oracle_party_nonempty(),
    ) {
        let a = from_oracle_party(&oa);
        let b = from_oracle_party(&ob);
        let bit_eq = a.as_bits().to_buf() == b.as_bits().to_buf();
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
// These checks pin the complete size trajectory of repeated forks, catching
// output growth that would compound across otherwise cheap calls.

/// An iterated fork chain's id sizes are exactly affine.
///
/// Following the forked-off child each round (the mover lineage descends one
/// level per split), both halves read exactly `2 + 2·k` encoded bits after the
/// k-th fork, for every `k`: one two-bit tree level per fork.
#[test]
fn fork_chain_orbit_sizes_are_exactly_affine() {
    let mut p = Party::seed();
    assert_eq!(p.encoded_bits(), 2, "the seed is the 2-bit whole region");
    for k in 1u64..=512 {
        let q = p.fork();
        assert_eq!(p.encoded_bits(), 2 + 2 * k, "keeper id bits after fork {k}");
        assert_eq!(q.encoded_bits(), 2 + 2 * k, "mover id bits after fork {k}");
        p = q;
    }
}

/// An iterated fork fan grows exactly affine and unwinds exactly.
///
/// Each round forks a fresh child off the root lineage (the keeper deepens one
/// level per split), both halves reading exactly `2 + 2·k` encoded bits at the
/// k-th fork; rejoining the children in reverse order then walks the root back
/// down the same trajectory, ending byte-identical to the seed.
#[test]
fn fork_fan_orbit_grows_affine_and_unwinds_to_seed() {
    let mut root = Party::seed();
    let mut children = Vec::new();
    for k in 1u64..=512 {
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
            2 + 2 * (511 - i as u64),
            "root id bits after unwind join {i}"
        );
    }
    assert!(root.is_seed(), "the fully unwound fan is the seed again");
}
