//! The event-tree mutation core: `merge` (event-tree join) and `tick` (=
//! `fill`, else `grow`, the latter in the [`grow`] submodule). Everything
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
//! # Recursion and threading
//!
//! Every two-tree machine here ([`EvReader::join`], [`EvReader::fill`], and the
//! [`grow`] submodule's probe and emit) — and `EvReader::causal_cmp` in
//! [`super::compare`] next door — recurses on tree depth, returning a small
//! per-subtree *report* and threading right-child positions instead of
//! re-scanning to find them:
//!
//! - Each recursive call returns its just-finished subtree's report: a reader
//!   just past it in each input tree, plus a per-walk payload — the output root
//!   it produced (`fill`'s `Filled`, the grow emit's `Grown`, `join`'s
//!   `Joined`), the subtree's cost (`grow`'s `Probed`), or just the end readers
//!   (`compare`). A right child is evaluated starting where its left sibling's
//!   report says it ended, so it never re-scans.
//!
//! - Recursion is guarded by [`crate::recurse`]: a shallow, near-balanced tree
//!   recurses on the program stack at native speed, while a deep one grows the
//!   stack onto the heap before it can overflow. (`sum` in `party::ops` plays
//!   the same role with an output builder plus its `Summed` report, since it
//!   must also finalize normalized output nodes.)
//!
//! # Traversal-idiom taxonomy
//!
//! Every tree walk in the crate recurses on depth, guarded against overflow by
//! [`crate::recurse`]. There are three distinct shapes; each is internally
//! consistent, and knowing which one a given machine uses tells you how to read
//! it:
//!
//! | Idiom | Shape | Where |
//! |-------|-------|-------|
//! | Recursive walk returning a per-subtree report | threaded DFS / fold / build | `causal_cmp`, `join`, `fill`, [`EvReader::max`], the [`grow`] probe/emit, `party::ops::sum`, `party::ops::compare`, `party::ops::is_disjoint` |
//! | Recursive single-tree build/print | one-tree parse/print | `codec` parse/write of ids and event trees, `party::ops::split`'s pass 1 |
//! | Pending-children counter (`pending: i64`) | subtree-span scan | [`idbits::skip_subtree`](crate::idbits::skip_subtree) (shared by `IdReader::skip` and `EvReader::skip`), [`Builder::copy_reader`] |
//!
//! The first idiom dominates (a single-output fold like `max` returns just the
//! subtree maximum and its end; a two-tree build threads two end positions); the
//! third is the one genuine loop — a pure span scan with an `O(1)` stack — kept
//! iterative because it has no per-node work to recurse for.

use crate::codec::BitsSlice;

use super::compare::EvReader;
use super::working::WorkingVersion;

mod builder;
mod fill;
mod grow;
mod join;
mod max;

use builder::Builder;

/// Advance `id`'s component of the event tree by one event. `fill` first (it
/// may simplify the tree using the available id); if it changes nothing,
/// `grow`. The id is the packed `enc_id` stream; `ev` is the current working
/// form. `O(n + m)`.
pub(crate) fn tick(id: &BitsSlice, ev: &WorkingVersion) -> WorkingVersion {
    let view = EvReader::working(ev);
    let filled = view.fill(id);
    if filled.topo != ev.topo || filled.base != ev.base {
        filled
    } else {
        view.grow(id)
    }
}
