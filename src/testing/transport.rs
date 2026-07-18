//! Stackable, deterministic adversity for test transports across the crate.

use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Which endpoint owns an observed transport operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The first proxy endpoint in the test harness.
    Left,
    /// The second proxy endpoint in the test harness.
    Right,
}

/// One asynchronous transport surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    /// Reading peer-produced bytes.
    Read,
    /// Writing locally-produced bytes.
    Write,
    /// Flushing a complete frame.
    Flush,
    /// Opening an outgoing data stream.
    Connect,
    /// Accepting an incoming data stream.
    Accept,
}

/// Unit in which a transport failure threshold is measured.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultUnit {
    /// Completed operations of the selected kind.
    Operations,
    /// Successfully transferred bytes of the selected kind.
    Bytes,
}

/// One transport failure injected after a precise successful prefix.
///
/// A read fault fires in place of the first *payload-bearing* read beyond
/// the prefix: end-of-stream probes pass through untouched. This keeps
/// "would the fault fire?" a function of the clean run's successful-read
/// counts alone — the link world checks for end-of-stream after every
/// stream's end control, so attempts-after-last-success are structural and
/// must not trip an operations-counted fault.
///
/// Stream-supply faults follow the same discipline, counted in operations
/// only. A connect fault fires in place of the call (a healthy supply's
/// connects always succeed, so calls and successes coincide); an accept
/// fault fires in place of the next *successful* accept, so the final
/// forever-pending accept a session parks on against an honest peer is
/// structural and cannot trip the threshold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoFault {
    /// Surface which fails.
    pub operation: Operation,
    /// Successful prefix admitted before failure.
    pub after: usize,
    /// Whether `after` counts operations or bytes.
    pub unit: FaultUnit,
}

/// Typed source retained inside the injected [`io::Error`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("injected {operation:?} failure on {side:?} after {after} {unit:?}")]
pub struct InjectedIo {
    /// Endpoint which failed.
    pub side: Side,
    /// Surface which failed.
    pub operation: Operation,
    /// Configured successful prefix.
    pub after: usize,
    /// Unit of the configured prefix.
    pub unit: FaultUnit,
}

/// I/O adversity applied independently at one endpoint.
#[derive(Clone, Debug)]
pub struct IoPlan {
    /// Most bytes one successful read may reveal.
    pub read_chunk: usize,
    /// Most bytes one successful write may accept.
    pub write_chunk: usize,
    /// Self-waking delays assigned to successive read operations.
    pub read_delays: Vec<u8>,
    /// Self-waking delays assigned to successive write operations.
    pub write_delays: Vec<u8>,
    /// Self-waking delays assigned to successive flush operations.
    pub flush_delays: Vec<u8>,
    /// Whether writes remain private until the next flush.
    pub hold_until_flush: bool,
    /// Optional failure after a successful operation or byte prefix.
    pub fault: Option<IoFault>,
}

impl Default for IoPlan {
    fn default() -> Self {
        Self {
            read_chunk: usize::MAX,
            write_chunk: usize::MAX,
            read_delays: Vec::new(),
            write_delays: Vec::new(),
            flush_delays: Vec::new(),
            hold_until_flush: false,
            fault: None,
        }
    }
}

/// Completed operations and injected delays observed at one endpoint.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IoReport {
    /// Successful nonempty reads.
    pub reads: usize,
    /// Bytes delivered by successful reads.
    pub read_bytes: usize,
    /// Successful nonempty writes.
    pub writes: usize,
    /// Bytes accepted by successful writes.
    pub write_bytes: usize,
    /// Successful flushes.
    pub flushes: usize,
    /// Successfully opened outgoing data streams.
    pub connects: usize,
    /// Successfully accepted incoming data streams.
    pub accepts: usize,
    /// Polls deliberately suspended by the test schedule.
    pub delayed_polls: usize,
    /// Largest successful read.
    pub largest_read: usize,
    /// Largest successful write.
    pub largest_write: usize,
    /// Failure actually injected, if its threshold was reached.
    pub injected: Option<InjectedIo>,
}

/// Shared observation handle retained outside the wrapped transport.
#[derive(Clone)]
pub struct IoReportHandle(Arc<Mutex<State>>);

impl IoReportHandle {
    /// Snapshot the completed transport observations.
    pub fn snapshot(&self) -> IoReport {
        self.0.lock().expect("transport report lock").report
    }
}

struct State {
    side: Side,
    plan: IoPlan,
    report: IoReport,
    read_step: usize,
    write_step: usize,
    flush_step: usize,
}

impl State {
    /// Take the next bounded delay assigned to `operation`.
    fn delay(&mut self, operation: Operation) -> u8 {
        let (delays, step) = match operation {
            Operation::Read => (&self.plan.read_delays, &mut self.read_step),
            Operation::Write => (&self.plan.write_delays, &mut self.write_step),
            Operation::Flush => (&self.plan.flush_delays, &mut self.flush_step),
            // Supply operations have no delay schedule.
            Operation::Connect | Operation::Accept => return 0,
        };
        let delay = delays.get(*step).copied().unwrap_or(0).min(2);
        *step += 1;
        delay
    }

    /// Return a typed failure once its configured prefix has completed.
    fn failure(&mut self, operation: Operation) -> Option<io::Error> {
        let fault = self.plan.fault?;
        if fault.operation != operation {
            return None;
        }
        if let Some(injected) = self.report.injected {
            return Some(io::Error::other(injected));
        }
        let completed = match (operation, fault.unit) {
            (Operation::Read, FaultUnit::Operations) => self.report.reads,
            (Operation::Read, FaultUnit::Bytes) => self.report.read_bytes,
            (Operation::Write, FaultUnit::Operations) => self.report.writes,
            (Operation::Write, FaultUnit::Bytes) => self.report.write_bytes,
            (Operation::Flush, _) => self.report.flushes,
            // Supply operations transfer no bytes; only operation counting
            // is meaningful for them.
            (Operation::Connect, _) => self.report.connects,
            (Operation::Accept, _) => self.report.accepts,
        };
        if completed < fault.after {
            return None;
        }
        let injected = InjectedIo {
            side: self.side,
            operation,
            after: fault.after,
            unit: fault.unit,
        };
        self.report.injected = Some(injected);
        Some(io::Error::other(injected))
    }

    /// Whether a read fault's successful prefix is exhausted, so the next
    /// payload-bearing read must fire in its place.
    fn read_fault_armed(&self) -> bool {
        let Some(fault) = self.plan.fault else {
            return false;
        };
        if fault.operation != Operation::Read {
            return false;
        }
        match fault.unit {
            FaultUnit::Operations => self.report.reads >= fault.after,
            FaultUnit::Bytes => self.report.read_bytes >= fault.after,
        }
    }

    /// Record the read fault as injected and mint its error.
    fn inject_read(&mut self) -> io::Error {
        let fault = self.plan.fault.expect("an armed fault is configured");
        let injected = InjectedIo {
            side: self.side,
            operation: Operation::Read,
            after: fault.after,
            unit: fault.unit,
        };
        self.report.injected.get_or_insert(injected);
        io::Error::other(self.report.injected.expect("just recorded"))
    }

    /// Remaining byte prefix before a byte-counted fault must fire.
    fn remaining_bytes(&self, operation: Operation) -> usize {
        let Some(fault) = self.plan.fault else {
            return usize::MAX;
        };
        if fault.operation != operation || fault.unit != FaultUnit::Bytes {
            return usize::MAX;
        }
        let completed = match operation {
            Operation::Read => self.report.read_bytes,
            Operation::Write => self.report.write_bytes,
            Operation::Flush => self.report.flushes,
            // Supply operations transfer no bytes, so a byte-counted fault
            // never constrains them.
            Operation::Connect | Operation::Accept => return usize::MAX,
        };
        fault.after.saturating_sub(completed)
    }
}

/// A reader with deterministic fragmentation and self-waking delays.
pub struct AdversarialRead<R> {
    inner: R,
    state: Arc<Mutex<State>>,
    delay: Option<u8>,
}

impl<R: AsyncRead + Unpin> AsyncRead for AdversarialRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if suspend(&this.state, &mut this.delay, Operation::Read, cx) {
            return Poll::Pending;
        }

        let (limit, armed) = {
            let mut state = this.state.lock().expect("transport state lock");
            // An already-injected read fault keeps failing every read.
            if state.report.injected.is_some()
                && let Some(error) = state.failure(Operation::Read)
            {
                this.delay = None;
                return Poll::Ready(Err(error));
            }
            let armed = state.read_fault_armed();
            let limit = if armed {
                // The budget is spent: read unclamped, so the next payload
                // is observed (and replaced by the fault) rather than being
                // zero-windowed into a spurious end-of-stream.
                state.plan.read_chunk.max(1).min(buf.remaining())
            } else {
                state
                    .plan
                    .read_chunk
                    .max(1)
                    .min(state.remaining_bytes(Operation::Read))
                    .min(buf.remaining())
            };
            (limit, armed)
        };
        let before = buf.filled().len();
        let window = buf.initialize_unfilled_to(limit);
        let mut limited = ReadBuf::new(window);
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Ready(Ok(())) => {
                let read = limited.filled().len();
                this.delay = None;
                if read > 0 {
                    let mut state = this.state.lock().expect("transport state lock");
                    if armed {
                        // The payload beyond the budget is discarded with
                        // the failing connection; nothing is advanced.
                        return Poll::Ready(Err(state.inject_read()));
                    }
                    state.report.reads += 1;
                    state.report.read_bytes += read;
                    state.report.largest_read = state.report.largest_read.max(read);
                }
                buf.advance(read);
                debug_assert_eq!(buf.filled().len() - before, read);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                this.delay = None;
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A writer with deterministic fragmentation, delays, and flush buffering.
pub struct AdversarialWrite<W> {
    inner: W,
    state: Arc<Mutex<State>>,
    write_delay: Option<u8>,
    flush_delay: Option<u8>,
    buffered: Vec<u8>,
    sent: usize,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for AdversarialWrite<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if suspend(&this.state, &mut this.write_delay, Operation::Write, cx) {
            return Poll::Pending;
        }
        let (limit, buffered) = {
            let mut state = this.state.lock().expect("transport state lock");
            if let Some(error) = state.failure(Operation::Write) {
                this.write_delay = None;
                return Poll::Ready(Err(error));
            }
            (
                state
                    .plan
                    .write_chunk
                    .max(1)
                    .min(state.remaining_bytes(Operation::Write))
                    .min(bytes.len()),
                state.plan.hold_until_flush,
            )
        };
        let result = if buffered {
            this.buffered.extend_from_slice(&bytes[..limit]);
            Poll::Ready(Ok(limit))
        } else {
            Pin::new(&mut this.inner).poll_write(cx, &bytes[..limit])
        };
        match result {
            Poll::Ready(Ok(written)) => {
                this.write_delay = None;
                if written > 0 {
                    let mut state = this.state.lock().expect("transport state lock");
                    state.report.writes += 1;
                    state.report.write_bytes += written;
                    state.report.largest_write = state.report.largest_write.max(written);
                }
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => {
                this.write_delay = None;
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if suspend(&this.state, &mut this.flush_delay, Operation::Flush, cx) {
            return Poll::Pending;
        }
        if let Some(error) = this
            .state
            .lock()
            .expect("transport state lock")
            .failure(Operation::Flush)
        {
            this.flush_delay = None;
            return Poll::Ready(Err(error));
        }
        while this.sent < this.buffered.len() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.buffered[this.sent..]) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Poll::Ready(Ok(written)) => this.sent += written,
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        match Pin::new(&mut this.inner).poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                this.buffered.clear();
                this.sent = 0;
                this.flush_delay = None;
                this.state
                    .lock()
                    .expect("transport state lock")
                    .report
                    .flushes += 1;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                this.flush_delay = None;
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => Pin::new(&mut self.get_mut().inner).poll_shutdown(cx),
            other => other,
        }
    }
}

/// Wrap one endpoint's ordered transport halves and retain its observations.
pub fn wrap_io<R, W>(
    side: Side,
    plan: IoPlan,
    read: R,
    write: W,
) -> (AdversarialRead<R>, AdversarialWrite<W>, IoReportHandle) {
    let state = Arc::new(Mutex::new(State {
        side,
        plan,
        report: IoReport::default(),
        read_step: 0,
        write_step: 0,
        flush_step: 0,
    }));
    (
        AdversarialRead {
            inner: read,
            state: state.clone(),
            delay: None,
        },
        wrap_write(write, state.clone()),
        IoReportHandle(state),
    )
}

/// Build a writer wrapper sharing already-created adversity state.
fn wrap_write<W>(write: W, state: Arc<Mutex<State>>) -> AdversarialWrite<W> {
    AdversarialWrite {
        inner: write,
        state,
        write_delay: None,
        flush_delay: None,
        buffered: Vec::new(),
        sent: 0,
    }
}

/// Wrap one endpoint's whole [`Link`](crate::link::Link): the control
/// halves and every data
/// stream the link ever supplies share one plan and one report.
///
/// Delay schedules and fault thresholds count operations across all of the
/// side's streams in poll order, so a single plan exercises (or fails)
/// whichever surface reaches the threshold first — exactly the coverage the
/// single-pipe wrapper provided when every stream shared one pipe.
pub fn wrap_link<CR, CW, C, A>(
    side: Side,
    plan: IoPlan,
    link: crate::link::Link<CR, CW, C, A>,
) -> (AdversarialLink<CR, CW, C, A>, IoReportHandle)
where
    CR: tokio::io::AsyncRead + Unpin + Send,
    CW: tokio::io::AsyncWrite + Unpin + Send,
    C: crate::link::Connector,
    A: crate::link::Acceptor,
{
    let parts = link.into_parts();
    let state = Arc::new(Mutex::new(State {
        side,
        plan,
        report: IoReport::default(),
        read_step: 0,
        write_step: 0,
        flush_step: 0,
    }));
    let wrapped = crate::link::LinkParts {
        control_read: AdversarialRead {
            inner: parts.control_read,
            state: state.clone(),
            delay: None,
        },
        control_write: wrap_write(parts.control_write, state.clone()),
        connector: AdversarialConnector {
            inner: parts.connector,
            state: state.clone(),
        },
        acceptor: AdversarialAcceptor {
            inner: parts.acceptor,
            state: state.clone(),
        },
        session: parts.session,
    }
    .into_link();
    (wrapped, IoReportHandle(state))
}

/// A link wholly wrapped in one side's shared adversity state.
pub type AdversarialLink<CR, CW, C, A> = crate::link::Link<
    AdversarialRead<CR>,
    AdversarialWrite<CW>,
    AdversarialConnector<C>,
    AdversarialAcceptor<A>,
>;

/// A [`Connector`](crate::link::Connector) whose opened streams write
/// through the side's shared adversity state.
pub struct AdversarialConnector<C> {
    inner: C,
    state: Arc<Mutex<State>>,
}

impl<C: Clone> Clone for AdversarialConnector<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: self.state.clone(),
        }
    }
}

impl<C: crate::link::Connector> crate::link::Connector for AdversarialConnector<C> {
    type Tx = AdversarialWrite<C::Tx>;

    async fn connect(&self) -> io::Result<Self::Tx> {
        // The fault fires in place of the call: a healthy supply's connects
        // always succeed, so the clean run's success count is also its call
        // count and "would the fault fire?" remains a function of it.
        if let Some(error) = self
            .state
            .lock()
            .expect("transport state lock")
            .failure(Operation::Connect)
        {
            return Err(error);
        }
        let tx = self.inner.connect().await?;
        self.state
            .lock()
            .expect("transport state lock")
            .report
            .connects += 1;
        Ok(wrap_write(tx, self.state.clone()))
    }
}

/// An [`Acceptor`](crate::link::Acceptor) whose accepted streams read
/// through the side's shared adversity state.
pub struct AdversarialAcceptor<A> {
    inner: A,
    state: Arc<Mutex<State>>,
}

impl<A: crate::link::Acceptor> crate::link::Acceptor for AdversarialAcceptor<A> {
    type Rx = AdversarialRead<A::Rx>;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        // An already-injected accept fault keeps failing without consuming
        // further arrivals.
        {
            let mut state = self.state.lock().expect("transport state lock");
            if state.report.injected.is_some()
                && let Some(error) = state.failure(Operation::Accept)
            {
                return Err(error);
            }
        }
        let rx = self.inner.accept().await?;
        let mut state = self.state.lock().expect("transport state lock");
        // The fault fires in place of a successful accept, so the final
        // forever-pending accept a session parks on against an honest peer
        // cannot trip an operations-counted threshold (the same rule
        // payload-bearing reads follow).
        if let Some(error) = state.failure(Operation::Accept) {
            // The arrival is discarded with the failing supply.
            drop(rx);
            return Err(error);
        }
        state.report.accepts += 1;
        drop(state);
        Ok(AdversarialRead {
            inner: rx,
            state: self.state.clone(),
            delay: None,
        })
    }
}

/// Cooperative yields an accept spends genuinely waiting for a further
/// arrival to reorder before releasing what it already holds.
///
/// Each yield hands the whole closed-world topology one more poll, so the
/// budget bounds the wait in peer progress rather than wall time. It must
/// be generous enough for a concurrently working peer to open its next
/// stream; expiring is always safe — the held batch releases, at worst
/// unreordered.
const REORDER_PATIENCE: u8 = 32;

/// An [`Acceptor`](crate::link::Acceptor) delivering arrivals in reversed
/// batches: worst-case-legal stream reordering.
///
/// The link contract leaves cross-stream arrival order unspecified, so a
/// session must pair streams by label alone; this decorator inverts arrival
/// order whenever the traffic admits it. Each accept awaits one arrival and
/// then *holds it*, genuinely waiting — bounded by a patience budget of
/// cooperative yields (`REORDER_PATIENCE`), so a lone final stream still
/// flows and a peer wedged behind the held stream cannot deadlock the
/// harness — for further arrivals, up to `batch` in total, and releases the
/// accumulated batch newest-first.
///
/// Every batch of two or more is a genuine inversion, recorded in the
/// shared `reordered` counter so a test can assert the adversity's actual
/// disposition instead of assuming it. (An earlier draft only drained
/// arrivals already `Ready` under a noop-waker poll; under the
/// deterministic scheduler a second arrival was never queued at that
/// instant, so the decorator silently degenerated to pass-through — and
/// nothing said so.)
///
/// A sibling of the conformance suite's `ReversingAcceptor`
/// (`src/conformance/tests.rs`), duplicated so this crate-internal seam does
/// not depend on the public `conformance` feature.
pub struct ReorderingAcceptor<A: crate::link::Acceptor> {
    inner: A,
    held: VecDeque<A::Rx>,
    /// Arrivals buffered before each reversed release.
    batch: usize,
    /// Batches of two or more released: genuine inversions.
    reordered: Arc<AtomicUsize>,
}

impl<A: crate::link::Acceptor> crate::link::Acceptor for ReorderingAcceptor<A> {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        if let Some(held) = self.held.pop_front() {
            return Ok(held);
        }
        let first = self.inner.accept().await?;
        self.held.push_front(first);
        // Hold the arrival and genuinely wait for company: each round polls
        // a fresh inner accept once — dropping it while pending is exactly
        // the cancellation tolerance the link contract demands — then
        // yields, giving the peer polls in which to open its next stream.
        let mut patience = REORDER_PATIENCE;
        while self.held.len() < self.batch {
            let mut next = std::pin::pin!(self.inner.accept());
            match std::future::poll_fn(|cx| Poll::Ready(next.as_mut().poll(cx))).await {
                Poll::Ready(Ok(rx)) => {
                    self.held.push_front(rx);
                    patience = REORDER_PATIENCE;
                }
                // Errored: stop batching and release what is held; a real
                // error resurfaces from the next accept call.
                Poll::Ready(Err(_)) => break,
                Poll::Pending => {
                    let Some(remaining) = patience.checked_sub(1) else {
                        break;
                    };
                    patience = remaining;
                    yield_once().await;
                }
            }
        }
        if self.held.len() > 1 {
            self.reordered.fetch_add(1, Ordering::Relaxed);
        }
        Ok(self.held.pop_front().expect("at least one arrival is held"))
    }
}

/// Yield to the executor exactly once: `Pending` with an immediate
/// self-wake.
///
/// Runtime-agnostic (the deterministic driver is no runtime at all), unlike
/// `tokio::task::yield_now`; a copy of `conformance`'s helper, on the same
/// feature seam that keeps [`ReorderingAcceptor`] separate from its
/// `ReversingAcceptor` sibling.
async fn yield_once() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if std::mem::replace(&mut yielded, true) {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

/// Wrap `link`'s acceptor so arrivals release in reversed batches of `batch`,
/// counting genuine inversions into `reordered`.
///
/// Always assert on the counter after the run: nonzero where the topology
/// admits reordering (that is the proof the adversity fired), zero as a
/// tripwire where it provably cannot — without either, the decorator is
/// indistinguishable from pass-through and the test's claims rot silently.
pub fn reorder_accepts<CR, CW, C, A>(
    link: crate::link::Link<CR, CW, C, A>,
    batch: usize,
    reordered: Arc<AtomicUsize>,
) -> crate::link::Link<CR, CW, C, ReorderingAcceptor<A>>
where
    CR: tokio::io::AsyncRead + Unpin + Send,
    CW: tokio::io::AsyncWrite + Unpin + Send,
    C: crate::link::Connector,
    A: crate::link::Acceptor,
{
    let parts = link.into_parts();
    crate::link::LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: parts.connector,
        acceptor: ReorderingAcceptor {
            inner: parts.acceptor,
            held: VecDeque::new(),
            batch,
            reordered,
        },
        session: parts.session,
    }
    .into_link()
}

/// Suspend one operation according to its next scheduled self-waking delay.
fn suspend(
    state: &Arc<Mutex<State>>,
    delay: &mut Option<u8>,
    operation: Operation,
    cx: &Context<'_>,
) -> bool {
    if delay.is_none() {
        *delay = Some(state.lock().expect("transport state lock").delay(operation));
    }
    if delay.is_some_and(|remaining| remaining > 0) {
        *delay = delay.map(|remaining| remaining - 1);
        state
            .lock()
            .expect("transport state lock")
            .report
            .delayed_polls += 1;
        cx.waker().wake_by_ref();
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use futures::{pin_mut, poll};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex, split};

    use super::{IoPlan, Side, wrap_io};
    use crate::testing::run_to_quiescence;

    /// Flush buffering keeps completed writes invisible to the peer until the
    /// corresponding flush is polled.
    #[test]
    fn flush_buffering_withholds_bytes_until_flush() {
        let (left, right) = duplex(8);
        let (read, write) = split(left);
        let plan = IoPlan {
            hold_until_flush: true,
            ..IoPlan::default()
        };
        let (_read, mut write, report) = wrap_io(Side::Left, plan, read, write);
        let (mut peer_read, _peer_write) = split(right);

        let received = run_to_quiescence(async {
            write.write_all(b"abcd").await.unwrap();

            let mut bytes = [0; 4];
            let receive = peer_read.read_exact(&mut bytes);
            pin_mut!(receive);
            assert!(poll!(receive.as_mut()).is_pending());

            write.flush().await.unwrap();
            receive.await.unwrap();
            bytes
        })
        .expect("the buffered transport should remain live");

        assert_eq!(received, *b"abcd");
        let snapshot = report.snapshot();
        assert_eq!(snapshot.writes, 1);
        assert_eq!(snapshot.write_bytes, 4);
        assert_eq!(snapshot.flushes, 1);
    }

    /// Fragmentation, delays, and flush buffering compose without losing bytes.
    #[test]
    fn successful_adversity_is_lossless() {
        let (left, right) = duplex(1);
        let (read, write) = split(left);
        let plan = IoPlan {
            read_chunk: 1,
            write_chunk: 2,
            read_delays: vec![1; 8],
            write_delays: vec![1; 8],
            flush_delays: vec![1],
            hold_until_flush: true,
            fault: None,
        };
        let (mut read, mut write, report) = wrap_io(Side::Left, plan, read, write);
        let (mut peer_read, mut peer_write) = split(right);
        let (sent, received, peer_received) = run_to_quiescence(async {
            futures::join!(
                async {
                    write.write_all(b"abcd").await.unwrap();
                    write.flush().await.unwrap();
                },
                async {
                    let mut bytes = [0; 2];
                    read.read_exact(&mut bytes).await.unwrap();
                    bytes
                },
                async {
                    let mut bytes = [0; 4];
                    peer_read.read_exact(&mut bytes).await.unwrap();
                    peer_write.write_all(b"xy").await.unwrap();
                    peer_write.flush().await.unwrap();
                    bytes
                },
            )
        })
        .expect("the closed transport should remain live");
        assert_eq!(sent, ());
        assert_eq!(received, *b"xy");
        assert_eq!(peer_received, *b"abcd");
        let snapshot = report.snapshot();
        assert_eq!(snapshot.write_bytes, 4);
        assert_eq!(snapshot.read_bytes, 2);
        assert!(snapshot.delayed_polls > 0);
    }
}
