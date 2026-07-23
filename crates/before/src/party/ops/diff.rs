use crate::codec::{Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;
use crate::step;

use super::build::{Built, IdBuilder};

impl IdReader<'_> {
    /// The region *difference* `self \ other` (normal-form ids): the part of
    /// `self`'s region that `other` does not own, as a normalized id.
    ///
    /// Unlike [`sum`](IdReader::sum), `diff` is *total* — overlap is the whole
    /// point, not an error — and its result may be the **empty** `0` id (the
    /// empty bit stream), exactly when `other` covers `self`. The caller
    /// ([`Party::without`](crate::Party::without))
    /// maps that empty result to `None`, since a `Party` is a nonzero share.
    ///
    /// The result is always a subregion of `self` (`self \ other ⊆ self`), so it
    /// introduces no region `self` did not already own. That is what keeps it
    /// linearity-safe where a general id *meet* is not (see the note on the
    /// absent `BitAnd for Clock` in [`oracle`](crate::oracle)): carving a
    /// sub-share out of a region you already hold, and consuming the original,
    /// can never synthesize a region shared with a third live party.
    ///
    /// `O(n + m)`: the both-internal case threads (no skip); `diff(0, b)` and
    /// `diff(a, 1)` skip the dominated side once; `diff(a, 0)` copies `a` and
    /// `diff(1, b)` complements `b`, each bounded by the output size.
    ///
    /// The cursor form of `oracle::Party::without`. It recurses only where
    /// *both* operands are internal — so, as in [`sum`](IdReader::sum), one
    /// shallow operand caps the recursion depth, and [`crate::recurse`]
    /// guards what remains. The one-sided `diff(1, b)` arm emits
    /// `complement(b)` iteratively (see [`DiffWalk::complement`]), so a deep
    /// subtrahend under a full region drives no recursion at all.
    pub(crate) fn diff(mut self, mut other: IdReader) -> Bits {
        let mut walk = DiffWalk {
            // `self \ other` is a subregion of `self`, but `diff(1, b)` emits
            // `complement(b)`, which can be as large as `other`. Both inputs
            // combined is a safe bound; normalization only shrinks it.
            out: IdBuilder::with_capacity(self.bits().len() + other.bits().len()),
        };
        descend!(0, walk.rec(&mut self, &mut other, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`diff`](IdReader::diff) walk; the `&mut`
/// readers carry the traversal state, exactly as in [`sum`](IdReader::sum).
struct DiffWalk {
    out: IdBuilder,
}

impl DiffWalk {
    /// Difference the subtrees at the two `&mut` readers, emitting into `out`
    /// and advancing both readers past their subtrees.
    ///
    /// Reads as a match on the
    /// two id nodes: `diff(0, b) = 0` and `diff(a, 1) = 0` keep nothing (skip
    /// both sides), `diff(a, 0) = a` copies the survivor verbatim, `diff(1, b) =
    /// complement(b)` keeps what `b` lacks, and two nodes recurse and normalize
    /// on close.
    ///
    /// The kept side is [`peek`](IdReader::peek)ed, not read, so `copy_reader`
    /// can splice its whole subtree.
    fn rec(&mut self, a: &mut IdReader, b: &mut IdReader, depth: usize) -> Built {
        match (a.peek(), b.peek()) {
            // diff(0, b) = 0: `self` owns nothing here. Skip both to resync.
            (IdNode::Empty, _) => {
                a.skip();
                b.skip();
                Built::Empty
            }
            // diff(a, 0) = a: `other` owns nothing here, so keep `a` verbatim.
            (_, IdNode::Empty) => {
                let out_root = self.out.copy_reader(a);
                b.skip();
                out_root
            }
            // diff(a, 1) = 0: `other` owns the whole region, nothing survives.
            (_, IdNode::Full) => {
                a.skip();
                b.skip();
                Built::Empty
            }
            // diff(1, b) = complement(b): `self` owns everything here, so the
            // survivors are exactly the region `b` does *not* own.
            (IdNode::Full, _) => {
                a.skip(); // consume the full `1` leaf
                self.complement(b)
            }
            // Both internal: difference each child pair (threading the real
            // cursor into present children, a synthetic `Empty` into absent
            // ones), then close the node, which normalizes.
            (
                IdNode::Internal {
                    left: al,
                    right: ar,
                },
                IdNode::Internal {
                    left: bl,
                    right: br,
                },
            ) => {
                a.read();
                b.read();
                let node = self.out.open();
                let left = self.child(a, al, b, bl, depth);
                let right = self.child(a, ar, b, br, depth);
                self.out.close_node(node, left, right)
            }
        }
    }

    /// Difference one child pair: thread the real cursor where the child is
    /// present, a synthetic [`Empty`](IdReader::Empty) where it is absent.
    fn child(
        &mut self,
        a: &mut IdReader,
        a_present: bool,
        b: &mut IdReader,
        b_present: bool,
        depth: usize,
    ) -> Built {
        let mut empty_a = IdReader::Empty;
        let mut empty_b = IdReader::Empty;
        let ca = if a_present { a } else { &mut empty_a };
        let cb = if b_present { b } else { &mut empty_b };
        descend!(depth + 1, self.rec(ca, cb, depth + 1))
    }

    /// Emit `complement(b)` — the region `b` does *not* own — advancing `b` past
    /// its subtree.
    ///
    /// `complement(0) = 1`, `complement(1) = 0`, and an internal node
    /// complements each child: an absent child (a `0`) complements to a
    /// terminal, a terminal child to an absent `0`. On a normal id this is a
    /// *structure-preserving map*: an internal node's complement is always
    /// internal (two terminal children would complement from `(0, 0)`, two
    /// absent children from `(1, 1)`, neither representable), so the
    /// complemented tree is already normal and the output is a straight
    /// retagging of the input tag stream, with no close-time collapse.
    ///
    /// Iterative, two passes over the subtree's fixed-width tag stream, so a
    /// deep `b` costs neither recursion frames nor per-level heap frames —
    /// the traversal state is a handful of bits per level:
    ///
    /// - Every output tag bit is local to the node's own read except one: a
    ///   both-children node's *right*-presence bit, which flips on whether
    ///   its right child is internal, and that child's tag sits an entire
    ///   left subtree ahead. Pass 1 resolves it by scanning the tags
    ///   *backward* (reverse preorder: children before parents), stacking
    ///   each completed subtree's root kind on a bit stack — a node pops its
    ///   children's kinds, left on top — and recording every both-children
    ///   node's right-child kind; pass 2 visits those nodes in the exact
    ///   reverse order, so it pops the records straight off the stack.
    /// - Pass 2 emits each output tag at its node's read and orders the
    ///   deferred emission — the terminal in an absent right child's slot,
    ///   due only after the left subtree's output — through a two-bit
    ///   pending entry per open ancestor: the [`skip_subtree`] counter
    ///   refined into *what to do* when the subtree under it closes.
    ///
    /// [`skip_subtree`]: crate::idbits::skip_subtree
    fn complement(&mut self, b: &mut IdReader) -> Built {
        // complement(0) = 1: only ever a synthetic reader (an absent child).
        if matches!(b, IdReader::Empty) {
            return self.out.terminal();
        }
        // complement(1) = 0: consume the terminal, emit nothing.
        if matches!(b.peek(), IdNode::Full) {
            b.skip();
            return Built::Empty;
        }
        let bits = b.bits();
        let start = b.pos();
        b.skip();
        let end = b.pos();

        // Pass 1 (backward): resolve each both-children node's right-child
        // kind, in reverse preorder.
        let mut kinds = Bits::new();
        let mut right_kinds = Bits::new();
        let mut at = end;
        while at > start {
            at -= 2;
            step!();
            match (bits[at], bits[at + 1]) {
                // A terminal: a completed leaf subtree.
                (false, false) => kinds.push(false),
                // Both children present: their subtrees completed last (left)
                // and second-to-last (right). Record the right kind.
                (true, true) => {
                    kinds.pop().expect("left child kind is on the stack");
                    let right = kinds.pop().expect("right child kind is on the stack");
                    right_kinds.push(right);
                    kinds.push(true);
                }
                // One child: consume its completed subtree.
                _ => {
                    kinds.pop().expect("the only child's kind is on the stack");
                    kinds.push(true);
                }
            }
        }
        debug_assert_eq!(kinds.len(), 1, "the scan completes exactly one subtree");

        // Pass 2 (forward): emit the retagged stream in preorder. An output
        // presence bit is set where the input child complements to something
        // present: an absent or internal input child (a terminal child
        // complements to an absent `0`).
        let mut pending = Bits::new();
        let mut at = start;
        while at < end {
            step!();
            let tag = (bits[at], bits[at + 1]);
            at += 2;
            match tag {
                // A terminal complements to an absent child: emit nothing.
                // Its subtree is complete, so unwind the ancestors it closes.
                (false, false) => loop {
                    let Some((deferred, synthetic)) = pop_pending(&mut pending) else {
                        debug_assert_eq!(at, end, "the unwind past the root ends the subtree");
                        break;
                    };
                    if deferred && synthetic {
                        // The ancestor's absent right child complements to a
                        // terminal in this slot; the ancestor then closes too.
                        self.out.terminal();
                    } else if deferred {
                        // The ancestor's real right subtree is next in the
                        // stream; it stays open, nothing more due at its close.
                        push_pending(&mut pending, false, false);
                        break;
                    }
                    // A pass-through ancestor closes with its child.
                },
                // Absent right child: its slot complements to a terminal,
                // due after the left subtree's output.
                (true, false) => {
                    self.out.push_tag(next_is_internal(bits, at), true);
                    push_pending(&mut pending, true, true);
                }
                // Absent left child: its slot complements to a terminal,
                // due right here (preorder: left output precedes right).
                (false, true) => {
                    self.out.push_tag(true, next_is_internal(bits, at));
                    self.out.terminal();
                    push_pending(&mut pending, false, false);
                }
                // Both children: the left kind is local (its tag is next);
                // the right kind was recorded by pass 1.
                (true, true) => {
                    let right = right_kinds
                        .pop()
                        .expect("pass 1 recorded this node's right kind");
                    self.out.push_tag(next_is_internal(bits, at), right);
                    push_pending(&mut pending, true, false);
                }
            }
        }
        debug_assert!(pending.is_empty(), "every opened ancestor closes");
        debug_assert!(
            right_kinds.is_empty(),
            "pass 2 consumes every recorded kind"
        );
        Built::Node
    }
}

/// Whether the tag at `at` is an internal node (any child present) — the
/// complement emitter's one-tag lookahead at the next node in the stream.
fn next_is_internal(bits: &BitsSlice, at: usize) -> bool {
    bits[at] || bits[at + 1]
}

/// Push one open ancestor's two-bit pending entry.
///
/// `deferred` = an action is due when the subtree now being read closes;
/// `synthetic` = that action is emitting the terminal an absent right child
/// complements to (otherwise the real right subtree follows in the stream).
fn push_pending(stack: &mut Bits, deferred: bool, synthetic: bool) {
    stack.push(deferred);
    stack.push(synthetic);
}

/// Pop one two-bit pending entry as `(deferred, synthetic)`, or `None` when
/// no ancestor is open.
fn pop_pending(stack: &mut Bits) -> Option<(bool, bool)> {
    let synthetic = stack.pop()?;
    let deferred = stack.pop().expect("pending entries are two bits");
    Some((deferred, synthetic))
}
