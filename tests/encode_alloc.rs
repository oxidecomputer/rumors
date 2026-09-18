//! Allocator metering for the streaming codec's frame writer.
//!
//! A counting global allocator prices each frame write in allocation
//! events, holding the encoder to the buffers a frame genuinely needs:
//! a frame's fixed heads are a few bytes with a known maximum width, so
//! writing them must not cost a heap allocation per frame, and a supply
//! run is borrowed, not copied. A query's listing is the one variable body
//! the writer renders, and it is priced as exactly one buffer. The
//! meter counts only the thread driving the write, so test-harness work on
//! other threads cannot perturb an exact result.

#[path = "support/allocation.rs"]
mod allocation;

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;

use allocation::{Stats, measure};
use rumors::testing::{FrameShape, PreparedFrame, prepare_frame, write_prepared_frame};

/// Capacity reserved for the written frame ahead of the meter: above any
/// frame these shapes produce, so the output vector never grows inside
/// the metered region.
const OUT_CAPACITY: usize = 128 * 1024;

/// Allocation events the meter's own harness performs inside every
/// metered region: `pollster::block_on` allocates its waker once.
///
/// Calibrated by `harness_allocations`, so the frame constants below
/// state the writer's allocations alone.
const HARNESS_ALLOCATIONS: usize = 1;

/// Allocation events a body-free frame's write performs.
const BODY_FREE_ALLOCATIONS: usize = 0;

/// Allocation events a supply frame's write performs: the run is
/// borrowed and its heads are rendered on the stack.
const SUPPLY_ALLOCATIONS: usize = 0;

/// Allocation events a full-fan query's write performs: the rendered
/// listing alone.
const QUERY_ALLOCATIONS: usize = 1;

/// Reallocation events a full-fan query's write performs: none, the
/// listing being reserved at its exact rendered length.
const QUERY_REALLOCATIONS: usize = 0;

/// The run length of the metered supply frame: wide enough that the run's
/// byte-string head takes its widest form below the u32 range.
const SUPPLY_RUN_LEN: usize = 70_000;

/// Write `frame` into a pre-reserved vector under the meter, returning the
/// counter movement and the bytes written.
fn metered_write(frame: &PreparedFrame) -> (Stats, usize) {
    let mut out = Vec::with_capacity(OUT_CAPACITY);
    let (change, ()) = measure(|| pollster::block_on(write_prepared_frame(frame, &mut out)));
    assert!(
        out.len() <= OUT_CAPACITY,
        "the metered frame outgrew its reserved output"
    );
    (change, out.len())
}

/// Driving an empty future through the meter's harness performs exactly
/// `HARNESS_ALLOCATIONS` allocation events, so every frame count below
/// subtracts a calibrated constant rather than a guess.
#[test]
fn harness_allocations() {
    let (change, ()) = measure(|| pollster::block_on(async {}));
    assert_eq!(change.allocations, HARNESS_ALLOCATIONS);
    assert_eq!(change.reallocations, 0);
}

/// Allocations on another thread do not enter the current thread's count.
///
/// This recreates the test-harness interference that made the old process-wide
/// meter vary with scheduling, while one local allocation gives the assertion
/// a nonzero control.
#[test]
fn concurrent_allocations_are_excluded() {
    let start = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let remote_allocations = Arc::new(AtomicUsize::new(0));
    let worker = {
        let start = Arc::clone(&start);
        let stop = Arc::clone(&stop);
        let remote_allocations = Arc::clone(&remote_allocations);
        thread::spawn(move || {
            while !start.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
            while !stop.load(Ordering::Acquire) {
                let allocation = Vec::<u8>::with_capacity(64);
                std::hint::black_box(allocation);
                remote_allocations.fetch_add(1, Ordering::Release);
            }
        })
    };

    let (change, local_allocation) = measure(|| {
        start.store(true, Ordering::Release);
        while remote_allocations.load(Ordering::Acquire) < 128 {
            std::hint::spin_loop();
        }
        let allocation = Vec::<u8>::with_capacity(64);
        stop.store(true, Ordering::Release);
        allocation
    });
    worker.join().expect("allocation worker completes");
    std::hint::black_box(local_allocation);

    assert_eq!(change.allocations, 1);
    assert_eq!(change.reallocations, 0);
    assert_eq!(change.bytes_allocated, 64);
}

/// A body-free frame's write performs exactly `BODY_FREE_ALLOCATIONS`
/// allocation events and no reallocation.
///
/// The frame is the most frequent one a session writes; the written
/// length is asserted positive so the count cannot pass vacuously.
#[test]
fn body_free_frame_allocations() {
    let frame = prepare_frame(FrameShape::BodyFree);
    let (change, written) = metered_write(&frame);
    assert!(written > 0, "the frame was written");
    assert_eq!(
        change.allocations,
        HARNESS_ALLOCATIONS + BODY_FREE_ALLOCATIONS
    );
    assert_eq!(change.reallocations, 0);
}

/// A supply frame's write performs exactly `SUPPLY_ALLOCATIONS`
/// allocation events and no reallocation: the run travels borrowed.
#[test]
fn supply_frame_allocations() {
    let frame = prepare_frame(FrameShape::Supply {
        run_len: SUPPLY_RUN_LEN,
    });
    let (change, written) = metered_write(&frame);
    assert!(
        written > SUPPLY_RUN_LEN,
        "the run was written behind its heads"
    );
    assert_eq!(change.allocations, HARNESS_ALLOCATIONS + SUPPLY_ALLOCATIONS);
    assert_eq!(change.reallocations, 0);
}

/// A full-fan query's write performs exactly `QUERY_ALLOCATIONS`
/// allocation events and `QUERY_REALLOCATIONS` reallocations: the listing
/// is the one variable body the writer renders.
#[test]
fn full_fan_query_allocations() {
    let frame = prepare_frame(FrameShape::Query {
        children: usize::from(u8::MAX) + 1,
    });
    let (change, written) = metered_write(&frame);
    assert!(written > 0, "the frame was written");
    assert_eq!(change.allocations, HARNESS_ALLOCATIONS + QUERY_ALLOCATIONS);
    assert_eq!(change.reallocations, QUERY_REALLOCATIONS);
}
