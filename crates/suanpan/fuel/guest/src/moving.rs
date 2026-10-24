//! Makes every reallocation move its block, so the copies a buffer's growth
//! makes depend only on the sizes requested.
//!
//! `dlmalloc` grows a block in place when the memory after it is free, and
//! moves it, copying its contents, otherwise. Whether the memory after a
//! block is free depends on where earlier blocks landed, which the sizes of
//! a program's operands change in no proportional way: the same program can
//! grow a buffer in place at one scale and copy it at the next. `suanpan`'s
//! third cost rule allows those copies, less than twice a buffer's final
//! length over its growth, but a reading that includes them at one scale and
//! omits them at another grows faster than the program's sizes, and the
//! harness compares readings across scales.
//!
//! [`MovingAllocator`] therefore serves allocations and frees from the fuel
//! board's [`WindowAllocator`] and leaves `realloc` to [`GlobalAlloc`]'s
//! provided method, which allocates the new block, copies the old contents,
//! and frees the old block. Every reallocation copies, so a buffer's copies
//! are a function of the sequence of capacities it requests, and that
//! sequence scales with the program.

use std::alloc::{GlobalAlloc, Layout};

use crate::window::WindowAllocator;

/// The guest's global allocator: the window allocator, with reallocation by
/// moving.
pub struct MovingAllocator;

// SAFETY: Every method delegates to `WindowAllocator`, whose own `GlobalAlloc`
// implementation upholds the trait's contract, with the arguments it
// received, which satisfy the same contract. A block `alloc` or
// `alloc_zeroed` returns came from `WindowAllocator`, so `dealloc` returns it
// to the allocator that made it. The provided `realloc` is built from these
// three methods alone, so it upholds the contract as they do. A global
// allocator can only be installed through this trait; it lives in this
// tool-side guest alone, which no production crate links.
unsafe impl GlobalAlloc for MovingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller guarantees `layout` has a nonzero size.
        unsafe { WindowAllocator.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller guarantees `layout` has a nonzero size.
        unsafe { WindowAllocator.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // SAFETY: The caller guarantees `block` is live and was allocated by
        // this allocator, and so by `WindowAllocator`, with `layout`.
        unsafe { WindowAllocator.dealloc(block, layout) }
    }
}
