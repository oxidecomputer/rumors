//! Per-thread allocation metering for resource tests.
//!
//! The Rust test harness may allocate on its coordinator thread while a test
//! runs. Process-wide counters therefore make exact allocation assertions
//! depend on scheduling. This allocator records only the thread inside
//! [`measure`], while still routing every allocation through [`System`].

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Allocation traffic observed while a measured closure runs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    /// Fresh allocation requests.
    pub allocations: usize,
    /// Reallocation requests.
    pub reallocations: usize,
    /// Bytes requested by fresh allocations and reallocation growth.
    ///
    /// This counts net growth for a reallocation, so reserving the final
    /// capacity at once and growing to it incrementally have the same total.
    pub bytes_allocated: usize,
}

thread_local! {
    /// Whether this thread is currently inside [`measure`].
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    /// Counters for the current thread's measured region.
    static STATS: Cell<Stats> = const { Cell::new(Stats {
        allocations: 0,
        reallocations: 0,
        bytes_allocated: 0,
    }) };
}

/// The system allocator with per-thread measurement hooks.
struct MeteredSystem;

/// Route allocation requests through [`System`] and count an active thread.
unsafe impl GlobalAlloc for MeteredSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(|stats| {
            stats.allocations = stats.allocations.saturating_add(1);
            stats.bytes_allocated = stats.bytes_allocated.saturating_add(layout.size());
        });
        // SAFETY: This forwards the unchanged allocation request to `System`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: This forwards the unchanged deallocation request to `System`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(|stats| {
            stats.allocations = stats.allocations.saturating_add(1);
            stats.bytes_allocated = stats.bytes_allocated.saturating_add(layout.size());
        });
        // SAFETY: This forwards the unchanged allocation request to `System`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(|stats| {
            stats.reallocations = stats.reallocations.saturating_add(1);
            stats.bytes_allocated = stats
                .bytes_allocated
                .saturating_add(new_size.saturating_sub(layout.size()));
        });
        // SAFETY: This forwards the unchanged reallocation request to `System`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Install the metered system allocator for this test binary.
#[global_allocator]
static ALLOCATOR: MeteredSystem = MeteredSystem;

/// Update this thread's counters when a measurement is active.
fn record(update: impl FnOnce(&mut Stats)) {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = STATS.try_with(|stats| {
                let mut current = stats.get();
                update(&mut current);
                stats.set(current);
            });
        }
    });
}

/// Marks one thread as inactive when its measured closure leaves scope.
struct Scope;

impl Scope {
    /// Start a fresh measurement on the current thread.
    fn enter() -> Self {
        ACTIVE.with(|active| {
            assert!(!active.replace(true), "allocation measurements cannot nest");
        });
        STATS.set(Stats::default());
        Self
    }
}

/// Stop counting allocations on the current thread.
impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.set(false);
    }
}

/// Return allocation traffic from `f` on the current thread and its result.
///
/// Allocations on other threads are deliberately excluded. The measured work
/// must therefore remain on this thread, as the resource tests' synchronous
/// joins and `pollster`-driven futures do.
pub fn measure<T>(f: impl FnOnce() -> T) -> (Stats, T) {
    let scope = Scope::enter();
    let value = f();
    let stats = STATS.get();
    drop(scope);
    (stats, value)
}
