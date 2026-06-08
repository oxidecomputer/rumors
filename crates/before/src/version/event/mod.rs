//! The event-tree mutation core: `merge` (event-tree join `|` and its dual meet
//! `&`, the pointwise lattice combine in the [`combine`] submodule) and `tick`
//! (= `fill`, else `grow`, the latter in the [`grow`] submodule). Everything
//! operates on the fixed-width working form and walks the packed id
//! ([`idbits`]) alongside it where needed.
//!
//! All three are `O(n + m)` in their inputs. Output is built into fresh
//! `topo`/`base` arrays in preorder via a [`Builder`] — the one type that owns
//! event normalization, so every emitting walk stays normal-form-correct for
//! free (the id side's analogue is the `id_node`/`id_leaf` pair in `party::ops`,
//! which needs no working form to thread through). Normalization is the constant
//! "sink" — pushing the children's common minimum up to the parent — done as an
//! `O(1)` base backpatch ([`Builder::close_node`]) the moment a node's children
//! are known, exactly the back-reference the fixed-width form exists for.
//!
//! # How to read these traversals
//!
//! Every operation here recurses on tree depth over **single-use cursors**
//! ([`EvReader`], and [`IdReader`](crate::idbits::IdReader) on the id side). A cursor is
//! `!Copy`/`!Clone`: [`read`](EvReader::read) decodes the node at the cursor and
//! advances it *in place*, so a recursive call threads a `&mut` cursor that
//! comes back positioned just past the subtree it consumed — the right child
//! then resumes from it with no re-scan and no returned "end" to thread. The
//! call returns only its payload: an output root, a cost, a verdict. This is the
//! whole shape of `causal_cmp`, `join`/`meet`, `fill`, [`EvReader::max`], the [`grow`]
//! probe and emit, and `party::ops`'s `sum`/`is_disjoint`/`split` next door.
//!
//! Two things follow from the cursor being single-use:
//!
//! - **A re-scan is a compile error.** Reading the same subtree twice would mean
//!   copying or rewinding a cursor, which the types forbid; that is the `O(n+m)`
//!   guarantee made structural rather than reviewed. The lone operation that
//!   reads one tree twice — `grow`'s cost probe then emit — rebuilds a fresh
//!   cursor from the source for each pass (as `tick` does), visibly distinct
//!   from in-pass threading.
//!
//! - **A broadcast constructs, it does not copy.** Where a two-tree walk meets a
//!   leaf on one side and a node on the other, the leaf is "broadcast" to both
//!   of the node's children — the paper's rule that a leaf `n` behaves as
//!   `(n, 0, 0)`. The leaf side hands each child a *freshly built* synthetic
//!   cursor, never a copy of itself: [`EvReader::Zero`] (a `Leaf(0)`; also
//!   `grow`'s virtual leaf) and, on the id side, [`IdReader::Full`](crate::idbits::IdReader::Full)
//!   (the `1` re-presented to both event children in `grow`'s full-id case).
//!
//! Recursion is guarded by [`crate::recurse`]: a shallow, near-balanced tree
//! recurses on the program stack at native speed; a deep one grows the stack
//! onto the heap before it can overflow.
//!
//! # Incidental vs. intrinsic complexity
//!
//! The cursor and the [`Builder`] absorb what is *incidental* to these walks —
//! bit-offset arithmetic, the packed-vs-working representation split, gamma
//! decoding, end-position threading, stack growth, the normalizing sink — so the
//! operation bodies read as the paper's recursive equations (`oracle.rs` is the
//! same equations as plain enums; each op names the oracle method it mirrors).
//! What remains is *intrinsic* to single-pass, zero-copy traversal of a preorder
//! encoding, and is called out where it lives rather than hidden:
//!
//! - The **leaf/full broadcast** above: a single-pass two-tree walk cannot avoid
//!   re-presenting one side against both of the other's children.
//! - `fill`'s **[`deferred_leaf`](Builder::deferred_leaf) backpatch**: a
//!   collapsed *left* child's value depends on its right sibling, but a preorder
//!   builder must emit the left first — so it emits a placeholder and resolves it
//!   once the right is built.
//! - `grow`'s **two passes** and its position-keyed `Route`: the cheapest
//!   inflation is a bottom-up cost fold, but emission is top-down preorder, so
//!   the two cannot fuse.
//!
//! # The one genuine loop
//!
//! Three walks do *not* recurse: the subtree-span scans. [`idbits::skip_subtree`](crate::idbits::skip_subtree)
//! (shared by [`IdReader::skip`](crate::idbits::IdReader::skip) and [`EvReader::skip`]) and the
//! emit-while-scanning [`Builder::copy_reader`] run a pending-children counter
//! (`pending: i64`) with an `O(1)` stack — kept iterative because a span scan has
//! no per-node work to recurse for.

use crate::codec::BitsSlice;

use super::compare::EvReader;
use super::working::WorkingVersion;

mod builder;
mod combine;
mod fill;
mod grow;
mod max;
mod min_ticks;

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
