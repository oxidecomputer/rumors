//! Independent checks of `Party` against the recursive oracle.

use proptest::prelude::*;

use super::Party;
use crate::party::io::PartyReader;
use crate::testing::bridge::{from_oracle_party, to_oracle_party};
use crate::testing::generators::{arb_oracle_party_nonempty, arb_party_family};
use crate::testing::optrace::{run, world_strategy};
use crate::testing::oracles::{function, tree};

// ───────────────────────────── the join fold ─────────────────────────────

/// `join_all` and the sequential oracle reunite the same disjoint forks.
#[test]
fn join_all_agrees_with_oracle_when_none_overlap() {
    let mut acc = Party::seed();
    let shares: Vec<Party> = acc.forks(5u64).collect();
    assert_join_all_matches_all_models(acc, shares);
}

/// An overlap among inputs is reported without losing or duplicating any
/// region.
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
    assert_join_all_matches_all_models(acc, vec![a, b, alias, c, d, e]);
}

/// An overlap between the receiver and disjoint inputs is reported without
/// losing or duplicating any region.
///
/// The inputs fold into one group without error, so the overlap appears only
/// when that group meets the receiver: this population reaches the error path
/// after the fold, where `join_all_preserves_regions_on_overlap` stops inside
/// it.
#[test]
fn join_all_preserves_regions_when_the_receiver_overlaps() {
    let mut acc = Party::seed();
    let share = acc.fork();
    let alias = acc.dangerously_alias();
    assert_join_all_matches_all_models(acc, vec![share, alias]);
}

/// Return the union of some oracle parties.
fn oracle_union_all(parties: impl IntoIterator<Item = tree::Party>) -> tree::Party {
    parties
        .into_iter()
        .fold(tree::Party::Leaf(false), tree::Party::union)
}

/// Compare `join_all` with both independent semantic models.
///
/// All three implementations must agree on success. On failure, the balanced
/// production fold may group inputs differently from the sequential models;
/// the contract requires each result to return every region it did not retain,
/// so each model must conserve the complete union. Production must also
/// conserve multiplicity: every point is owned as many times across the final
/// receiver and returned parties as across the initial receiver and inputs, so
/// no region is lost or comes back twice.
fn assert_join_all_matches_all_models(mut acc: Party, inputs: Vec<Party>) {
    let initial = to_oracle_party(&acc);
    let oracle_inputs: Vec<tree::Party> = inputs.iter().map(to_oracle_party).collect();
    let received: Vec<tree::Party> = std::iter::once(initial.clone())
        .chain(oracle_inputs.iter().cloned())
        .collect();
    let expected = oracle_union_all(received.iter().cloned());
    let mut recursive_acc = initial;
    let function_acc = function::lift_id(recursive_acc.clone());
    let function_inputs = oracle_inputs
        .iter()
        .cloned()
        .map(function::lift_id)
        .collect::<Vec<_>>();
    let recursive_result = recursive_acc.join_all(oracle_inputs);
    let (function_acc, function_rejected) =
        function::join_all_parties(function_acc, function_inputs);
    let production_result = acc.join_all(inputs);

    assert_eq!(
        production_result.is_ok(),
        recursive_result.is_ok(),
        "recursive-oracle verdict differs"
    );
    assert_eq!(
        production_result.is_ok(),
        function_rejected.is_empty(),
        "function-space verdict differs"
    );
    match (production_result, recursive_result) {
        (Ok(()), Ok(())) => {
            assert_eq!(
                to_oracle_party(&acc),
                recursive_acc,
                "final accumulators differ"
            );
            let grid = function::fs_grid(&[
                function::id_depth(&recursive_acc),
                function_acc.res_ceiling(),
            ]);
            assert_eq!(
                function::id_order(
                    &function_acc,
                    &function::lift_id(recursive_acc.clone()),
                    grid,
                ),
                Some(std::cmp::Ordering::Equal),
                "function-space accumulator differs"
            );
        }
        (Err(rejected), Err(recursive_rejected)) => {
            let held: Vec<tree::Party> = std::iter::once(to_oracle_party(&acc))
                .chain(rejected.iter().map(to_oracle_party))
                .collect();
            let actual = oracle_union_all(held.iter().cloned());
            assert_eq!(actual, expected, "join_all lost or invented a region");
            assert!(
                tree::Party::same_multiplicity(
                    &received.iter().collect::<Vec<_>>(),
                    &held.iter().collect::<Vec<_>>(),
                ),
                "join_all returned a region more or fewer times than it received it",
            );
            let recursive_actual =
                oracle_union_all(std::iter::once(recursive_acc).chain(recursive_rejected));
            assert_eq!(recursive_actual, expected, "recursive model lost a region");
            let function_actual = function_rejected
                .into_iter()
                .fold(function_acc, function::sum);
            let grid =
                function::fs_grid(&[function::id_depth(&expected), function_actual.res_ceiling()]);
            assert_eq!(
                function::id_order(&function_actual, &function::lift_id(expected.clone()), grid,),
                Some(std::cmp::Ordering::Equal),
                "function-space model lost a region"
            );
        }
        _ => unreachable!("verdict equality was checked above"),
    }
}

proptest! {
    /// `join_all` matches both semantic models on success. After overlap,
    /// every model conserves the complete region, and production conserves
    /// how many times each point is owned.
    #[test]
    fn party_join_all_matches_all_models(
        (oacc, oracle_inputs) in arb_party_family(),
    ) {
        let acc = from_oracle_party(&oacc);
        let inputs = oracle_inputs.iter().map(from_oracle_party).collect();
        assert_join_all_matches_all_models(acc, inputs);
    }
}

/// Forking a one-byte left share from a deep right share retains only that
/// one-byte result allocation.
#[test]
fn asymmetric_fork_sizes_the_small_result_independently() {
    let left = constructed::full();
    let right = constructed::spine(8_192, false, constructed::full());
    let mut parent = Party::from_test_bits(constructed::node(Some(&left), Some(&right)));

    let _large_right_share = parent.fork();
    assert_eq!(parent.as_bytes().len(), 1);
    assert_eq!(parent.0.allocation_capacity(), 1);
}

// ───────────────────────────── differential vs oracle ─────────────────────────────

proptest! {
    /// `fork` yields two disjoint halves, both matching the oracle's fork;
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

        // Both halves match the oracle's fork.
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
    /// Forking an arbitrary nonempty party matches the recursive oracle.
    #[test]
    fn fork_arbitrary(op in arb_oracle_party_nonempty()) {
        let mut oracle_self = op.clone();
        let oracle_give = oracle_self.fork();

        let mut keep = from_oracle_party(&op);
        let give = keep.fork();

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
        oa in arb_oracle_party_nonempty(),
        ob in arb_oracle_party_nonempty(),
    ) {
        let mut a = from_oracle_party(&oa);
        let b = from_oracle_party(&ob);

        if oa.is_disjoint(&ob) {
            let mut oracle_joined = oa.clone();
            oracle_joined.join(ob.clone()).expect("disjoint, just checked");
            prop_assert!(a.join(b).is_ok(), "disjoint parties must join");
            prop_assert!(a == from_oracle_party(&oracle_joined));
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
    /// Party synchronization exactly matches joining and then forking.
    ///
    /// This checks both result bytes for disjoint inputs and the rejection of
    /// overlaps, including unions that collapse to a terminal.
    #[test]
    fn party_sync_matches_join_then_fork_for_arbitrary_trees(
        oa in arb_oracle_party_nonempty(),
        ob in arb_oracle_party_nonempty(),
    ) {
        let (a, b) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let fused = a.sync(&b);

        let (mut joined, other) = (from_oracle_party(&oa), from_oracle_party(&ob));
        let composed = joined.join(other).ok().map(|()| {
            let give = joined.fork();
            (joined, give)
        });
        prop_assert_eq!(fused, composed);
    }
}

/// Synchronization handles the smallest join that collapses to a terminal.
///
/// The two seed halves form two full children, whose normal form is the seed's
/// terminal. The fused operation must therefore match forking that terminal.
#[test]
fn sync_handles_a_join_that_collapses_to_a_terminal() {
    let mut keep = Party::seed();
    let give = keep.fork();
    let fused = keep.sync(&give).expect("the seed's halves are disjoint");
    assert_eq!(fused.0, keep);
    assert_eq!(fused.1, give);
}

// ──────────────────────── constructed party encodings ────────────────────────

/// Hand-built canonical party encodings for deep tests.
///
/// Each encoding is emitted in one pass, so its depth does not increase the
/// number of allocations.
mod constructed {
    use crate::bits::BitsWriter;

    /// The full `1` leaf: terminal tag `00`.
    pub fn full() -> BitsWriter {
        let mut b = BitsWriter::new();
        b.push(false);
        b.push(false);
        b
    }

    /// A branch over the present children (normal form is the caller's
    /// obligation: at least one child, never two terminals).
    pub fn node(left: Option<&BitsWriter>, right: Option<&BitsWriter>) -> BitsWriter {
        let mut b = BitsWriter::new();
        b.push(left.is_some());
        b.push(right.is_some());
        if let Some(l) = left {
            b.extend_from_writer(l);
        }
        if let Some(r) = right {
            b.extend_from_writer(r);
        }
        b
    }

    /// `levels` unary nodes toward `left_side` over `tail` (built tags-first,
    /// so a deep spine costs one pass, not one per level).
    pub fn spine(levels: usize, left_side: bool, tail: BitsWriter) -> BitsWriter {
        let mut b = BitsWriter::with_capacity(2 * levels as u64 + tail.len());
        for _ in 0..levels {
            b.push(left_side);
            b.push(!left_side);
        }
        b.extend_from_writer(&tail);
        b
    }

    /// The leftmost `2^-k` cell: a `k`-level left-unary spine over `1`.
    pub fn leftmost(k: usize) -> BitsWriter {
        spine(k, true, full())
    }

    /// The complement of [`leftmost`]`(k)`: the right half owned at every
    /// level.
    ///
    /// Built by one preorder pass — `k − 1` both-present nodes whose left child
    /// continues and whose right child is full, then the deepest right-only
    /// cell.
    pub fn complement_leftmost(k: usize) -> BitsWriter {
        let mut b = BitsWriter::with_capacity(4 * k as u64);
        for _ in 1..k {
            b.push(true);
            b.push(true);
        }
        b.push(false);
        b.push(true);
        b.extend_from_writer(&full());
        for _ in 1..k {
            b.extend_from_writer(&full());
        }
        b
    }
}

/// Deep constructed parties keep synchronization equal to join then fork.
///
/// The cases cover deep collapse, whole-branch reuse, overlap detected at depth,
/// and empty or full operands. Kilolevel inputs also check iterative traversal.
mod sync_constructed {
    use super::constructed::{complement_leftmost, full, leftmost, node, spine};
    use super::*;
    use crate::bits::BitsWriter;

    /// Adopt a hand-built canonical party tree.
    fn party(bits: &BitsWriter) -> Party {
        Party::from_test_bits(bits.clone())
    }

    /// The fused walk against its composition on one party pair, in both operand
    /// orders (byte equality, `None` arms included).
    fn assert_matches_composition(a: &BitsWriter, b: &BitsWriter) {
        for (x, y) in [(a, b), (b, a)] {
            let fused = party(x).sync(&party(y));
            let mut joined = party(x);
            let composed = joined.join(party(y)).ok().map(|()| {
                let give = joined.fork();
                (joined, give)
            });
            assert_eq!(fused, composed);
        }
    }

    /// Levels enough that no recursive generator plausibly reaches them and a
    /// per-level stack frame would overflow.
    const DEEP: usize = 10_000;

    /// Adjacent sibling cells at depth `DEEP`: the lockstep spine runs the
    /// whole way down and the union collapses at the deepest branch (both
    /// children full), followed by the terminal fork far from the root.
    #[test]
    fn deep_adjacent_cells_collapse_at_the_branch() {
        let a = leftmost(DEEP);
        let b = spine(DEEP - 1, true, node(None, Some(&full())));
        assert_matches_composition(&a, &b);
    }

    /// A cell and its exact complement under a shared spine: joining the shared
    /// suffix collapses every level to one terminal before the final fork.
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

    /// A two-child operand against a right-only one at a deep branch: the left
    /// child is copied unchanged and joining the right child collapses it.
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

    /// Synchronization does no more encoded-bit work than separate join and
    /// fork operations.
    ///
    /// Deep cases cover merging a shared child, copying an exclusively owned
    /// child, and following a long one-child chain. Copying exclusive children
    /// must read only the two root tags and write each output bit once.
    #[cfg(feature = "scan-meter")]
    #[test]
    fn sync_scan_work_never_exceeds_join_then_fork() {
        let compare = |name: &str, a: &BitsWriter, b: &BitsWriter| -> u64 {
            crate::testing::instrument::scan::reset();
            party(a).sync(&party(b));
            let fused = crate::testing::instrument::scan::scan_bits();

            crate::testing::instrument::scan::reset();
            let mut joined = party(a);
            if joined.join(party(b)).is_ok() {
                let _ = joined.fork();
            }
            let composed = crate::testing::instrument::scan::scan_bits();
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
                spliced,
                4 + a.len() + b.len(),
                "a splice-resolved pair reads each two-bit root tag once and \
                 writes both results once",
            );
            let a = leftmost(k);
            let b = spine(k - 1, true, node(None, Some(&full())));
            compare(&format!("adjacent k={k}"), &a, &b);
        }
    }

    /// A root-owning party overlaps every nonempty party, including a deeply
    /// nested cell.
    #[test]
    fn root_owner_overlaps_nested_parties() {
        assert_matches_composition(&full(), &leftmost(3));
        assert_matches_composition(&full(), &leftmost(DEEP));
    }
}

/// Constructed pairs exercise all ways `without` can handle a deep subtree:
/// copy it unchanged, discard it whole, or descend to build its complement.
///
/// Every scale is checked against exact expected bytes. Scales the recursive
/// oracle can handle are also checked semantically against that oracle.
mod without_constructed {
    use super::constructed::{complement_leftmost, full, leftmost, node};
    use super::*;
    use crate::bits::BitsWriter;
    use crate::testing::generators::STACK_SAFETY_DEPTH;

    /// Depths used by every constructed family; the deepest overflows any walk
    /// that keeps one call frame per level.
    const SCALES: [usize; 3] = [256, 4096, STACK_SAFETY_DEPTH];

    /// Greatest depth safe for the recursive oracle used as a second check.
    const ORACLE_SCALE_MAX: usize = 4096;

    /// Check exact bytes, plus the recursive oracle where its stack permits.
    fn assert_without(a: &BitsWriter, b: &BitsWriter, expected: &BitsWriter, k: usize) {
        let (a_bits, b_bits) = (a.clone().finalize(), b.clone().finalize());
        let d = PartyReader::from_bits(&a_bits).without(PartyReader::from_bits(&b_bits));
        let expected = (!expected.is_empty()).then(|| Party::from_test_bits(expected.clone()));
        assert_eq!(
            d.as_ref(),
            expected.as_ref(),
            "without diverged from the constructed expectation (k={k})"
        );
        if k <= ORACLE_SCALE_MAX {
            let oa = to_oracle_party(&Party::from_test_bits(a.clone()));
            let ob = to_oracle_party(&Party::from_test_bits(b.clone()));
            let oracle_diff = oa.without(&ob);
            if let Some(d) = &d {
                assert_eq!(
                    to_oracle_party(d),
                    oracle_diff,
                    "without diverged from the recursive oracle (k={k})"
                );
            } else {
                assert!(
                    oracle_diff.is_empty(),
                    "the oracle kept a remainder the sweep dropped (k={k})"
                );
            }
        }
    }

    /// A deep `self` subtree under an unowned `other` cover survives whole:
    /// the remainder is `self` itself, byte for byte, at every scale.
    ///
    /// `self` is the leftmost `2^-k` cell and `other` owns only the right
    /// half. The whole spine therefore survives and can be copied without
    /// rebuilding its regions.
    #[test]
    fn deep_subtree_under_unowned_cover_splices_verbatim() {
        for k in SCALES {
            let a = leftmost(k);
            let b = node(None, Some(&full()));
            assert_without(&a, &b, &a, k);
        }
    }

    /// A deep `self` subtree under an owned `other` cover vanishes whole: the
    /// remainder is empty, at every scale.
    ///
    /// Here `other` owns the half containing the spine, so that subtree can be
    /// discarded without visiting its individual regions.
    #[test]
    fn deep_subtree_under_owned_cover_vanishes() {
        for k in SCALES {
            let a = leftmost(k);
            let b = node(Some(&full()), None);
            assert_without(&a, &b, &BitsWriter::new(), k);
        }
    }

    /// The complement dual: carving a deep cell out of the seed emits exactly
    /// the cell's complement, at every scale.
    ///
    /// Because `self` owns the whole interval, the result changes at every
    /// boundary in `other`. The walk must visit those boundaries and build the
    /// exact complement.
    #[test]
    fn seed_without_deep_cell_is_its_complement() {
        for k in SCALES {
            assert_without(&full(), &leftmost(k), &complement_leftmost(k), k);
        }
    }

    /// Copying or discarding a covered subtree scans no more than its input and
    /// copied output, and no more than constructing its complement.
    ///
    /// The lower bound reads both operands once. The upper bound adds one write
    /// of the surviving subtree and a small fixed amount for the root. A walk
    /// that rebuilt the subtree region by region would exceed this envelope.
    #[cfg(feature = "scan-meter")]
    #[test]
    fn without_covered_subtree_scan_never_exceeds_the_complement_walk() {
        /// Constant scan overhead of a settled block beyond its operand reads
        /// and output write: the root-level tag reservations and patches.
        const BLOCK_SLACK: u64 = 8;
        let scan = |a: &BitsWriter, b: &BitsWriter| -> u64 {
            crate::testing::instrument::scan::reset();
            let (a_bits, b_bits) = (a.clone().finalize(), b.clone().finalize());
            PartyReader::from_bits(&a_bits).without(PartyReader::from_bits(&b_bits));
            crate::testing::instrument::scan::scan_bits()
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
    /// `decode ∘ encode == identity` over arbitrary non-empty normal-form parties,
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
    /// (marker-padded tail), over arbitrary non-empty parties — the
    /// `id_node`/`extend` build path.
    #[test]
    fn as_bytes_matches_encode(op in arb_oracle_party_nonempty()) {
        let p = from_oracle_party(&op);
        let encoded = p.encode();
        prop_assert_eq!(p.as_bytes(), encoded.as_slice());
    }

    /// The invariant holds for both halves produced by `fork` (the fork path),
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
    /// Byte-level equality (``==``) agrees with a plain
    /// bit-level compare of the live party streams, in both operand orders.
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
        let bit_eq = crate::party::instrument::bits(&a) == crate::party::instrument::bits(&b);
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

/// An iterated fork chain's party sizes are exactly affine.
///
/// Following the forked-off child each round (the mover lineage descends one
/// level per fork), both halves read exactly `2 + 2·k` encoded bits after the
/// k-th fork, for every `k`: one two-bit tree level per fork.
#[test]
fn fork_chain_orbit_sizes_are_exactly_affine() {
    let mut p = Party::seed();
    assert_eq!(p.encoded_bits(), 2, "the seed is the 2-bit whole region");
    for k in 1u64..=512 {
        let q = p.fork();
        assert_eq!(
            p.encoded_bits(),
            2 + 2 * k,
            "keeper party bits after fork {k}"
        );
        assert_eq!(
            q.encoded_bits(),
            2 + 2 * k,
            "mover party bits after fork {k}"
        );
        p = q;
    }
}

/// An iterated fork fan grows exactly affine and unwinds exactly.
///
/// Each round forks a fresh child off the root lineage (the keeper deepens one
/// level per fork), both halves reading exactly `2 + 2·k` encoded bits at the
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
            "root party bits after fork {k}"
        );
        assert_eq!(
            q.encoded_bits(),
            2 + 2 * k,
            "child party bits after fork {k}"
        );
        children.push(q);
    }
    for (i, q) in children.into_iter().rev().enumerate() {
        root.join(q)
            .expect("fan children are disjoint from the root");
        assert_eq!(
            root.encoded_bits(),
            2 + 2 * (511 - i as u64),
            "root party bits after unwind join {i}"
        );
    }
    assert!(root.is_seed(), "the fully unwound fan is the seed again");
}
