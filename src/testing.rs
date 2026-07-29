//! Executor-agnostic test support shared across protocol and API suites.

mod memnet;
mod transport;

pub use memnet::{MemoryDial, MemoryListen, MemoryName, MemoryNet};
pub use transport::{
    AdversarialAcceptor, AdversarialConnector, AdversarialRead, AdversarialWrite,
    FaultUnit as IoFaultUnit, InjectedIo, IoFault, IoPlan, IoReport, IoReportHandle,
    Operation as IoOperation, ReorderingAcceptor, Side as IoSide, reorder_accepts, wrap_io,
    wrap_link,
};

pub use crate::tree::mirror::streaming::remote::LinkCapture;

/// Render captured V2 link traffic grouped by labeled logical streams.
pub fn render_v2_capture(a: &LinkCapture, b: &LinkCapture) -> String {
    crate::tree::mirror::streaming::remote::render_v2_capture(a, b)
}

/// Drain a [`Snapshot`](crate::Snapshot)'s message stream into a `Vec`,
/// synchronously.
///
/// The in-memory backend's stream items are all immediately ready and its
/// error is uninhabited, so suites can compare snapshot contents without
/// carrying an executor. Order is the stream's own (unspecified).
pub fn collect<T: Send + Sync>(
    snapshot: &crate::Snapshot<T>,
) -> Vec<(crate::Key, crate::Version, std::sync::Arc<T>)> {
    use futures::TryStreamExt;
    pollster::block_on(snapshot.iter().try_collect()).expect("the in-memory backend is infallible")
}

/// Chainable form of [`collect`]: drain a snapshot's stream into an owned
/// iterator, synchronously, at the end of a method chain.
pub trait SnapshotCollect<T> {
    /// Drain into `Vec` and hand back its iterator; see [`collect`].
    fn collected(&self) -> std::vec::IntoIter<(crate::Key, crate::Version, std::sync::Arc<T>)>;
}

impl<T: Send + Sync> SnapshotCollect<T> for crate::Snapshot<T> {
    fn collected(&self) -> std::vec::IntoIter<(crate::Key, crate::Version, std::sync::Arc<T>)> {
        collect(self).into_iter()
    }
}

/// Drain the messages of a [`Snapshot`](crate::Snapshot) whose versions
/// fall in the causal `range` into a `Vec`, synchronously.
///
/// The range-filtered sibling of [`collect`], with
/// [`Snapshot::range`](crate::Snapshot::range)'s bound semantics.
pub fn collect_range<T: Send + Sync, R: std::ops::RangeBounds<crate::Version>>(
    snapshot: &crate::Snapshot<T>,
    range: R,
) -> Vec<(crate::Key, crate::Version, std::sync::Arc<T>)> {
    use futures::TryStreamExt;
    pollster::block_on(snapshot.range(range).try_collect())
        .expect("the in-memory backend is infallible")
}

/// Commit a [`Batch`](crate::Batch), synchronously.
///
/// The in-memory backend's commit future suspends only at the commit
/// lock, so suites without a concurrent committer can commit inline.
pub fn commit<T: Send + Sync>(batch: crate::Batch<'_, T>) {
    pollster::block_on(batch.commit()).expect("the in-memory backend is infallible")
}

/// A snapshot of the crate-wide census of live tree-node handles.
#[derive(Clone, Copy, Debug)]
pub struct NodeCensus {
    /// Handles alive at the snapshot.
    pub live: usize,
    /// The most handles ever concurrently alive since the last
    /// [`node_census_reset`].
    pub peak: usize,
}

/// Read the census of live tree-node handles.
///
/// Every constructed or cloned node handle counts one and every drop
/// releases one, so `peak` is exact concurrent residency: the measurable
/// shadow of the memory bound
/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) derives.
/// The counters are process-global; tests that assert on them must own
/// the process (one test per process under nextest).
pub fn node_census() -> NodeCensus {
    let (live, peak) = crate::tree::typed::untyped::census::read();
    NodeCensus { live, peak }
}

/// Restart the census high-water mark from the current live count.
pub fn node_census_reset() {
    crate::tree::typed::untyped::census::reset_peak();
}

/// The two wire-cost constants of the dispute walk, as `(envelope, wire)`.
///
/// The envelope is the session-envelope bytes one in-flight disputed scope
/// is charged; the wire figure is the end-to-end wire bytes of one disputed
/// message at the design record size. The wire figure is an anchor, not an
/// input — nothing derives from it; it is exposed so the calibration suite
/// (`tests/dispute_wire.rs`) can hold it against deterministic byte counts.
pub fn envelope_and_wire_bytes() -> (usize, usize) {
    (
        crate::tree::mirror::streaming::window::SCOPE_ENVELOPE_BYTES,
        crate::tree::mirror::streaming::window::DISPUTE_WIRE_BYTES,
    )
}

/// End-to-end wire bytes of one disputed message beyond its record's
/// encoded payload: the calibrated intercept of the affine per-message
/// wire law.
///
/// Exposed so the calibration suite (`tests/dispute_wire.rs`) can hold
/// the constant the closed form quotes against deterministic byte
/// counts at several record sizes.
pub fn dispute_overhead_bytes() -> usize {
    crate::tree::mirror::streaming::window::DISPUTE_OVERHEAD_BYTES
}

/// Worst-case bytes one session's decode fans keep resident, under the
/// in-memory backend's pricing.
///
/// This is the flat pre-charge that comes off a budget before the
/// dispute-scope solve. The operator suite denominates its measured cells
/// in a budget's dispute share, so it needs the pre-charge to add back.
pub fn supply_decode_envelope_bytes() -> usize {
    crate::tree::mirror::streaming::window::SUPPLY_DECODE_ENVELOPE_BYTES
}

/// Renders the sync-budget trade-off table that
/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget)'s docs
/// include, from the real window derivation.
///
/// Each budget row's window comes from the same solve sessions run at
/// handshake time ([`window_capacities`]'s derivation), evaluated at
/// the design session, and each record-size column applies the
/// measured wave form `slowdown = max(1, BDP_messages / K)` at the
/// spec bandwidth-delay product. Pure deterministic arithmetic:
/// `examples/window_tradeoff.rs` prints it (`just window-tradeoff`
/// moves the output into place atomically), and the window suite
/// byte-compares the committed file against this rendering, so the
/// table cannot drift from the derivation it tabulates.
pub fn window_tradeoff_table() -> String {
    use std::fmt::Write;

    use crate::tree::mirror::streaming::window::{
        DEFAULT_SYNC_MEMORY_BUDGET, DESIGN_RECORD_BYTES, DISPUTE_OVERHEAD_BYTES, SPEC_BDP_BYTES,
        Window,
    };

    /// The design session's corpus scale per side: the spec BDP in
    /// design-size records, the scale `SCOPE_ENVELOPE_BYTES` is pinned
    /// at.
    const DESIGN_SESSION_MESSAGES: u64 = 62_500;

    /// The widest window the solve grants `budget` at a symmetric
    /// `corpus`-message session under the in-memory backend's pricing.
    fn solve_window(corpus: u64, budget: usize) -> u64 {
        let window = Window::from_budget(
            corpus,
            corpus,
            0,
            0,
            budget,
            crate::tree::mirror::streaming::Local::node_bytes,
        );
        (0..=32)
            .map(|height| window.capacity(height) as u64)
            .max()
            .expect("thirty-three heights")
    }

    /// The budget rows, smallest to largest; the default is labeled at
    /// render time so the table cannot go stale against it.
    const BUDGETS: &[(&str, usize)] = &[
        ("256 KiB", 256 << 10),
        ("1 MiB", 1 << 20),
        ("4 MiB", 4 << 20),
        ("16 MiB", 16 << 20),
        ("64 MiB", 64 << 20),
        ("256 MiB", 256 << 20),
        ("512 MiB", 512 << 20),
        ("2 GiB", 2 << 30),
    ];

    /// The mean-encoded-record-size columns: the minimal `u64` record,
    /// a mid value, the design record, and a fat value.
    const RECORD_SIZES: &[(usize, &str)] = &[
        (8, "m = 8 (u64)"),
        (64, "m = 64"),
        (DESIGN_RECORD_BYTES, "m = 172 (design record)"),
        (1024, "m = 1024"),
    ];

    let mut table = String::new();
    let _ = writeln!(
        table,
        "<!-- Generated by `just window-tradeoff`; do not edit. -->"
    );

    let mut header = String::from("| budget | window (scopes) |");
    let mut rule = String::from("|---|---|");
    for (_, label) in RECORD_SIZES {
        let _ = write!(header, " {label} |");
        rule.push_str("---|");
    }
    let _ = writeln!(table, "{header}");
    let _ = writeln!(table, "{rule}");

    for &(label, budget) in BUDGETS {
        let default = if budget == DEFAULT_SYNC_MEMORY_BUDGET {
            " (default)"
        } else {
            ""
        };
        let window = solve_window(DESIGN_SESSION_MESSAGES, budget);
        let _ = write!(table, "| {label}{default} | {window} |");
        for &(m, _) in RECORD_SIZES {
            let bdp_messages = SPEC_BDP_BYTES as f64 / (DISPUTE_OVERHEAD_BYTES + m) as f64;
            let slowdown = (bdp_messages / window as f64).max(1.0);
            let _ = write!(table, " {slowdown:.1}× |");
        }
        let _ = writeln!(table);
    }
    table
}

/// The largest canonical version-bound encoding in a snapshot's tree, in
/// bytes.
///
/// Covers every bound the tree holds (leaf versions and every interior
/// ceiling and floor); the result is the exact per-node aggregate the
/// greeting exchanges.
pub fn max_version_bytes<T: Send + Sync + 'static>(snapshot: &crate::Snapshot<T>) -> usize {
    snapshot.tree().max_version_bytes()
}

/// The largest canonical per-node version-bound encoding in a snapshot's
/// tree, recomputed by direct walk with no aggregate memo consulted.
///
/// The session memory model prices every bound a session can hold
/// within the exchanged pair sum (`local_max + remote_max`), and
/// deletion-honoring can assemble bounds over survivor subsets neither
/// input materialized; this walk measures a reconciled tree against the
/// pre-session exchange, so tests can pin the model side of the account
/// to reality.
pub fn max_bound_bytes<T: Send + Sync + 'static>(snapshot: &crate::Snapshot<T>) -> usize {
    snapshot.tree().max_bound_bytes()
}

/// The per-height channel capacities a session derives from its budget
/// and the two replicas' exchanged set sizes.
///
/// Indexed by typed height (`0` = leaves, `32` = root) and priced as the
/// in-memory backend prices its nodes: one pointer per handle at any
/// version bound. Exposed so integration suites can compute, from the same
/// derivation sessions use, where a divergence must saturate and serialize.
pub fn window_capacities(local_len: u64, remote_len: u64, budget_bytes: usize) -> Vec<usize> {
    let window = crate::tree::mirror::streaming::window::Window::from_budget(
        local_len,
        remote_len,
        0,
        0,
        budget_bytes,
        crate::tree::mirror::streaming::Local::node_bytes,
    );
    (0..=32).map(|height| window.capacity(height)).collect()
}

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

/// Why polling stopped before a closed in-memory future completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quiescence {
    /// The future returned `Pending` without arranging another poll.
    Stalled,
    /// The future kept self-waking beyond the runaway guard.
    PollBudget,
}

struct WakeFlag(AtomicBool);

impl Wake for WakeFlag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

/// Poll a closed, in-memory future until it completes or becomes quiescent.
///
/// Every legitimate suspension must arrange another wake. A `Pending` poll
/// without one is therefore a deterministic deadlock witness rather than a
/// wall-clock guess. Futures waiting on external events do not satisfy this
/// closed-world premise and should use their real liveness mechanism instead.
/// Tokio's cooperative budget is disabled around the subject so that invoking
/// this detector from within a Tokio task cannot turn a scheduler yield into a
/// false deadlock report.
pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {
    const MAX_POLLS: usize = 1_000_000;

    let wake = Arc::new(WakeFlag(AtomicBool::new(true)));
    let waker = Waker::from(wake.clone());
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(tokio::task::coop::unconstrained(future));

    for _ in 0..MAX_POLLS {
        wake.0.store(false, Ordering::Release);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return Ok(output),
            Poll::Pending if !wake.0.swap(false, Ordering::AcqRel) => {
                return Err(Quiescence::Stalled);
            }
            Poll::Pending => {}
        }
    }
    Err(Quiescence::PollBudget)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A self-wake is progress while a permanently parked future is stalled.
    #[test]
    fn observes_wake_contract() {
        let mut first = true;
        let self_waking = std::future::poll_fn(move |cx| {
            if std::mem::take(&mut first) {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(7)
            }
        });
        assert_eq!(run_to_quiescence(self_waking), Ok(7));
        assert_eq!(
            run_to_quiescence(std::future::pending::<()>()),
            Err(Quiescence::Stalled),
        );
    }

    /// An inherited Tokio task budget cannot masquerade as protocol quiescence.
    #[tokio::test(flavor = "current_thread")]
    async fn ignores_tokio_cooperative_yields() {
        const ITEMS: usize = 256;

        let (send, mut receive) = tokio::sync::mpsc::channel(ITEMS);
        for item in 0..ITEMS {
            send.try_send(item).expect("channel has room");
        }

        let received = run_to_quiescence(async move {
            let mut items = Vec::with_capacity(ITEMS);
            while let Some(item) = receive.recv().await {
                items.push(item);
                if items.len() == ITEMS {
                    break;
                }
            }
            items
        });

        assert_eq!(received, Ok((0..ITEMS).collect()));
    }
}
