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
/// frames between two probes cannot be allowed to overrun the red zone. A power
/// of two so `depth % STRIDE` lowers to a mask.
const STRIDE: usize = 64;

/// Grow the stack when fewer than this many bytes of headroom remain.
///
/// Sized from a frame-size measurement (aarch64 release): the heaviest
/// traversal frame is ~0.5 KiB/level — established by per-level stack-pointer
/// deltas and cross-checked against each recursive function's prologue `sub sp`
/// (summed over its `rec` shell and body closure). With [`STRIDE`] = 64 the
/// inter-probe burst is therefore ~32 KiB, so 256 KiB leaves roughly an 8x
/// cushion — ample headroom for wider frames on other targets (e.g. x86_64) and
/// for arbitrary-precision `Base` arithmetic temporaries in the deepest frame.
const RED_ZONE: usize = 256 * 1024;

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

/// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
#[inline]
pub(crate) fn should_grow(depth: usize) -> bool {
    depth.is_multiple_of(STRIDE)
}

/// Grow the stack onto the heap if under [`RED_ZONE`], then run `f`.
#[inline]
pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE, STACK_GROWTH, f)
}

/// Recurse into one child, guarding the descent without wrapping the caller's
/// body in a closure: the common path is a direct call (the body stays one
/// frame and inlines), and only every [`STRIDE`] levels is the call routed
/// through [`grow`]. Use at each recursive call site:
/// `descend!(depth + 1, self.rec(child_args, depth + 1))`.
macro_rules! descend {
    ($depth:expr, $call:expr) => {
        if $crate::recurse::should_grow($depth) {
            $crate::recurse::grow(|| $call)
        } else {
            $call
        }
    };
}
pub(crate) use descend;
