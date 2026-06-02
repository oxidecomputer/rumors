//! The event-tree mutation core: `merge` (event-tree join) and `tick` (=
//! `fill`, else `grow`, the latter in the [`grow`] submodule). Everything
//! operates on the fixed-width working form and walks the packed id
//! ([`idbits`]) alongside it where needed.
//!
//! All three are iterative and `O(n + m)` in their inputs. Output is built into
//! fresh `topo`/`base` arrays in preorder via a [`Builder`] — the one type that
//! owns event normalization, so every emitting walk stays normal-form-correct
//! for free (the id side's analogue is the `id_node`/`id_leaf` pair in
//! `party::ops`, which needs no working form to thread through). Normalization
//! is the constant "sink" — pushing the children's common minimum up to the
//! parent — done as an `O(1)` base backpatch ([`Builder::close_node`]) the
//! moment a node's children are known, exactly the back-reference the
//! fixed-width form exists for.
//!
//! # The thread register
//!
//! Every two-tree machine here ([`EvView::join`], [`EvView::fill`], and the
//! [`grow`] submodule's probe and emit) — and `EvView::causal_cmp` in
//! [`super::compare`] next door — drives a single iterative DFS off an explicit
//! job stack, threading right-child positions instead of re-scanning to find
//! them. They all speak the same protocol, the **thread register**:
//!
//! - A mutable `ret`, a small named struct, holds the just-finished subtree's
//!   report: the position just past it in each input tree, plus a per-walk
//!   payload — the output root it produced (`join`'s register, [`Built`] shared
//!   by `fill` and the grow emit), the subtree's cost (`grow`'s `Probed`), or
//!   nothing (`compare`'s `Ends`).
//!
//! - Every `Eval` arm finishes by *writing* `ret` (a completed leaf, or a
//!   `Close`/ `Combine` arm folding two children).
//!
//! - Every deferred-sibling frame (`Right`/`Close`/`Combine`) *reads* `ret` to
//!   resume: a right child starts where its left sibling's subtree ended, so it
//!   never re-scans.
//!
//! LIFO push order is what makes the bare register sound: a node pushes its
//! `Close` frame, then its `Right` frame, then its left `Eval`, so by the time
//! a frame pops and reads `ret`, the most recent write is exactly the sibling
//! subtree it is waiting on. (`sum` in `party::ops` plays the same role with an
//! output builder plus its `Summed` register, since it must also finalize
//! normalized output nodes.)
//!
//! # Traversal-idiom taxonomy
//!
//! Every tree walk in the crate is iterative (no recursion on depth). There are
//! three distinct shapes; each is internally consistent, and knowing which one
//! a given machine uses tells you how to read it:
//!
//! | Idiom | Shape | Where |
//! |-------|-------|-------|
//! | Job-stack + `ret` thread register (`Eval`/`Right`/`Close`/`Combine`) | threaded DFS / fold | `causal_cmp`, `join`, `fill`, [`EvView::max`], the [`grow`] probe/emit, `party::ops::sum`, `party::ops::is_disjoint` |
//! | `NeedLeft`/`NeedRight` frame stack | single-tree build/print | `codec` parse/write of ids and event trees, `party::ops::split`'s pass 1 |
//! | Pending-children counter (`pending: i64`) | subtree-span scan | [`idbits::skip_subtree`](crate::idbits::skip_subtree) (shared by `idbits::skip` and `EvView::skip`), [`Builder::copy`] |
//!
//! The first idiom dominates (a single-output fold like `max` drops the `Close`
//! arm and threads only the end position); the others appear where the goal is
//! narrower (a one-tree print, a pure span scan) and the full
//! `Eval`/`Right`/`Close` protocol would be overkill.

use crate::codec::BitsSlice;

use super::compare::EvView;
use super::working::WorkingVersion;

mod builder;
mod fill;
mod grow;
mod join;
mod max;

use builder::{Builder, Built};

/// Sentinel event position: a virtual `Leaf(0)`, used by [`grow`] when it
/// expands an event leaf into a node to follow the id deeper. Never a real bit
/// offset.
pub(super) const VIRTUAL: usize = usize::MAX;

/// Advance `id`'s component of the event tree by one event. `fill` first (it
/// may simplify the tree using the available id); if it changes nothing,
/// `grow`. The id is the packed `enc_id` stream; `ev` is the current working
/// form. `O(n + m)`.
pub(crate) fn tick(id: &BitsSlice, ev: &WorkingVersion) -> WorkingVersion {
    let view = EvView::Working(ev);
    let filled = view.fill(id);
    if filled.topo != ev.topo || filled.base != ev.base {
        filled
    } else {
        view.grow(id)
    }
}
