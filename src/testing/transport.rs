//! Stackable, deterministic adversity for test transports across the crate.

use std::{
    io,
    pin::Pin,
    sync::{Arc, Mutex},
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

        let limit = {
            let mut state = this.state.lock().expect("transport state lock");
            if let Some(error) = state.failure(Operation::Read) {
                this.delay = None;
                return Poll::Ready(Err(error));
            }
            state
                .plan
                .read_chunk
                .max(1)
                .min(state.remaining_bytes(Operation::Read))
                .min(buf.remaining())
        };
        let before = buf.filled().len();
        let window = buf.initialize_unfilled_to(limit);
        let mut limited = ReadBuf::new(window);
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Ready(Ok(())) => {
                let read = limited.filled().len();
                buf.advance(read);
                debug_assert_eq!(buf.filled().len() - before, read);
                this.delay = None;
                if read > 0 {
                    let mut state = this.state.lock().expect("transport state lock");
                    state.report.reads += 1;
                    state.report.read_bytes += read;
                    state.report.largest_read = state.report.largest_read.max(read);
                }
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
        AdversarialWrite {
            inner: write,
            state: state.clone(),
            write_delay: None,
            flush_delay: None,
            buffered: Vec::new(),
            sent: 0,
        },
        IoReportHandle(state),
    )
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
