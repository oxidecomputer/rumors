//! Positive schedules and faulty transports for the public contract probes.

use super::*;
use futures::FutureExt;
use proptest::prelude::*;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::AtomicBool;
use tokio::io::BufWriter;
use tokio::sync::watch;

/// Incorrectly parks stream opens behind a blocked control write.
#[derive(Clone)]
struct ControlGatedConnector {
    /// The otherwise independent stream supply.
    inner: MemoryConnector,
    /// Shared with this link's control writer.
    state: Arc<Mutex<CoupledControl>>,
}

impl Connector for ControlGatedConnector {
    /// An ordinary memory writer once opened.
    type Tx = DuplexStream;

    /// Wait for unrelated control traffic before opening a data stream.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        std::future::poll_fn(|cx| {
            let mut state = self.state.lock().unwrap();
            if state.write_blocked {
                state.parked = Some(cx.waker().clone());
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
        self.inner.connect().await
    }
}

/// Blocked control writes must not prevent opening and delivering data streams.
#[test]
fn control_pressure_catches_blocked_data_opens() {
    let (a, b) = memory_with_capacity(1);
    let a = a.into_parts();
    let state = Arc::new(Mutex::new(CoupledControl {
        write_blocked: false,
        parked: None,
    }));
    let a = LinkParts {
        control_read: a.control_read,
        control_write: CoupledWrite {
            inner: a.control_write,
            state: state.clone(),
        },
        connector: ControlGatedConnector {
            inner: a.connector,
            state,
        },
        acceptor: a.acceptor,
        session: a.session,
    }
    .into_link();
    assert_eq!(
        run_to_quiescence(super::super::check_control_data_independence(
            async || (a, b),
            std::future::pending
        )),
        Err(Quiescence::Stalled)
    );
}

/// Buffers outgoing bytes but discards that buffer at either stream ending.
#[derive(Clone)]
struct LossyCompletion(MemoryConnector);

impl Connector for LossyCompletion {
    /// A writer whose drop does not flush its buffer.
    type Tx = BufWriter<DuplexStream>;

    /// Return a completion callback which also discards unflushed bytes.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (tx, done) = self.0.connect().await?;
        Ok((
            BufWriter::new(tx),
            Done::new(move |tx: Self::Tx| done.complete(tx.into_inner())),
        ))
    }
}

/// Accepted bytes must survive abort even when the caller did not flush.
#[test]
#[should_panic(expected = "contract: a stream delivers its exact bytes in order")]
fn abort_catches_lost_unflushed_bytes() {
    let (a, b) = memory();
    let a = a.into_parts();
    let mut b = b.into_parts();
    run_to_quiescence(super::super::probe_stream(
        &LossyCompletion(a.connector),
        &mut b.acceptor,
    ))
    .unwrap();
}

/// Accepted bytes must survive completion even when the caller did not flush.
#[test]
#[should_panic(expected = "contract: a completed stream delivers its bytes")]
fn completion_catches_lost_unflushed_bytes() {
    let (a, b) = memory();
    let a = a.into_parts();
    let mut b = b.into_parts();
    run_to_quiescence(super::super::probe_completed_streams(
        &LossyCompletion(a.connector),
        &mut b.acceptor,
    ))
    .unwrap();
}

/// Rejects overlapping opens even though sequential opens work.
#[derive(Clone)]
struct SingleOpen {
    /// The healthy stream supply.
    inner: MemoryConnector,
    /// Shared by the original handle and its clones.
    opening: Arc<AtomicBool>,
}

impl Connector for SingleOpen {
    /// An ordinary memory stream once opened.
    type Tx = DuplexStream;

    /// Leave a pending interval in which another call incorrectly fails.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        if self.opening.swap(true, Ordering::SeqCst) {
            return Err(io::Error::other("overlapping opens rejected"));
        }
        super::super::yield_once().await;
        let result = self.inner.connect().await;
        self.opening.store(false, Ordering::SeqCst);
        result
    }
}

/// The concurrency check must overlap opens through shared and cloned handles.
#[test]
#[should_panic(expected = "overlapping opens rejected")]
fn overlapping_opens_are_exercised() {
    let (a, b) = memory();
    let a = a.into_parts();
    let a = LinkParts {
        control_read: a.control_read,
        control_write: a.control_write,
        connector: SingleOpen {
            inner: a.connector,
            opening: Arc::new(AtomicBool::new(false)),
        },
        acceptor: a.acceptor,
        session: a.session,
    }
    .into_link();
    run_to_quiescence(super::super::check_concurrency(
        async || (a, b),
        std::future::pending,
    ))
    .unwrap();
}

/// Blocks all opens after a producer finishes until its receiver also finishes.
#[derive(Clone)]
struct GatedConnector {
    /// The otherwise independent stream supply.
    inner: MemoryConnector,
    /// Whether a producer has finished ahead of its consumer.
    blocked: watch::Sender<bool>,
}

impl Connector for GatedConnector {
    /// An ordinary memory writer.
    type Tx = DuplexStream;

    /// Incorrectly wait on an unrelated receiver before opening a new stream.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        self.blocked
            .subscribe()
            .wait_for(|blocked| !blocked)
            .await
            .unwrap();
        let (tx, done) = self.inner.connect().await?;
        let blocked = self.blocked.clone();
        Ok((
            tx,
            Done::new(move |tx| {
                blocked.send_replace(true);
                done.complete(tx);
            }),
        ))
    }
}

/// Reopens the faulty connector's gate when a consumer completes.
struct GatedAcceptor {
    /// The underlying incoming stream supply.
    inner: MemoryAcceptor,
    /// Shared with the peer's connector.
    blocked: watch::Sender<bool>,
}

impl Acceptor for GatedAcceptor {
    /// An ordinary memory reader.
    type Rx = DuplexStream;

    /// Attach the consumer callback that releases unrelated opens.
    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        let (rx, done) = self.inner.accept().await?;
        let blocked = self.blocked.clone();
        Ok((
            rx,
            Done::new(move |rx| {
                blocked.send_replace(false);
                done.complete(rx);
            }),
        ))
    }
}

/// A completed producer must not tie later streams to its unfinished consumer.
#[test]
fn delayed_consumer_catches_coupled_reuse() {
    let (a, b) = memory();
    let a = a.into_parts();
    let b = b.into_parts();
    let (blocked, _) = watch::channel(false);
    let connector = GatedConnector {
        inner: a.connector,
        blocked: blocked.clone(),
    };
    let mut acceptor = GatedAcceptor {
        inner: b.acceptor,
        blocked,
    };
    assert_eq!(
        run_to_quiescence(super::super::probe_delayed_completion(
            &connector,
            &mut acceptor,
            false,
            1,
        )),
        Err(Quiescence::Stalled)
    );
}

proptest! {
    /// A deadline cancels stalled pair construction or I/O, releases the pair,
    /// and identifies the failed check without relying on a runtime.
    #[test]
    fn deadlines_cancel_setup_and_io(polls in 1usize..8, during_setup in any::<bool>()) {
        let (a, b) = memory_with_capacity(1);
        let (a, b) = (coupled(a), coupled(b));
        let alive = Arc::downgrade(&a.control_read.state);
        let pair = async move || {
            if during_setup {
                std::future::pending::<()>().await;
            }
            (a, b)
        };
        let deadline = || async {
            for _ in 0..polls {
                super::super::yield_once().await;
            }
        };
        let outcome = run_to_quiescence(AssertUnwindSafe(
            super::super::check_control_duplex(pair, deadline),
        ).catch_unwind()).expect("the deadline must end the stall");
        let panic = outcome.expect_err("a stalled check must time out");
        prop_assert_eq!(panic.downcast_ref::<String>().map(String::as_str),
            Some("conformance: check_control_duplex timed out"));
        prop_assert!(alive.upgrade().is_none(), "timeout retained the pair");
    }

    /// A successful check releases its pending deadline instead of leaving
    /// timer state alive after the check returns.
    #[test]
    fn successful_checks_release_deadlines(capacity in 1usize..128) {
        let (signal, deadline) = futures::channel::oneshot::channel::<()>();
        run_to_quiescence(super::super::check_control(
            async || memory_with_capacity(capacity),
            || async { let _ = deadline.await; },
        )).expect("healthy control traffic completes");
        prop_assert!(signal.is_canceled(), "the finished check retained its timer");
    }

    /// Independent memory streams tolerate overlapping opens, either completion
    /// order, and repeated cancellation after different numbers of polls.
    #[test]
    fn stream_schedules_conform(
        capacity in 1usize..128,
        polls in 1usize..12,
        deliveries in 2usize..=STREAM_COUNT,
        rounds in 1usize..24,
        receiver_first in any::<bool>(),
    ) {
        let (a, b) = memory_with_capacity(capacity);
        let a = a.into_parts();
        let mut b = b.into_parts();
        run_to_quiescence(async {
            super::super::probe_concurrency(&a.connector, &mut b.acceptor, true).await;
            super::super::probe_delayed_completion(&a.connector, &mut b.acceptor, receiver_first, rounds).await;
            super::super::probe_cancellation(&a.connector, &mut b.acceptor, polls, deliveries).await;
        }).expect("independent streams stay live across schedules");
    }

    /// A shared pool needs room for control as well as all data streams. Leaving
    /// out control headroom must fail the combined probe at every buffer size.
    #[test]
    fn combined_pressure_checks_control_headroom(capacity in 1usize..64, control_headroom in any::<bool>()) {
        let budget = (STREAM_COUNT + usize::from(control_headroom)) * capacity;
        let (a, b) = windowed_pair(budget, capacity);
        let result = run_to_quiescence(super::super::check_control_data_independence(async || (a, b), std::future::pending));
        if control_headroom {
            prop_assert_eq!(result, Ok(()));
        } else {
            prop_assert_eq!(result, Err(Quiescence::Stalled));
        }
    }

    /// Cancellation must catch a lost delivery even when dequeue takes more
    /// than one poll to reach.
    #[test]
    fn delayed_dequeue_loss_is_caught(before_dequeue in 0usize..8) {
        let (a, b) = memory();
        let a = a.into_parts();
        let b = b.into_parts();
        let mut acceptor = LossyAcceptor { inner: b.acceptor, before_dequeue };
        prop_assert_eq!(run_to_quiescence(super::super::probe_cancellation(
            &a.connector, &mut acceptor, before_dequeue + 1, 2,
        )), Err(Quiescence::Stalled));
    }
}
