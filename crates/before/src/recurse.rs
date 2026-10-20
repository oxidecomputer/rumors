//! Stack-growth guard for depth-recursive tree traversals.
//!
//! A test traversal that must survive deep inputs routes each recursive call
//! through here. A shallow, near-balanced tree recurses on the program stack at
//! native speed; before a deep, unbalanced tree can approach the stack limit,
//! `grow` extends the stack onto the heap (via `stacker`), so deep inputs
//! cannot overflow.
//!
//! Every library traversal is iterative: depth lives on explicit heap stacks,
//! never the call stack, so the guard machinery compiles only for the test
//! surface. A recursive test helper whose descents all pass through `descend!`
//! survives depths the program stack alone could not carry.
//!
//! The recursive tree oracle's own operations (`testing::oracles::tree`) are
//! not guarded. They transcribe the paper's recursions on native frames by
//! design, and each harness that drives them bounds its input depth; the
//! oracle's "Operating envelope" section gives the reasoning. Deep-input tests
//! therefore exercise the production traversals' iterative implementations
//! directly.
//!
//! The headroom probe is amortized: a traversal routes each recursive call
//! through the `descend!` macro, which probes only once every `STRIDE` levels
//! and recurses directly in between. `descend!` guards the *descent*, not the
//! body, so the common path is a plain recursive call that stays one inlined
//! frame; wrapping the body in a closure to pass to `maybe_grow` would force a
//! second frame and call per node. The shallow case therefore pays almost
//! nothing, and only deep inputs ever trip a heap growth.

/// Recurse this many levels between stack-headroom probes.
///
/// Must satisfy `STRIDE * max_frame_bytes < RED_ZONE`: a burst of `STRIDE`
/// frames between two probes cannot be allowed to overrun the red zone. A power
/// of two so `depth % STRIDE` lowers to a mask.
#[cfg(test)]
const STRIDE: usize = 64;

/// Grow the stack when fewer than this many bytes of headroom remain.
///
/// Sized from a frame-size measurement (aarch64 release): the heaviest
/// traversal frame is roughly 0.5 KiB/level — established by per-level
/// stack-pointer deltas and cross-checked against each recursive function's
/// prologue `sub sp`. With [`STRIDE`] = 64 the inter-probe burst is therefore
/// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom
/// for wider frames on other targets (e.g. x86_64) and for arbitrary-precision
/// `BigUint` arithmetic temporaries in the deepest frame.
#[cfg(test)]
const RED_ZONE: usize = 256 * 1024;

/// Size of each heap-allocated stack segment allocated when growth triggers.
#[cfg(test)]
const STACK_GROWTH: usize = 1024 * 1024;

/// Whether to probe stack headroom on entering `depth` (every [`STRIDE`] levels).
#[cfg(test)]
#[inline]
pub(crate) fn should_grow(depth: usize) -> bool {
    depth.is_multiple_of(STRIDE)
}

/// Grow the stack onto the heap if under [`RED_ZONE`], then run `f`.
///
/// Uses the same policy as `stacker::maybe_grow`: an unknown amount of
/// remaining stack is treated conservatively and grows too.
#[cfg(test)]
#[inline]
pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
        f()
    } else {
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
#[cfg(test)]
macro_rules! descend {
    ($depth:expr, $call:expr) => {
        if $crate::recurse::should_grow($depth) {
            $crate::recurse::grow(|| $call)
        } else {
            $call
        }
    };
}
#[cfg(test)]
pub(crate) use descend;
