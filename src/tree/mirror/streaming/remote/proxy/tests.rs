//! End-to-end sessions between materialized peers and protocol-start proxies.

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::join;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::link::memory_with_capacity;
use crate::testing::{
    IoPlan, IoReportHandle, IoSide, Quiescence, reorder_accepts, run_to_quiescence, wrap_link,
};
use crate::tree::mirror::handshake::{self, Intent};
use crate::tree::mirror::streaming::channel::{
    ChannelReport, QueueKind, with_observation, with_schedule,
};
use crate::tree::{
    Action, Root as TreeRoot, Tree,
    arb::{arb_divergent_pair, arb_wide_divergent_pair, early_first_child_dispute_pair, nth_party},
    mirror::streaming::{
        Failing, FailingNode, Failure, Local, Operation, Root,
        materialized::{Error as MaterializedError, Handshaking},
        mirror,
        remote::{
            Error as RemoteError, Handshaking as RemoteHandshaking,
            proxy::work::progress::{Trace, with_trace},
        },
    },
};
use crate::{Version, message::Message, tree::mirror::Error as MirrorError};

type BackendFailure = Failure<Infallible>;
type LocalFailure = MaterializedError<BackendFailure>;
type ProxyFailure = RemoteError<BackendFailure>;
type LeftFailure = MirrorError<LocalFailure, ProxyFailure>;
type RightFailure = MirrorError<ProxyFailure, LocalFailure>;

mod failures;
mod harness;
mod malformed;
mod transport;

/// Bytes buffered by each per-stream pipe before backpressure applies.
const TRANSPORT_CAPACITY: usize = 37;

/// Drive two local starts, each paired directly with its remote protocol start.
async fn reconcile(a: TreeRoot<()>, b: TreeRoot<()>) -> (TreeRoot<()>, TreeRoot<()>) {
    let a = Handshaking::start(Local, Root::from(a));
    let b = Handshaking::start(Local, Root::from(b));

    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    let remote_b = RemoteHandshaking::start(Local, a_link);
    let remote_a = RemoteHandshaking::start(Local, b_link);

    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
    let (_control, b) = b.expect("endpoint B should reconcile through its proxy");
    (a.into(), b.into())
}

/// Drive the production topology: each materialized local is the client of
/// its own proxy, so both physical endpoints execute `Accept` concurrently.
async fn reconcile_symmetric_accepts<T>(
    a: TreeRoot<T>,
    b: TreeRoot<T>,
    transport_capacity: usize,
) -> (TreeRoot<T>, TreeRoot<T>)
where
    T: borsh::BorshDeserialize + Send + Sync + 'static,
{
    let a = Handshaking::start(Local, Root::from(a));
    let b = Handshaking::start(Local, Root::from(b));
    let (a_link, b_link) = memory_with_capacity(transport_capacity);
    let remote_b = RemoteHandshaking::start(Local, a_link);
    let remote_a = RemoteHandshaking::start(Local, b_link);

    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)),);
    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
    let (b, _control) = b.expect("endpoint B should reconcile through its proxy");
    (a.into(), b.into())
}

/// Arrivals held and released newest-first by the reordering acceptor: deep
/// enough to invert most bursts, small enough that batching never starves a
/// stream.
const REORDER_BATCH: usize = 3;

/// [`reconcile_symmetric_accepts`] with both acceptors delivering arrivals
/// in reversed batches: worst-case-legal stream reordering on both ends.
///
/// `reordered` counts the genuine inversions both ends release; the caller
/// asserts its disposition across the run.
async fn reconcile_symmetric_accepts_reordered<T>(
    a: TreeRoot<T>,
    b: TreeRoot<T>,
    transport_capacity: usize,
    reordered: Arc<AtomicUsize>,
) -> (TreeRoot<T>, TreeRoot<T>)
where
    T: borsh::BorshDeserialize + Send + Sync + 'static,
{
    let a = Handshaking::start(Local, Root::from(a));
    let b = Handshaking::start(Local, Root::from(b));
    let (a_link, b_link) = memory_with_capacity(transport_capacity);
    let a_link = reorder_accepts(a_link, REORDER_BATCH, reordered.clone());
    let b_link = reorder_accepts(b_link, REORDER_BATCH, reordered);
    let remote_b = RemoteHandshaking::start(Local, a_link);
    let remote_a = RemoteHandshaking::start(Local, b_link);

    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)),);
    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
    let (b, _control) = b.expect("endpoint B should reconcile through its proxy");
    (a.into(), b.into())
}

/// Drive the production proxy topology after the shared preamble on the same
/// transport halves, proving that neither phase consumes the other's bytes.
async fn reconcile_after_preamble<T>(a: TreeRoot<T>, b: TreeRoot<T>) -> (TreeRoot<T>, TreeRoot<T>)
where
    T: borsh::BorshDeserialize + Send + Sync + 'static,
{
    let a = Handshaking::start(Local, Root::from(a));
    let b = Handshaking::start(Local, Root::from(b));
    let (mut a_link, mut b_link) = memory_with_capacity(64 * 1024);
    let network = crate::Network::from_bytes([1; 16]);
    let mut a_staged = handshake::Staged::new();
    let mut b_staged = handshake::Staged::new();
    let (seen_a, seen_b) = join!(
        handshake::preamble(
            crate::Protocol::V2,
            network,
            Intent::Remain,
            &mut a_staged,
            &mut a_link.control_read,
            &mut a_link.control_write,
        ),
        handshake::preamble(
            crate::Protocol::V2,
            network,
            Intent::Remain,
            &mut b_staged,
            &mut b_link.control_read,
            &mut b_link.control_write,
        ),
    );
    seen_a.expect("A preamble");
    seen_b.expect("B preamble");

    let remote_b = RemoteHandshaking::start(Local, a_link);
    let remote_a = RemoteHandshaking::start(Local, b_link);
    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)),);
    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
    let (b, _control) = b.expect("endpoint B should reconcile through its proxy");
    (a.into(), b.into())
}

/// Reconcile the same pair entirely in process as the behavioral oracle.
async fn reconcile_locally(a: TreeRoot<()>, b: TreeRoot<()>) -> (TreeRoot<()>, TreeRoot<()>) {
    let a = Handshaking::start(Local, Root::from(a));
    let b = Handshaking::start(Local, Root::from(b));
    let (a, b) = Box::pin(mirror(a, b))
        .await
        .expect("two honest local participants should reconcile");
    (a.into(), b.into())
}

/// Translate a local root into the composable failing backend's node type.
fn failing_root(root: TreeRoot<()>) -> Root<Failing<Local>, ()> {
    Root {
        ceiling: root.ceiling,
        root: root.root.map(FailingNode::new),
    }
}

/// Reconcile with exactly one proxy using the supplied failing backend.
async fn reconcile_with_failing_proxy(
    a: TreeRoot<()>,
    b: TreeRoot<()>,
    failing: Failing<Local>,
    fail_left: bool,
) -> (Result<(), LeftFailure>, Result<(), RightFailure>) {
    reconcile_with_stacked_failures(a, b, failing, fail_left, IoPlan::default())
        .await
        .0
}

/// Reconcile with independently stackable backend and transport failures.
async fn reconcile_with_stacked_failures(
    a: TreeRoot<()>,
    b: TreeRoot<()>,
    failing: Failing<Local>,
    fail_left: bool,
    io_plan: IoPlan,
) -> (
    (Result<(), LeftFailure>, Result<(), RightFailure>),
    IoReportHandle,
) {
    let a = Handshaking::start(Failing::after(Local, usize::MAX), failing_root(a));
    let b = Handshaking::start(Failing::after(Local, usize::MAX), failing_root(b));

    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    let (a_link, a_io) = wrap_link(
        IoSide::Left,
        if fail_left {
            io_plan.clone()
        } else {
            IoPlan::default()
        },
        a_link,
    );
    let (b_link, b_io) = wrap_link(
        IoSide::Right,
        if fail_left {
            IoPlan::default()
        } else {
            io_plan
        },
        b_link,
    );
    let left_backend = if fail_left {
        failing.clone()
    } else {
        Failing::after(Local, usize::MAX)
    };
    let right_backend = if fail_left {
        Failing::after(Local, usize::MAX)
    } else {
        failing
    };
    let remote_b = RemoteHandshaking::start(left_backend, a_link);
    let remote_a = RemoteHandshaking::start(right_backend, b_link);

    let (left, right) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
    (
        (left.map(|_| ()), right.map(|_| ())),
        if fail_left { a_io } else { b_io },
    )
}

/// Extract the injected backend operation from a proxy conversion failure.
fn injected_operation(error: &ProxyFailure) -> Option<Operation> {
    use crate::tree::mirror::streaming::remote::{DecodeError, EncodeError};

    match error {
        RemoteError::Encode(EncodeError::Backend(Failure::Injected(operation)))
        | RemoteError::Decode(DecodeError::Backend(Failure::Injected(operation))) => {
            Some(*operation)
        }
        _ => None,
    }
}

/// Equal versions close every unused logical stream without opening descent.
#[pollster::test]
async fn equal_versions_return_both_roots() {
    let root = TreeRoot {
        ceiling: Version::new(),
        root: None,
    };
    let (a, b) = reconcile(root.clone(), root.clone()).await;
    assert_eq!(a, root);
    assert_eq!(b, root);
}

/// Concurrent content-addressed leaves cross every proxy layer and converge.
#[pollster::test]
async fn divergent_leaves_converge() {
    let mut a = Tree::new();
    a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut b = Tree::new();
    b.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let mut expected = a.clone();
    expected.join(b.clone());

    let (a, b) = reconcile(a.root, b.root).await;
    assert_eq!(a, expected.root);
    assert_eq!(b, expected.root);
}

/// The same client/proxy pairing used by both public API endpoints remains
/// live under deterministic closed-world polling.
#[test]
fn symmetric_accept_handshakes_are_live() {
    let mut a = Tree::new();
    a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut b = Tree::new();
    b.act(&nth_party(1), [Action::Insert(Message::new(()))]);

    let (a, b) = run_to_quiescence(reconcile_symmetric_accepts(a.root, b.root, 1))
        .expect("the production proxy topology became quiescent");
    assert_eq!(a, b);
}

/// Distinct payloads exercise supplied-leaf paths different from the unit
/// payload used by the broad protocol properties.
#[test]
fn symmetric_accepts_with_distinct_payloads_are_live() {
    let mut a_party = before::Party::seed();
    let b_party = a_party.fork();
    let mut a = Tree::new();
    a.act(&a_party, [Action::Insert(Message::new(1_u64))]);
    let mut b = Tree::new();
    b.act(&b_party, [Action::Insert(Message::new(2_u64))]);

    let (a, b) = run_to_quiescence(reconcile_after_preamble(a.root, b.root))
        .expect("distinct-payload proxy topology became quiescent");
    assert_eq!(a, b);
}

proptest! {
    /// The production topology, in which both endpoints connect their local
    /// participant to an accepting proxy concurrently, remains live and
    /// matches the materialized protocol for arbitrary valid divergence.
    #[test]
    fn symmetric_accepts_match_local((a, b) in arb_divergent_pair()) {
        let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
            .expect("local reconciliation should remain live");
        let actual = run_to_quiescence(reconcile_symmetric_accepts(a, b, TRANSPORT_CAPACITY))
            .map_err(|stopped| TestCaseError::fail(format!(
                "symmetric proxy reconciliation became quiescent: {stopped:?}",
            )))?;
        prop_assert_eq!(actual, expected);
    }

    /// For arbitrary valid divergence, crossing the codec and per-stream
    /// transport is observationally identical to the in-process protocol.
    #[test]
    fn wire_reconciliation_matches_local(
        (a, b) in arb_divergent_pair(),
        schedule in vec(0_u8..=2, 0..128),
    ) {
        let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
        .expect("local reconciliation should remain live");
        let (actual, channels, trace) = instrumented_reconcile(a, b, schedule);
        let actual = actual
            .map_err(|stopped| TestCaseError::fail(format!(
                "wire reconciliation became quiescent: {stopped:?}",
            )))?;
        trace.assert_valid();
        assert_proxy_channels_are_bounded(&channels);
        prop_assert_eq!(actual, expected);
    }

    /// The wide-budget generator matches the in-process protocol too.
    ///
    /// Wide budgets reach the streaming deadlock's trigger geometry — wide
    /// roots mixing disputes and provisions — which the small budget rarely
    /// does (see `design/streaming-wire-deadlock.md` §6–7); this drives it
    /// at the smallest per-stream buffers.
    #[test]
    fn wide_symmetric_accepts_match_local((a, b) in arb_wide_divergent_pair()) {
        let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
            .expect("local reconciliation should remain live");
        let actual = run_to_quiescence(reconcile_symmetric_accepts(a, b, 1))
            .map_err(|stopped| TestCaseError::fail(format!(
                "wide symmetric proxy reconciliation became quiescent: {stopped:?}",
            )))?;
        prop_assert_eq!(actual, expected);
    }

    /// Every reached proxy backend failure terminates both endpoints and
    /// survives transport cancellation with its exact operation identity.
    #[test]
    fn proxy_backend_failures_are_fail_fast(
        (a, b) in arb_divergent_pair(),
        operations in 0usize..32,
        fail_left in any::<bool>(),
        schedule in vec(0_u8..=2, 0..128),
    ) {
        let failing = Failing::after(Local, operations);
        let result = with_schedule(schedule, || {
            run_to_quiescence(reconcile_with_failing_proxy(
                a,
                b,
                failing.clone(),
                fail_left,
            ))
        })
        .map_err(|stopped| TestCaseError::fail(format!(
            "backend failure left the wire session quiescent: {stopped:?}",
        )))?;
        let history = failing.history();

        if let Some(expected) = history.get(operations).copied() {
            let actual = if fail_left {
                match &result.0 {
                    Err(MirrorError::Server(error)) => injected_operation(error),
                    other => return Err(TestCaseError::fail(format!(
                        "left proxy failure was masked: {other:?}",
                    ))),
                }
            } else {
                match &result.1 {
                    Err(MirrorError::Client(error)) => injected_operation(error),
                    other => return Err(TestCaseError::fail(format!(
                        "right proxy failure was masked: {other:?}",
                    ))),
                }
            };
            let observed = if fail_left {
                format!("{:?}", result.0)
            } else {
                format!("{:?}", result.1)
            };
            prop_assert_eq!(
                actual,
                Some(expected),
                "proxy failure was masked by {}",
                observed,
            );
        } else {
            prop_assert!(result.0.is_ok(), "left endpoint failed without injection: {:?}", result.0);
            prop_assert!(result.1.is_ok(), "right endpoint failed without injection: {:?}", result.1);
        }
    }
}

/// Wide-budget divergence still matches the materialized oracle with both
/// acceptors decorated for reversed-batch delivery at one-byte windows —
/// with the honest caveat that the reordering provably never fires here.
///
/// In this topology no inversion is reachable: the deterministic driver
/// joins two whole-endpoint futures, so while an accept holds its first
/// arrival waiting for a second, this endpoint's own control writes are
/// suspended — and the peer needs exactly those bytes before it opens its
/// next stream. A second in-flight arrival can never materialize (verified
/// empirically: zero batches across the run, at any patience budget and at
/// one-byte and 37-byte windows alike), so as exercised this property pins
/// no more than [`reconcile_symmetric_accepts`]; the decorator's inversion
/// genuinely firing is proven instead by the conformance suite's
/// `ReversingAcceptor` tests, whose probes connect streams concurrently.
///
/// The final assertion is the tripwire keeping this caveat honest: if the
/// topology ever admits a genuine inversion, it fails, and this doc's
/// claims must be rewritten upward. The test runs against a manual
/// [`proptest::test_runner::TestRunner`] rather than the `proptest!` macro
/// so that assertion can run once, after every case.
#[test]
fn wide_symmetric_accepts_reordered_match_local() {
    /// Cases for this property, fewer than the default.
    ///
    /// The wide generator plus the decorator's added accept latency make
    /// each case expensive, and the trigger geometry is pinned separately
    /// by the deterministic [`early_first_child_dispute_pair`] fixture.
    const CASES: u32 = 48;

    let reordered = Arc::new(AtomicUsize::new(0));
    let mut config = ProptestConfig {
        cases: CASES,
        ..ProptestConfig::default()
    };
    config.source_file = Some(file!());
    let mut runner = proptest::test_runner::TestRunner::new(config);
    let counter = reordered.clone();
    let cases = runner.run(&arb_wide_divergent_pair(), move |(a, b)| {
        let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
            .expect("local reconciliation should remain live");
        let actual = run_to_quiescence(reconcile_symmetric_accepts_reordered(
            a,
            b,
            1,
            counter.clone(),
        ))
        .map_err(|stopped| {
            TestCaseError::fail(format!(
                "reordered symmetric proxy reconciliation became quiescent: {stopped:?}",
            ))
        })?;
        prop_assert_eq!(actual, expected);
        Ok(())
    });
    if let Err(failure) = cases {
        panic!("{failure}\n{runner}");
    }
    assert_eq!(
        reordered.load(Ordering::Relaxed),
        0,
        "the topology now admits a genuine inversion: this test's doc \
         undersells it — rewrite the claims and flip this tripwire to > 0",
    );
}

/// The streaming wire deadlock's trigger geometry reconciles cleanly.
///
/// The deterministic shape — the radix-first root child disputed with
/// branching content while six-plus provisions queue behind it — must
/// reconcile to the materialized oracle's result under closed-world
/// polling.
///
/// This is `design/streaming-wire-deadlock.md` §2's counterexample made
/// permanent at the tier that should have owned it: under the deleted
/// mux/demux session layer this exact shape deadlocked (independently of
/// buffer size — the original stall reproduced from 64-byte to 16 MiB
/// buffers, so the standard small capacity here loses no coverage).
#[test]
fn early_first_child_dispute_is_live() {
    let (a, b) = early_first_child_dispute_pair();
    let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
        .expect("local reconciliation of the trigger geometry remains live");
    let (left, right) = run_to_quiescence(reconcile_symmetric_accepts(a, b, TRANSPORT_CAPACITY))
        .expect("the trigger geometry must reconcile over the wire");
    assert_eq!((left, right), expected);
}

/// Every proxy queue kind is exercised and remains within its one-slot bound.
#[test]
fn instrumented_channels_cover_every_proxy_edge() {
    let mut a = Tree::new();
    a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut b = Tree::new();
    b.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let (result, report, trace) = instrumented_reconcile(a.root, b.root, Vec::new());
    result.expect("the instrumented wire session should remain live");
    trace.assert_valid();
    assert_proxy_channels_are_bounded(&report);
    for kind in QueueKind::PROXY {
        assert!(
            report.kind(kind).channels > 0,
            "proxy queue kind {kind:?} was not exercised",
        );
    }
}

/// Reconcile once under channel scheduling while collecting both instruments.
fn instrumented_reconcile(
    a: TreeRoot<()>,
    b: TreeRoot<()>,
    schedule: Vec<u8>,
) -> (
    Result<(TreeRoot<()>, TreeRoot<()>), Quiescence>,
    ChannelReport,
    Trace,
) {
    let ((result, channels), trace) = with_trace(|| {
        with_observation(|| with_schedule(schedule, || run_to_quiescence(reconcile(a, b))))
    });
    (result, channels, trace)
}

/// Every observed proxy queue retains at most its documented single item.
fn assert_proxy_channels_are_bounded(report: &ChannelReport) {
    for (role, stats) in report
        .roles()
        .filter(|(role, _)| QueueKind::PROXY.contains(&role.kind))
    {
        assert_eq!(
            stats.effective_capacity, 1,
            "unexpected capacity for {role:?}"
        );
        assert!(stats.high_water <= 1, "queue {role:?} exceeded one item");
        assert_eq!(
            stats.sends, stats.receives,
            "queue {role:?} did not drain cleanly",
        );
    }
}
