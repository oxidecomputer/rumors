use crate::codec::Bits;
use crate::idbits::{IdNode, IdReader};

/// Single-buffer builder for normalized id output.
///
/// A node reserves a 2-bit tag
/// placeholder before its children are emitted; [`close_node`](Self::close_node)
/// patches the tag from which children turned out present, collapsing
/// `(1, 1) → 1` (both terminal) and `(0, 0) → 0` (both empty). This mirrors the
/// event-side `Builder`, but the id payload is only the tag bits.
pub(super) struct IdBuilder {
    bits: Bits,
}

/// What an emitted child turned out to be, so its parent's
/// [`close_node`](IdBuilder::close_node) can pick a tag and collapse.
///
/// `Empty` contributed no bits (a `0`), `Terminal` a lone owned leaf (a `1`),
/// `Node` an internal subtree. Carries no position: the tag is patched in place
/// at the parent's reserved slot and the children already sit contiguously
/// after it.
#[derive(Clone, Copy)]
pub(super) enum Built {
    /// The empty `0` region: no bits emitted.
    Empty,
    /// A single owned terminal (`1`).
    Terminal,
    /// An internal subtree.
    Node,
}

/// A just-reserved tag placeholder, awaiting its children and a
/// [`close_node`](IdBuilder::close_node).
///
/// `!Clone` and `#[must_use]`: the token
/// must be closed exactly once, and the borrow checker stops it being reused or
/// dropped silently — so an open with no matching close cannot compile.
#[must_use = "an opened node must be closed with close_node"]
pub(super) struct Open(usize);

impl IdBuilder {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        IdBuilder {
            bits: Bits::with_capacity(capacity),
        }
    }

    /// Append an owned terminal (the `1` leaf): the tag `00` (no children).
    pub(super) fn terminal(&mut self) -> Built {
        self.bits.push(false);
        self.bits.push(false);
        Built::Terminal
    }

    /// Reserve a node's 2-bit tag; its children are emitted next, then it is
    /// closed (and normalized) with [`close_node`](Self::close_node). The
    /// placeholder is patched to the real presence bits on close.
    pub(super) fn open(&mut self) -> Open {
        let root = self.bits.len();
        self.bits.push(false);
        self.bits.push(false);
        Open(root)
    }

    /// Copy one already-normal source subtree into the output, advancing `src`
    /// past it and reporting what it was.
    ///
    /// The source subtree is copied exactly
    /// once (a verbatim bit-range splice). A synthetic empty reader contributes
    /// nothing and reports [`Built::Empty`].
    pub(super) fn copy_reader(&mut self, src: &mut IdReader) -> Built {
        if matches!(src, IdReader::Empty) {
            return Built::Empty;
        }
        let is_terminal = matches!(src.peek(), IdNode::Full);
        let start = src.pos();
        src.skip();
        self.bits
            .extend_from_bitslice(&src.bits()[start..src.pos()]);
        if is_terminal {
            Built::Terminal
        } else {
            Built::Node
        }
    }

    /// Normalize and close the node opened at `node` from what its two children
    /// turned out to be, consuming the open token:
    ///
    /// - both empty ⇒ collapse to `0` (drop the tag, emit nothing);
    /// - both terminal ⇒ collapse to a single `1` (`(1, 1) → 1`);
    /// - otherwise patch the tag to record which children are present.
    pub(super) fn close_node(&mut self, node: Open, left: Built, right: Built) -> Built {
        let node = node.0;
        match (left, right) {
            (Built::Empty, Built::Empty) => {
                self.bits.truncate(node); // (0, 0) → 0
                Built::Empty
            }
            (Built::Terminal, Built::Terminal) => {
                self.bits.truncate(node); // (1, 1) → 1
                self.terminal()
            }
            (left, right) => {
                // bit 0 = left present, bit 1 = right present.
                self.bits.set(node, !matches!(left, Built::Empty));
                self.bits.set(node + 1, !matches!(right, Built::Empty));
                Built::Node
            }
        }
    }

    pub(super) fn finish(self) -> Bits {
        self.bits
    }
}
