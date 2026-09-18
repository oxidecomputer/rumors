//! Divides an owned region and removes selected descendants from it.
//!
//! A party identifier is a pruned binary tree encoded in preorder. Each node
//! has a two-bit tag saying which children are present; `00` is an owned
//! terminal, while an absent region has no encoding. The left subtree, when
//! present, immediately follows the tag. Finding the right subtree requires
//! skipping the complete left subtree first.
//!
//! Balanced division selects descendants by a path of left/right choices. The
//! direct path algorithms below compose all of those choices in one traversal:
//! one builds only the selected descendant, and one rebuilds the source without
//! it. Repeatedly splitting complete identifiers or applying a general set
//! operation would copy and rescan growing prefixes. Here, skipped and retained
//! source subtrees are disjoint, so each is traversed at most once.
//!
//! Paths may be much deeper than the encoded source when choices continue
//! below a terminal. The algorithms are iterative and retain only compact bits
//! per open ancestor, preventing path depth from becoming a machine-word-sized
//! allocation at every level.

use crate::codec::{extend_from_view, BitBuilder, BitStack, BitsBuf, BitsView};
use crate::idbits::{IdNode, IdReader};

use super::build::{Built, IdBuilder, Open, PosStack};

impl IdReader<'_> {
    /// Split this identifier into two disjoint identifiers whose union is it.
    ///
    /// The shared unary prefix cannot distinguish the halves, so the algorithm
    /// follows it to the first node with two present children or to a terminal.
    /// At a branch, each result keeps one child. At a terminal, the results
    /// become a left-owned and a right-owned unary node. The walk is iterative
    /// and linear in the encoded identifier.
    ///
    /// The cursor implementation of `oracle::Party::split`.
    pub(crate) fn split(self) -> (BitsBuf, BitsBuf) {
        // split(0) = (0, 0): the empty id splits into two empties.
        if let IdNode::Empty = self.peek() {
            return (BitsBuf::new(), BitsBuf::new());
        }
        let start = self.pos();
        build_split(self.bits(), start)
    }

    /// Select one descendant by a sequence of binary splits.
    ///
    /// Each `false` keeps the left half of the current region and each `true`
    /// keeps the right. This produces the same bits as repeatedly calling
    /// [`split`](Self::split) and retaining the selected half, but descends
    /// through the source only once and builds only the final descendant.
    pub(crate) fn split_path(self, path: impl IntoIterator<Item = bool>) -> BitsBuf {
        let IdReader::At { bits, pos } = self else {
            return BitsBuf::new();
        };
        build_split_path(bits, pos, path)
    }

    /// Remove the owned descendant selected by a left/right path.
    ///
    /// The path may end at an internal source subtree or continue below a
    /// terminal, where each choice conceptually splits that terminal in half.
    /// The selected region is known to be owned. Every retained sibling is
    /// copied once, and [`IdBuilder`] normalizes the reconstructed ancestors.
    /// Compact frames keep the auxiliary space proportional to path bits rather
    /// than machine words per path level.
    pub(crate) fn remove_path(
        self,
        path: impl IntoIterator<Item = bool>,
        output_capacity: u64,
    ) -> BitsBuf {
        let IdReader::At { bits, pos } = self else {
            return BitsBuf::new();
        };
        build_without_path(bits, pos, path, output_capacity)
    }
}

/// What unwinding needs to know about an unselected sibling.
///
/// A sibling before the selected child in preorder has already been emitted,
/// so its semantic kind is enough to close the parent. A sibling after the
/// selected child remains at the source cursor and is [`Deferred`](Self::Deferred)
/// until unwind. A conceptual split below a terminal has no source bits; its
/// retained terminal is represented explicitly.
#[derive(Clone, Copy)]
enum Sibling {
    /// The sibling is absent.
    Empty,
    /// A terminal, either already emitted or to be emitted during unwind.
    Terminal,
    /// An internal subtree already emitted before selecting its right sibling.
    Node,
    /// The sibling is a source subtree immediately after the selected left
    /// subtree, to be copied during unwind.
    Deferred,
}

/// Compact state for ancestors awaiting reconstruction.
///
/// Each ancestor needs three semantic bits: the selected direction and one of
/// four sibling states. Its output tag was reserved before descent, so its
/// position must also be recovered on unwind. [`PosStack`] run-compresses the
/// adjacent tag positions produced by deep descent.
struct RemoveFrames {
    /// Three bits per ancestor: selected direction followed by sibling code.
    bits: BitStack,
    /// Reserved output tag positions.
    opens: PosStack,
}

/// Stores and restores the removal walk's open ancestors.
impl RemoveFrames {
    /// Construct an empty frame stack.
    fn new() -> Self {
        Self {
            bits: BitStack::new(),
            opens: PosStack::new(),
        }
    }

    /// Retain one ancestor until its selected child has been removed.
    ///
    /// The fields are pushed in the inverse order used by [`pop`](Self::pop),
    /// keeping every logical frame aligned across the two compact stacks.
    fn push(&mut self, open: Open, right: bool, sibling: Sibling) {
        let code = match sibling {
            Sibling::Empty => 0,
            Sibling::Terminal => 1,
            Sibling::Node => 2,
            Sibling::Deferred => 3,
        };
        self.bits.push(right);
        self.bits.push(code & 2 != 0);
        self.bits.push(code & 1 != 0);
        self.opens.push(open);
    }

    /// Pop the innermost retained ancestor.
    fn pop(&mut self) -> Option<(Open, bool, Sibling)> {
        let low = self.bits.pop()?;
        let high = self
            .bits
            .pop()
            .expect("each removal frame carries three bits");
        let right = self
            .bits
            .pop()
            .expect("each removal frame carries three bits");
        let sibling = match (high, low) {
            (false, false) => Sibling::Empty,
            (false, true) => Sibling::Terminal,
            (true, false) => Sibling::Node,
            (true, true) => Sibling::Deferred,
        };
        Some((self.opens.pop(), right, sibling))
    }
}

/// Remove the source region reached by `path` and rebuild its ancestors.
///
/// The traversal maintains two positions:
///
/// - `pos` is the next unread source tag on the selected path, or the first
///   deferred right sibling after the selected subtree has been skipped;
/// - `out` is the complete preorder output before the selected region.
///
/// During descent, a sibling to the left of the selection must be copied
/// immediately because it precedes the selection in preorder. A sibling to the
/// right cannot be reached until the selected subtree has been skipped, so its
/// frame records [`Sibling::Deferred`] without storing a source position.
/// Once descent ends, skipping the selected subtree places `pos` exactly at the
/// innermost deferred right sibling. Unwinding consumes such siblings in order,
/// closes each reserved parent, and lets [`IdBuilder`] apply canonical
/// collapses.
///
/// No copied or skipped source subtree overlaps another. The work is therefore
/// linear in source, path, and output size. Per-ancestor state is three bits
/// plus a compressed reserved-tag position.
fn build_without_path(
    bits: BitsView<'_>,
    start: u64,
    path: impl IntoIterator<Item = bool>,
    output_capacity: u64,
) -> BitsBuf {
    let mut out = IdBuilder::with_capacity(output_capacity);
    let mut frames = RemoveFrames::new();
    let mut pos = start;
    let mut below_terminal = false;

    for right in path {
        // This parent must remain open until the selected child has been
        // removed and its sibling is available.
        let open = out.open();
        if below_terminal {
            // A terminal can be divided conceptually without reading source
            // bits. Its unselected half is another terminal. When selecting
            // right, that left sibling precedes the selection and is emitted
            // now; when selecting left, emission waits for unwind.
            if right {
                out.terminal();
            }
            frames.push(open, right, Sibling::Terminal);
            continue;
        }

        crate::codec::scan::record_bits(2);
        let (left_present, right_present) = (bits.bit(pos), bits.bit(pos + 1));
        pos += 2;
        if !left_present && !right_present {
            // Further path choices refine this owned terminal rather than
            // descend through source nodes that do not exist.
            below_terminal = true;
            if right {
                out.terminal();
            }
            frames.push(open, right, Sibling::Terminal);
            continue;
        }

        if right {
            assert!(right_present, "the removed interval is owned");
            // Preorder places the retained left sibling before the selected
            // right child. Copying it advances `pos` to that right child.
            let left = if left_present {
                let mut reader = IdReader::at(bits, pos);
                let built = out.copy_reader(&mut reader);
                pos = reader.pos();
                built
            } else {
                Built::Empty
            };
            let sibling = match left {
                Built::Empty => Sibling::Empty,
                Built::Terminal => Sibling::Terminal,
                Built::Node => Sibling::Node,
            };
            frames.push(open, true, sibling);
        } else {
            assert!(left_present, "the removed interval is owned");
            // The selected left child begins at `pos`. A present right sibling
            // follows the complete selected subtree, so defer it without an
            // offset; skipping the selection below will land on it.
            frames.push(
                open,
                false,
                if right_present {
                    Sibling::Deferred
                } else {
                    Sibling::Empty
                },
            );
        }
    }

    if !below_terminal {
        // Removing an encoded subtree means consuming it without emitting it.
        // The cursor now points at the first right sibling deferred on descent.
        let mut removed = IdReader::at(bits, pos);
        removed.skip();
        pos = removed.pos();
    }

    // From the innermost parent's perspective, the selected child built an
    // empty region. Each closed parent becomes the selected child at the next
    // outer level.
    let mut built = Built::Empty;
    while let Some((open, right, sibling)) = frames.pop() {
        let sibling = match (right, sibling) {
            (false, Sibling::Deferred) => {
                // Deferred right siblings occur in the same inner-to-outer
                // order as unwind and are contiguous at the source cursor.
                let mut reader = IdReader::at(bits, pos);
                let built = out.copy_reader(&mut reader);
                pos = reader.pos();
                built
            }
            (false, Sibling::Terminal) => out.terminal(),
            (_, Sibling::Empty) => Built::Empty,
            (true, Sibling::Terminal) => Built::Terminal,
            (true, Sibling::Node) => Built::Node,
            (true, Sibling::Deferred) | (false, Sibling::Node) => {
                unreachable!("the sibling code matches its selected direction")
            }
        };
        // Restore source child order when closing the parent. `built` is the
        // selected child after removal; `sibling` is the retained child.
        built = if right {
            out.close_node(open, sibling, built)
        } else {
            out.close_node(open, built, sibling)
        };
    }
    debug_assert_eq!(pos, bits.len(), "removing one interval consumes the source");
    out.finish()
}

/// Where the spine descent of [`split`](IdReader::split) ended.
enum SpineEnd {
    /// A both-present node: split keeps one child per half.
    Branch,
    /// A terminal `1` leaf: split is `(1,0)`/`(0,1)`.
    Terminal,
}

/// Build the two split halves of the id rooted at `start` in `bits`.
///
/// Walks the unary spine to the branch (or terminal), then splices: each half
/// is the spine prefix, a retagged node, and the kept child — a bulk verbatim
/// copy of already-normal bit ranges, normal by construction (the kept child is
/// nonempty, so no collapse can arise). Iterative: the spine walk is a loop, so
/// deep ids cannot overflow.
fn build_split(bits: BitsView<'_>, start: u64) -> (BitsBuf, BitsBuf) {
    let mut pos = start;
    let (prefix_end, kind) = loop {
        match (bits.bit(pos), bits.bit(pos + 1)) {
            (false, false) => break (pos, SpineEnd::Terminal), // the `1` leaf
            (true, true) => break (pos, SpineEnd::Branch),     // both-present branch
            _ => pos += 2, // unary: descend the single present child (at pos + 2)
        }
    };
    let prefix = start..prefix_end;
    let prefix_len = prefix_end - start;

    match kind {
        SpineEnd::Branch => {
            // The left child occupies `left_child..right_child`; the right
            // occupies `right_child..branch_end`. Both halves keep the spine.
            let left_child = prefix_end + 2;
            let right_child = subtree_end(bits, left_child);
            let branch_end = subtree_end(bits, right_child);
            debug_assert_eq!(
                branch_end,
                bits.len(),
                "the branch subtree is the spine's tail",
            );

            // Each half keeps one child and drops the other, so its length is
            // exact: prefix + the 2-bit retagged branch + the kept child.
            let mut a = BitsBuf::with_capacity(prefix_len + 2 + (right_child - left_child));
            extend_from_view(&mut a, bits, prefix.start, prefix.end);
            a.push(true); // branch → Left-only: keep the left child ...
            a.push(false); // ... drop the right
            extend_from_view(&mut a, bits, left_child, right_child);

            let mut b = BitsBuf::with_capacity(prefix_len + 2 + (branch_end - right_child));
            extend_from_view(&mut b, bits, prefix.start, prefix.end);
            b.push(false); // branch → Right-only: drop the left child ...
            b.push(true); // ... keep the right
            extend_from_view(&mut b, bits, right_child, branch_end);

            (a, b)
        }
        SpineEnd::Terminal => {
            // split(1) = ((1, 0), (0, 1)): the terminal becomes a unary node
            // over a terminal on each side.
            debug_assert_eq!(
                prefix_end + 2,
                bits.len(),
                "the terminal is the spine's tail",
            );
            let mut a = BitsBuf::with_capacity(prefix_len + 4);
            extend_from_view(&mut a, bits, prefix.start, prefix.end);
            a.push(true); // (1, 0): Left-only ...
            a.push(false);
            a.push(false); // ... over a terminal
            a.push(false);

            let mut b = BitsBuf::with_capacity(prefix_len + 4);
            extend_from_view(&mut b, bits, prefix.start, prefix.end);
            b.push(false); // (0, 1): Right-only ...
            b.push(true);
            b.push(false); // ... over a terminal
            b.push(false);

            (a, b)
        }
    }
}

/// Apply a sequence of binary splits while building only the selected result.
///
/// One ordinary [`split`](IdReader::split) first follows the region's unary
/// prefix: both halves inherit that prefix, so it cannot separate them. At the
/// first branch it keeps one child and changes the branch to the corresponding
/// unary tag. At a terminal it creates that unary node and puts a terminal in
/// the selected child. Repeating `split` would copy the prefix accumulated so
/// far on every step.
///
/// This builder composes those steps in one forward descent. Its loop
/// maintains:
///
/// - `out` is exactly the final encoding before the current selected region;
/// - `pos` is the root of that region in the original `bits`.
///
/// For each direction in `path`, it walks to the next place that can split:
///
/// 1. A unary source node belongs to both possible halves. Copy its tag and
///    advance to its only child, which immediately follows the tag.
/// 2. At a branch, append the unary tag for the chosen half and move `pos` to
///    that child. The left child begins after the branch tag; choosing right
///    scans past the discarded left subtree once.
/// 3. At a terminal, the source has no deeper structure. Every remaining
///    choice therefore becomes a new unary node, followed by one terminal, and
///    the result is complete.
///
/// If the path ends inside the source, the whole selected subtree is unchanged
/// and is appended verbatim. Discarded sibling subtrees are disjoint, so their
/// skip scans do not overlap; selected source structure is never revisited from
/// the root. Once the path is exhausted, one final scan locates the end of the
/// selected subtree and that range is copied into `out`. The work is
/// `O(|bits| + |path| + |output|)` and the only allocated tree is the output.
fn build_split_path(
    bits: BitsView<'_>,
    start: u64,
    path: impl IntoIterator<Item = bool>,
) -> BitsBuf {
    let mut out = BitBuilder::with_capacity(0);
    let mut pos = start;
    let mut path = path.into_iter();

    while let Some(right) = path.next() {
        loop {
            // This walk reads tags directly rather than through `IdReader`.
            crate::codec::scan::record_bits(2);
            match (bits.bit(pos), bits.bit(pos + 1)) {
                (false, false) => {
                    push_unary(&mut out, right);
                    for right in path {
                        push_unary(&mut out, right);
                    }
                    out.push_bit(false);
                    out.push_bit(false);
                    return out.finish();
                }
                (true, true) => {
                    let left = pos + 2;
                    push_unary(&mut out, right);
                    pos = if right {
                        // Preorder stores the complete left subtree before the
                        // right subtree. Its end is therefore the right root.
                        subtree_end(bits, left)
                    } else {
                        left
                    };
                    break;
                }
                (left_present, right_present) => {
                    // A unary node's only child starts immediately after its
                    // tag; both split halves inherit this path unchanged.
                    out.push_bit(left_present);
                    out.push_bit(right_present);
                    pos += 2;
                }
            }
        }
    }

    // No requested split reaches inside this region, so it survives exactly as
    // encoded in the source.
    let end = subtree_end(bits, pos);
    out.splice(bits, pos, end);
    out.finish()
}

/// Append a unary node retaining the selected child.
fn push_unary(out: &mut BitBuilder, right: bool) {
    out.push_bit(!right);
    out.push_bit(right);
}

/// The bit position just past the subtree at `pos` (the shared
/// [`skip`](IdReader::skip) scan), for slicing a branch child's verbatim range.
fn subtree_end(bits: BitsView<'_>, pos: u64) -> u64 {
    let mut r = IdReader::at(bits, pos);
    r.skip();
    r.pos()
}
