//! Honest-peer behavior and the shared in-memory driver harness.
//!
//! Capacity/scheduling stress lives in [`capacity`], connected abort and
//! lifecycle checks in [`faults`], and deterministic tree builders in
//! [`fixtures`].

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use proptest::prelude::*;

use crate::tree::Root;
use crate::tree::arb::{
    arb_divergent_pair, arb_tree_root, leaf_parent_dispute_pair, leaf_parent_redaction_pair,
};
use crate::tree::mirror::alternating;
use crate::tree::mirror::streaming::backend::with_local_schedule;
use crate::tree::mirror::streaming::materialized::channel::with_schedule;
use crate::tree::mirror::streaming::materialized::progress::with_trace;
use crate::tree::mirror::streaming::{
    Handshaking, Local, Root as StreamingRoot, mirror as drive_streaming,
};

mod capacity;
mod faults;
mod fixtures;

/// Reconcile `a` and `b` through the streaming local backend, returning both
/// sides' reconciled roots in argument order, with no convergence assertion.
fn streaming_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    streaming_mirror_sides_with_schedule(a, b, Vec::new())
}

/// Why polling stopped before the session completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Quiescence {
    /// The future returned `Pending` without arranging another poll.
    Stalled,
    /// The future kept self-waking beyond the test's runaway guard.
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

/// Poll a closed, in-memory session until it completes or becomes quiescent.
///
/// The local backend starts no external I/O: every legitimate suspension is
/// paired with a synchronous channel wake or a test-injected self-wake. A
/// `Pending` poll with no wake is therefore a deterministic deadlock witness,
/// not a wall-clock guess that the machine has taken too long.
fn run_to_quiescence<F: Future>(
    runtime: &tokio::runtime::Runtime,
    future: F,
) -> Result<F::Output, Quiescence> {
    const MAX_POLLS: usize = 1_000_000;

    let _entered = runtime.enter();
    let wake = Arc::new(WakeFlag(AtomicBool::new(true)));
    let waker = Waker::from(wake.clone());
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);

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

/// Quiescence distinguishes a legitimate self-wake from a permanently parked future.
#[test]
fn quiescence_detector_observes_wake_contract() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("the test runtime should build");
    let mut first = true;
    let self_waking = std::future::poll_fn(move |cx| {
        if std::mem::take(&mut first) {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(7)
        }
    });
    assert_eq!(run_to_quiescence(&runtime, self_waking), Ok(7));
    assert_eq!(
        run_to_quiescence(&runtime, std::future::pending::<()>()),
        Err(Quiescence::Stalled),
    );
}

/// Reconcile under an explicit, shrinkable channel-poll schedule.
fn streaming_mirror_sides_with_schedule(
    a: Root<()>,
    b: Root<()>,
    schedule: Vec<u8>,
) -> (Root<()>, Root<()>) {
    streaming_mirror_sides_with_schedules(a, b, schedule, Vec::new())
}

/// Reconcile under independent channel and Local-backend poll schedules.
fn streaming_mirror_sides_with_schedules(
    a: Root<()>,
    b: Root<()>,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> (Root<()>, Root<()>) {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a.clone());
    let server = Handshaking::start(Local, b.clone());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("the test runtime should build");
    let (result, trace) = with_trace(|| {
        with_schedule(channel_schedule, || {
            with_local_schedule(backend_schedule, || {
                run_to_quiescence(&runtime, drive_streaming(client, server))
            })
        })
    });
    let (ours, theirs) = result
        .expect("streaming mirror became quiescent before completion")
        // `Local` is infallible, so the session's only inhabited errors are
        // violations — which two honest local endpoints must never speak.
        .expect("local mirror speaks no violations");
    trace.assert_valid();
    (ours.into(), theirs.into())
}

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides(a, b);
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile under an explicit channel-poll schedule, asserting convergence.
fn scheduled_streaming_mirror(a: Root<()>, b: Root<()>, schedule: Vec<u8>) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides_with_schedule(a, b, schedule);
    assert_eq!(
        ours, theirs,
        "scheduled streaming endpoints should converge"
    );
    ours
}

/// Reconcile under independent channel and Local-backend poll schedules.
fn fully_scheduled_streaming_mirror(
    a: Root<()>,
    b: Root<()>,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> Root<()> {
    let (ours, theirs) =
        streaming_mirror_sides_with_schedules(a, b, channel_schedule, backend_schedule);
    assert_eq!(
        ours, theirs,
        "fully scheduled streaming endpoints should converge"
    );
    ours
}

/// Reconcile `a` and `b` through the alternating implementation — the
/// behavioral oracle the streaming protocol must reproduce exactly —
/// returning both sides' roots in argument order, with no convergence
/// assertion.
fn alternating_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    pollster::block_on(async {
        let local_a = alternating::local::Exchange::start(a);
        let local_b = alternating::local::Exchange::start(b);
        match alternating::mirror(local_a, local_b).await {
            Err(e) => match e {},
            Ok(pair) => pair,
        }
    })
}

/// Reconcile `a` and `b` through the alternating oracle, asserting the two
/// sides converge to the same root, and return it.
fn alternating_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = alternating_mirror_sides(a, b);
    assert_eq!(ours, theirs, "oracle endpoints should converge");
    ours
}

/// A dispute that survives to leaf-parent height — both sides hold the same
/// `S<Z>` prefix with different leaf sets — converges to the union.
///
/// The responder's closing `uncertain` lists its leaves, and the leaf-height
/// `Closing`/`Complete` words carry the difference in both directions.
#[test]
fn converges_on_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_dispute_pair();
    assert_eq!(
        streaming_mirror(a, b),
        expected,
        "both sides should hold the union",
    );
}

/// A leaf redacted on one side under a disputed leaf-parent must disappear
/// from the other side too: the closing request for it prunes against the
/// redactor's version and drops on both sides instead of shipping.
#[test]
fn honors_redaction_under_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_redaction_pair();
    for (left, right) in [(a.clone(), b.clone()), (b, a)] {
        for (channel_schedule, backend_schedule) in [
            (Vec::new(), Vec::new()),
            (
                vec![2; 2_048],
                (0..2_048).map(|step| (step % 3) as u8).collect(),
            ),
        ] {
            assert_eq!(
                fully_scheduled_streaming_mirror(
                    left.clone(),
                    right.clone(),
                    channel_schedule,
                    backend_schedule,
                ),
                expected,
                "the redacted leaf should survive nowhere",
            );
        }
    }
}

proptest! {
    /// On divergent trees sharing causal history — matched subtrees, one-sided
    /// inserts, and redactions the other side must honor — the streaming
    /// mirror reconciles both sides to exactly the alternating oracle's root.
    #[test]
    fn matches_oracle_on_divergent_pair((a, b) in arb_divergent_pair()) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(streaming_mirror(a, b), expected);
    }

    /// On causally independent trees — including the bootstrap shape, where
    /// one side is empty and receives everything — the streaming mirror
    /// matches the alternating oracle.
    #[test]
    fn matches_oracle_on_independent_trees(
        a in arb_tree_root(0, 0..=8),
        b in arb_tree_root(1, 0..=8),
    ) {
        let expected = alternating_mirror(a.clone(), b.clone());
        prop_assert_eq!(streaming_mirror(a, b), expected);
    }

    /// Mirroring a tree with itself is a no-op: the handshake versions are
    /// equal, the session short-circuits before reconciliation, and both
    /// sides come back unchanged.
    #[test]
    fn idempotent(a in arb_tree_root(0, 0..=8)) {
        prop_assert_eq!(streaming_mirror(a.clone(), a.clone()), a);
    }
}
