//! Over-the-wire reconciliation benchmark: the same divergence grid as
//! `in_memory.rs`, but reconciled through [`Known::gossip`] over a simulated
//! wire instead of an in-process [`Known::join`]. Comparing the two cell for
//! cell isolates the cost the gossip protocol adds — handshake, framing, and
//! the round-trip exchange chain — over the bare in-memory merge.
//!
//! # The wire
//!
//! Two peers gossip by driving the protocol concurrently, one on each end of a
//! duplex connection. We simulate that connection with a pair of
//! [`std::io::pipe`]s (`a → b` and `b → a`) and run peer B on a dedicated
//! worker thread while peer A runs on the Criterion thread.
//!
//! # Why a *persistent* worker
//!
//! A naive harness would spawn a thread and allocate fresh pipes per Criterion
//! iteration, folding thread spawn/join and pipe setup into every sample. The
//! gossip exchange chain is statically bounded and self-delimiting (it closes
//! on its own counter, never on EOF), so a completed session leaves both pipes
//! drained and balanced — ready to carry the next session's bytes with no
//! framing of our own. That lets us stand the pipes and one worker thread up
//! *once* in [`Wire::new`] and reuse them across every iteration of every cell,
//! so each measured round trip pays only for the gossip protocol itself.

use std::hint::black_box;
use std::io::{PipeReader, PipeWriter, pipe};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rumors::sync::Known;

// The shared grid module exposes a superset of helpers; each bench binary uses
// a subset, so the unused remainder is expected per-binary.
#[allow(dead_code)]
#[path = "support/grid.rs"]
mod grid;

use grid::sample_size_for;

/// A reusable in-memory "wire": two OS pipes plus a persistent worker thread
/// driving peer B. Each [`round_trip`](Wire::round_trip) reconciles one fresh
/// pair without re-allocating the pipes or re-spawning the thread.
struct Wire {
    /// `b → a` reader and `a → b` writer: the ends peer A (the Criterion
    /// thread) drives directly.
    a_read: PipeReader,
    a_write: PipeWriter,
    /// Hands a freshly-built peer B to the worker for the next session. Wrapped
    /// in `Option` so [`Drop`] can close the channel before joining the worker.
    work: Option<Sender<Known<()>>>,
    /// Receives the reconciled peer B back from the worker.
    done: Receiver<Known<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Wire {
    fn new() -> Self {
        let (a_to_b_r, a_to_b_w) = pipe().expect("pipe a→b");
        let (b_to_a_r, b_to_a_w) = pipe().expect("pipe b→a");
        let (work_tx, work_rx) = channel::<Known<()>>();
        let (done_tx, done_rx) = channel::<Known<()>>();

        // The worker owns peer B's pipe ends for its whole lifetime, gossiping
        // each peer handed to it over `work_rx` and returning the reconciled
        // result over `done_tx`. It exits when `work_rx` closes (on `Drop`).
        let worker = thread::spawn(move || {
            let mut read = a_to_b_r;
            let mut write = b_to_a_w;
            while let Ok(b) = work_rx.recv() {
                let b_out = b.gossip(&mut read, &mut write).expect("worker peer gossip");
                if done_tx.send(b_out).is_err() {
                    break;
                }
            }
        });

        Wire {
            a_read: b_to_a_r,
            a_write: a_to_b_w,
            work: Some(work_tx),
            done: done_rx,
            worker: Some(worker),
        }
    }

    /// Reconcile one pair over the wire: hand B to the worker, drive A here, and
    /// collect both reconciled peers once the exchange completes.
    fn round_trip(&mut self, a: Known<()>, b: Known<()>) -> (Known<()>, Known<()>) {
        self.work
            .as_ref()
            .expect("worker still running")
            .send(b)
            .expect("hand peer B to worker");
        let a_out = a
            .gossip(&mut self.a_read, &mut self.a_write)
            .expect("peer A gossip");
        let b_out = self.done.recv().expect("recv reconciled peer B");
        (a_out, b_out)
    }
}

impl Drop for Wire {
    fn drop(&mut self) {
        // Close the work channel so the worker's `recv` loop ends, then join it
        // so the pipe ends and thread tear down deterministically.
        self.work.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// `gossip` across the divergence grid.
///
/// The grid and throughput accounting mirror `in_memory.rs`'s `join` bench
/// exactly (see [`grid`]); the only difference is that each pair reconciles
/// over the wire via [`Wire::round_trip`] rather than in-process. The identical
/// corner reports latency in a separate `gossip_identical` group.
///
/// Both peers' lazy memos are warmed in untimed setup, so the timed body
/// measures the steady-state protocol rather than first-touch memoization.
fn bench_gossip(c: &mut Criterion) {
    let mut wire = Wire::new();

    // Cells that transfer something: throughput is charged against divergence.
    let mut grid_group = c.benchmark_group("gossip_grid");
    for cell in grid::cells().filter(|cell| cell.divergence() > 0) {
        grid_group.sample_size(sample_size_for(cell.build_magnitude()));
        grid_group.throughput(Throughput::Elements(cell.divergence()));
        grid_group.bench_function(BenchmarkId::from_parameter(cell.id()), |b| {
            b.iter_batched(
                || warmed(cell),
                |(left, right)| black_box(wire.round_trip(left, right)),
                BatchSize::PerIteration,
            )
        });
    }
    grid_group.finish();

    // The identical corner reconciles nothing; report latency only.
    let mut identical_group = c.benchmark_group("gossip_identical");
    for cell in grid::cells().filter(|cell| cell.divergence() == 0) {
        identical_group.sample_size(sample_size_for(cell.build_magnitude()));
        identical_group.bench_function(BenchmarkId::from_parameter(cell.id()), |b| {
            b.iter_batched(
                || warmed(cell),
                |(left, right)| black_box(wire.round_trip(left, right)),
                BatchSize::PerIteration,
            )
        });
    }
    identical_group.finish();
}

/// Build a cell's peers and warm both trees' lazy memos in untimed setup.
fn warmed(cell: grid::Cell) -> (Known<()>, Known<()>) {
    let (left, right) = grid::build(cell);
    left.warm_caches();
    right.warm_caches();
    (left, right)
}

criterion_group!(benches, bench_gossip);
criterion_main!(benches);
