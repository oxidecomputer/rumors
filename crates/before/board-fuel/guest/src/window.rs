//! Gives each measured window a fresh heap, so that a fuel reading counts the
//! allocator's work for the body's own allocations and none of the heap state
//! that preparation left behind.
//!
//! The board reads fuel immediately before a cell's body runs and again once
//! the body has produced its result. The span between the two reads is the
//! *measured window*. What an allocator call costs depends on the heap it runs
//! against: which free lists hold blocks, whether a block can grow in place,
//! where the top of the heap lies. Preparation builds the cell's operands in
//! the same instance just before the window opens, so with a single heap the
//! body's allocations would run against whatever preparation left, and the
//! reading would move whenever preparation's allocations change, even though
//! the operation under test does not.
//!
//! [`WindowAllocator`] therefore keeps two kinds of `dlmalloc` heap. The
//! *ordinary heap* serves every allocation outside a window. A *window heap*
//! is created when a window opens ([`enter`]) and retires when it closes
//! ([`exit`]). It is fresh and *primed*: before the window opens, one
//! allocation and free set it up, growing its first memory and initializing
//! its bins, so that one-time setup stays out of the reading. Inside a
//! window:
//!
//! - every allocation comes from the window heap;
//! - a block belongs to the window heap if it lies at or above the window's
//!   *base*, the linear-memory size in pages when the window opened;
//! - freeing a window-heap block returns it to the window heap;
//! - freeing an older block does nothing: the block stays allocated;
//! - reallocating an older block allocates the new block from the window
//!   heap, copies the contents, and leaves the old block allocated.
//!
//! Ownership by address is sound because nothing but the window heap grows
//! linear memory while a window is open: the ordinary heap receives no calls
//! until the window closes. The window heap draws all its memory by growing
//! linear memory after the base was recorded, so all of its blocks lie at or
//! above the base, and every older block lies below it.
//!
//! One premise rests on `dlmalloc` 0.2.14's wasm backend rather than on this
//! module. The first request from any heap that reaches the backend sets a
//! flag that every heap shares, and the backend hands that request the
//! linker's spare heap region, which lies below every base, if the region can
//! hold it; once the flag is set, the backend only grows linear memory.
//! [`enter`] makes the ordinary heap allocate before every window opens, and
//! an empty heap's first allocation always reaches the backend, so the flag
//! is set before any window heap asks for memory. A window-heap block below
//! the base would otherwise be freed into the wrong heap, so every
//! window-heap allocation checks its address and panics if the premise fails.
//!
//! The fuel spent inside a window is thus the window heap's work for the
//! body's own allocations; a copy of each older block the body reallocates,
//! whose cost grows with the block's size; and a fixed cost on every
//! allocator call, a few dozen instructions for the lock, the routing, and the
//! ownership or base check. It depends only on the sequence of allocator calls
//! the body makes.
//!
//! When a window closes, its heap retires: the pages it grew are recorded, and
//! a later free of one of its blocks is dropped, because a retired heap never
//! allocates again and returning a block to it would serve nothing. The
//! retired memory stays allocated until the instance ends, and so does every
//! older block the body freed or reallocated, such as an operand it consumed:
//! the ordinary heap never reuses it. Neither costs the reading anything, and
//! the memory comes back when the instance ends. The harness runs one cell per
//! instance, and each of the cell's samples opens one window, so the memory an
//! instance holds but no longer uses is at most what that cell's windows grew,
//! plus the older blocks their bodies freed or reallocated.

use std::alloc::{GlobalAlloc, Layout};
use std::arch::wasm32;
use std::ops::Range;
use std::ptr;

use dlmalloc::Dlmalloc;
use spin::mutex::{SpinMutex, SpinMutexGuard};

/// The size of a wasm page in bytes, the unit in which linear memory grows.
const PAGE_BYTES: usize = 64 * 1024;

/// The most pages a wasm32 linear memory holds: its 4 GiB address space in
/// 64 KiB pages.
///
/// Every address therefore lies on a page whose index is below this, and
/// `memory_size` never returns more, so [`RetiredPages`] covers every page.
const MAX_PAGES: usize = 1 << 16;

/// The guest's global allocator: the ordinary heap outside measured windows,
/// and a fresh heap inside each one (the module docs give the rules).
pub struct WindowAllocator;

/// Every heap and the bookkeeping that routes calls among them.
static HEAPS: SpinMutex<Heaps> = SpinMutex::new(Heaps {
    ordinary: OrdinaryHeap {
        heap: Dlmalloc::new(),
        retired: RetiredPages([0; MAX_PAGES / 64]),
    },
    window: None,
});

/// Locks the heaps.
///
/// The lock exists because a `static` must be `Sync`. The guest is
/// single-threaded, so the lock is free whenever a call takes it, unless that
/// call re-entered the allocator while an outer call held it, which no method
/// here does. One attempt therefore suffices, and a failed attempt panics
/// instead of spinning forever.
///
/// The lock is `spin`'s rather than `std`'s `Mutex` because fuel counts it on
/// every allocator call inside a measured window. On this target `std`'s
/// `Mutex` checks for re-entry with an assertion whose operand it keeps on
/// the shadow stack, so every allocator call would also set up a stack frame.
///
/// # Panics
///
/// Panics if the heaps are already locked. The message is a literal, so the
/// panic allocates nothing.
#[inline]
fn lock() -> SpinMutexGuard<'static, Heaps> {
    let Some(heaps) = HEAPS.try_lock() else {
        panic!("the window allocator is never re-entered");
    };
    heaps
}

/// The allocator's state.
struct Heaps {
    /// The ordinary heap, and the pages of the retired window heaps.
    ordinary: OrdinaryHeap,
    /// The open window, if one is open.
    window: Option<Window>,
}

/// The heap that serves every call outside a measured window.
struct OrdinaryHeap {
    /// The ordinary heap itself.
    heap: Dlmalloc,
    /// The pages that retired window heaps grew. Every block on them belongs
    /// to a retired heap; every other block outside the open window belongs
    /// to the ordinary heap.
    retired: RetiredPages,
}

/// An open measured window.
struct Window {
    /// The window's heap, primed and otherwise empty when the window opened.
    heap: Dlmalloc,
    /// The linear-memory size in pages when the window opened. The window
    /// heap's blocks lie on this page or above it; every older block lies
    /// below it.
    base: usize,
}

/// A set of wasm pages, one bit per page.
struct RetiredPages([u64; MAX_PAGES / 64]);

impl RetiredPages {
    /// Adds `pages` to the set.
    fn insert(&mut self, pages: Range<usize>) {
        for page in pages {
            self.0[page / 64] |= 1 << (page % 64);
        }
    }

    /// Returns whether the page holding `address` is in the set.
    fn holds(&self, address: usize) -> bool {
        let page = page_of(address);
        self.0[page / 64] & (1 << (page % 64)) != 0
    }
}

/// Returns the index of the wasm page that holds `address`.
#[inline]
fn page_of(address: usize) -> usize {
    address / PAGE_BYTES
}

/// Returns the current size of linear memory, in pages.
fn memory_pages() -> usize {
    wasm32::memory_size::<0>()
}

impl Window {
    /// Makes one allocation from the empty window heap and frees it.
    ///
    /// A heap's first allocation also sets it up: it grows linear memory for
    /// the heap's first segment and initializes its bins. Priming does that
    /// before the window opens, so the body starts on a primed, fresh heap
    /// whose state depends on nothing outside this function.
    fn prime(&mut self) {
        // SAFETY: One byte at alignment one is a layout of nonzero size.
        let block = unsafe { self.heap.malloc(1, 1) };
        let block = self.checked(block);
        if !block.is_null() {
            // SAFETY: `block` is live, and the window heap allocated it with
            // size one and alignment one.
            unsafe { self.heap.free(block, 1, 1) };
        }
    }

    /// Returns whether the block at `address` belongs to this window's heap.
    #[inline]
    fn owns(&self, address: usize) -> bool {
        page_of(address) >= self.base
    }

    /// Returns `block`, a pointer the window heap returned, after checking
    /// that it lies at or above the base, as the module docs' ownership
    /// argument requires.
    ///
    /// # Panics
    ///
    /// Panics if `block` is a block below the base. The message is a literal,
    /// so the panic allocates nothing while the heaps are locked.
    #[inline]
    fn checked(&self, block: *mut u8) -> *mut u8 {
        assert!(
            block.is_null() || self.owns(block.addr()),
            "a window heap allocated below its window's base"
        );
        block
    }

    /// Reallocates `block` to `new_size` bytes inside the window.
    ///
    /// # Safety
    ///
    /// `block`, `layout`, and `new_size` satisfy the contract of
    /// [`GlobalAlloc::realloc`].
    unsafe fn reallocate(&mut self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if self.owns(block.addr()) {
            // SAFETY: The caller upholds `realloc`'s contract, and `block` is
            // the window heap's own block, allocated with `layout`.
            let moved = unsafe {
                self.heap
                    .realloc(block, layout.size(), layout.align(), new_size)
            };
            return self.checked(moved);
        }
        // SAFETY: The caller upholds `realloc`'s contract for `block`,
        // `layout`, and `new_size`. The older block is left allocated.
        let moved = unsafe { reallocate_by_copy(&mut self.heap, block, layout, new_size) };
        self.checked(moved)
    }
}

impl OrdinaryHeap {
    /// Makes one allocation from the ordinary heap and frees it.
    ///
    /// An empty heap's first allocation always reaches `dlmalloc`'s backend,
    /// so afterwards the ordinary heap has asked the backend for memory before
    /// any window heap does (the module docs' premise).
    fn prime(&mut self) {
        // SAFETY: One byte at alignment one is a layout of nonzero size.
        let block = unsafe { self.heap.malloc(1, 1) };
        if !block.is_null() {
            // SAFETY: `block` is live, and the ordinary heap allocated it with
            // size one and alignment one.
            unsafe { self.heap.free(block, 1, 1) };
        }
    }

    /// Frees `block` outside any window: into the ordinary heap, or nowhere if
    /// it belongs to a retired window heap.
    ///
    /// # Safety
    ///
    /// `block` is a live block that this allocator returned with `layout`, and
    /// no window heap that is still open owns it.
    unsafe fn release(&mut self, block: *mut u8, layout: Layout) {
        if self.retired.holds(block.addr()) {
            return;
        }
        // SAFETY: `block` is live, allocated with `layout`, and lies on no
        // page a window heap grew, so the ordinary heap allocated it.
        unsafe { self.heap.free(block, layout.size(), layout.align()) };
    }

    /// Reallocates `block` to `new_size` bytes outside any window.
    ///
    /// A block of a retired window heap moves into the ordinary heap.
    ///
    /// # Safety
    ///
    /// As for [`release`](Self::release), and `new_size` satisfies the
    /// contract of [`GlobalAlloc::realloc`].
    unsafe fn reallocate(&mut self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if !self.retired.holds(block.addr()) {
            // SAFETY: As in `release`, the ordinary heap allocated `block`
            // with `layout`, and the caller upholds `realloc`'s contract.
            return unsafe {
                self.heap
                    .realloc(block, layout.size(), layout.align(), new_size)
            };
        }
        // SAFETY: The caller upholds `realloc`'s contract for `block`,
        // `layout`, and `new_size`. The retired block is left allocated.
        unsafe { reallocate_by_copy(&mut self.heap, block, layout, new_size) }
    }
}

/// Allocates a block of `new_size` bytes from `heap`, aligned as `layout`
/// requires, and copies `block`'s contents into it, leaving `block` itself
/// allocated; returns the new block, or null if the allocation failed.
///
/// # Safety
///
/// `block` is live and valid for `layout.size()` bytes, and `new_size` is
/// nonzero and forms a valid layout with `layout.align()`, as
/// [`GlobalAlloc::realloc`] requires.
unsafe fn reallocate_by_copy(
    heap: &mut Dlmalloc,
    block: *mut u8,
    layout: Layout,
    new_size: usize,
) -> *mut u8 {
    // SAFETY: `new_size` is nonzero and forms a valid layout with
    // `layout.align()`, by the caller's contract.
    let moved = unsafe { heap.malloc(new_size, layout.align()) };
    if !moved.is_null() {
        // SAFETY: `block` is valid for `layout.size()` bytes and `moved` for
        // `new_size`; `moved` is a new block, distinct from the live `block`,
        // so they do not overlap.
        unsafe { ptr::copy_nonoverlapping(block, moved, layout.size().min(new_size)) };
    }
    moved
}

impl Heaps {
    /// Allocates a block for `layout`, zeroed if `zeroed`, from the open
    /// window's heap if a window is open and from the ordinary heap otherwise.
    ///
    /// # Safety
    ///
    /// `layout` has a nonzero size, as [`GlobalAlloc::alloc`] requires.
    //
    // Inlined into `alloc` and `alloc_zeroed`, so that `zeroed` folds to a
    // constant and no call is added to every allocation: fuel counts these
    // instructions inside every measured window.
    #[inline(always)]
    unsafe fn allocate(&mut self, layout: Layout, zeroed: bool) -> *mut u8 {
        let (size, align) = (layout.size(), layout.align());
        match &mut self.window {
            Some(window) => {
                // SAFETY: The caller guarantees `layout` has a nonzero size.
                let block = unsafe {
                    if zeroed {
                        window.heap.calloc(size, align)
                    } else {
                        window.heap.malloc(size, align)
                    }
                };
                window.checked(block)
            }
            // SAFETY: The caller guarantees `layout` has a nonzero size.
            None => unsafe {
                if zeroed {
                    self.ordinary.heap.calloc(size, align)
                } else {
                    self.ordinary.heap.malloc(size, align)
                }
            },
        }
    }
}

// SAFETY: `GlobalAlloc` requires every block it returns to be valid for its
// layout and aligned to it, and every block it frees to go back to the
// allocator that returned it. Both heaps are `dlmalloc` instances, which
// return blocks valid for the requested size and alignment, and each block is
// freed into the heap that allocated it, or not at all, by the module docs'
// ownership argument. Every method takes the heaps' lock, makes no call that
// allocates while holding it, and never unwinds, since the guest aborts on
// panic. A global allocator can only be installed through this trait; it
// lives in this tool-side guest alone, which no production crate links.
unsafe impl GlobalAlloc for WindowAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller guarantees `layout` has a nonzero size.
        unsafe { lock().allocate(layout, false) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller guarantees `layout` has a nonzero size.
        unsafe { lock().allocate(layout, true) }
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        let mut guard = lock();
        let heaps = &mut *guard;
        match &mut heaps.window {
            Some(window) => {
                // An older block stays allocated (the module docs say why).
                if window.owns(block.addr()) {
                    // SAFETY: The caller guarantees `block` is live and was
                    // allocated with `layout`, and it lies at or above the
                    // base, so the window heap allocated it.
                    unsafe { window.heap.free(block, layout.size(), layout.align()) }
                }
            }
            // SAFETY: The caller guarantees `block` is live and was allocated
            // with `layout`, and no window is open.
            None => unsafe { heaps.ordinary.release(block, layout) },
        }
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let mut guard = lock();
        let heaps = &mut *guard;
        match &mut heaps.window {
            // SAFETY: The caller upholds `realloc`'s contract.
            Some(window) => unsafe { window.reallocate(block, layout, new_size) },
            // SAFETY: The caller upholds `realloc`'s contract, and no window
            // is open.
            None => unsafe { heaps.ordinary.reallocate(block, layout, new_size) },
        }
    }
}

/// Opens a measured window: every allocation from here until [`exit`] comes
/// from a fresh, primed heap.
///
/// The guest's `enter_window` hook calls this just before the fuel origin is
/// read, so the work done here stays out of the reading.
///
/// # Panics
///
/// Panics if a window is already open.
pub fn enter() {
    let mut heaps = lock();
    assert!(heaps.window.is_none(), "measured windows do not nest");
    heaps.ordinary.prime();
    // The base is read after the ordinary heap's priming, which can grow
    // linear memory.
    let mut window = Window {
        heap: Dlmalloc::new(),
        base: memory_pages(),
    };
    window.prime();
    heaps.window = Some(window);
}

/// Closes the measured window and retires its heap.
///
/// The guest's `exit_window` hook calls this just after the final fuel read,
/// so retiring the heap stays out of the reading.
///
/// # Panics
///
/// Panics unless a window is open.
pub fn exit() {
    let mut guard = lock();
    let heaps = &mut *guard;
    // Dropping the window heap leaves its memory allocated: `Dlmalloc` has no
    // destructor.
    let Some(Window { base, .. }) = heaps.window.take() else {
        panic!("a measured window closes only after it opens");
    };
    heaps.ordinary.retired.insert(base..memory_pages());
}
