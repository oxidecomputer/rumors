//! Stackable, deterministic adversity for test transports across the crate.

use std::{
    io::{self, IoSlice},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf};

use crate::testing::schedule::{Countdown, MAX_SCHEDULED_DELAY};

/// Which endpoint owns an observed transport operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The endpoint the test labels as left.
    Left,
    /// The endpoint the test labels as right.
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
/// counts alone: the pipe reads for a clean goodbye at every session
/// boundary, so attempts-after-last-success are structural and must not
/// trip an operations-counted fault.
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
    /// Whether the writer takes vectored writes across their slices.
    ///
    /// A vectored writer takes up to `write_chunk` bytes across a write's
    /// slices as one operation, as a TCP socket does. Otherwise it takes
    /// only a vectored write's first nonempty slice, as the trait's default
    /// does.
    pub vectored: bool,
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
            vectored: false,
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

/// Hide shared transport state while making the observation handle debuggable.
impl std::fmt::Debug for IoReportHandle {
    /// Format an opaque observation handle.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IoReportHandle").finish_non_exhaustive()
    }
}

impl IoReportHandle {
    /// Snapshot the completed transport observations.
    pub fn snapshot(&self) -> IoReport {
        self.0.lock().expect("transport report lock").report
    }
}

/// One endpoint's shared fault plan, progress counters, and delay cursors.
struct State {
    side: Side,
    plan: IoPlan,
    report: IoReport,
    read_step: usize,
    write_step: usize,
    flush_step: usize,
}

/// Advance and query one endpoint's adversity state.
impl State {
    /// Take the next bounded delay assigned to `operation`.
    fn delay(&mut self, operation: Operation) -> u8 {
        let (delays, step) = match operation {
            Operation::Read => (&self.plan.read_delays, &mut self.read_step),
            Operation::Write => (&self.plan.write_delays, &mut self.write_step),
            Operation::Flush => (&self.plan.flush_delays, &mut self.flush_step),
        };
        let delay = delays
            .get(*step)
            .copied()
            .unwrap_or(0)
            .min(MAX_SCHEDULED_DELAY);
        *step += 1;
        delay
    }

    /// Return an error once its configured prefix has completed.
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

    /// Record the read fault as injected and produce its error.
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
        };
        fault.after.saturating_sub(completed)
    }
}

/// A reader with deterministic fragmentation and self-waking delays.
pub struct AdversarialRead<R> {
    inner: R,
    state: Arc<Mutex<State>>,
    delay: Countdown,
}

/// Summarize a reader without requiring its transport to be debuggable.
impl<R> std::fmt::Debug for AdversarialRead<R> {
    /// Format whether a scheduled delay is in progress.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdversarialRead")
            .field("delayed", &self.delay.is_active())
            .finish_non_exhaustive()
    }
}

/// Apply the configured read fragmentation, delays, and fault boundary.
impl<R: AsyncRead + Unpin> AsyncRead for AdversarialRead<R> {
    /// Poll one read through the shared adversity state.
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
                this.delay.clear();
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
        let window = buf.initialize_unfilled_to(limit);
        let mut limited = ReadBuf::new(window);
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Ready(Ok(())) => {
                let read = limited.filled().len();
                this.delay.clear();
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
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                this.delay.clear();
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A writer with deterministic fragmentation, delays, and flush buffering.
pub struct AdversarialWrite<W> {
    inner: W,
    /// Whether vectored writes are taken across slices ([`IoPlan::vectored`]).
    vectored: bool,
    state: Arc<Mutex<State>>,
    write_delay: Countdown,
    flush_delay: Countdown,
    buffered: Vec<u8>,
    sent: usize,
}

/// Summarize a writer without requiring its transport to be debuggable.
impl<W> std::fmt::Debug for AdversarialWrite<W> {
    /// Format pending delays and buffered-byte progress.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdversarialWrite")
            .field("write_delayed", &self.write_delay.is_active())
            .field("flush_delayed", &self.flush_delay.is_active())
            .field("buffered", &self.buffered.len())
            .field("sent", &self.sent)
            .finish_non_exhaustive()
    }
}

impl<W: AsyncWrite + Unpin> AdversarialWrite<W> {
    /// Admit one write offering `offered` bytes through the shared adversity
    /// state.
    ///
    /// The state suspends the write, fails it, or takes at most
    /// `write_chunk` of the bytes, which `hold` copies into the flush buffer
    /// when writes are held until a flush, and `forward` writes to the inner
    /// writer otherwise.
    fn poll_admit(
        &mut self,
        cx: &mut Context<'_>,
        offered: usize,
        hold: impl FnOnce(&mut Vec<u8>, usize),
        forward: impl FnOnce(Pin<&mut W>, &mut Context<'_>, usize) -> Poll<io::Result<usize>>,
    ) -> Poll<io::Result<usize>> {
        if suspend(&self.state, &mut self.write_delay, Operation::Write, cx) {
            return Poll::Pending;
        }
        let (limit, buffered) = {
            let mut state = self.state.lock().expect("transport state lock");
            if let Some(error) = state.failure(Operation::Write) {
                self.write_delay.clear();
                return Poll::Ready(Err(error));
            }
            (
                state
                    .plan
                    .write_chunk
                    .max(1)
                    .min(state.remaining_bytes(Operation::Write))
                    .min(offered),
                state.plan.hold_until_flush,
            )
        };
        let result = if buffered {
            hold(&mut self.buffered, limit);
            Poll::Ready(Ok(limit))
        } else {
            forward(Pin::new(&mut self.inner), cx, limit)
        };
        match result {
            Poll::Ready(Ok(written)) => {
                self.write_delay.clear();
                if written > 0 {
                    let mut state = self.state.lock().expect("transport state lock");
                    state.report.writes += 1;
                    state.report.write_bytes += written;
                    state.report.largest_write = state.report.largest_write.max(written);
                }
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => {
                self.write_delay.clear();
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The first `limit` bytes of `slices`, as slices of their own.
fn leading<'s>(slices: &'s [IoSlice<'_>], limit: usize) -> Vec<IoSlice<'s>> {
    let mut room = limit;
    let mut taken = Vec::new();
    for slice in slices {
        if room == 0 {
            break;
        }
        let take = room.min(slice.len());
        taken.push(IoSlice::new(&slice[..take]));
        room -= take;
    }
    taken
}

/// Apply configured fragmentation, buffering, delays, and faults to writes.
impl<W: AsyncWrite + Unpin> AsyncWrite for AdversarialWrite<W> {
    /// Admit one write through the shared adversity state.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().poll_admit(
            cx,
            bytes.len(),
            |held, limit| held.extend_from_slice(&bytes[..limit]),
            |inner, cx, limit| inner.poll_write(cx, &bytes[..limit]),
        )
    }

    /// Admit one vectored write: across slices when vectored, and as its
    /// first nonempty slice otherwise.
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        slices: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        if !self.vectored {
            let first = slices
                .iter()
                .find(|slice| !slice.is_empty())
                .map_or(&[][..], |slice| &**slice);
            return self.poll_write(cx, first);
        }
        let offered = slices.iter().map(|slice| slice.len()).sum();
        self.get_mut().poll_admit(
            cx,
            offered,
            |held, limit| {
                for slice in leading(slices, limit) {
                    held.extend_from_slice(&slice);
                }
            },
            |inner, cx, limit| inner.poll_write_vectored(cx, &leading(slices, limit)),
        )
    }

    /// Whether vectored writes are taken across slices.
    fn is_write_vectored(&self) -> bool {
        self.vectored
    }

    /// Deliver buffered bytes and apply the configured flush behavior.
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
            this.flush_delay.clear();
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
                this.flush_delay.clear();
                this.state
                    .lock()
                    .expect("transport state lock")
                    .report
                    .flushes += 1;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => {
                this.flush_delay.clear();
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    /// Flush buffered bytes before shutting down the wrapped writer.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => Pin::new(&mut self.get_mut().inner).poll_shutdown(cx),
            other => other,
        }
    }
}

/// Wrap one endpoint's ordered transport halves and retain its observations.
///
/// The adversarial reader models the transport, so its fragmentation,
/// delays, and faults apply to the transport's reads; the session reads
/// through a `BufReader` over it, as it requires of every read half.
pub fn wrap_io<R, W>(
    side: Side,
    plan: IoPlan,
    read: R,
    write: W,
) -> (
    BufReader<AdversarialRead<R>>,
    AdversarialWrite<W>,
    IoReportHandle,
)
where
    R: AsyncRead + Unpin,
{
    let state = Arc::new(Mutex::new(State {
        side,
        plan,
        report: IoReport::default(),
        read_step: 0,
        write_step: 0,
        flush_step: 0,
    }));
    (
        BufReader::new(AdversarialRead {
            inner: read,
            state: state.clone(),
            delay: Countdown::default(),
        }),
        wrap_write(write, state.clone()),
        IoReportHandle(state),
    )
}

/// Build a writer wrapper sharing already-created adversity state.
fn wrap_write<W>(write: W, state: Arc<Mutex<State>>) -> AdversarialWrite<W> {
    let vectored = state.lock().expect("transport state lock").plan.vectored;
    AdversarialWrite {
        inner: write,
        vectored,
        state,
        write_delay: Countdown::default(),
        flush_delay: Countdown::default(),
        buffered: Vec::new(),
        sent: 0,
    }
}

/// Suspend one operation according to its next scheduled self-waking delay.
fn suspend(
    state: &Arc<Mutex<State>>,
    delay: &mut Countdown,
    operation: Operation,
    cx: &Context<'_>,
) -> bool {
    if delay.suspend(
        || state.lock().expect("transport state lock").delay(operation),
        cx,
    ) {
        state
            .lock()
            .expect("transport state lock")
            .report
            .delayed_polls += 1;
        true
    } else {
        false
    }
}

/// Unit tests for this support module.
#[cfg(test)]
mod tests;
