//! The event-tree mutation core: `merge` (the join `|` and its dual meet
//! `&`, the pointwise lattice combine in [`combine`]) and `tick` (`fill`,
//! falling back to [`grow`]). Every operation works on the fixed-width
//! working form, walking the packed id ([`idbits`](crate::idbits)) alongside
//! it where needed.
//!
//! All three are `O(n + m)` in their inputs. Output is built into fresh
//! `topo`/`base` arrays in preorder via a [`Builder`], the one type that owns
//! event normalization, so every emitting walk produces normal form. (The id
//! side's analogue is the `id_node`/`id_leaf` pair in `party::ops`, which
//! needs no working form.) Normalization is the constant "sink": the
//! children's common minimum is pushed up to the parent by an `O(1)` base
//! backpatch in [`Builder::close_node`] as soon as a node's children are
//! known. That back-reference is what the fixed-width form exists for.
//!
//! # How to read these traversals
//!
//! Every operation recurses on tree depth over single-use cursors:
//! [`EvReader`] here, [`IdReader`](crate::idbits::IdReader) on the id side.
//! A cursor is `!Copy` and `!Clone`; [`read`](EvReader::read) decodes the
//! node at the cursor and advances it in place. A recursive call therefore
//! threads a `&mut` cursor that comes back positioned just past the subtree
//! it consumed, so the right child resumes from it with no re-scan and no
//! returned end position. The call returns only its payload: an output root,
//! a cost, a verdict. `causal_cmp`, `join`/`meet`, `fill`,
//! [`EvReader::max`], the [`grow`] probe and emit, and `party::ops`'s
//! `sum`/`is_disjoint`/`split` all share this shape.
//!
//! Because the cursor is single-use:
//!
//! - **A re-scan is a compile error.** Reading the same subtree twice would
//!   require copying or rewinding a cursor, which the types forbid, so the
//!   `O(n + m)` bound holds by construction. The one operation that reads a
//!   tree twice, `grow`'s cost probe followed by its emit, builds a fresh
//!   cursor from the source for each pass (as `tick` does).
//!
//! - **A broadcast constructs, it does not copy.** Where a two-tree walk
//!   meets a leaf on one side and a node on the other, the leaf is
//!   "broadcast" to both of the node's children, per the paper's rule that a
//!   leaf `n` behaves as `(n, 0, 0)`. The leaf side hands each child a
//!   freshly built synthetic cursor, never a copy of itself:
//!   [`EvReader::Zero`] (a `Leaf(0)`, also `grow`'s virtual leaf) and, on
//!   the id side, [`IdReader::Full`](crate::idbits::IdReader::Full) (the `1`
//!   re-presented to both event children in `grow`'s full-id case).
//!
//! Recursion is guarded by [`crate::recurse`]: a shallow, near-balanced tree
//! recurses on the program stack at native speed, and a deep one grows the
//! stack onto the heap before it can overflow.
//!
//! The cursor and the [`Builder`] absorb the bookkeeping these walks share
//! (bit-offset arithmetic, the packed-versus-working representation split,
//! gamma decoding, end-position threading, stack growth, the normalizing
//! sink), so the operation bodies read as the paper's recursive equations.
//! `oracle.rs` states the same equations over plain enums, and each
//! operation names the oracle method it mirrors. Three consequences of
//! single-pass traversal over a preorder encoding cannot be absorbed, and
//! are documented where they live:
//!
//! - the leaf/full broadcast above: a single-pass two-tree walk must
//!   re-present one side against both of the other's children;
//! - `fill`'s [`deferred_leaf`](Builder::deferred_leaf) backpatch: a
//!   collapsed left child's value depends on its right sibling, but a
//!   preorder builder must emit the left first, so it emits a placeholder
//!   and resolves it once the right is built;
//! - `grow`'s two passes and its position-keyed `Route`: the cheapest
//!   inflation is found by a bottom-up cost fold, but emission is top-down
//!   preorder, so the two cannot fuse.
//!
//! Three walks do not recurse at all: the subtree-span scans.
//! [`idbits::skip_subtree`](crate::idbits::skip_subtree) (shared by
//! [`IdReader::skip`](crate::idbits::IdReader::skip) and [`EvReader::skip`])
//! and the emit-while-scanning [`Builder::copy_reader`] run a
//! pending-children counter with an `O(1)` stack; a span scan has no
//! per-node work to recurse for.

use crate::codec::BitsSlice;

use super::compare::EvReader;
use super::working::WorkingVersion;

mod builder;
mod combine;
mod fill;
mod grow;
mod max;
mod min_ticks;
mod project;

use builder::{Builder, Slot};

/// Advance `id`'s component of the event tree by one event. `fill` first (it
/// may simplify the tree using the available id); if it changes nothing,
/// `grow`. The id is the packed `enc_id` stream; `ev` is the current working
/// form. `O(n + m)`.
pub(crate) fn tick(id: &BitsSlice, ev: &WorkingVersion) -> WorkingVersion {
    // `fill` and `grow` each consume a cursor (cursors are single-use), so build
    // a fresh one per pass from the source working form rather than reusing one.
    let filled = EvReader::working(ev).fill(id);
    if filled.topo != ev.topo || filled.base != ev.base {
        filled
    } else {
        grow::grow(ev, id)
    }
}
