//! A global allocator that meters each thread's heap use, for resource tests.
//!
//! Linking this crate installs a global allocator that forwards every request
//! to [`System`] and records it in a ledger kept by the calling thread.
//! [`measure`] runs a closure and returns what the calling thread's ledger
//! recorded while it ran:
//!
//! ```
//! let (stats, buffer) = alloc_meter::measure(|| Vec::<u8>::with_capacity(64));
//! assert_eq!(stats.allocations, 1);
//! assert_eq!(stats.net_bytes, 64);
//!
//! // Freeing the buffer releases what the first measurement left live, and
//! // live bytes never rise above where this measurement began.
//! let (stats, ()) = alloc_meter::measure(|| drop(buffer));
//! assert_eq!(stats.net_bytes, -64);
//! assert_eq!(stats.peak_bytes, 0);
//! ```
//!
//! A harness that accepts plain function pointers rather than a closure reads
//! the same ledger through [`reset`] and [`reading`].
//!
//! # Why each thread keeps its own ledger
//!
//! A test harness allocates on its own threads while a test runs. libtest's
//! main thread, for example, records each test thread it spawns and registers
//! to receive that test's result, and a preemption can delay either step until
//! the test is already measuring. A process-wide counter folds those
//! allocations into the test's reading, so an exact assertion passes or fails
//! by scheduling. A thread's ledger changes only when that thread calls the
//! allocator, so a reading taken on one thread counts exactly that thread's
//! requests, whatever other threads do.
//!
//! The same property is the hazard to keep in mind: work that a measured
//! closure hands to another thread (a pool, a spawned task, a runtime's
//! worker) never reaches the reading. Keep measured work on the measuring
//! thread, as a synchronous call or a future driven by `pollster::block_on`
//! does.
//!
//! # What a reading counts
//!
//! A [`Stats`] reports the requests one thread made since its reading began:
//! event counts of allocations and reallocations, the bytes they requested,
//! and the net change in live bytes along with its peak. The counting rules
//! are those of a process-wide peak counter, restricted to one thread:
//!
//! - Only successful requests count; a failed request changes nothing.
//! - A reallocation counts as allocating the new block and then freeing the
//!   old one, so the peak includes both blocks whether or not the system
//!   resizes in place. Readings therefore never depend on the system
//!   allocator's in-place behavior.
//! - Freeing a block lowers the net bytes of the thread that frees it,
//!   whichever thread allocated it. A closure that drops an input built before
//!   its reading began reads a negative net, as does one that frees a block
//!   another thread allocated.
//!
//! # Installation
//!
//! Naming any item of this crate links it, and linking it installs its
//! allocator as the binary's `#[global_allocator]`. A binary that links this
//! crate cannot declare a global allocator of its own; the compiler rejects the
//! pair. A test binary that never names the crate is not linked against it,
//! even when its package lists the crate as a dev-dependency, so sibling
//! binaries keep whatever allocator they install.
//!
//! Every thread keeps its ledger from its first allocator request, whether or
//! not anything reads it. The cost is a few thread-local additions per allocator
//! call.
//!
//! # Limits
//!
//! - Counters saturate rather than wrap, so an overflowed quantity never reads
//!   as a small one. On 64-bit targets no test can allocate enough to reach
//!   saturation. On 32-bit targets, `bytes_allocated` counts cumulative
//!   traffic and saturates after 4 GiB of it, however little is live at once.
//! - On targets without native thread-local storage, each thread's ledger has
//!   a destructor that runs as the thread exits. A request made during or
//!   after it, from another thread-local destructor, reaches no reading. The
//!   allocator cannot report the miss by panicking, because a global allocator
//!   must not unwind.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[cfg(test)]
mod tests;

/// Allocator traffic and heap use recorded on one thread over one reading.
///
/// A reading begins with every field zero, at [`measure`]'s entry or at
/// [`reset`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    /// Successful fresh allocation requests, zeroed allocations included.
    pub allocations: usize,
    /// Successful reallocation requests.
    pub reallocations: usize,
    /// Bytes requested by fresh allocations and by reallocation growth.
    ///
    /// A reallocation contributes only its growth, so reserving the final
    /// capacity at once and growing to it incrementally have the same total.
    pub bytes_allocated: usize,
    /// The highest [`net_bytes`](Self::net_bytes) reached during the reading,
    /// with both blocks of a reallocation counted; zero if net bytes never
    /// rose above their starting level.
    pub peak_bytes: usize,
    /// Bytes allocated minus bytes freed: the heap the reading's work left
    /// live, negative when it freed more than it allocated.
    pub net_bytes: isize,
}

impl Stats {
    /// The reading every thread starts with, before it has allocated anything.
    const EMPTY: Self = Self {
        allocations: 0,
        reallocations: 0,
        bytes_allocated: 0,
        peak_bytes: 0,
        net_bytes: 0,
    };

    /// Records a successful fresh allocation of `size` bytes.
    fn record_allocation(&mut self, size: usize) {
        self.allocations = self.allocations.saturating_add(1);
        self.bytes_allocated = self.bytes_allocated.saturating_add(size);
        self.grow(size);
    }

    /// Records a successful reallocation from `old_size` to `new_size` bytes.
    ///
    /// The new block is counted live before the old one is released, so the
    /// peak covers the moment a moving reallocation holds both.
    fn record_reallocation(&mut self, old_size: usize, new_size: usize) {
        self.reallocations = self.reallocations.saturating_add(1);
        self.bytes_allocated = self
            .bytes_allocated
            .saturating_add(new_size.saturating_sub(old_size));
        self.grow(new_size);
        self.shrink(old_size);
    }

    /// Records freeing a block of `size` bytes.
    fn record_free(&mut self, size: usize) {
        self.shrink(size);
    }

    /// Adds `size` live bytes and raises the peak to the new level.
    fn grow(&mut self, size: usize) {
        self.net_bytes = self.net_bytes.saturating_add_unsigned(size);
        if let Ok(level) = usize::try_from(self.net_bytes) {
            self.peak_bytes = self.peak_bytes.max(level);
        }
    }

    /// Removes `size` live bytes.
    fn shrink(&mut self, size: usize) {
        self.net_bytes = self.net_bytes.saturating_sub_unsigned(size);
    }

    /// Returns the reading of this reading's work followed by `later`'s.
    ///
    /// Counts and net bytes add. The peak is the higher of this reading's own
    /// peak and `later`'s peak, raised by the net level `later` started from.
    /// A reading's peak is never below its net bytes, so the combined peak is
    /// never below the combined net.
    fn then(self, later: Self) -> Self {
        let later_peak = self.net_bytes.saturating_add_unsigned(later.peak_bytes);
        Self {
            allocations: self.allocations.saturating_add(later.allocations),
            reallocations: self.reallocations.saturating_add(later.reallocations),
            bytes_allocated: self.bytes_allocated.saturating_add(later.bytes_allocated),
            peak_bytes: self
                .peak_bytes
                .max(usize::try_from(later_peak).unwrap_or(0)),
            net_bytes: self.net_bytes.saturating_add(later.net_bytes),
        }
    }
}

// Clippy's `missing_const_for_thread_local` can reject const-block initializers
// on targets that lower `thread_local!` through fallback TLS. The allow keeps
// `-D warnings` clean on those targets.
thread_local! {
    /// The calling thread's reading.
    ///
    /// The allocator reaches this ledger on every request, so reaching it must
    /// never call the global allocator. With native thread-local storage, a
    /// `Cell` of a `Copy` value with a const initializer is a plain
    /// thread-local static, with no lazy initialization and no destructor.
    /// Without it (illumos, for one), the standard library allocates each
    /// thread's slot on first access, and it takes that slot from `System`
    /// directly, so reaching the ledger still never enters this allocator.
    #[allow(clippy::missing_const_for_thread_local)]
    static LEDGER: Cell<Stats> = const { Cell::new(Stats::EMPTY) };
}

/// Applies `update` to the calling thread's ledger, when the ledger is
/// reachable.
///
/// Called from inside the allocator, so it must neither allocate nor panic:
/// `try_with` reports an unreachable ledger instead of panicking, and the
/// update is lost in that case (see the crate's limits).
fn record(update: impl FnOnce(&mut Stats)) {
    let _ = LEDGER.try_with(|ledger| {
        let mut stats = ledger.get();
        update(&mut stats);
        ledger.set(stats);
    });
}

/// The system allocator, recording each successful request in the calling
/// thread's ledger.
struct MeteredSystem;

/// Installs the metered allocator in every binary that links this crate.
#[global_allocator]
static ALLOCATOR: MeteredSystem = MeteredSystem;

// SAFETY: Each method forwards its request unchanged to `System` and returns
// `System`'s result unchanged, so `System` upholds every guarantee
// `GlobalAlloc` requires of the returned memory. The bookkeeping touches only
// the calling thread's `Cell` through `record`, which never re-enters this
// allocator (see `LEDGER` for why reaching the ledger cannot) and never
// unwinds, as a global allocator must not.
unsafe impl GlobalAlloc for MeteredSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller upholds `alloc`'s contract for `layout`, which
        // this forwards unchanged.
        let block = unsafe { System.alloc(layout) };
        if !block.is_null() {
            record(|stats| stats.record_allocation(layout.size()));
        }
        block
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller upholds `alloc_zeroed`'s contract for `layout`,
        // which this forwards unchanged.
        let block = unsafe { System.alloc_zeroed(layout) };
        if !block.is_null() {
            record(|stats| stats.record_allocation(layout.size()));
        }
        block
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The caller guarantees `ptr` came from this allocator with
        // `layout`; every block this allocator hands out is `System`'s, so
        // forwarding to `System` frees it as allocated.
        unsafe { System.dealloc(ptr, layout) };
        record(|stats| stats.record_free(layout.size()));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The caller upholds `realloc`'s contract for `ptr`, `layout`,
        // and `new_size`, and `ptr` is `System`'s block, as in `dealloc`.
        let block = unsafe { System.realloc(ptr, layout, new_size) };
        // A failed reallocation leaves the old block live and unchanged.
        if !block.is_null() {
            record(|stats| stats.record_reallocation(layout.size(), new_size));
        }
        block
    }
}

/// Restores the enclosing reading when a [`measure`] call ends, normally or by
/// unwinding.
struct Enclosing(Stats);

impl Drop for Enclosing {
    /// Folds the finished measurement into the reading it interrupted.
    fn drop(&mut self) {
        LEDGER.set(self.0.then(LEDGER.get()));
    }
}

/// Runs `f` and returns what the calling thread's ledger recorded while it
/// ran, with `f`'s result.
///
/// Allocator requests from other threads are excluded, so the measured work
/// must stay on this thread (see [the crate docs](crate#why-each-thread-keeps-its-own-ledger)).
///
/// Measurements nest. An enclosing measurement, or a reading begun by
/// [`reset`], still receives everything `f` records, exactly as if this call
/// were absent. Calling [`reset`] inside `f` restarts this measurement, and
/// the work before that reset reaches neither this measurement nor any
/// enclosing reading.
///
/// # Panics
///
/// A panic in `f` propagates, after the enclosing reading is restored. Like
/// any `thread_local!` access, `measure` also panics when called from a
/// thread-local destructor after this thread's ledger is gone.
pub fn measure<T>(f: impl FnOnce() -> T) -> (Stats, T) {
    let enclosing = Enclosing(LEDGER.replace(Stats::EMPTY));
    let value = f();
    let stats = LEDGER.get();
    drop(enclosing);
    (stats, value)
}

/// Begins a fresh reading on the calling thread, with every counter zero.
///
/// [`reading`] then reports the calling thread's requests since this call.
/// Together they serve harnesses that take plain function pointers; prefer
/// [`measure`] wherever a closure fits, since it also nests.
///
/// # Panics
///
/// Like any `thread_local!` access, panics when called from a thread-local
/// destructor after this thread's ledger is gone.
pub fn reset() {
    LEDGER.set(Stats::EMPTY);
}

/// Returns the calling thread's reading: its requests since the innermost
/// enclosing [`measure`] began, or, outside any measurement, since the last
/// [`reset`] or the thread's start.
///
/// # Panics
///
/// Like any `thread_local!` access, panics when called from a thread-local
/// destructor after this thread's ledger is gone.
pub fn reading() -> Stats {
    LEDGER.get()
}
