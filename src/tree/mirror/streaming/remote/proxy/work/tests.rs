use std::future;

use futures::StreamExt;
use tokio::io::DuplexStream;
use tokio::sync::oneshot;

use super::{Physical, Work};
use crate::link::{MemoryAcceptor, MemoryLink, memory};
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::window::Window;
use crate::tree::mirror::streaming::{
    Failing, Failure, Local, Operation,
    remote::{
        adapter::EncodeError,
        codec::{Origin, RunBudget, Speaker, Stream},
        proxy::Error,
        streams::{
            AcceptDriver, Claims, ErrorRoute, SendError, StreamError, StreamReceiver, claims,
            error_route,
        },
    },
    stats::Recorder,
};

/// A parked session's `Work` executor over a fresh in-memory link, wired
/// the way the session wires it, with everything the tests must keep alive.
struct ParkedSession {
    work: Work<Failing<Local>, (), DuplexStream, DuplexStream, MemoryAcceptor>,
    /// The claim table the protocol owns in production. Keeping it alive
    /// ensures no stream-layer closure can accidentally win the error race.
    claims: Claims<DuplexStream>,
    /// A publishing half of the error route, for tests that report to it.
    route: ErrorRoute,
    /// The peer link; dropping it would close the stream supply.
    peer: MemoryLink,
}

/// Wire a fresh in-memory link into a [`ParkedSession`].
fn parked_session() -> ParkedSession {
    let (link, peer) = memory();
    let parts = link.into_parts();
    let (slots, claims) = claims();
    let (route, errors) = error_route();
    let accept = AcceptDriver::new(
        parts.acceptor,
        parts.session.epoch(),
        Speaker::Responder,
        slots,
        route.clone(),
    );
    let work = Work::new(
        Failing::after(Local, usize::MAX),
        Window::FLOOR,
        RunBudget::default(),
        u64::MAX,
        u64::MAX,
        Vec::new(),
        Physical {
            control_read: parts.control_read,
            control_write: parts.control_write,
            remote: Speaker::Responder,
            accept,
            errors,
        },
    );
    ParkedSession {
        work,
        claims,
        route,
        peer,
    }
}

/// A pump error cancels parked peers and retains its original error identity.
#[test]
fn pump_failure_preempts_parked_pumps() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route: _route,
        peer: _peer,
    } = parked_session();

    // Poll the parked task first so this specifically exercises fail-fast
    // aggregation rather than relying on the error being the first item.
    for _ in 0..31 {
        work.spawn(future::pending());
    }
    work.spawn(async {
        Err(Error::Encode(EncodeError::Backend(Failure::Injected(
            Operation::Children { height: 1 },
        ))))
    });

    let result = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()));
    let error = result
        .expect("a pump failure must terminate the work executor")
        .expect_err("the injected pump must fail");

    assert!(matches!(
        error,
        Error::Encode(EncodeError::Backend(Failure::Injected(
            Operation::Children { height: 1 },
        )))
    ));
}

/// A deposited supply-failure cause outranks a racing consequence surface,
/// even when the consequence resolves selection before the accept driver
/// is ever polled with its failure ready.
///
/// The selection invariant: an operator debugging a dead session gets the
/// root cause, never the consequence. Here the stream supply is already
/// dead when the session starts and a pump fails with a bare transport
/// symptom in the very first poll wave — the biased select resolves on the
/// protocol arm without reaching the accept arm, so only the terminal's
/// final non-waiting poll of the accept driver can flush the supply
/// failure into the deposit slot. The session must surface `SupplyClosed`
/// carrying the supply's own I/O cause, not the pump's bare symptom.
#[test]
fn deposited_supply_failure_outranks_a_racing_consequence() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route: _route,
        peer,
    } = parked_session();

    // The peer link is gone before the session's first poll: the accept
    // driver's very first accept resolves to the supply's own failure.
    drop(peer);

    // A consequence of the dead transport, ready in the first wave: a bare
    // BrokenPipe surfacing through the Send family, exactly the symptom an
    // endpoint sees when its own write hits the severed link.
    work.spawn(async {
        Err(Error::Send(SendError::Connect {
            origin: Origin::stream(Speaker::Responder, Stream::new(0).expect("stream 0 exists")),
            source: std::io::ErrorKind::BrokenPipe.into(),
        }))
    });

    let error = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()))
        .expect("the racing consequence must resolve the session, not hang it")
        .expect_err("the session must fail");

    match error {
        Error::Stream(StreamError::SupplyClosed {
            origin,
            source: Some(cause),
        }) => {
            // No stream provably needed the supply (every claim is still
            // alive), so the cause is attributed at direction granularity.
            assert_eq!(origin, Origin::direction(Speaker::Responder));
            // The deposited cause is the acceptor's own transport error.
            assert_eq!(cause.kind(), std::io::ErrorKind::UnexpectedEof);
        }
        other => panic!("the consequence outranked the deposited cause: {other:?}"),
    }
}

/// A typed backend failure racing a dead stream supply surfaces as itself:
/// the supply-outranking terminal exempts backend-typed errors.
///
/// The terminal outranks protocol errors with a deposited supply failure
/// because a consequence of the dead transport is a symptom, not a cause —
/// but the local store failing is independent of the transport, so a dead
/// supply cannot have caused it. `Error`'s attribution contract ("errors
/// the supply did not cause surface from the failing operation itself")
/// requires the backend error's identity to survive the race, never to be
/// replaced by a direction-granularity `SupplyClosed`.
#[test]
fn an_independent_backend_error_survives_a_deposited_supply_failure() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route: _route,
        peer,
    } = parked_session();

    // The peer link is gone before the session's first poll, exactly as in
    // `deposited_supply_failure_outranks_a_racing_consequence`.
    drop(peer);

    // An INDEPENDENT local failure, not a symptom of the dead transport:
    // the backend (the local store) failing during conversion.
    work.spawn(async {
        Err(Error::Encode(EncodeError::Backend(Failure::Injected(
            Operation::Children { height: 1 },
        ))))
    });

    let error = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()))
        .expect("the racing backend failure must resolve the session")
        .expect_err("the session must fail");

    // The backend error's identity survives: the deposited supply failure
    // never outranks an error it cannot have caused.
    assert!(
        matches!(
            error,
            Error::Encode(EncodeError::Backend(Failure::Injected(
                Operation::Children { height: 1 },
            )))
        ),
        "the backend error must surface as itself, got: {error:?}",
    );
}

/// The terminal's queued-`SupplyClosed` recovery arm, reached
/// deterministically.
///
/// A reporter that provably needed the dead supply publishes its
/// stream-granularity `SupplyClosed` and parks; a consequence in the same
/// poll wave resolves the protocol arm first, so the biased select never
/// receives the queued report. The terminal must drain the queue, prefer
/// the queued report's *stream*-granularity origin over the deposit's
/// direction granularity, and attach the deposited I/O cause to it.
#[test]
fn queued_supply_closed_outranks_a_selected_consequence_at_stream_granularity() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route,
        peer,
    } = parked_session();

    // The supply is dead before the first poll: the accept driver's flush
    // poll deposits the acceptor's own I/O failure.
    drop(peer);

    // A reporter that needed the supply: publishes SupplyClosed at stream
    // granularity, then parks. Spawned first so it publishes in the same
    // wave the consequence resolves.
    let (claim_send, claim_receive) =
        oneshot::channel::<(DuplexStream, crate::link::Done<DuplexStream>)>();
    drop(claim_send);
    let mut receiver: StreamReceiver<DuplexStream, ()> = StreamReceiver::new(
        claim_receive,
        Speaker::Initiator,
        Stream::new(3).expect("stream index 3 exists"),
        RunBudget::default(),
        route,
        Recorder::default(),
    );
    work.spawn(async move {
        receiver.next().await;
        unreachable!("a reporter parks forever after publishing");
    });

    // The consequence, resolving the protocol arm in the same wave.
    work.spawn(async {
        Err(Error::Send(SendError::Connect {
            origin: Origin::stream(Speaker::Responder, Stream::new(0).expect("stream 0 exists")),
            source: std::io::ErrorKind::BrokenPipe.into(),
        }))
    });

    let error = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()))
        .expect("the racing consequence must resolve the session, not hang it")
        .expect_err("the session must fail");

    match error {
        Error::Stream(StreamError::SupplyClosed {
            origin,
            source: Some(cause),
        }) => {
            // The queued report names the stream that provably needed the
            // supply: finer than the deposit's direction granularity.
            assert_eq!(
                origin,
                Origin::stream(Speaker::Initiator, Stream::new(3).expect("stream 3 exists")),
                "the queued report's stream-granularity origin must win",
            );
            assert_eq!(cause.kind(), std::io::ErrorKind::UnexpectedEof);
        }
        other => panic!("the queued SupplyClosed was not recovered: {other:?}"),
    }
}

/// A published stream error resolves a session whose every future is parked.
///
/// The incoming-stream reporters publish to the error route and then park
/// forever, so the session's liveness rests entirely on `execute`'s select
/// observing the route: the protocol arm never completes (its pumps and
/// terminal operation are parked), the biased poll order visits it first, and
/// the error arm must still win the poll rather than hang. This is the property
/// every publish-then-park path in the stream layer rests on.
#[test]
fn published_stream_error_preempts_a_parked_protocol() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route,
        peer: _peer,
    } = parked_session();

    // A receiver whose claim sender is already gone: its first poll reports
    // `SupplyClosed` to the error route and parks, never resolving — the
    // pump driving it is the parked reporter the select must not wait for.
    let (claim_send, claim_receive) =
        oneshot::channel::<(DuplexStream, crate::link::Done<DuplexStream>)>();
    drop(claim_send);
    let mut receiver: StreamReceiver<DuplexStream, ()> = StreamReceiver::new(
        claim_receive,
        Speaker::Initiator,
        Stream::new(0).expect("stream index 0 exists"),
        RunBudget::default(),
        route,
        Recorder::default(),
    );
    work.spawn(async move {
        receiver.next().await;
        unreachable!("a reporter parks forever after publishing");
    });

    let error = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()))
        .expect("a published stream error must resolve the parked session, not hang it")
        .expect_err("the published stream error must surface");

    assert!(matches!(
        error,
        Error::Stream(StreamError::SupplyClosed { source: None, .. })
    ));
}
