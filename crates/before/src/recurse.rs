//! Stack-growth guard for the experimental recursive traversals.
//!
//! The recursive variants (behind the `internals` feature) recurse on tree
//! depth, where the iterative stack machines walk an explicit heap stack. A
//! shallow, near-balanced tree recurses on the program stack at native speed;
//! before a deep, unbalanced tree can approach the stack limit, [`guard`] grows
//! the stack onto the heap (via `stacker`), preserving the overflow-safety that
//! the explicit stacks were written for.
//!
//! The headroom probe is *amortized*: a caller invokes [`guard`] only once every
//! [`STRIDE`] levels, recursing directly in between. So the common shallow case
//! pays essentially nothing (one probe per `STRIDE` frames), and only genuinely
//! deep inputs ever trip a heap growth.

/// Recurse this many levels between stack-headroom probes.
///
/// Must satisfy `STRIDE * max_frame_bytes < RED_ZONE`: a burst of `STRIDE`
/// frames between two probes cannot be allowed to overrun the red zone. The
/// traversal frames here are well under 1 KiB, so `64 * 1 KiB = 64 KiB` stays
/// comfortably inside [`RED_ZONE`]. A power of two so `depth % STRIDE` lowers to
/// a mask.
pub(crate) const STRIDE: usize = 64;

/// Grow the stack when fewer than this many bytes of headroom remain.
///
/// Provisional: chosen as a conservative fraction of a typical 1–8 MiB thread
/// stack, large enough to absorb a `STRIDE`-frame burst with wide margin. To be
/// tuned against an empirical per-traversal frame-size measurement before the
/// recursive variants are promoted off the `internals` feature.
const RED_ZONE: usize = 128 * 1024;

/// Size of each heap-allocated stack segment allocated when growth triggers.
const STACK_GROWTH: usize = 1024 * 1024;

/// Run `f`, first ensuring at least [`RED_ZONE`] bytes of stack headroom and
/// growing the stack onto the heap if less remains. Call once every [`STRIDE`]
/// recursion levels; recurse directly in between.
#[inline]
pub(crate) fn guard<R>(f: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE, STACK_GROWTH, f)
}
