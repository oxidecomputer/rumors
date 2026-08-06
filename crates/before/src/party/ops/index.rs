//! A per-call random-access index over one fixed id operand, for folds that
//! test many independent operands against it.
//!
//! The packed id coding has no random access: locating a node's right child
//! means scanning past its whole left subtree, so a cursor-walk disjointness
//! test costs `O(n + m)` in *both* operands. That is the right shape for one
//! call, and the wrong shape for a fold: testing each of `k` inputs against one
//! fixed operand by cursor re-walks the fixed side `k` times — `Θ(k × n)` on a
//! `Θ(n + Σ inputs)` operand set.
//!
//! [`IdIndex`] restores the missing primitive for exactly that pattern. Built
//! once per fold in `O(n)`, it tabulates every both-present node's right-child
//! position, so an indexed walk addresses any child in `O(1)` and skips an
//! unvisited subtree for free (it simply never touches it). Each per-input test
//! costs `O(input)` node visits plus one `O(log n)` table search per node both
//! sides own — so the fold's up-front tests total `O(Σ inputs + B log n)`, `B`
//! the inputs' both-present node count. The search term is not bounded by the
//! operands: on populations whose regions interleave deeply (every skeleton
//! node shared) it dominates the tests, which is a price this module pays
//! knowingly — the cursor alternative re-walks the fixed side per input,
//! quadratic on exactly the many-small-inputs populations the fold exists for
//! (the overlap rows and the flatness pin in `meter::board`'s tests price that
//! regression), while the searches tie or win wall time on every committed fold
//! population, by measurement. Every probe records one table word in the scan
//! currency ([`metered_partition_point`]), the board's weave family prices the
//! term under its declared search allowance, and the parity-halves liveness
//! floor in `party/tests.rs` trips if the searches ever go unmetered.
//!
//! The index answers the same predicate as the cursor walk
//! ([`IdReader::is_disjoint`]) with the identical verdict — the fold
//! differentials in `party/tests.rs` and `clock/tests.rs` pin the equivalence —
//! and it exists *only* because the fold repeats the test against one fixed
//! side; single-shot predicates stay on the cursor walk, which needs no table.

use crate::codec::BitsSlice;
use crate::idbits::{IdNode, IdReader};

/// A random-access view of one packed id operand: the operand's bits plus a
/// table locating every both-present node's right child.
///
/// Build once with [`build`](IdIndex::build), then run
/// [`is_disjoint`](IdIndex::is_disjoint) against any number of independent
/// operands. Transient state is at most one `u32` per both-present node of the
/// indexed operand — strictly smaller than the operand itself.
pub(crate) struct IdIndex<'a> {
    /// The indexed operand's packed preorder tag stream.
    bits: &'a BitsSlice,
    /// `rights[i]` is the right child's bit position for the `i`-th
    /// both-present node in preorder. Only both-present nodes need an entry: a
    /// right-only node's child follows its tag directly, and a left child
    /// always does.
    ///
    /// `None` when the stream's positions do not fit `u32` (≥ 512 MiB of packed
    /// id); [`is_disjoint`](IdIndex::is_disjoint) then falls back to the
    /// per-input cursor walk, which answers the same predicate at the fold's
    /// unindexed cost.
    rights: Option<Vec<u32>>,
}

impl<'a> IdIndex<'a> {
    /// Index `bits` (a normal-form packed id stream) in two linear passes:
    /// count the both-present nodes to size the table exactly, then fill each
    /// node's right-child position.
    ///
    /// An id stream is 2-bit presence tags in preorder and nothing else, so the
    /// counting pass is a flat pair scan. The filling pass resolves each
    /// both-present node's left-subtree end with a stack of open both-present
    /// frames — single-child nodes are transparent (their completion is their
    /// child's) — threading the frames' entry slots through the not-yet-filled
    /// table itself, so the only transient state beyond the table is one `bool`
    /// per open frame.
    pub(crate) fn build(bits: &'a BitsSlice) -> IdIndex<'a> {
        if bits.len() > u32::MAX as usize {
            return IdIndex { bits, rights: None };
        }
        // Pass 1: count. Reads every tag once.
        crate::codec::scan::record_bits(bits.len());
        let mut count = 0usize;
        let mut p = 0;
        while p < bits.len() {
            if bits[p] && bits[p + 1] {
                count += 1;
            }
            p += 2;
        }
        // Pass 2: fill. Reads every tag once more. While a frame awaits its
        // left subtree's end, its table slot holds the entry index of the
        // next-outer awaiting frame (`u32::MAX` terminates the chain); the real
        // right-child position overwrites the link the moment the left subtree
        // completes.
        crate::codec::scan::record_bits(bits.len());
        let mut rights = vec![0u32; count];
        // One bit per open both-present frame, innermost last: `true` while the
        // frame awaits its left subtree's end, `false` while it awaits its
        // right's.
        let mut awaiting_left: Vec<bool> = Vec::new();
        let mut chain_head = u32::MAX;
        let mut next_entry = 0usize;
        let mut p = 0;
        while p < bits.len() {
            let (left, right) = (bits[p], bits[p + 1]);
            p += 2;
            if left && right {
                rights[next_entry] = chain_head;
                chain_head = next_entry as u32;
                next_entry += 1;
                awaiting_left.push(true);
            } else if !left && !right {
                // A terminal closes one subtree; propagate the completion
                // through every frame it finishes. Single-child nodes between
                // frames queued nothing, so their completion is
                // exactly this one.
                loop {
                    match awaiting_left.last_mut() {
                        None => break,
                        Some(state @ true) => {
                            // The innermost frame's left subtree ends here, so
                            // its right child starts here.
                            let entry = chain_head as usize;
                            chain_head = rights[entry];
                            rights[entry] = p as u32;
                            *state = false;
                            break;
                        }
                        Some(false) => {
                            // The frame's right subtree ends: the whole frame
                            // completes, finishing one child of the frame
                            // outside it.
                            awaiting_left.pop();
                        }
                    }
                }
            }
            // A single-child tag opens nothing and closes nothing.
        }
        debug_assert_eq!(next_entry, count, "the two passes saw the same tags");
        debug_assert!(
            awaiting_left.is_empty() && chain_head == u32::MAX,
            "a normal-form preorder stream closes every frame it opens"
        );
        IdIndex {
            bits,
            rights: Some(rights),
        }
    }

    /// Whether the indexed operand and `other` (both normal-form ids) share no
    /// owned region: the same verdict as [`IdReader::is_disjoint`] on the same
    /// pair, in `O(other)` node visits.
    ///
    /// The walk is driven entirely by `other`'s cursor: a pair is visited only
    /// where `other` has a present node, an indexed-side child is addressed in
    /// `O(1)` through the table, and an indexed-side subtree standing against
    /// an absent `other` child is skipped by never being visited. Where the
    /// *indexed* side is absent, `other`'s subtree is skip-scanned once to
    /// resync its cursor — cost charged to `other`. Iterative: the per-ancestor
    /// state is a queued right-pair stack, bounded by `other`'s depth, so a
    /// deep operand cannot overflow the call stack.
    pub(crate) fn is_disjoint(&self, mut other: IdReader) -> bool {
        let Some(rights) = &self.rights else {
            // Positions overflowed the table at build: cursor-walk the
            // pair instead (the same predicate, unindexed).
            return IdReader::root(self.bits).is_disjoint(other);
        };
        // The current pair's indexed side: the bit position of its present
        // node, or `None` for an absent (empty) region.
        let mut node: Option<usize> = (!self.bits.is_empty()).then_some(0);
        // The table index of the first entry at or after the current node — the
        // node's own entry whenever it is both-present.
        let mut entry = 0usize;
        // Queued right pairs, innermost last: the indexed side's position
        // (`None` = absent child) and its entry bound. `other`'s right children
        // need no bookkeeping — its cursor reaches each one in stream order,
        // exactly as in the cursor walk.
        let mut pending: Vec<(Option<usize>, usize)> = Vec::new();
        loop {
            // One pair per iteration, driven by the input side.
            let b = other.read();
            match node {
                None => {
                    // The indexed side owns nothing here: disjoint. Skip
                    // `other`'s subtree to resync its cursor.
                    other.skip_present_children(b);
                }
                Some(p) => {
                    crate::codec::scan::record_bits(2); // one 2-bit tag read
                    let (al, ar) = (self.bits[p], self.bits[p + 1]);
                    match b {
                        // Only an empty *stream* reads `Empty` here (an absent
                        // child pair is never visited): vacuously disjoint.
                        IdNode::Empty => {}
                        // A present indexed node against a full `other` leaf:
                        // overlap.
                        IdNode::Full => return false,
                        // A full indexed leaf against a present `other` node:
                        // overlap.
                        IdNode::Internal { .. } if !al && !ar => return false,
                        // Both internal: descend over the child pairs `other`
                        // has, addressing the indexed side's children by
                        // position.
                        IdNode::Internal {
                            left: bl,
                            right: br,
                        } => {
                            let a_left = al.then(|| p + 2);
                            let (a_right, right_entry) = if al && ar {
                                // The node's entry is `entry` itself; its left
                                // subtree's entries follow it and keep targets
                                // strictly below the right child's position, so
                                // one partition finds the right subtree's
                                // entries.
                                let target = rights[entry];
                                let after_left = entry
                                    + 1
                                    + metered_partition_point(&rights[entry + 1..], target);
                                (Some(target as usize), after_left)
                            } else if ar {
                                // Right-only: the child follows the tag.
                                (Some(p + 2), entry)
                            } else {
                                (None, 0)
                            };
                            let left_entry = entry + usize::from(al && ar);
                            match (bl, br) {
                                (true, true) => {
                                    pending.push((a_right, right_entry));
                                    (node, entry) = (a_left, left_entry);
                                    continue;
                                }
                                (true, false) => {
                                    (node, entry) = (a_left, left_entry);
                                    continue;
                                }
                                (false, true) => {
                                    (node, entry) = (a_right, right_entry);
                                    continue;
                                }
                                (false, false) => {
                                    unreachable!("an internal id node has a present child")
                                }
                            }
                        }
                    }
                }
            }
            // The current pair closed disjoint: step into the innermost queued
            // right pair, or report the whole walk disjoint.
            match pending.pop() {
                Some(next) => (node, entry) = next,
                None => return true,
            }
        }
    }
}

/// The number of table entries strictly below `target` in the sorted slice —
/// `partition_point`, with every probe metered.
///
/// Each probe reads one `u32` table word, recorded in the scan currency (32
/// bits per probe): the table is derived verbatim from the packed stream's
/// positions, so probing it is stream examination by another route, and an
/// unmetered search would leave the fold's per-node `O(log n)` term visible to
/// no deterministic counter. The board's fold cells and the envelope suite's
/// both-present-rich scenario read the searches through this recording.
fn metered_partition_point(rights: &[u32], target: u32) -> usize {
    let (mut lo, mut hi) = (0usize, rights.len());
    while lo < hi {
        crate::codec::scan::record_bits(32);
        let mid = lo + (hi - lo) / 2;
        if rights[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}
