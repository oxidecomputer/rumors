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
//! board's [`WindowAllocator`] and reallocates by allocating the new block,
//! copying the old contents, and freeing the old block, as [`GlobalAlloc`]'s
//! provided method does. Every reallocation copies, so a buffer's copies are
//! a function of the sequence of capacities it requests. It also counts the
//! bytes it copies ([`copied`]), so the harness can tell a reading's copies
//! from the rest of its work: whether a buffer's last doubling falls inside
//! a program or just past it depends on where its length lies among the
//! capacities, which scaling does not preserve exactly.

use std::alloc::{GlobalAlloc, Layout};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::window::WindowAllocator;

/// The guest's global allocator: the window allocator, with reallocation by
/// moving.
pub struct MovingAllocator;

/// The bytes every reallocation so far has copied.
static COPIED: AtomicU64 = AtomicU64::new(0);

/// Returns the bytes every reallocation so far has copied.
pub fn copied() -> u64 {
    COPIED.load(Ordering::Relaxed)
}

// SAFETY: Every method but `realloc` delegates to `WindowAllocator`, whose
// own `GlobalAlloc` implementation upholds the trait's contract, with the
// arguments it received, which satisfy the same contract. A block `alloc` or
// `alloc_zeroed` returns came from `WindowAllocator`, so `dealloc` returns it
// to the allocator that made it. `realloc` is the trait's provided method,
// built from those methods alone, with a count added. A global
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

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The caller guarantees that `new_size` is nonzero and forms
        // a valid layout with `layout.align()`.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // SAFETY: `new_layout` has a nonzero size.
        let moved = unsafe { self.alloc(new_layout) };
        if !moved.is_null() {
            let bytes = layout.size().min(new_size);
            // SAFETY: `block` is live and valid for `layout.size()` bytes and
            // `moved` for `new_size`; `moved` is a new block, so they do not
            // overlap. `block` was allocated by this allocator with `layout`.
            unsafe {
                ptr::copy_nonoverlapping(block, moved, bytes);
                self.dealloc(block, layout);
            }
            COPIED.fetch_add(bytes as u64, Ordering::Relaxed);
        }
        moved
    }
}
