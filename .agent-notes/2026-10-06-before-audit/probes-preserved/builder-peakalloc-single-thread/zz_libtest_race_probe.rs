//! Throwaway probe (never committed): does libtest's main thread allocate
//! after spawning the test thread, under each libtest concurrency path?
//!
//! The allocator delays every allocation made by the process's first
//! allocating thread (libtest's main thread) by two milliseconds, so the test
//! thread reaches its baseline before main's post-spawn allocations land.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Live bytes across the process.
static LIVE: AtomicUsize = AtomicUsize::new(0);
/// Allocations made by the main thread.
static MAIN_ALLOCS: AtomicUsize = AtomicUsize::new(0);
/// Sizes of the most recent main-thread allocations, as a ring.
static MAIN_SIZES: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];
/// Identity of the first allocating thread.
static MAIN_ID: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// A per-thread address used as a thread identity without allocating.
    static DUMMY: u8 = const { 0 };
}

/// Returns this thread's identity without allocating.
fn thread_id() -> usize {
    DUMMY.with(|x| x as *const u8 as usize)
}

/// Records and delays main-thread allocations.
fn on_alloc(size: usize) {
    let id = thread_id();
    let main = match MAIN_ID.compare_exchange(0, id, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => true,
        Err(existing) => existing == id,
    };
    if main {
        let n = MAIN_ALLOCS.fetch_add(1, Ordering::SeqCst);
        MAIN_SIZES[n % 16].store(size, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Counting, main-delaying allocator.
struct Probe;

unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        on_alloc(layout.size());
        LIVE.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        on_alloc(new_size);
        LIVE.fetch_add(new_size, Ordering::SeqCst);
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static HEAP: Probe = Probe;

/// Main-thread allocations must not land inside a test's measurement window.
#[test]
fn main_thread_stays_out_of_the_window() {
    let main_before = MAIN_ALLOCS.load(Ordering::SeqCst);
    let live_before = LIVE.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(500));
    let live_after = LIVE.load(Ordering::SeqCst);
    let main_after = MAIN_ALLOCS.load(Ordering::SeqCst);
    let sizes: Vec<usize> = (main_before..main_after)
        .map(|n| MAIN_SIZES[n % 16].load(Ordering::SeqCst))
        .collect();
    eprintln!(
        "PROBE threads_env={:?} args={:?} main_allocs_in_window={} sizes={:?} live_delta={}",
        std::env::var("RUST_TEST_THREADS"),
        std::env::args().skip(1).collect::<Vec<_>>(),
        main_after - main_before,
        sizes,
        live_after as isize - live_before as isize,
    );
    assert_eq!(main_after, main_before, "libtest's main thread allocated inside the window");
}
