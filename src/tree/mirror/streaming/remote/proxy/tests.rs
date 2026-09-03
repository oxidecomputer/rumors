//! End-to-end sessions between materialized peers and protocol-start proxies.

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::join;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::link::memory_with_capacity;
use crate::observe::SessionHandle;
use crate::testing::{
    IoPlan, IoReportHandle, IoSide, Quiescence, reorder_accepts, run_to_quiescence, wrap_link,
};
use crate::tree::mirror::handshake::{self, Intent};
use crate::tree::mirror::streaming::channel::{
    ChannelReport, QueueKind, with_observation, with_schedule,
};
use crate::tree::mirror::streaming::window::WindowConfig;
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
use crate::{
    Version,
    message::{Message, PayloadCodec, PayloadDepthLimit},
    tree::mirror::Error as MirrorError,
};

use harness::{Backends, EndpointError, Topology, codec, drive};

type BackendFailure = Failure<Infallible>;
type LocalFailure = MaterializedError<BackendFailure>;
type ProxyFailure = RemoteError<BackendFailure>;
type EndpointFailure = EndpointError<BackendFailure>;
type LeftFailure = MirrorError<LocalFailure, ProxyFailure>;
type RightFailure = MirrorError<ProxyFailure, LocalFailure>;

mod containment;
mod declarations;
mod failures;
mod greeting;
mod harness;
mod malformed;
mod transport;

/// Bytes buffered by each per-stream pipe before backpressure applies.
const TRANSPORT_CAPACITY: usize = 37;

/// Drive the production topology: each materialized local is the client of
/// its own proxy, so both physical endpoints execute `Accept` concurrently.
async fn reconcile_symmetric_accepts(
    a: TreeRoot,
    b: TreeRoot,
    transport_capacity: usize,
) -> (TreeRoot, TreeRoot) {
    let (a_link, b_link) = memory_with_capacity(transport_capacity);
    let (a, b) = drive(
        Topology::Production,
        Backends::local(),
        a,
        b,
        a_link,
        b_link,
        codec::<()>(),
        WindowConfig::FLOOR,
    )
    .await;
    (
        a.expect("endpoint A should reconcile through its proxy"),
        b.expect("endpoint B should reconcile through its proxy"),
    )
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
async fn reconcile_symmetric_accepts_reordered(
    a: TreeRoot,
    b: TreeRoot,
    transport_capacity: usize,
    reordered: Arc<AtomicUsize>,
) -> (TreeRoot, TreeRoot) {
    let (a_link, b_link) = memory_with_capacity(transport_capacity);
    let a_link = reorder_accepts(a_link, REORDER_BATCH, reordered.clone());
    let b_link = reorder_accepts(b_link, REORDER_BATCH, reordered);
    let (a, b) = drive(
        Topology::Production,
        Backends::local(),
        a,
        b,
        a_link,
        b_link,
        codec::<()>(),
        WindowConfig::FLOOR,
    )
    .await;
    (
        a.expect("endpoint A should reconcile through its proxy"),
        b.expect("endpoint B should reconcile through its proxy"),
    )
}

/// Drive the production proxy topology after the shared preamble on the same
/// transport halves, proving that neither phase consumes the other's bytes.
async fn reconcile_after_preamble<T>(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let (mut a_link, mut b_link) = memory_with_capacity(64 * 1024);
    let network = crate::Network::from_bytes([1; 16]);
    let mut a_staged = handshake::Staged::new();
    let mut b_staged = handshake::Staged::new();
    let observe = SessionHandle::default();
    let (seen_a, seen_b) = join!(
        handshake::preamble(
            network,
            Intent::Remain,
            &mut a_staged,
            &mut a_link.control_read,
            &mut a_link.control_write,
            &observe
        ),
        handshake::preamble(
            network,
            Intent::Remain,
            &mut b_staged,
            &mut b_link.control_read,
            &mut b_link.control_write,
            &observe
        ),
    );
    seen_a.expect("A preamble");
    seen_b.expect("B preamble");

    let (a, b) = drive(
        Topology::Production,
        Backends::local(),
        a,
        b,
        a_link,
        b_link,
        codec::<T>(),
        WindowConfig::FLOOR,
    )
    .await;
    (
        a.expect("endpoint A should reconcile through its proxy"),
        b.expect("endpoint B should reconcile through its proxy"),
    )
}

/// Reconcile the same pair entirely in process as the behavioral oracle.
async fn reconcile_locally(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot) {
    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
    let (a, b) = Box::pin(mirror(a, b))
        .await
        .expect("two honest local participants should reconcile");
    (a.into(), b.into())
}

/// Reconcile with exactly one proxy using the supplied failing backend.
async fn reconcile_with_failing_proxy(
    a: TreeRoot,
    b: TreeRoot,
    failing: Failing<Local>,
    fail_left: bool,
) -> (
    Result<TreeRoot, EndpointFailure>,
    Result<TreeRoot, EndpointFailure>,
) {
    reconcile_with_stacked_failures(a, b, failing, fail_left, IoPlan::default())
        .await
        .0
}

/// Reconcile with independently stackable backend and transport failures.
async fn reconcile_with_stacked_failures(
    a: TreeRoot,
    b: TreeRoot,
    failing: Failing<Local>,
    fail_left: bool,
    io_plan: IoPlan,
) -> (
    (
        Result<TreeRoot, EndpointFailure>,
        Result<TreeRoot, EndpointFailure>,
    ),
    IoReportHandle,
) {
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
    let sound = || Failing::after(Local, usize::MAX);
    let backends = Backends {
        left: sound(),
        left_proxy: if fail_left { failing.clone() } else { sound() },
        right: sound(),
        right_proxy: if fail_left { sound() } else { failing },
    };
    let results = drive(
        Topology::Production,
        backends,
        a,
        b,
        a_link,
        b_link,
        codec::<()>(),
        WindowConfig::FLOOR,
    )
    .await;
    (results, if fail_left { a_io } else { b_io })
}

/// Translate a local root into the composable failing backend's node type.
fn failing_root(root: TreeRoot) -> Root<Failing<Local>> {
    Root {
        ceiling: root.ceiling,
        root: root.root.map(FailingNode::new),
    }
}

/// Reconcile with exactly one materialized participant using the supplied
/// failing backend; both proxies keep whole backends.
async fn reconcile_with_failing_walk(
    a: TreeRoot,
    b: TreeRoot,
    failing: Failing<Local>,
    fail_left: bool,
) -> (Result<(), LeftFailure>, Result<(), RightFailure>) {
    let whole = || Failing::after(Local, usize::MAX);
    let (left_backend, right_backend) = if fail_left {
        (failing, whole())
    } else {
        (whole(), failing)
    };
    let a = Handshaking::start(left_backend, failing_root(a)).window(WindowConfig::FLOOR);
    let b = Handshaking::start(right_backend, failing_root(b)).window(WindowConfig::FLOOR);

    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    let remote_b = RemoteHandshaking::start(
        whole(),
        a_link,
        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
    )
    .window(WindowConfig::FLOOR);
    let remote_a = RemoteHandshaking::start(
        whole(),
        b_link,
        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
    )
    .window(WindowConfig::FLOOR);

    let (left, right) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
    (left.map(|_| ()), right.map(|_| ()))
}

/// The operation a failing materialized participant reported, read from the
/// endpoint that holds it.
fn walk_injected_operation(
    fail_left: bool,
    (left, right): &(Result<(), LeftFailure>, Result<(), RightFailure>),
) -> Result<Operation, TestCaseError> {
    match (fail_left, left, right) {
        (true, Err(MirrorError::Client(MaterializedError::Backend(Failure::Injected(op)))), _)
        | (false, _, Err(MirrorError::Server(MaterializedError::Backend(Failure::Injected(op))))) => {
            Ok(*op)
        }
        (_, left, right) => Err(TestCaseError::fail(format!(
            "materialized failure was masked: left {left:?}, right {right:?}",
        ))),
    }
}

/// Extract the injected backend operation from a proxy conversion failure.
fn injected_operation(error: &ProxyFailure) -> Option<Operation> {
    use crate::tree::mirror::streaming::remote::{ReplyDecodeError, ReplyEncodeError};

    match error {
        RemoteError::Encode(ReplyEncodeError::Backend(Failure::Injected(operation)))
        | RemoteError::Decode(ReplyDecodeError::Backend(Failure::Injected(operation))) => {
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
    let (a, b) = reconcile_symmetric_accepts(root.clone(), root.clone(), TRANSPORT_CAPACITY).await;
    assert_eq!(a, root);
    assert_eq!(b, root);
}

/// Concurrent version-addressed leaves cross every proxy layer and converge.
#[pollster::test]
async fn divergent_leaves_converge() {
    let mut a = Tree::<()>::new();
    a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut b = Tree::new();
    b.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let mut expected = a.clone();
    expected.join(b.clone());

    let (a, b) = reconcile_symmetric_accepts(a.root, b.root, TRANSPORT_CAPACITY).await;
    assert_eq!(a, expected.root);
    assert_eq!(b, expected.root);
}

/// The same client/proxy pairing used by both public API endpoints remains
/// live under deterministic closed-world polling.
#[test]
fn symmetric_accept_handshakes_are_live() {
    let mut a = Tree::<()>::new();
    a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut b = Tree::<()>::new();
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
    let mut a = Tree::<u64>::new();
    a.act(&a_party, [Action::Insert(Message::new(1_u64))]);
    let mut b = Tree::<u64>::new();
    b.act(&b_party, [Action::Insert(Message::new(2_u64))]);

    let (a, b) = run_to_quiescence(reconcile_after_preamble::<u64>(a.root, b.root))
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
        let divergent = a.ceiling != b.ceiling;
        let (actual, channels, trace) = instrumented_reconcile(a, b, schedule);
        let actual = actual
            .map_err(|stopped| TestCaseError::fail(format!(
                "wire reconciliation became quiescent: {stopped:?}",
            )))?;
        trace.assert_valid();
        if divergent {
            trace.assert_covers_divergent_session();
        }
        assert_proxy_channels_are_bounded(&channels);
        prop_assert_eq!(actual, expected);
    }

    /// No wire reply ever arrives before the question that scopes it was
    /// flushed.
    ///
    /// Across arbitrary divergence and adversarial channel schedules,
    /// every decode finds its scope already registered by a prior local
    /// emission — the FIFO head, never a scope that is not.
    /// This is the receive-side complement of the send-side ordering
    /// `Trace::assert_valid` pins: registration happens at encode time,
    /// attached to the exact outgoing frame which makes the question
    /// publishable, and this pins that ordering against drift.
    #[test]
    fn context_registration_is_causal(
        (a, b) in arb_divergent_pair(),
        schedule in vec(0_u8..=2, 0..128),
    ) {
        let divergent = a.ceiling != b.ceiling;
        let (result, _channels, trace) = instrumented_reconcile(a, b, schedule);
        result.map_err(|stopped| TestCaseError::fail(format!(
            "wire reconciliation became quiescent: {stopped:?}",
        )))?;
        if divergent {
            trace.assert_covers_divergent_session();
        }
        trace.assert_registration_causality();
    }

    /// The wide-budget generator matches the in-process protocol too.
    ///
    /// Wide budgets reach the streaming deadlock's trigger geometry — wide
    /// roots mixing disputes and provisions — which the small budget rarely
    /// does; this drives it at the smallest per-stream buffers.
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
            let (side, faulted) = if fail_left {
                ("left", &result.0)
            } else {
                ("right", &result.1)
            };
            let actual = match faulted {
                Err(EndpointError::Proxy(error)) => injected_operation(error),
                other => return Err(TestCaseError::fail(format!(
                    "{side} proxy failure was masked: {other:?}",
                ))),
            };
            let observed = format!("{faulted:?}");
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

    /// Every reached materialized backend failure terminates both wire
    /// endpoints and surfaces from the failing walk with its exact
    /// operation identity; every unreached failure is inert.
    ///
    /// The wire twin of `materialized_backend_failures_are_fail_fast`: a
    /// walk's error item reaches the session through the proxy pump that
    /// carries its response stream, wherever in the walk it is raised. The
    /// wide generator is what puts failures on that path: a stage loop
    /// explodes nodes only under disputed scopes, and the small generator's
    /// few leaves rarely collide into one.
    #[test]
    fn materialized_backend_failures_are_fail_fast_over_the_wire(
        (a, b) in arb_wide_divergent_pair(),
        operations in 0usize..32,
        fail_left in any::<bool>(),
        schedule in vec(0_u8..=2, 0..128),
    ) {
        let failing = Failing::after(Local, operations);
        let result = with_schedule(schedule, || {
            run_to_quiescence(reconcile_with_failing_walk(
                a,
                b,
                failing.clone(),
                fail_left,
            ))
        })
        .map_err(|stopped| TestCaseError::fail(format!(
            "materialized backend failure left the wire session quiescent: {stopped:?}",
        )))?;
        let history = failing.history();

        if let Some(expected) = history.get(operations).copied() {
            let actual = walk_injected_operation(fail_left, &result)?;
            prop_assert_eq!(actual, expected);
        } else {
            prop_assert!(result.0.is_ok(), "left endpoint failed without injection: {:?}", result.0);
            prop_assert!(result.1.is_ok(), "right endpoint failed without injection: {:?}", result.1);
        }
    }
}

/// A materialized error raised before the responder's opening reply yields
/// returns over a wire instead of stalling the joint session.
///
/// The responder's opening explodes every disputed root child before it
/// yields its one reply, so a backend failure there is an error item ahead
/// of any reply on the walk's response stream. The proxy carrying that
/// stream must observe the item without waiting on wire progress the
/// missing reply would have unlocked: the faulted endpoint reports the
/// injected operation, and its counterparty terminates on the cut.
#[test]
fn opening_failure_before_the_first_yield_returns_over_the_wire() {
    let (a, b) = early_first_child_dispute_pair();
    // The failing walk is the elected responder, whose opening explodes
    // the disputed first root child.
    let fail_left = !harness::left_initiates(&a, &b);
    // Operation 0 is the greeting's root explosion; operation 1 is the
    // opening's, at the root child's height.
    let failing = Failing::after(Local, 1);
    let result = run_to_quiescence(reconcile_with_failing_walk(
        a,
        b,
        failing.clone(),
        fail_left,
    ))
    .expect("an opening failure must terminate both sessions, not stall them");
    let expected = Operation::Children { height: 31 };
    assert_eq!(
        failing.history().get(1).copied(),
        Some(expected),
        "the failure lands in the responder's opening: {:?}",
        failing.history(),
    );
    assert_eq!(
        walk_injected_operation(fail_left, &result).unwrap_or_else(|masked| panic!("{masked}")),
        expected,
    );
    assert!(
        result.0.is_err() && result.1.is_err(),
        "the counterparty must terminate on the cut: {result:?}",
    );
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
/// genuinely firing is proven instead by its unit witness in
/// `testing::transport` (a batch of two formed by the patient wait) and by
/// the link conformance suite, which runs under the same decorator with
/// its probes connecting streams concurrently.
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
         undersells it: rewrite the claims and flip this tripwire to > 0",
    );
}

/// The streaming wire deadlock's trigger geometry reconciles cleanly.
///
/// The deterministic shape — the radix-first root child disputed with
/// branching content while six-plus provisions queue behind it — must
/// reconcile to the materialized oracle's result under closed-world
/// polling.
///
/// This pins the wait-cycle trigger geometry at the tier that owns stream
/// scheduling. Liveness for this shape may not depend on transport buffer
/// capacity (the link contract admits any positive capacity), so the
/// standard small capacity here is the demanding case and loses no
/// coverage.
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
    let mut a = Tree::<()>::new();
    a.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut b = Tree::<()>::new();
    b.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let (result, report, trace) = instrumented_reconcile(a.root, b.root, Vec::new());
    result.expect("the instrumented wire session should remain live");
    trace.assert_valid();
    trace.assert_covers_divergent_session();
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
    a: TreeRoot,
    b: TreeRoot,
    schedule: Vec<u8>,
) -> (
    Result<(TreeRoot, TreeRoot), Quiescence>,
    ChannelReport,
    Trace,
) {
    let ((result, channels), trace) = with_trace(|| {
        with_observation(|| {
            with_schedule(schedule, || {
                run_to_quiescence(reconcile_symmetric_accepts(a, b, TRANSPORT_CAPACITY))
            })
        })
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
