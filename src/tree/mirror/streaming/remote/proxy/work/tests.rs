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
        codec::{Speaker, Stream},
        proxy::Error,
        streams::{
            AcceptDriver, Claims, ErrorRoute, StreamError, StreamReceiver, claims, error_route,
        },
    },
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
        parts.session.epoch,
        Speaker::Responder,
        slots,
        route.clone(),
    );
    let work = Work::new(
        Failing::after(Local, usize::MAX),
        Window::FLOOR,
        Physical {
            control_read: parts.control_read,
            control_write: parts.control_write,
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
    let (claim_send, claim_receive) = oneshot::channel::<DuplexStream>();
    drop(claim_send);
    let mut receiver: StreamReceiver<DuplexStream, ()> = StreamReceiver::new(
        claim_receive,
        Speaker::Initiator,
        Stream::new(0).expect("stream index 0 exists"),
        route,
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
