//! Error selection and shutdown behavior for the proxy work executor.

use crate::tree::mirror::streaming::message::Grandchildren;
use std::{
    future,
    io::ErrorKind,
    pin::Pin,
    task::{Context, Poll},
};

use proptest::prelude::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, DuplexStream, ReadBuf};
use tokio::sync::oneshot;

use super::{Ingress, Physical, Work, progress::Progress};
use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::testing::{MemoryEnd, memory_connection, run_to_quiescence};
use crate::tree::mirror::cbor;
use crate::tree::mirror::streaming::window::Window;
use crate::tree::mirror::streaming::{
    Failing, Failure, Local, Operation,
    channel::{QueueKind, QueueRole, channel},
    message::listed,
    remote::{
        adapter::{DecodeError, EncodeError, Scope, ScopeError},
        codec::{
            self, DecodeErrorKind, EncodeErrorKind, End, Flow, Frame, FramePart, Origin, Reaction,
            RunBudget, Speaker, Stream,
        },
        pipe::{Demultiplexer, Multiplexer, PipeError},
        proxy::Error,
    },
    stats::Recorder,
};
use crate::tree::typed::{
    Hash,
    height::{Height, UnderRoot},
};

/// Pending tasks placed before the injected failure to exercise aggregation.
const PARKED_PUMPS: usize = 3;

/// The remote speaker of every session here: the local side initiates.
const REMOTE: Speaker = Speaker::Responder;

/// The completion marker that closes a session: a CBOR text item.
const MARKER: [u8; 2] = crate::testing::COMPLETION_MARKER;

/// An executor over a fresh in-memory pipe, with the peer's end held open.
struct ParkedSession {
    /// The executor under test, with a backend whose errors retain identity.
    work: Work<Failing<Local>, DuplexStream, DuplexStream>,
    /// The peer's end; dropping its write half ends the pipe toward us.
    peer: MemoryEnd,
}

/// Wire a fresh in-memory connection into a [`ParkedSession`].
fn parked_session() -> ParkedSession {
    let (end, peer) = memory_connection(crate::testing::MEMORY_CAPACITY);
    let work = session_over(end.read, end.write);
    ParkedSession { work, peer }
}

/// An executor over the pipe halves `read` and `write`, with a backend
/// whose errors retain identity.
fn session_over<R, W>(read: R, write: W) -> Work<Failing<Local>, R, W> {
    Work::new(
        Ingress::new(
            Failing::after(Local, usize::MAX),
            u64::MAX,
            u64::MAX,
            Grandchildren::MAX,
            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
        ),
        Window::FLOOR,
        RunBudget::default(),
        Vec::new(),
        Default::default(),
        Physical {
            read,
            write,
            remote: REMOTE,
            multiplexer: Multiplexer::default(),
            demultiplexer: Demultiplexer::default(),
        },
        Recorder::default(),
        crate::observe::SessionHandle::default(),
    )
}

/// A read half that fails once with its error kind, then reports end of
/// stream: a transport whose failure is not sticky.
struct FailsOnce(Option<ErrorKind>);

/// Report the error on the first read and end of stream on every later one.
impl AsyncRead for FailsOnce {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(match self.get_mut().0.take() {
            Some(kind) => Err(kind.into()),
            None => Ok(()),
        })
    }
}

/// A write failure of the local multiplexer.
fn failed_write<E>() -> Error<E> {
    Error::Pipe(PipeError::Encode(codec::EncodeError {
        origin: Origin::direction(REMOTE.other()),
        kind: EncodeErrorKind::Write {
            part: FramePart::FrameHead,
            source: ErrorKind::BrokenPipe.into(),
        },
    }))
}

/// Encode `frame` on `stream` as the remote speaker writes it.
fn remote_frame(stream: Stream, frame: Frame) -> Vec<u8> {
    let mut bytes = Vec::new();
    codec::encode(REMOTE, &(stream, frame), &mut bytes).expect("test frames encode");
    bytes
}

/// A pump failure cancels parked work and retains its backend error.
#[test]
fn pump_failure_preempts_parked_pumps() {
    let ParkedSession {
        mut work,
        peer: _peer,
    } = parked_session();

    // Poll the parked task first so this specifically exercises fail-fast
    // aggregation rather than relying on the error being the first item.
    for _ in 0..PARKED_PUMPS {
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

/// The peer's departure between frames ends a session whose protocol work is
/// all parked: the pipe's end of stream is the departure.
#[test]
fn departure_ends_a_parked_protocol() {
    let ParkedSession { work, peer } = parked_session();
    drop(peer);
    let error = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()))
        .expect("departure must resolve the session, not leave it parked")
        .expect_err("the session must fail");
    assert!(
        matches!(&error, Error::Pipe(PipeError::Departed(source)) if source.kind() == ErrorKind::UnexpectedEof),
        "{error:?}"
    );
}

/// A violation the demultiplexer has ready explains a write failure the
/// protocol selected first: the peer that broke the protocol may also have
/// closed the connection.
#[test]
fn a_ready_violation_explains_a_write_failure() {
    let ParkedSession { work, mut peer } = parked_session();
    // A frame whose state names no signal.
    let mut malformed = Vec::new();
    cbor::write_head(&mut malformed, cbor::MAJOR_ARRAY, 2);
    cbor::write_head(&mut malformed, cbor::MAJOR_UINT, 1);
    cbor::write_head(&mut malformed, cbor::MAJOR_UINT, u64::from(u8::MAX));
    let error = run_to_quiescence(async {
        peer.write.write_all(&malformed).await.unwrap();
        work.execute(async { Err::<(), _>(failed_write()) }).await
    })
    .expect("the session resolves")
    .expect_err("the session must fail");
    assert!(
        matches!(&error, Error::Pipe(PipeError::Decode(decoded)) if matches!(decoded.kind, DecodeErrorKind::InvalidSignal(_))),
        "the write failure outranked the violation: {error:?}"
    );
}

/// A write failure stands when the demultiplexer has only the peer's
/// departure ready: both are transport failures, and the protocol's came
/// first.
#[test]
fn a_write_failure_stands_beside_departure() {
    let ParkedSession { work, peer } = parked_session();
    drop(peer);
    let error = run_to_quiescence(work.execute(async { Err::<(), _>(failed_write()) }))
        .expect("the session resolves")
        .expect_err("the session must fail");
    assert!(
        matches!(error, Error::Pipe(PipeError::Encode(_))),
        "{error:?}"
    );
}

proptest! {
    /// A decode failure reaches the executor without waiting for an unread
    /// or dropped response consumer, whether or not an earlier reply waits
    /// parked.
    ///
    /// Each reply answers a record with one position, and the failing reply
    /// matches twice, so its second match has no position to take. The
    /// terminal step never finishes, so only the failure can end the
    /// session.
    #[test]
    fn response_failure_does_not_wait_for_its_consumer(
        buffered in any::<bool>(),
        dropped in any::<bool>(),
    ) {
        let ParkedSession { mut work, mut peer } = parked_session();
        // A closed consumer may stop a pump before later items are read.
        let buffered = buffered && !dropped;
        let record = Scope::opening_unbounded(&[listed(0, Hash::default())]);
        let (mut records, questions) =
            channel(QueueRole::new(QueueKind::ProxyLocalQuestions, UnderRoot::HEIGHT), 2);
        let stream = Stream::at_height(REMOTE, UnderRoot::HEIGHT)
            .expect("the responder replies to the opening");
        let mut frames = Vec::new();
        if buffered {
            records.try_send((record.clone(), Default::default())).expect("room for two records");
            frames.extend(remote_frame(stream, Frame::Reaction(Reaction::Match, Flow::End)));
        }
        records.try_send((record, Default::default())).expect("room for two records");
        frames.extend(remote_frame(stream, Frame::Reaction(Reaction::Match, Flow::Continue)));
        frames.extend(remote_frame(stream, Frame::Reaction(Reaction::Match, Flow::End)));
        let responses = work.respond::<UnderRoot>(questions, None, Progress::new());
        let _responses = (!dropped).then_some(responses);
        let result = run_to_quiescence(async {
            peer.write.write_all(&frames).await.unwrap();
            work.execute(future::pending::<Result<(), _>>()).await
        });
        prop_assert!(
            matches!(
                result,
                Ok(Err(Error::Decode(DecodeError::Scope(ScopeError::UnpositionedMatch))))
            ),
            "response error was lost: {:?}", result
        );
    }

    /// The peer's closing items, arriving before or after the protocol
    /// completes and split at any byte, return unread with the read half:
    /// the closing exchange reads every byte, in order.
    #[test]
    fn completed_work_returns_the_closing_items_unread(
        trailing in proptest::collection::vec(any::<u8>(), 0..64),
        cut in any::<prop::sample::Index>(),
        arrives_first in any::<bool>(),
    ) {
        let ParkedSession { work, mut peer } = parked_session();
        let mut closing = MARKER.to_vec();
        closing.extend_from_slice(&trailing);
        let cut = cut.index(closing.len() + 1);
        let actual = run_to_quiescence(async {
            let (finish, finished) = oneshot::channel();
            let mut execute = Box::pin(work.execute(async { finished.await.unwrap(); Ok(()) }));
            if arrives_first {
                peer.write.write_all(&closing[..cut]).await.unwrap();
            }
            assert!(futures::poll!(&mut execute).is_pending());
            finish.send(()).unwrap();
            let ((), mut read, _) = execute.await.unwrap();
            let rest = if arrives_first { &closing[cut..] } else { &closing[..] };
            peer.write.write_all(rest).await.unwrap();
            drop(peer);
            let mut actual = Vec::new();
            read.read_to_end(&mut actual).await.unwrap();
            actual
        }).expect("completed work hands the closing items on");
        prop_assert_eq!(actual, closing);
    }
}

/// A completed protocol succeeds although the peer has since departed: the
/// closing exchange, which reads next, reports the departure in its own
/// phase.
#[test]
fn completed_work_defers_departure_to_the_closing_exchange() {
    let ParkedSession { work, peer } = parked_session();
    drop(peer);
    let result =
        run_to_quiescence(work.execute(async { Ok(()) })).expect("completed work resolves");
    assert!(result.is_ok(), "{:?}", result.err());
}

/// A read error the demultiplexer meets after the protocol completes
/// reaches the closing exchange as itself.
///
/// The returned read half reports it on its next read, although the
/// transport would report only end of stream if read again.
#[test]
fn completed_work_keeps_a_read_error_for_the_closing_exchange() {
    let work = session_over(
        FailsOnce(Some(ErrorKind::ConnectionReset)),
        tokio::io::sink(),
    );
    let error = run_to_quiescence(async {
        let ((), mut read, _) = work
            .execute(async { Ok(()) })
            .await
            .expect("completed work succeeds");
        read.read(&mut [0; 1])
            .await
            .expect_err("the next read reports the kept error")
    })
    .expect("completed work resolves");
    assert_eq!(error.kind(), ErrorKind::ConnectionReset);
}

/// A whole frame waiting on the pipe when the protocol completes answers no
/// question, and fails the session.
#[test]
fn a_frame_after_completion_is_unasked() {
    let ParkedSession { work, mut peer } = parked_session();
    let frame = remote_frame(
        Stream::new(1).unwrap(),
        Frame::Reaction(Reaction::Match, Flow::End),
    );
    let error = run_to_quiescence(async {
        peer.write.write_all(&frame).await.unwrap();
        work.execute(async { Ok(()) }).await
    })
    .expect("the session resolves")
    .expect_err("an unasked frame must fail the session");
    assert!(
        matches!(error, Error::Pipe(PipeError::Unasked { .. })),
        "{error:?}"
    );
}

/// A frame the demultiplexer has begun but cannot finish when the protocol
/// completes answers no question either, and fails the session instead of
/// leaving the read half mid-frame.
#[test]
fn a_partial_frame_after_completion_is_unasked() {
    let ParkedSession { work, mut peer } = parked_session();
    let frame = remote_frame(Stream::new(1).unwrap(), Frame::End(End::Reply));
    let error = run_to_quiescence(async {
        peer.write.write_all(&frame[..1]).await.unwrap();
        work.execute(async { Ok(()) }).await
    })
    .expect("the session resolves")
    .expect_err("a begun frame must fail the session");
    assert!(
        matches!(error, Error::Pipe(PipeError::Unasked { origin }) if origin == Origin::direction(REMOTE)),
        "{error:?}"
    );
}
