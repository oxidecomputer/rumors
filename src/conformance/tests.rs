//! The conformance suite validated against the in-memory instantiation —
//! including adversarial-but-legal variants the contract admits.

use std::collections::VecDeque;
use std::io;

use crate::link::{Acceptor, Link, LinkParts, MemoryLink, memory, memory_with_capacity};
use crate::testing::run_to_quiescence;

/// The reference instantiation passes the whole suite under the
/// deterministic closed-world driver.
#[test]
fn memory_link_conforms() {
    run_to_quiescence(super::check(async || memory())).expect("the suite stays live");
}

/// A one-byte-buffered instantiation is legal: tiny windows slow streams
/// down but violate no clause, and full sessions still converge over it.
#[test]
fn one_byte_windows_conform() {
    run_to_quiescence(super::check(async || memory_with_capacity(1)))
        .expect("the suite stays live at one-byte windows");
}

/// An acceptor that delivers arrivals in batches of reversed order.
///
/// Legal under the contract — arrival order is whatever the transport says
/// it is, and no cross-stream ordering may be assumed — so the protocol
/// must tolerate it: the session's claim table pairs streams by label, not
/// position.
struct ReversingAcceptor<A: Acceptor> {
    inner: A,
    held: VecDeque<A::Rx>,
    /// Arrivals buffered before each reversed release.
    batch: usize,
}

impl<A: Acceptor> Acceptor for ReversingAcceptor<A> {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        if let Some(held) = self.held.pop_front() {
            return Ok(held);
        }
        // Await one arrival, then swallow whatever else is immediately
        // ready — without blocking, so a lone stream still flows — and
        // release the accumulated batch newest-first.
        let first = self.inner.accept().await?;
        self.held.push_front(first);
        for _ in 1..self.batch {
            let mut next = std::pin::pin!(self.inner.accept());
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            match std::future::Future::poll(next.as_mut(), &mut cx) {
                std::task::Poll::Ready(Ok(rx)) => self.held.push_front(rx),
                // Pending or errored: stop batching; a real error resurfaces
                // from the next accept call.
                _ => break,
            }
        }
        Ok(self.held.pop_front().expect("at least one arrival is held"))
    }
}

/// Reorder one memory end's arrivals in reversed batches.
fn reversing(
    link: MemoryLink,
    batch: usize,
) -> Link<
    tokio::io::DuplexStream,
    tokio::io::DuplexStream,
    crate::link::MemoryConnector,
    ReversingAcceptor<crate::link::MemoryAcceptor>,
> {
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: parts.connector,
        acceptor: ReversingAcceptor {
            inner: parts.acceptor,
            held: VecDeque::new(),
            batch,
        },
        epoch: parts.epoch,
    }
    .into_link()
}

/// Worst-case accept reordering is adversarial but legal: the labeled claim
/// table absorbs it, so full sessions still converge.
///
/// A batched reversing acceptor can hold an accepted stream until another
/// arrives, so the focused single-stream probes would deadlock against it by
/// construction; the sessions check — where reordering is actually possible
/// — is the meaningful clause and runs here.
#[test]
fn reordered_accepts_still_converge() {
    run_to_quiescence(async {
        let (a, b) = memory();
        super::check_sessions(reversing(a, 3), reversing(b, 3)).await;
    })
    .expect("sessions stay live under reordered accepts");
}
