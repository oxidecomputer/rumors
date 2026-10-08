//! Tests of the per-thread ledger against the counting rules in the crate docs.
//!
//! The lib test binary links this crate, so the metered allocator is installed
//! here. The property tests issue raw allocator requests of exact sizes, so the
//! model knows every request the allocator sees, and they compare each reading
//! with a recomputation from the list of live block sizes.

use std::alloc::{self, Layout};
use std::panic;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

use super::{Stats, measure, reading, reset};

/// A heap block requested from the global allocator with an exact size and
/// an alignment of one.
struct Block {
    /// The block's start, as the allocator returned it.
    ptr: NonNull<u8>,
    /// The layout the block currently has.
    layout: Layout,
}

impl Block {
    /// Allocates `size` bytes, zeroed when `zeroed` is set.
    ///
    /// # Panics
    ///
    /// Panics if `size` is zero, which the allocator's contract forbids.
    fn new(size: usize, zeroed: bool) -> Self {
        assert!(size > 0, "a block has at least one byte");
        let layout =
            Layout::from_size_align(size, 1).expect("a byte-aligned size is a valid layout");
        // SAFETY: `layout` has a nonzero size, as checked above.
        let ptr = unsafe {
            if zeroed {
                alloc::alloc_zeroed(layout)
            } else {
                alloc::alloc(layout)
            }
        };
        let ptr = NonNull::new(ptr).unwrap_or_else(|| alloc::handle_alloc_error(layout));
        Self { ptr, layout }
    }

    /// Reallocates the block to `size` bytes.
    ///
    /// # Panics
    ///
    /// Panics if `size` is zero, which the allocator's contract forbids.
    fn resize(&mut self, size: usize) {
        assert!(size > 0, "a block has at least one byte");
        let layout =
            Layout::from_size_align(size, 1).expect("a byte-aligned size is a valid layout");
        // SAFETY: `self.ptr` is live and was allocated with `self.layout`;
        // `size` is nonzero and, rounded up to an alignment of one, cannot
        // overflow `isize`, since `layout` was constructed from it.
        let ptr = unsafe { alloc::realloc(self.ptr.as_ptr(), self.layout, size) };
        self.ptr = NonNull::new(ptr).unwrap_or_else(|| alloc::handle_alloc_error(layout));
        self.layout = layout;
    }
}

impl Drop for Block {
    /// Frees the block.
    fn drop(&mut self) {
        // SAFETY: `self.ptr` is live and was allocated with `self.layout`, and
        // dropping the block is its last use.
        unsafe { alloc::dealloc(self.ptr.as_ptr(), self.layout) };
    }
}

/// One allocator request, aimed at a live block by position where it needs
/// one.
#[derive(Clone, Copy, Debug)]
enum Op {
    /// Allocate a fresh block of `size` bytes, through `alloc_zeroed` when
    /// `zeroed` is set.
    Allocate { size: usize, zeroed: bool },
    /// Reallocate a live block to `size` bytes; skipped when nothing is live.
    Resize { block: Index, size: usize },
    /// Free a live block; skipped when nothing is live.
    Free { block: Index },
}

/// Generates requests of a few kilobytes at most, so a history crosses both
/// growing and shrinking reallocations.
fn arb_op() -> impl Strategy<Value = Op> {
    let size = 1..=4_096usize;
    prop_oneof![
        (size.clone(), any::<bool>()).prop_map(|(size, zeroed)| Op::Allocate { size, zeroed }),
        (any::<Index>(), size).prop_map(|(block, size)| Op::Resize { block, size }),
        any::<Index>().prop_map(|block| Op::Free { block }),
    ]
}

/// Issues `ops` against the allocator, keeping live blocks in `blocks`.
///
/// `blocks` must have spare capacity for every allocation in `ops`, so that
/// no request but the ops' own reaches the allocator.
fn apply(blocks: &mut Vec<Block>, ops: &[Op]) {
    for op in ops {
        match *op {
            Op::Allocate { size, zeroed } => {
                assert!(blocks.len() < blocks.capacity(), "blocks never reallocates");
                blocks.push(Block::new(size, zeroed));
            }
            Op::Resize { block, size } => {
                if !blocks.is_empty() {
                    let index = block.index(blocks.len());
                    blocks[index].resize(size);
                }
            }
            Op::Free { block } => {
                if !blocks.is_empty() {
                    let index = block.index(blocks.len());
                    drop(blocks.swap_remove(index));
                }
            }
        }
    }
}

/// Returns the reading the counting rules predict for `ops`, starting from
/// live blocks of the sizes in `live`, and updates `live` to match.
///
/// The model recomputes every level from the whole live list instead of
/// tracking a running total, and measures each level against the total live
/// when the reading began. A reallocation's level is taken with the new block
/// added and the old one still present.
fn model(live: &mut Vec<usize>, ops: &[Op]) -> Stats {
    /// Returns the live total of `live` relative to `start`.
    fn level(live: &[usize], start: usize) -> isize {
        live.iter().sum::<usize>() as isize - start as isize
    }

    let start = live.iter().sum::<usize>();
    let mut expected = Stats::default();
    let mut peak = 0isize;
    for op in ops {
        match *op {
            Op::Allocate { size, .. } => {
                expected.allocations += 1;
                expected.bytes_allocated += size;
                live.push(size);
                peak = peak.max(level(live, start));
            }
            Op::Resize { block, size } => {
                if live.is_empty() {
                    continue;
                }
                let index = block.index(live.len());
                let old = live[index];
                expected.reallocations += 1;
                expected.bytes_allocated += size.saturating_sub(old);
                peak = peak.max(level(live, start) + size as isize);
                live[index] = size;
            }
            Op::Free { block } => {
                if !live.is_empty() {
                    let index = block.index(live.len());
                    live.swap_remove(index);
                }
            }
        }
    }
    expected.peak_bytes = peak as usize;
    expected.net_bytes = level(live, start);
    expected
}

/// Splits `ops` at `at`, chosen over every boundary including both ends.
fn split(ops: &[Op], at: Index) -> (&[Op], &[Op]) {
    ops.split_at(at.index(ops.len() + 1))
}

proptest! {
    /// Every reading equals the counting rules' prediction, read both
    /// mid-measurement through `reading` and at the measurement's end.
    ///
    /// The history mixes plain and zeroed allocations, growing and shrinking
    /// reallocations, and frees, so each allocator entry point and each rule
    /// (growth-only byte totals, the reallocation's two-block peak, frees
    /// lowering the net) decides some reading.
    #[test]
    fn readings_follow_the_counting_rules(ops in vec(arb_op(), 0..48), at in any::<Index>()) {
        let (first, second) = split(&ops, at);
        let mut blocks = Vec::with_capacity(ops.len());
        let (whole, partial) = measure(|| {
            apply(&mut blocks, first);
            let partial = reading();
            apply(&mut blocks, second);
            partial
        });
        drop(blocks);

        let mut live = Vec::new();
        prop_assert_eq!(partial, model(&mut live, first));
        let mut live = Vec::new();
        prop_assert_eq!(whole, model(&mut live, &ops));
    }

    /// A nested measurement leaves its enclosing reading exactly as if the
    /// nested call were absent, and reads its own work relative to the live
    /// level it began at.
    ///
    /// The nested part may resize or free blocks the enclosing part
    /// allocated, so its own reading covers negative nets and peaks that
    /// start above zero in the enclosing one.
    #[test]
    fn nested_measurements_are_transparent(
        ops in vec(arb_op(), 0..48),
        first_at in any::<Index>(),
        second_at in any::<Index>(),
    ) {
        let (before, rest) = split(&ops, first_at);
        let (nested, after) = split(rest, second_at);
        let mut blocks = Vec::with_capacity(ops.len());
        let (outer, inner) = measure(|| {
            apply(&mut blocks, before);
            let (inner, ()) = measure(|| apply(&mut blocks, nested));
            apply(&mut blocks, after);
            inner
        });
        drop(blocks);

        let mut live = Vec::new();
        prop_assert_eq!(outer, model(&mut live, &ops));
        let mut live = Vec::new();
        model(&mut live, before);
        prop_assert_eq!(inner, model(&mut live, nested));
    }
}

/// Bytes the measuring thread allocates inside the foreign-thread window.
const LOCAL_BYTES: usize = 64;
/// Bytes of each block the foreign thread allocates inside that window.
const FOREIGN_BYTES: usize = 4_096;

/// Blocks until `step` reaches `value`, without allocating.
fn wait_for(step: &AtomicUsize, value: usize) {
    while step.load(Ordering::Acquire) != value {
        thread::yield_now();
    }
}

/// Another thread's allocations, reallocations, and frees inside a
/// measurement change none of its fields.
///
/// A handshake over one atomic forces the foreign thread's requests to
/// land between the measurement's start and end, which is the interleaving a
/// preempted test-harness thread produces only by chance. The foreign thread
/// frees one block inside the window and keeps another past it, so a
/// process-wide counter would misread the peak, the net, and every count.
#[test]
fn foreign_thread_requests_are_excluded() {
    let step = Arc::new(AtomicUsize::new(0));
    let foreign = {
        let step = Arc::clone(&step);
        thread::spawn(move || {
            wait_for(&step, 1);
            drop(Vec::<u8>::with_capacity(FOREIGN_BYTES));
            let mut kept = Vec::<u8>::with_capacity(1);
            kept.reserve_exact(FOREIGN_BYTES);
            step.store(2, Ordering::Release);
            wait_for(&step, 3);
            drop(kept);
        })
    };

    let (stats, local) = measure(|| {
        let local = Vec::<u8>::with_capacity(LOCAL_BYTES);
        step.store(1, Ordering::Release);
        wait_for(&step, 2);
        local
    });
    step.store(3, Ordering::Release);
    foreign.join().expect("the foreign thread completes");
    drop(local);

    assert_eq!(
        stats,
        Stats {
            allocations: 1,
            reallocations: 0,
            bytes_allocated: LOCAL_BYTES,
            peak_bytes: LOCAL_BYTES,
            net_bytes: LOCAL_BYTES as isize,
        }
    );
}

/// `reset` begins a reading that counts only later requests, and a block
/// freed after the reset lowers its net even though it was allocated before.
#[test]
fn reset_begins_a_fresh_reading() {
    let earlier = Block::new(16, false);
    reset();
    let later = Block::new(64, false);
    drop(earlier);
    let stats = reading();
    drop(later);

    assert_eq!(
        stats,
        Stats {
            allocations: 1,
            reallocations: 0,
            bytes_allocated: 64,
            peak_bytes: 64,
            net_bytes: 48,
        }
    );
}

/// `reset` inside a measurement restarts it, and the work before the reset
/// reaches neither that measurement nor the one enclosing it.
#[test]
fn reset_inside_a_measurement_restarts_it() {
    let (outer, (inner, blocks)) = measure(|| {
        let enclosing = Block::new(8, false);
        let (inner, nested) = measure(|| {
            let lost = Block::new(32, false);
            reset();
            (lost, Block::new(64, false))
        });
        (inner, (enclosing, nested))
    });
    drop(blocks);

    let after_reset = Stats {
        allocations: 1,
        reallocations: 0,
        bytes_allocated: 64,
        peak_bytes: 64,
        net_bytes: 64,
    };
    assert_eq!(inner, after_reset);
    assert_eq!(
        outer,
        Stats {
            allocations: 2,
            reallocations: 0,
            bytes_allocated: 72,
            peak_bytes: 72,
            net_bytes: 72,
        }
    );
}

/// A refused allocation or reallocation changes no field of the reading, and a
/// refused reallocation leaves the old block counted live.
///
/// A request for `isize::MAX` bytes, the largest size a byte-aligned layout
/// admits, exceeds the address space of every 64-bit target, so the system
/// allocator refuses both requests. A 32-bit target could grant it, so the
/// test runs only on 64-bit targets.
#[cfg(target_pointer_width = "64")]
#[test]
fn refused_requests_change_nothing() {
    let block = Block::new(16, false);
    let huge = isize::MAX.unsigned_abs();
    let layout = Layout::from_size_align(huge, 1)
        .expect("isize::MAX bytes at alignment one is a valid layout");
    let (stats, ()) = measure(|| {
        // SAFETY: `layout` has a nonzero size.
        let refused = unsafe { alloc::alloc(layout) };
        assert!(
            refused.is_null(),
            "no 64-bit system allocator grants isize::MAX bytes"
        );
        // SAFETY: `block.ptr` is live and was allocated with `block.layout`;
        // `huge` is nonzero and, at an alignment of one, does not overflow
        // `isize`.
        let refused = unsafe { alloc::realloc(block.ptr.as_ptr(), block.layout, huge) };
        assert!(
            refused.is_null(),
            "no 64-bit system allocator grants isize::MAX bytes"
        );
    });
    drop(block);

    assert_eq!(stats, Stats::default());
}

/// A panic inside a measurement restores the enclosing reading before it
/// propagates, so the enclosing reading keeps what it recorded before the
/// nested call.
///
/// `resume_unwind` skips the panic hook, so the unwind writes no captured
/// output; the unwinder's own exception object is allocated and freed on this
/// thread and leaves the net unchanged.
#[test]
fn a_panicking_measurement_restores_the_enclosing_reading() {
    let (outer, (enclosing, unwound)) = measure(|| {
        let enclosing = Block::new(8, false);
        let unwound = panic::catch_unwind(|| {
            measure(|| {
                let _scratch = Block::new(16, false);
                panic::resume_unwind(Box::new(()));
            })
        });
        (enclosing, unwound)
    });
    drop(enclosing);

    assert!(unwound.is_err(), "the nested measurement unwinds");
    assert_eq!(outer.net_bytes, 8);
    assert!(
        outer.allocations >= 2,
        "both blocks reach the enclosing reading"
    );
}
