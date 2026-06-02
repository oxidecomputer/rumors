//! Stack-growth guard for the recursive tree traversals.
//!
//! Every traversal in this crate recurses on tree depth. A shallow,
//! near-balanced tree recurses on the program stack at native speed; before a
//! deep, unbalanced tree can approach the stack limit, [`guarded`] grows the
//! stack onto the heap (via `stacker`), so deep inputs cannot overflow.
//!
//! The headroom probe is *amortized*: a traversal routes each recursive entry
//! through [`guarded`], which actually probes only once every [`STRIDE`] levels
//! and recurses directly in between. So the common shallow case pays essentially
//! nothing (one probe per `STRIDE` frames), and only genuinely deep inputs ever
//! trip a heap growth.

/// Recurse this many levels between stack-headroom probes.
///
/// Must satisfy `STRIDE * max_frame_bytes < RED_ZONE`: a burst of `STRIDE`
/// frames between two probes cannot be allowed to overrun the red zone. The
/// traversal frames here are well under 1 KiB, so `64 * 1 KiB = 64 KiB` stays
/// comfortably inside [`RED_ZONE`]. A power of two so `depth % STRIDE` lowers to
/// a mask.
const STRIDE: usize = 64;

/// Grow the stack when fewer than this many bytes of headroom remain.
///
/// Provisional: chosen as a conservative fraction of a typical 1–8 MiB thread
/// stack, large enough to absorb a `STRIDE`-frame burst with wide margin. To be
/// tuned against an empirical per-traversal frame-size measurement.
const RED_ZONE: usize = 128 * 1024;

/// Size of each heap-allocated stack segment allocated when growth triggers.
const STACK_GROWTH: usize = 1024 * 1024;

/// Enter one recursion level at `depth`, ensuring stack headroom first.
///
/// Every [`STRIDE`] levels this probes the remaining stack and, if under
/// [`RED_ZONE`], grows it onto the heap before running `f`; in between it just
/// runs `f` directly. Route every recursive call through this: pass the call's
/// own depth and a closure that does the work for that node.
#[inline]
pub(crate) fn guarded<R>(depth: usize, f: impl FnOnce() -> R) -> R {
    if depth.is_multiple_of(STRIDE) {
        stacker::maybe_grow(RED_ZONE, STACK_GROWTH, f)
    } else {
        f()
    }
}
