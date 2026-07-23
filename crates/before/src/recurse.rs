//! Stack-growth guard for the recursive tree traversals.
//!
//! Every traversal in this crate recurses on tree depth. A shallow,
//! near-balanced tree recurses on the program stack at native speed; before a
//! deep, unbalanced tree can approach the stack limit, [`grow`] extends the
//! stack onto the heap (via `stacker`), so deep inputs cannot overflow.
//!
//! The headroom probe is amortized: a traversal routes each recursive call
//! through the [`descend!`] macro, which probes only once every [`STRIDE`]
//! levels and recurses directly in between. `descend!` guards the *descent*,
//! not the body, so the common path is a plain recursive call that stays one
//! inlined frame; wrapping the body in a closure to pass to `maybe_grow`
//! would force a second frame and call per node. The shallow case therefore
//! pays almost nothing, and only deep inputs ever trip a heap growth.

use std::sync::atomic::{AtomicU64, Ordering};

/// Recurse this many levels between stack-headroom probes.
///
/// Must satisfy `STRIDE * max_frame_bytes < RED_ZONE`: a burst of `STRIDE`
/// frames between two probes cannot be allowed to overrun the red zone. A power
/// of two so `depth % STRIDE` lowers to a mask.
const STRIDE: usize = 64;

/// Grow the stack when fewer than this many bytes of headroom remain.
///
/// Sized from a frame-size measurement (aarch64 release): the heaviest traversal
/// frame is roughly 0.5 KiB/level — established by per-level stack-pointer deltas
/// and cross-checked against each recursive function's prologue `sub sp`. With
/// [`STRIDE`] = 64 the inter-probe burst is therefore well under 32 KiB, so
/// 256 KiB leaves roughly an 8x cushion — ample headroom for wider frames on
/// other targets (e.g. x86_64) and for arbitrary-precision `Base` arithmetic
/// temporaries in the deepest frame.
const RED_ZONE: usize = 256 * 1024;

/// Size of each heap-allocated stack segment allocated when growth triggers.
const STACK_GROWTH: usize = 1024 * 1024;

/// Number of heap stack segments grown since the last reset.
///
/// The resource envelopes need a deterministic stand-in for recursion-driven
/// stack consumption, and the segments `stacker` allocates never pass through
/// the global allocator, so no heap meter can see them. Counting here — the
/// one place a segment is created on the psm-supported native targets of
/// record (`stacker`'s fallback arm runs the callback on the current stack,
/// allocating nothing) — is the honest signal. Always compiled: the
/// bump sits on the growth path only, whose cost is already a segment
/// allocation, so production traversals pay nothing on the probe or call
/// paths. Process-global (relaxed) because the meter's test binaries run one
/// scenario per process.
static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);

/// The number of heap stack segments grown since the last
/// [`reset_segments_grown`].
///
/// Compiled only for the meter surface: the counter is always written (the
/// bump is inseparable from the growth arm), but nothing outside the meters
/// ever reads it.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn segments_grown() -> u64 {
    SEGMENTS_GROWN.load(Ordering::Relaxed)
}

/// Reset the grown-segment counter to zero.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn reset_segments_grown() {
    SEGMENTS_GROWN.store(0, Ordering::Relaxed);
}

/// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
#[inline]
pub(crate) fn should_grow(depth: usize) -> bool {
    depth.is_multiple_of(STRIDE)
}

/// Grow the stack onto the heap if under [`RED_ZONE`], then run `f`.
///
/// Open-codes `stacker::maybe_grow`'s headroom branch (same probe, same
/// growth policy: an unknown remaining stack also grows) so the growth arm —
/// and only that arm — can count the segment in [`SEGMENTS_GROWN`].
#[inline]
pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
        f()
    } else {
        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
        stacker::grow(STACK_GROWTH, f)
    }
}

/// Recurse into one child, guarding the descent without wrapping the caller's
/// body in a closure.
///
/// The common path is a direct call (the body stays one frame and inlines), and
/// only every [`STRIDE`] levels is the call routed through [`grow`]. Use at
/// each recursive call site: `descend!(depth + 1, self.rec(child_args, depth +
/// 1))`.
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
