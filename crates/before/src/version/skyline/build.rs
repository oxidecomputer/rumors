//! The canonical skyline output builder: append one leaf per plateau, collapse
//! equal sibling leaves by truncation.
//!
//! An emitter drives this builder with the output's leaf sequence in preorder —
//! each call one plateau, as a depth and a payload code — and gets back a
//! canonical stream: the preorder leaf depths of a dyadic tiling determine the
//! tree, so the builder derives every topology flag itself, and the one
//! normalization the coding leaves (an internal node whose two children are
//! equal-height leaves — a zero right-sibling delta) it performs as it closes
//! each node. Both collapse repairs are subtractive, on the [`PackedBuilder`]
//! move set:
//!
//! - **Absorb** (the pair's right leaf arrives): the incoming delta code
//!   is 1 bit exactly when the delta is zero — `gamma(zigzag(0))` is the
//!   lone 1-bit code — so a zero-delta right sibling of a held left leaf
//!   is recognized before anything is written, absorbed, and the pair's
//!   parent flag truncated off the stream. The held leaf's own code is
//!   untouched: its delta is against a predecessor outside the pair, and
//!   the merged leaf has the same height and the same predecessor.
//! - **Re-anchor** — the merged leaf adopts the left sibling's code, its
//!   delta re-anchoring to that sibling's predecessor (the case: the
//!   pair's right leaf is the merge of a whole
//!   subtree): the merged leaf's zero delta says it equals the completed
//!   left sibling leaf, so the pair truncates back over that sibling's
//!   code — parent flag, leaf flag, code — and the sibling's code is
//!   copied out first to become the held leaf's. Each copied bit is a
//!   bit being truncated, so the copy is paid by the deletion.
//!
//! Cascading is the loop over re-anchor: a merged leaf may in turn be a
//! zero-delta right sibling one level up. Each cascade step deletes at least
//! three stream bits and copies only a code already priced by that deletion, so
//! emission stays amortized O(1) per output bit; the wide code a deep uniform
//! region telescopes onto is *held*, never re-copied (the absorb repair moves
//! no code bits at all, whatever the held width).
//!
//! # The held leaf
//!
//! The most recent leaf is held out of the stream — flag and code — until the
//! next leaf's arrival decides whether it flushes or merges; the stream itself
//! always ends with the held leaf's preorder predecessor. Holding the code is
//! what makes absorb O(1): a deep uniform region collapses by truncating one
//! parent flag per level around a held code that never moves, where a flushed
//! code would shift left by one bit per level — quadratic on exactly the join
//! shapes (a flat operand raised over a deep one) that collapse everywhere.
//!
//! # Transient state
//!
//! Bits per open ancestor, and nothing per node: the branch-direction path, one
//! is-the-left-sibling-a-leaf bit per level, and — for the levels where that
//! left sibling *is* a leaf — its code's length on a pop-able bit stack
//! ([`PopStack`], ~2·log₂(length) bits per entry), the coordinate re-anchor
//! truncates to. The resource-envelope suite (`tests/meter.rs`, the
//! `skyline_join_*` rows) pins the whole emission's transient against these
//! bounds.

use crate::codec::{BitStack, BitsMut, BitsSlice, Code, PackedBuilder, PopStack};

/// The 1-bit payload code: `gamma(zigzag(0))`, the zero delta.
///
/// Gamma codes spend `2·floor(log2(m + 1)) + 1` bits on a mapped value `m`, so
/// one bit is exactly the code for zero — the recognition the collapse checks
/// ride on.
///
/// One other code shares the length: `gamma(0)`, the *absolute* code of a
/// height-zero first leaf. It never triggers a collapse. The absorb test reads
/// the incoming code, which is always a delta (the first leaf returns early
/// from [`leaf`](SkylineBuilder::leaf)); the cascade test reads the held code,
/// which is the absolute code only while the first leaf is held — and the first
/// leaf lies on the leftmost, all-`false` path, so cascade's right-child test
/// fails before the length is consulted.
const ZERO_DELTA_CODE_BITS: usize = 1;

/// A canonical-skyline stream builder driven by the output leaf sequence.
///
/// Create with [`with_capacity`](Self::with_capacity), feed every plateau in
/// preorder through [`leaf`](Self::leaf), and take the canonical stream with
/// [`finish`](Self::finish). The module doc carries the collapse discipline and
/// the cost argument.
pub(super) struct SkylineBuilder {
    out: PackedBuilder,
    /// The held leaf's payload code (the module doc's *held leaf*); `None` only
    /// before the first leaf arrives.
    held: Option<Code>,
    /// Root-to-held-leaf branch directions: `false` inside a left child, `true`
    /// inside a right.
    path: BitStack,
    /// Parallel to `path`: at a right-branch level, whether the completed left
    /// sibling is a single leaf (the collapse precondition).
    ///
    /// `false` is both the placeholder at left-branch levels and the record at
    /// right-branch levels [`continue_verbatim`](Self::continue_verbatim)
    /// splices in, where canonicity already rules the merge out.
    left_leaf: BitStack,
    /// Code lengths of the left-sibling leaves, one entry per right-branch
    /// level whose `left_leaf` bit is set, deepest last.
    lens: PopStack,
}

impl SkylineBuilder {
    /// Create a builder with room for `capacity` output bits.
    pub(super) fn with_capacity(capacity: usize) -> Self {
        SkylineBuilder {
            out: PackedBuilder::with_capacity(capacity),
            held: None,
            path: BitStack::new(),
            left_leaf: BitStack::new(),
            lens: PopStack::new(),
        }
    }

    /// Append the next plateau: a leaf at `depth` whose payload is `code`.
    ///
    /// `depth` is the leaf's tree depth (its plateau has width `2^-depth`), and
    /// `code` its complete payload — the absolute gamma code for the first
    /// leaf, the zigzag-gamma delta code for every later one. The leaf sequence
    /// must be the preorder tiling of one tree: each new depth must be
    /// reachable from the last by the forced flip-and-descend, which the
    /// builder debug-asserts.
    pub(super) fn leaf(&mut self, depth: usize, code: Code) {
        debug_assert!(code.len() > 0, "a leaf payload code is never empty");
        let Some(held) = self.held.take() else {
            // The first leaf: the leftmost path, one flag per ancestor.
            self.descend_to(depth);
            self.held = Some(code);
            return;
        };

        // The incoming leaf is the held leaf's direct right sibling exactly
        // when the held leaf is a left child at the same depth. A zero delta
        // there is the collapsible pair (the module doc's *absorb*): absorb the
        // incoming leaf, truncate the pair's parent flag (the stream's last
        // bit, since the parent is the held left child's preorder predecessor),
        // and let the merge cascade. The pair's left sibling is the held leaf
        // itself, which is why — unlike `cascade`'s test, where the held leaf
        // is the pair's *right* child under an arbitrary completed left sibling
        // — this test reads `path` alone and never consults `left_leaf`.
        if depth == self.path.len()
            && self.path.last().map(|bit| !bit).unwrap_or(false)
            && code.len() == ZERO_DELTA_CODE_BITS
        {
            self.out.truncate(self.out.len() - 1);
            // Bare pops: a left-branch level carries no lens entry (only a
            // completed left leaf records one), so unlike the flush loop's
            // right-branch pops there is no lens to retire.
            self.path.pop();
            self.left_leaf.pop();
            self.held = Some(held);
            self.cascade();
            return;
        }

        // Flush the held leaf and place the incoming one.
        let flushed_len = held.len();
        self.out.push_bit(true);
        self.out.push_code(&held);
        // Close the ancestors the flushed leaf completed: pop the trailing
        // right-branch levels, retiring their left-sibling records, then flip
        // the deepest left branch to its right child.
        let mut popped_rights = 0usize;
        loop {
            match self.path.pop() {
                Some(true) => {
                    if self.left_leaf.pop() == Some(true) {
                        self.lens.pop();
                    }
                    popped_rights += 1;
                }
                Some(false) => {
                    self.left_leaf.pop();
                    break;
                }
                None => panic!("a leaf arrived after the final plateau: the tiling is complete"),
            }
        }
        // The subtree completed at the flip level is a single leaf exactly when
        // the flushed leaf was itself the left child there.
        let left_is_leaf = popped_rights == 0;
        self.path.push(true);
        self.left_leaf.push(left_is_leaf);
        if left_is_leaf {
            debug_assert!(flushed_len > 0, "payload codes are never empty");
            self.lens.push(flushed_len as u64);
        }
        debug_assert!(
            depth >= self.path.len(),
            "a leaf depth above its forced flip level: the input is not one preorder tiling"
        );
        self.descend_to(depth);
        self.held = Some(code);
    }

    /// Whether the most recent [`leaf`](Self::leaf) survives as the held leaf
    /// at exactly `depth`, no absorb or cascade having merged it upward.
    ///
    /// The predicate behind
    /// [`continue_verbatim`](Self::continue_verbatim)'s held-first-leaf
    /// precondition: the splice extends exactly that leaf.
    pub(super) fn held_at(&self, depth: usize) -> bool {
        self.held.is_some() && self.path.len() == depth
    }

    /// Splice the remainder of a canonical multi-leaf subtree verbatim, holding
    /// its last leaf's code per the held-leaf discipline.
    ///
    /// Reached from the tick splice ([`grow`](super::grow)) and the fill walk's
    /// post-divergence region copy; the join/meet emission feeds every plateau
    /// through [`leaf`](Self::leaf) instead.
    ///
    /// The caller has just fed the subtree's *first* leaf through
    /// [`leaf`](Self::leaf) (verbatim or with a repaired code) at depth
    /// `root_depth + first_rel_depth`, and that leaf is still held there,
    /// unmerged — the depths alone guarantee it, and the splice
    /// debug-asserts it: absorb takes only the held leaf's direct *right*
    /// sibling, while a subtree's first leaf at positive relative depth lies
    /// on the subtree's leftmost path and so enters as a *left* child.
    /// `rest` is the subtree's stream from just past that leaf's payload
    /// code to the subtree's end. Because every consecutive-leaf delta
    /// strictly inside a canonical subtree is unchanged by anything outside
    /// it, the range is copied in one splice instead of leaf by leaf; only
    /// the held-leaf discipline is re-established around it — the last
    /// leaf's flag is withheld and its code (`last_code_len` bits, ending
    /// the range) becomes the held code. `first_rel_depth` and
    /// `last_rel_depth` are the first and last leaves' depths below the
    /// subtree root, each at least 1: a single-leaf subtree is fed wholly
    /// through [`leaf`](Self::leaf) instead.
    ///
    /// The spliced interior levels record no left-sibling-leaf collapse
    /// coordinates: a canonical subtree's rightmost leaf is never the equal
    /// right sibling of a left-sibling leaf (the pair would have collapsed at
    /// the source), so a cascade can never need to re-anchor into the spliced
    /// range, and the placeholder records only suppress merges canonicity
    /// already rules out.
    pub(super) fn continue_verbatim(
        &mut self,
        rest: &BitsSlice,
        root_depth: usize,
        first_rel_depth: usize,
        last_rel_depth: usize,
        last_code_len: usize,
    ) {
        debug_assert!(
            first_rel_depth >= 1 && last_rel_depth >= 1,
            "a multi-leaf subtree's first and last leaves sit below its root"
        );
        debug_assert!(
            self.held_at(root_depth + first_rel_depth),
            "the spliced subtree's first leaf is still held at its own depth: \
             absorb takes only a direct right sibling, and a first leaf below \
             its subtree root enters as a left child"
        );
        debug_assert!(
            rest.len() > last_code_len,
            "the continuation holds at least the last leaf's flag and code"
        );
        let last_flag = rest.len() - last_code_len - 1;
        debug_assert!(rest[last_flag], "the continuation ends with a leaf");
        let held = self
            .held
            .take()
            .expect("the subtree's first leaf is already held");
        // Flush the first leaf and copy everything up to the last leaf's flag;
        // the last code is withheld as the new held leaf.
        self.out.push_bit(true);
        self.out.push_code(&held);
        self.out.splice(&rest[..last_flag]);
        self.held = Some(Code::from_slice(&rest[last_flag + 1..]));
        // Re-anchor the per-level stacks from the first leaf's leftmost descent
        // to the last leaf's rightmost one. The popped levels were pushed by
        // `descend_to` (left branches, no left-sibling records), and the pushed
        // levels carry the placeholder records argued above.
        while self.path.len() > root_depth {
            let popped = self.path.pop();
            debug_assert_eq!(
                popped,
                Some(false),
                "a subtree's first leaf lies on its leftmost path"
            );
            self.left_leaf.pop();
        }
        for _ in 0..last_rel_depth {
            self.path.push(true);
            self.left_leaf.push(false);
        }
    }

    /// Take the finished canonical stream.
    ///
    /// # Panics
    ///
    /// Panics if no leaf was ever appended.
    pub(super) fn finish(mut self) -> BitsMut {
        let held = self
            .held
            .take()
            .expect("a skyline stream has at least one leaf");
        self.out.push_bit(true);
        self.out.push_code(&held);
        debug_assert!(
            self.path.all_set(),
            "the final leaf closes every open ancestor from the right"
        );
        self.out.finish()
    }

    /// Merge the held leaf upward while it is a zero-delta right sibling of a
    /// completed left-sibling leaf (the module doc's *re-anchor*).
    ///
    /// Called from the absorb branch alone, and that suffices for canonicality:
    /// a flush never exposes a collapsible pair. An incoming zero-delta leaf
    /// that would be the flushed leaf's direct right sibling is exactly the
    /// absorb condition, so a leaf that reaches the flush path with a 1-bit
    /// code lands either strictly deeper (a left child, no completed sibling)
    /// or as the right sibling of a completed multi-leaf subtree — a shape
    /// canonical form keeps.
    fn cascade(&mut self) {
        loop {
            let held = self.held.as_ref().expect("cascade runs with a held leaf");
            // The held delta must be zero, the held leaf a right child,
            // and that level's left sibling a single leaf.
            if held.len() != ZERO_DELTA_CODE_BITS
                || !self.path.last().unwrap_or(false)
                || !self.left_leaf.last().unwrap_or(false)
            {
                return;
            }
            // The stream ends with the pair's prefix: parent flag, left leaf
            // flag, left leaf code. The merged leaf keeps the left code — same
            // height, same predecessor — and the pair leaves the stream; each
            // copied bit is one being truncated.
            let code_len = self.lens.pop() as usize;
            let code = self.out.extract_code(self.out.len() - code_len);
            self.out.truncate(self.out.len() - code_len - 2);
            self.path.pop();
            self.left_leaf.pop();
            self.held = Some(code);
        }
    }

    /// Descend left from the current path to a leaf at `depth`, emitting one
    /// internal-node flag per level entered.
    fn descend_to(&mut self, depth: usize) {
        for _ in self.path.len()..depth {
            self.out.push_bit(false);
            self.path.push(false);
            self.left_leaf.push(false);
        }
    }
}

#[cfg(test)]
mod tests;
