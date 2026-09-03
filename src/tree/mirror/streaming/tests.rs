//! Honest-peer behavior and the shared in-memory driver harness.
//!
//! Capacity/scheduling stress lives in [`capacity`], connected abort and
//! lifecycle checks in [`faults`], and deterministic tree builders in
//! [`fixtures`].

use std::{
    cell::Cell,
    convert::Infallible,
    future::{self, Future},
    pin::pin,
    task::{Context, Poll, Waker},
};

use proptest::prelude::*;

use super::driver::try_join_mapped;
use crate::testing::{Quiescence, node_census, node_census_reset, run_to_quiescence};
use crate::tree::arb::{
    arb_divergent_pair, arb_tree_root, leaf_parent_dispute_pair, leaf_parent_redaction_pair,
    uncontained_supply_pair,
};
use crate::tree::mirror::streaming::backend::with_local_schedule;
use crate::tree::mirror::streaming::materialized::channel::{
    QueueKind, with_kind_capacity, with_schedule,
};
use crate::tree::mirror::streaming::materialized::progress::{Trace, with_trace};
use crate::tree::mirror::streaming::materialized::transcript::{Transcript, with_transcript};
use crate::tree::mirror::streaming::materialized::{Error as MaterializedError, Start};
use crate::tree::mirror::streaming::stats::{Recorder, SessionStats};
use crate::tree::mirror::streaming::window::{DEFAULT_SYNC_MEMORY_BUDGET, Window, WindowConfig};
use crate::tree::mirror::streaming::{Local, materialized::Handshaking, mirror as drive_streaming};
use crate::tree::{Root, Tree, mirror::Error as MirrorError};

mod announced;
mod capacity;
mod faults;
mod fixtures;
mod local_eq;
mod skeleton;
mod stats;
mod wedge;

/// Either terminal error preempts a peer which can no longer make progress.
#[test]
fn terminal_errors_preempt_parked_peers() {
    let left = try_join_mapped(
        future::ready(Err::<(), _>("left")),
        MirrorError::<&str, Infallible>::Client,
        future::pending::<Result<(), Infallible>>(),
        MirrorError::Server,
    );
    assert!(matches!(
        run_to_quiescence(left),
        Ok(Err(MirrorError::Client("left")))
    ));

    let right = try_join_mapped(
        future::pending::<Result<(), Infallible>>(),
        MirrorError::Client,
        future::ready(Err::<(), _>("right")),
        MirrorError::<Infallible, &str>::Server,
    );
    assert!(matches!(
        run_to_quiescence(right),
        Ok(Err(MirrorError::Server("right")))
    ));
}

/// The failure of a session between two `Local` endpoints: the backend is
/// infallible, so only protocol violations are inhabited.
type LocalSessionError = MirrorError<MaterializedError<Infallible>, MaterializedError<Infallible>>;

/// How a local session ended: both reconciled roots in argument order, a
/// violation, or quiescence before either.
type Verdict = Result<Result<(Root, Root), LocalSessionError>, Quiescence>;

/// One session between two `Local` endpoints under closed-world polling,
/// with the instruments a test asks for attached.
///
/// The floor window is the default; every schedule, capacity, and
/// instrument setting is optional and independent of the others.
struct LocalSession {
    client: Root,
    server: Root,
    window: WindowConfig,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
    kind_capacity: Option<(QueueKind, usize)>,
    stats: bool,
    trace: bool,
    transcript: bool,
}

impl LocalSession {
    /// A session in which `client` connects to `server`, at the floor
    /// window, with no instrument attached.
    fn new(client: Root, server: Root) -> Self {
        Self {
            client,
            server,
            window: WindowConfig::FLOOR,
            channel_schedule: Vec::new(),
            backend_schedule: Vec::new(),
            kind_capacity: None,
            stats: false,
            trace: false,
            transcript: false,
        }
    }

    /// Run at `window` instead of the floor.
    fn window(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }

    /// Delay channel polls by `schedule`, one entry per poll boundary.
    fn channel_schedule(mut self, schedule: Vec<u8>) -> Self {
        self.channel_schedule = schedule;
        self
    }

    /// Delay `Local` backend polls by `schedule`, one entry per poll
    /// boundary.
    fn backend_schedule(mut self, schedule: Vec<u8>) -> Self {
        self.backend_schedule = schedule;
        self
    }

    /// Cap every height of one queue kind at `limit` slots.
    fn kind_capacity(mut self, kind: QueueKind, limit: usize) -> Self {
        self.kind_capacity = Some((kind, limit));
        self
    }

    /// Record both sides' session stats.
    fn stats(mut self) -> Self {
        self.stats = true;
        self
    }

    /// Record the materialized publication trace.
    fn trace(mut self) -> Self {
        self.trace = true;
        self
    }

    /// Record the payload-erased wire transcript.
    fn transcript(mut self) -> Self {
        self.transcript = true;
        self
    }

    /// Drive the session until it completes or becomes quiescent.
    fn run(self) -> Outcome {
        let Self {
            client,
            server,
            window,
            channel_schedule,
            backend_schedule,
            kind_capacity,
            stats,
            trace,
            transcript,
        } = self;
        let recorders = stats.then(|| (Recorder::default(), Recorder::default()));
        let mut client = Handshaking::start(Local, client.into()).window(window);
        let mut server = Handshaking::start(Local, server.into()).window(window);
        if let Some((client_recorder, server_recorder)) = &recorders {
            client = client.stats(client_recorder.clone());
            server = server.stats(server_recorder.clone());
        }

        let run = move || run_to_quiescence(drive_streaming(client, server));
        let run = move || {
            with_schedule(channel_schedule, move || {
                with_local_schedule(backend_schedule, run)
            })
        };
        let run = move || match kind_capacity {
            Some((kind, limit)) => with_kind_capacity(kind, limit, run),
            None => run(),
        };
        let run = move || {
            if trace {
                let (verdict, trace) = with_trace(run);
                (verdict, Some(trace))
            } else {
                (run(), None)
            }
        };
        let ((verdict, trace), transcript) = if transcript {
            let (traced, transcript) = with_transcript(run);
            (traced, Some(transcript))
        } else {
            (run(), None)
        };

        Outcome {
            verdict: verdict
                .map(|session| session.map(|(ours, theirs)| (ours.into(), theirs.into()))),
            stats: recorders.map(|(client, server)| (client.snapshot(), server.snapshot())),
            trace,
            transcript,
        }
    }
}

/// What a [`LocalSession`] run produced: its verdict and the instruments it
/// was asked to attach.
struct Outcome {
    verdict: Verdict,
    stats: Option<(SessionStats, SessionStats)>,
    trace: Option<Trace>,
    transcript: Option<Transcript>,
}

impl Outcome {
    /// Both reconciled roots in argument order; a violation, a stall, and
    /// an exhausted poll budget each fail by name.
    fn sides(self) -> (Root, Root) {
        match self.verdict {
            Ok(Ok(sides)) => sides,
            Ok(Err(error)) => panic!("local mirror speaks no violations: {error:?}"),
            Err(Quiescence::Stalled) => panic!("streaming mirror stalled before completion"),
            Err(Quiescence::PollBudget) => {
                panic!("streaming mirror exhausted its poll budget before completion")
            }
        }
    }

    /// The one root both sides converged to.
    fn converged(self) -> Root {
        let (ours, theirs) = self.sides();
        assert_eq!(ours, theirs, "streaming endpoints should converge");
        ours
    }

    /// Both sides' stats in argument order, which the session was asked
    /// to record.
    fn stats(&self) -> &(SessionStats, SessionStats) {
        self.stats
            .as_ref()
            .expect("the session was asked to record its stats")
    }

    /// The publication trace, which the session was asked to record.
    fn trace(&self) -> &Trace {
        self.trace
            .as_ref()
            .expect("the session was asked to record its trace")
    }
}

/// A `Local` endpoint at the floor window, for the sites that hold the
/// undriven session: the ones that wrap it in a fault or failure decorator,
/// and the ones that poll it themselves.
fn floor_start(root: Root) -> Handshaking<Local, Start> {
    Handshaking::start(Local, root.into()).window(WindowConfig::FLOOR)
}

/// Reconcile `a` and `b` through the streaming local backend, returning both
/// sides' reconciled roots in argument order, with no convergence assertion.
fn streaming_mirror_sides(a: Root, b: Root) -> (Root, Root) {
    let outcome = LocalSession::new(a, b).trace().run();
    outcome.trace().assert_valid();
    outcome.sides()
}

/// Reconcile through the local backend, returning both roots, the validated
/// publication trace, and the payload-erased wire transcript.
fn transcribed_mirror_sides(a: Root, b: Root) -> (Root, Root, Trace, Transcript) {
    let mut outcome = LocalSession::new(a, b).trace().transcript().run();
    let trace = outcome
        .trace
        .take()
        .expect("the session was asked to record its trace");
    let transcript = outcome
        .transcript
        .take()
        .expect("the session was asked to record its transcript");
    trace.assert_valid();
    let (ours, theirs) = outcome.sides();
    (ours, theirs, trace, transcript)
}

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: Root, b: Root) -> Root {
    let (ours, theirs) = streaming_mirror_sides(a, b);
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile under independent channel and Local-backend poll schedules,
/// with the publication trace validated, asserting convergence.
fn fully_scheduled_streaming_mirror(
    a: Root,
    b: Root,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> Root {
    let outcome = LocalSession::new(a, b)
        .channel_schedule(channel_schedule)
        .backend_schedule(backend_schedule)
        .trace()
        .run();
    outcome.trace().assert_valid();
    outcome.converged()
}

/// Merge `a` and `b` through `Tree::join`: the in-memory oracle.
///
/// Its observational equivalence to wire reconciliation is the design of
/// record: `join` and the mirror delegate deletion honoring to the same
/// filter, so a reconciled endpoint must hold exactly the joined tree.
fn join_oracle(a: Root, b: Root) -> Root {
    let mut joined = Tree::<()>::from_root(a);
    joined.join(Tree::from_root(b));
    joined.root
}

/// Generated relationships for the independent join oracle.
///
/// The union covers shared-history divergence (including redactions and
/// matched subtrees), independent party histories (including empty bootstrap
/// shapes), and the equal-version short circuit.
fn arb_oracle_pair() -> impl Strategy<Value = (Root, Root)> {
    prop_oneof![
        4 => arb_divergent_pair(),
        2 => (arb_tree_root(0, 0..=8), arb_tree_root(1, 0..=8)),
        1 => arb_tree_root(0, 0..=8).prop_map(|root| (root.clone(), root)),
    ]
}

/// The corpus size the wide window is solved for: far past what the
/// generators produce, so the top stages get capacities above the floor.
const WIDE_CORPUS: u64 = 1_000_000;

/// The window the wide oracle arm runs under and its widest capacity: the
/// budget solve at [`WIDE_CORPUS`], asserted wider than the floor rather
/// than assumed so.
fn wide_window() -> (WindowConfig, u64) {
    let window = Window::from_budget(
        WIDE_CORPUS,
        WIDE_CORPUS,
        0,
        0,
        DEFAULT_SYNC_MEMORY_BUDGET,
        Local::node_bytes,
    );
    let widest = window.widest();
    assert!(
        widest > 1,
        "the wide arm's window solved to the floor: it exercises nothing the floor arm does not"
    );
    (WindowConfig::Fixed(window), widest)
}

/// Poll `future` at most `polls` times, then drop it: a session cancelled
/// at a drawn point. Returns the output when it completed within the
/// budget.
fn cancel_after<F: Future>(future: F, polls: usize) -> Option<F::Output> {
    let mut future = pin!(tokio::task::coop::unconstrained(future));
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..polls {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return Some(output);
        }
    }
    None
}

/// The polls a session between `a` and `b` takes to complete under the
/// same polling `cancel_after` applies: the length every drawn
/// cancellation point stays below.
fn session_length(a: Root, b: Root) -> usize {
    const MAX_POLLS: usize = 1_000_000;
    let mut session = pin!(tokio::task::coop::unconstrained(drive_streaming(
        floor_start(a),
        floor_start(b),
    )));
    let mut cx = Context::from_waker(Waker::noop());
    (1..=MAX_POLLS)
        .find(|_| session.as_mut().poll(&mut cx).is_ready())
        .expect("a local session completes within the poll budget")
}

/// A generated pair with its measured session length and a cancellation
/// point drawn below it, so every case with a session longer than one
/// poll cancels mid-session.
fn arb_cancellation() -> impl Strategy<Value = ((Root, Root), usize, usize)> {
    arb_oracle_pair().prop_flat_map(|(a, b)| {
        let length = session_length(a.clone(), b.clone());
        (Just((a, b)), Just(length), 1..length.max(2))
    })
}

/// Cancelling a session at a drawn poll before its measured completion
/// leaves nothing behind.
///
/// Every node handle the session built is released (the census returns
/// to its baseline), and a fresh session over the same inputs reaches the
/// join oracle.
/// The poll count is drawn in `1..length`, `length` being the session's
/// measured poll count, so every case with a session longer than one poll
/// cancels.
///
/// The run asserts that some case cancelled after the walk had built node
/// handles, which is where a cancellation could leak.
#[test]
fn cancelled_session_leaves_no_residue() {
    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
        source_file: Some(file!()),
        ..ProptestConfig::default()
    });
    let cancelled_after_building = Cell::new(0usize);
    let cases = runner.run(&arb_cancellation(), |((a, b), length, polls)| {
        let expected = join_oracle(a.clone(), b.clone());
        let baseline = node_census().live;
        let session = drive_streaming(floor_start(a.clone()), floor_start(b.clone()));
        node_census_reset();
        let before_polling = node_census().live;
        let completed = cancel_after(session, polls);
        let cancelled = completed.is_none();
        drop(completed);
        let census = node_census();
        prop_assert_eq!(
            cancelled,
            polls < length,
            "the measured session length is not reproducible"
        );
        prop_assert_eq!(
            census.live,
            baseline,
            "a cancelled session must release every node handle it built"
        );
        if cancelled && census.peak > before_polling {
            cancelled_after_building.set(cancelled_after_building.get() + 1);
        }
        let (ours, theirs) = streaming_mirror_sides(a, b);
        prop_assert_eq!(&ours, &expected);
        prop_assert_eq!(&theirs, &expected);
        Ok(())
    });
    if let Err(failure) = cases {
        panic!("{failure}\n{runner}");
    }
    assert!(
        cancelled_after_building.get() > 0,
        "no case cancelled a session after it had built node handles: the pin exercised nothing"
    );
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

/// A supplied leaf whose version escapes the sender's declared greeting
/// version fails the streaming session with a typed violation instead of
/// being absorbed.
///
/// The declared version of an honest replica contains every version it
/// transmits, so this shape marks a nonconforming implementation. Absorbed,
/// the escaped leaf would sit above every replica's session ceiling —
/// unredactable and re-shipped forever (the mechanism is pinned at the
/// tree tier by `escaped_version_defeats_redaction_in_a_poisoned_store`).
/// The wire twin (`uncontained_supply_is_rejected_at_the_wire`) drives the
/// same rejection through the frame codec and supply decoder.
#[test]
fn uncontained_supply_is_rejected_by_streaming() {
    use crate::tree::mirror::streaming::materialized::{Error, Violation};

    let (receiver, poisoned, _, _) = uncontained_supply_pair();
    let result = LocalSession::new(receiver, poisoned)
        .run()
        .verdict
        .expect("the rejecting session becomes quiescent");
    assert!(
        matches!(
            result,
            Err(MirrorError::Client(Error::Violation(
                Violation::UncontainedSupply
            ))),
        ),
        "the receiving side rejects the escaped leaf",
    );
}

proptest! {
    /// Streaming and the in-memory join oracle agree in both orientations,
    /// at the floor window and at a wide one.
    ///
    /// Across every generated causal relationship, both wire endpoints
    /// converge to exactly `Tree::join` of the two inputs. This ties the
    /// wire reconciliation to the in-memory merge the convergence suites
    /// are stated over: join prunes through `traverse::unknown` and the
    /// session through `materialized::unknown`, two implementations of one
    /// deletion-honoring contract, so a divergence here is a bug in one of
    /// them. The wide arm runs the pipeline window's real behavior, and
    /// each session reports the width it was granted so the arm cannot
    /// silently run at the floor.
    #[test]
    fn streaming_matches_join_oracle((a, b) in arb_oracle_pair(), wide in any::<bool>()) {
        let expected = join_oracle(a.clone(), b.clone());
        let (window, width) = if wide { wide_window() } else { (WindowConfig::FLOOR, 1) };
        // A window is derived only once the greetings disagree; an equal
        // pair ends at the greeting and grants nothing.
        let derives_window = a.ceiling != b.ceiling;
        for (left, right) in [(a.clone(), b.clone()), (b, a)] {
            let outcome = LocalSession::new(left, right).window(window).trace().stats().run();
            outcome.trace().assert_valid();
            let (ours_stats, theirs_stats) = *outcome.stats();
            let (ours, theirs) = outcome.sides();
            prop_assert_eq!(&ours, &expected);
            prop_assert_eq!(&theirs, &expected);
            if derives_window {
                prop_assert_eq!(ours_stats.window_granted, width);
                prop_assert_eq!(theirs_stats.window_granted, width);
            }
        }
    }

}
