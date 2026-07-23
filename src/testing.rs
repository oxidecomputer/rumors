//! Executor-agnostic test support shared across protocol and API suites.

mod transport;

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

/// The two constants the operator equations rest on, as
/// `(envelope, wire)`: bytes of session envelope one in-flight disputed
/// scope is charged, and wire bytes one disputed message costs end to
/// end.
///
/// `sync_memory_budget`'s documented closed forms —
/// `slowdown ≈ max(1, (envelope/wire) × BDP / budget)` and its inverse —
/// are denominated in these; exposed so the operator-equation suite can
/// hold the documented forms against measured sessions.
pub fn envelope_and_wire_bytes() -> (usize, usize) {
    (
        crate::tree::mirror::streaming::window::SCOPE_ENVELOPE_BYTES,
        crate::tree::mirror::streaming::window::DISPUTE_WIRE_BYTES,
    )
}

/// The largest canonical encoding among every version bound a
/// snapshot's tree holds — leaf versions and every interior ceiling and
/// floor — in bytes: the exact per-node aggregate the greeting
/// exchanges.
pub fn max_version_bytes<T>(snapshot: &crate::Snapshot<T>) -> usize {
    snapshot.tree().max_version_bytes()
}

/// The largest canonical encoding among every per-node version bound in
/// a snapshot's tree, recomputed by direct walk with no aggregate memo
/// consulted.
///
/// The session memory model prices every bound a session can hold
/// within the exchanged pair sum (`local_max + remote_max`), and
/// deletion-honoring can assemble bounds over survivor subsets neither
/// input materialized; this walk measures a reconciled tree against the
/// pre-session exchange, so tests can pin the model side of the account
/// to reality.
pub fn max_bound_bytes<T>(snapshot: &crate::Snapshot<T>) -> usize {
    snapshot.tree().max_bound_bytes()
}

/// The per-height channel capacities a session derives from its budget
/// and the two replicas' exchanged set sizes, indexed by typed height
/// (`0` = leaves, `32` = root).
///
/// Priced as the in-memory backend prices its nodes: one pointer per
/// handle at any version bound. Exposed so integration suites can
/// compute, from the same derivation sessions use, where a divergence
/// must saturate and serialize.
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
