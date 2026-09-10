use std::{future, pin::pin};

use futures::StreamExt;
use proptest::prelude::*;
use tokio::io::{AsyncWriteExt, DuplexStream};
use tokio::sync::oneshot;

use super::{Physical, Work};
use crate::link::{Connector, Done, MemoryAcceptor, MemoryLink, memory};
use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::observe::SessionHandle;
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::window::Window;
use crate::tree::mirror::streaming::{
    Failing, Failure, Local, Operation,
    remote::{
        adapter::{DecodeError, EncodeError},
        codec::{self, DecodeErrorKind, End, Frame, Origin, RunBudget, Speaker, Stream},
        proxy::Error,
        streams::{
            AcceptDriver, AcceptError, Claims, ErrorRoute, SendError, StreamError, StreamReceiver,
            claims, error_route,
        },
    },
    stats::Recorder,
};

/// A memory-link executor with its claims, error route, and peer held alive.
struct ParkedSession {
    /// The executor under test, with a backend whose errors retain identity.
    work: Work<Failing<Local>, DuplexStream, DuplexStream, MemoryAcceptor>,
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
        PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
    );
    ParkedSession {
        work,
        claims,
        route,
        peer,
    }
}

/// A pump failure cancels parked work and retains its backend error.
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

/// A ready supply failure explains a write failure selected before the acceptor is polled.
#[test]
fn deposited_supply_failure_outranks_a_racing_consequence() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route: _route,
        peer,
    } = parked_session();

    // The final accept poll must discover the closure beside the write error.
    drop(peer);

    // Select a write failure before polling the closed supply.
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

/// A backend failure keeps its identity when the peer has also closed its stream supply.
#[test]
fn an_independent_backend_error_survives_a_deposited_supply_failure() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route: _route,
        peer,
    } = parked_session();

    drop(peer);

    work.spawn(async {
        Err(Error::Encode(EncodeError::Backend(Failure::Injected(
            Operation::Children { height: 1 },
        ))))
    });

    let error = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()))
        .expect("the racing backend failure must resolve the session")
        .expect_err("the session must fail");

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

/// A queued supply-closed report preserves the affected stream when a write fails concurrently.
#[test]
fn queued_supply_closed_outranks_a_selected_consequence_at_stream_granularity() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route,
        peer,
    } = parked_session();

    // The final accept poll observes this closure.
    drop(peer);

    // Poll this receiver before the failing send so its report is queued
    // when the protocol arm resolves.
    let (claim_send, claim_receive) =
        oneshot::channel::<(DuplexStream, crate::link::Done<DuplexStream>)>();
    drop(claim_send);
    let mut receiver: StreamReceiver<DuplexStream> = StreamReceiver::new(
        claim_receive,
        Speaker::Initiator,
        Stream::new(3).expect("stream index 3 exists"),
        RunBudget::default(),
        route,
        Recorder::default(),
        SessionHandle::default(),
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

/// An incoming-stream report ends a session even when all protocol work is parked.
#[test]
fn published_stream_error_preempts_a_parked_protocol() {
    let ParkedSession {
        mut work,
        claims: _claims,
        route,
        peer: _peer,
    } = parked_session();

    // The failed claim reports to the route, then parks its receiver.
    let (claim_send, claim_receive) =
        oneshot::channel::<(DuplexStream, crate::link::Done<DuplexStream>)>();
    drop(claim_send);
    let mut receiver: StreamReceiver<DuplexStream> = StreamReceiver::new(
        claim_receive,
        Speaker::Initiator,
        Stream::new(0).expect("stream index 0 exists"),
        RunBudget::default(),
        route,
        Recorder::default(),
        SessionHandle::default(),
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

/// A write failure used to race incoming errors through the protocol arm.
fn failed_send<E>() -> Error<E> {
    Error::Send(SendError::Connect {
        origin: Origin::direction(Speaker::Responder),
        source: std::io::ErrorKind::BrokenPipe.into(),
    })
}

/// Decode supplied bytes through a real incoming stream's error-reporting path.
async fn receiver(bytes: &[u8], stream: Stream, route: ErrorRoute) -> StreamReceiver<DuplexStream> {
    let (mut tx, rx) = tokio::io::duplex(bytes.len().max(1));
    tx.write_all(bytes).await.unwrap();
    drop(tx);
    let (send, receive) = oneshot::channel();
    send.send((rx, Done::discard())).unwrap();
    StreamReceiver::new(
        receive,
        Speaker::Responder,
        stream,
        RunBudget::default(),
        route,
        Recorder::default(),
        SessionHandle::default(),
    )
}

proptest! {
    /// Frame violations retain their kind and origin whether selected directly
    /// or queued beside a write failure, before or after the supply fails.
    #[test]
    fn incoming_violation_survives_supply_failure(
        index in 0u8..Stream::COUNT,
        offset in 1u8..Stream::COUNT,
        invalid_array_len in 3u8..24,
        mislabel in any::<bool>(),
        queued in any::<bool>(),
        supply_first in any::<bool>(),
    ) {
        let ParkedSession { mut work, claims: _claims, route, peer } = parked_session();
        let stream = Stream::new(index).unwrap();
        let framed = Stream::new((index + offset) % Stream::COUNT).unwrap();
        let mut bytes = Vec::new();
        if mislabel {
            codec::encode(Speaker::Responder, &(framed, Frame::End(End::Reply)), &mut bytes).unwrap();
        } else {
            codec::encode(Speaker::Responder, &(stream, Frame::End(End::Reply)), &mut bytes).unwrap();
            bytes[0] = 0x80 | invalid_array_len;
        }
        let error = run_to_quiescence(async {
            let mut frames = receiver(&bytes, stream, route).await;
            let (start, ready) = oneshot::channel();
            work.spawn(async move {
                ready.await.unwrap();
                if queued {
                    // Decoding reports to the route and parks. Resolve this
                    // task in the same poll to select the write error first.
                    assert!(futures::poll!(frames.next()).is_pending());
                    Err(failed_send())
                } else {
                    frames.next().await;
                    unreachable!("a malformed stream reports and parks");
                }
            });
            drop(peer);
            let mut execute = pin!(work.execute(future::pending::<Result<(), _>>()));
            if supply_first {
                assert!(futures::poll!(execute.as_mut()).is_pending());
            }
            start.send(()).unwrap();
            execute.await
        }).unwrap().unwrap_err();
        if mislabel {
            prop_assert!(matches!(error, Error::Stream(StreamError::Mislabeled {
                origin, labeled, framed: actual,
            }) if origin == Origin::stream(Speaker::Responder, stream)
                && labeled == stream && actual == framed), "{error:?}");
        } else if invalid_array_len == 3 {
            prop_assert!(matches!(error, Error::Stream(StreamError::Decode(codec::DecodeError {
                origin, kind: DecodeErrorKind::FrameArity { expected: 2, found: 3 },
            })) if origin == Origin::stream(Speaker::Responder, stream)), "{error:?}");
        } else {
            prop_assert!(matches!(error, Error::Stream(StreamError::Decode(codec::DecodeError {
                origin, kind: DecodeErrorKind::FrameShape { .. },
            })) if origin == Origin::direction(Speaker::Responder)), "{error:?}");
        }
    }

    /// A declaration violation from protocol work is not replaced by a closed
    /// supply, even when that closure was already observed on an earlier poll.
    #[test]
    fn declaration_violation_survives_supply_failure(
        declared in 0u64..1024,
        excess in 1usize..1024,
        supply_first in any::<bool>(),
    ) {
        let ParkedSession { work, claims: _claims, route: _route, peer } = parked_session();
        let actual = declared as usize + excess;
        let error = run_to_quiescence(async {
            let (send, receive) = oneshot::channel();
            drop(peer);
            let mut execute = pin!(work.execute(async { receive.await.unwrap() }));
            if supply_first {
                assert!(futures::poll!(execute.as_mut()).is_pending());
            }
            send.send(Err::<(), _>(Error::Decode(DecodeError::OversizedVersion { declared, actual })))
                .unwrap();
            execute.await
        }).unwrap().unwrap_err();
        prop_assert!(matches!(error, Error::Decode(DecodeError::OversizedVersion {
            declared: found_declared, actual: found_actual,
        }) if found_declared == declared && found_actual == actual), "{error:?}");
    }

    /// A truncated frame still reports the failed supply as its cause, with
    /// no codec-violation exception hiding the transport failure.
    #[test]
    fn truncated_frame_uses_the_supply_cause(index in 0u8..Stream::COUNT, cut in 0usize..3) {
        let ParkedSession { mut work, claims: _claims, route, peer } = parked_session();
        let stream = Stream::new(index).unwrap();
        let mut bytes = Vec::new();
        codec::encode(Speaker::Responder, &(stream, Frame::End(End::Reply)), &mut bytes).unwrap();
        let error = run_to_quiescence(async {
            let mut frames = receiver(&bytes[..cut], stream, route).await;
            work.spawn(async move {
                frames.next().await;
                unreachable!("a truncated stream reports and parks");
            });
            drop(peer);
            work.execute(future::pending::<Result<(), _>>()).await
        }).unwrap().unwrap_err();
        prop_assert!(matches!(error, Error::Stream(StreamError::SupplyClosed {
            origin, source: Some(ref source),
        }) if origin == Origin::direction(Speaker::Responder)
            && source.kind() == std::io::ErrorKind::UnexpectedEof), "{error:?}");
    }

    /// Invalid stream labels surface through both normal acceptance and the
    /// final accept poll after a transport error; the driver is never repolled
    /// after it returns the violation.
    #[test]
    fn label_violation_reaches_the_executor(
        wrong_epoch in 1u8..24,
        write_fails in any::<bool>(),
    ) {
        let ParkedSession { work, claims: _claims, route: _route, peer } = parked_session();
        let error = run_to_quiescence(async {
            let (mut tx, _) = peer.connector.connect().await.unwrap();
            tx.write_all(&[wrong_epoch, 0]).await.unwrap();
            let finish = async {
                if write_fails { Err(failed_send()) }
                else { future::pending::<Result<(), _>>().await }
            };
            work.execute(finish).await
        }).unwrap().unwrap_err();
        prop_assert!(matches!(error, Error::Accept(AcceptError::Epoch {
            origin, expected: 0, actual,
        }) if origin == Origin::direction(Speaker::Responder) && actual == u64::from(wrong_epoch)),
            "{error:?}");
    }
}

/// Successful protocol completion wins over a ready accept-side violation.
#[test]
fn completed_protocol_does_not_poll_the_acceptor() {
    let ParkedSession {
        work,
        claims: _claims,
        route: _route,
        peer,
    } = parked_session();
    run_to_quiescence(async {
        let (mut tx, _) = peer.connector.connect().await.unwrap();
        tx.write_all(&[1, 0]).await.unwrap();
        work.execute(async { Ok(()) }).await.unwrap();
    })
    .unwrap();
}

/// A delivery rejected during protocol teardown cannot replace the failure
/// that caused teardown; the same rejection from live work is a violation.
#[test]
fn unasked_stream_is_attributed_before_or_after_protocol_teardown() {
    for protocol_fails in [false, true] {
        let ParkedSession {
            mut work,
            claims,
            route,
            peer,
        } = parked_session();
        let error = run_to_quiescence(async {
            let (mut tx, _) = peer.connector.connect().await.unwrap();
            tx.write_all(&[0, 0]).await.unwrap();
            let mut frames = receiver(&[], Stream::new(0).unwrap(), route).await;
            work.spawn(async move {
                drop(claims);
                if protocol_fails {
                    Err(failed_send())
                } else {
                    frames.next().await;
                    unreachable!("a truncated stream reports and parks");
                }
            });
            work.execute(future::pending::<Result<(), _>>()).await
        })
        .unwrap()
        .unwrap_err();
        if protocol_fails {
            assert!(
                matches!(error, Error::Send(SendError::Connect { .. })),
                "{error:?}"
            );
        } else {
            assert!(
                matches!(error, Error::Accept(AcceptError::Unexpected { .. })),
                "{error:?}"
            );
        }
    }
}
