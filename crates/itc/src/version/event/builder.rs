use crate::codec::{Base, Bits};

use crate::version::compare::{EvHeader, EvView};
use crate::version::working::WorkingVersion;

/// Accumulates the output event tree in preorder. A node's base is written as a
/// placeholder when the node opens and finalized by
/// [`close_node`](Self::close_node) once its children are in place. This is the
/// canonical output path shared by every emitting walk (`join`, `fill`, the
/// [`grow`](super::grow) emit); it is the single place event normalization
/// lives, so callers never re-implement the sink/collapse.
pub(super) struct Builder {
    pub(super) topo: Bits,
    pub(super) base: Vec<Base>,
}

impl Builder {
    pub(super) fn with_capacity(nodes: usize) -> Self {
        Builder {
            topo: Bits::with_capacity(nodes),
            base: Vec::with_capacity(nodes),
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    /// Append a leaf with the given base; return its index.
    pub(super) fn leaf(&mut self, base: Base) -> usize {
        let i = self.len();
        self.topo.push(false);
        self.base.push(base);
        i
    }

    /// Open an internal node with a placeholder base; its children are appended
    /// next. Return its index.
    pub(super) fn open(&mut self, base: Base) -> usize {
        let i = self.len();
        self.topo.push(true);
        self.base.push(base);
        i
    }

    /// Copy the subtree at `root` of `src` verbatim (it is already normalized);
    /// return `(new_root, src_end)` — its index here and the position just past
    /// it in `src`. Iterative single pass: the same pending-children scan as
    /// the shared [`idbits::skip_subtree`](crate::idbits::skip_subtree) core,
    /// but it keeps its own loop because it emits each visited node into the
    /// output as it goes rather than only computing the end position.
    pub(super) fn copy(&mut self, src: &EvView, root: usize) -> (usize, usize) {
        let out_root = self.len();
        let mut pos = root;
        let mut pending: i64 = 1;
        while pending > 0 {
            let EvHeader {
                internal,
                base,
                next,
            } = src.header(pos);
            self.topo.push(internal);
            self.base.push(base);
            pos = next;
            pending += if internal { 1 } else { -1 };
        }
        (out_root, pos)
    }

    /// Finalize the internal node at `node` whose left child is at `node + 1`
    /// and right child at `right`. Sinks the children's common minimum into the
    /// node's base (`O(1)`) and collapses `(n, m, m)` of two equal leaves to a
    /// single leaf, preserving normal form. The node's root index is unchanged.
    ///
    /// Adjacency precondition for the collapse: it fires only when *both*
    /// children are leaves (the `!self.topo[..]` guards). A leaf occupies
    /// exactly one slot, so the left child is `node + 1` and the right child is
    /// `node + 2 == right` — i.e. `[node, left, right]` are the final three
    /// slots in `topo`/`base`. That is why `truncate(node)` discards exactly
    /// those three and nothing earlier before pushing the single collapsed leaf
    /// in their place.
    pub(super) fn close_node(&mut self, node: usize, right: usize) {
        let left = node + 1;
        let m = if self.base[left] <= self.base[right] {
            self.base[left].clone()
        } else {
            self.base[right].clone()
        };
        self.base[node] += &m;
        self.base[left] -= &m;
        self.base[right] -= &m;

        // Collapse only when both children are leaves of equal (post-sink) base.
        if !self.topo[left] && !self.topo[right] && self.base[left] == self.base[right] {
            debug_assert_eq!(
                right,
                node + 2,
                "collapse precondition: both children are adjacent leaves",
            );
            let collapsed = self.base[node].clone(); // the common child base is 0 after the sink
            self.topo.truncate(node);
            self.base.truncate(node);
            self.leaf(collapsed);
        }
    }

    pub(super) fn finish(self) -> WorkingVersion {
        WorkingVersion {
            topo: self.topo,
            base: self.base,
        }
    }
}

/// The thread register for the id-driven emitting walks — `fill` and the `grow`
/// emit (see the module doc): the output root a just-finished subtree produced,
/// plus where it ended in the packed id stream and the event tree. An `Eval`
/// arm *writes* it (a leaf directly, or via a `*Close` arm folding a node);
/// deferred frames *read* it.
#[derive(Clone, Copy, Default)]
pub(super) struct Built {
    /// Output index of the subtree's root.
    pub(super) out_root: usize,
    /// Position just past the subtree in the packed id stream.
    pub(super) id_end: usize,
    /// Position just past the subtree in the event tree.
    pub(super) ev_end: usize,
}
