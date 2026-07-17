//! Unit tests for lazy stream establishment, labeling, and claim routing.

use futures::{StreamExt, future::join};
use tokio::io::AsyncWriteExt;

use crate::link::{Acceptor, Connector, memory};
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::remote::codec::{
    End, Flow, Frame, FrameWrite, Origin, Reaction, Speaker, Stream,
};

use super::{
    AcceptDriver, AcceptError, ReceiverFinish, ReplyFrame, StreamError, StreamReceiver,
    StreamSender, claims, error_route, label,
};

/// Session epoch shared by the violation tests; its value is arbitrary.
const EPOCH: u8 = 0;

/// A payload-free frame type for stream-layer tests.
type Unit = ();

/// The label is exactly two bytes: the session epoch then the stream index.
#[test]
fn label_is_epoch_then_stream() {
    let stream = Stream::new(3).expect("stream 3 exists");
    assert_eq!(label(7, stream), [7, 3]);
}

/// A sender that never carries a frame opens no transport stream at all —
/// `finish` completes without a connect — so vacuous levels leave no trace
/// on the wire.
#[test]
fn unopened_sender_finishes_without_connecting() {
    let (a, mut b) = memory();
    run_to_quiescence(async {
        let sender: StreamSender<_, Unit> = StreamSender::new(
            a.connector.clone(),
            0,
            Speaker::Initiator,
            Stream::new(1).expect("stream 1 exists"),
        );
        sender.finish().await.expect("vacuous finish succeeds");
        // The peer sees no announced stream: with the connector dropped, its
        // acceptor reports the supply closed rather than delivering one.
        drop(a);
        b.acceptor
            .accept()
            .await
            .expect_err("no stream was ever announced");
    })
    .expect("vacuous sender stays live");
}

/// A sender's first frame opens the stream, labels it, and the receiver
/// yields exactly the frames before the end control, consuming the end.
#[test]
fn frames_flow_sender_to_claimed_receiver() {
    let (a, mut b) = memory();
    let stream = Stream::new(2).expect("stream 2 exists");
    run_to_quiescence(async {
        let send = async {
            let mut sender: StreamSender<_, Unit> =
                StreamSender::new(a.connector.clone(), 9, Speaker::Initiator, stream);
            sender
                .frame(reply_frame(Frame::Reaction(Reaction::Match, Flow::End)))
                .await
                .expect("frame writes");
            sender
                .finish()
                .await
                .expect("finish writes the end control");
        };
        let receive = async {
            let (slots, mut claims) = claims();
            let (route, _errors) = error_route();
            let driver =
                AcceptDriver::new(&mut b.acceptor, 9, Speaker::Initiator, slots, route.clone());
            let mut receiver: StreamReceiver<_, Unit> =
                StreamReceiver::new(claims.take(stream), Speaker::Initiator, stream, route);
            let receive = async {
                assert_eq!(
                    receiver.next().await,
                    Some(Frame::Reaction(Reaction::Match, Flow::End)),
                );
                assert_eq!(receiver.finish().await, ReceiverFinish::Clean);
            };
            // Drive the accept loop beside the receiver; it never resolves
            // on success, so the receive arm finishing ends the race.
            tokio::select! {
                biased;
                () = receive => {}
                _error = driver.run() => panic!("accept driver failed"),
            }
        };
        join(send, receive).await;
    })
    .expect("stream exchange stays live");
}

/// A receiver that is never polled never claims, and `finish` passes
/// vacuously without touching the claim.
#[test]
fn unpolled_receiver_finishes_vacuously() {
    let (slots, mut claims) = claims::<tokio::io::DuplexStream>();
    let stream = Stream::new(0).expect("stream 0 exists");
    let (route, _errors) = error_route();
    let mut receiver: StreamReceiver<_, Unit> =
        StreamReceiver::new(claims.take(stream), Speaker::Responder, stream, route);
    run_to_quiescence(async {
        assert_eq!(receiver.finish().await, ReceiverFinish::Clean);
    })
    .expect("vacuous receiver finish resolves");
    drop(slots);
}

/// The accept driver rejects a stream labeled with another session's epoch.
#[test]
fn accept_driver_rejects_wrong_epoch() {
    let (a, mut b) = memory();
    run_to_quiescence(async {
        let send = async {
            let mut sender: StreamSender<_, Unit> = StreamSender::new(
                a.connector.clone(),
                4,
                Speaker::Initiator,
                Stream::new(0).expect("stream 0 exists"),
            );
            sender
                .frame(reply_frame(Frame::End(End::Reply)))
                .await
                .expect("frame writes");
            sender
        };
        let receive = async {
            let (slots, _claims) = claims();
            let (route, _errors) = error_route();
            let driver = AcceptDriver::new(&mut b.acceptor, 5, Speaker::Initiator, slots, route);
            driver.run().await
        };
        let (_sender, error) = join(send, receive).await;
        assert!(
            matches!(
                error,
                AcceptError::Epoch {
                    expected: 5,
                    actual: 4,
                    ..
                }
            ),
            "unexpected accept outcome: {error:?}",
        );
    })
    .expect("epoch rejection resolves");
}

/// The accept driver rejects a stream delivered for a level whose consumer
/// already finished without asking anything: an unasked reply, caught at
/// the label.
#[test]
fn accept_driver_rejects_unclaimed_delivery() {
    let (a, mut b) = memory();
    let stream = Stream::new(6).expect("stream 6 exists");
    run_to_quiescence(async {
        let send = async {
            let mut sender: StreamSender<_, Unit> =
                StreamSender::new(a.connector.clone(), 0, Speaker::Initiator, stream);
            sender
                .frame(reply_frame(Frame::End(End::Reply)))
                .await
                .expect("frame writes");
            sender
        };
        let receive = async {
            let (slots, mut claims) = claims::<tokio::io::DuplexStream>();
            // The pump for this level concluded without a question: its
            // claim is dropped, exactly as a vacuous level leaves it.
            drop(claims.take(stream));
            let (route, _errors) = error_route();
            let driver = AcceptDriver::new(&mut b.acceptor, 0, Speaker::Initiator, slots, route);
            driver.run().await
        };
        let (_sender, error) = join(send, receive).await;
        assert!(
            matches!(error, AcceptError::Unexpected { .. }),
            "unexpected accept outcome: {error:?}",
        );
    })
    .expect("unclaimed delivery rejection resolves");
}

/// A frame whose signal byte names a different logical stream than its
/// label is reported as `Mislabeled` carrying both indices, never yielded.
///
/// This is the label-equality tripwire of
/// `design/streaming-wire-deadlock.md` §8.6: a routing mistake in a
/// caller-built link — a miswired router, a pool lease crossing streams —
/// surfaces at the first frame as a precise `labeled`/`framed` pair on the
/// session error route instead of as garbled protocol.
#[test]
fn mislabeled_frame_is_reported_not_yielded() {
    const LABELED: u8 = 2;
    const FRAMED: u8 = 3;

    let (a, mut b) = memory();
    let labeled = Stream::new(LABELED).expect("stream 2 exists");
    let framed = Stream::new(FRAMED).expect("stream 3 exists");
    let error = run_to_quiescence(async {
        let send = async {
            // A deliberately miswiring transport: the label names one
            // stream, the frame's signal byte another. `StreamSender`
            // cannot express this, so the bytes come from the codec pieces
            // directly.
            let mut tx = a.connector.connect().await.expect("stream opens");
            tx.write_all(&label(EPOCH, labeled))
                .await
                .expect("label writes");
            let mut write = FrameWrite::new(Speaker::Initiator, tx);
            write
                .frame(&(framed, Frame::<Unit>::End(End::Reply)))
                .await
                .expect("the miswired frame writes");
        };
        let receive = async {
            let (slots, mut claims) = claims();
            let (route, mut errors) = error_route();
            let driver = AcceptDriver::new(
                &mut b.acceptor,
                EPOCH,
                Speaker::Initiator,
                slots,
                route.clone(),
            );
            let mut receiver: StreamReceiver<_, Unit> =
                StreamReceiver::new(claims.take(labeled), Speaker::Initiator, labeled, route);
            let observe = async {
                tokio::select! {
                    biased;
                    frame = receiver.next() => {
                        panic!("the mislabeled frame was yielded: {frame:?}")
                    }
                    error = errors.first() => error,
                }
            };
            tokio::select! {
                biased;
                error = observe => error,
                error = driver.run() => {
                    panic!("the correctly labeled stream was rejected: {error:?}")
                }
            }
        };
        join(send, receive).await.1
    })
    .expect("mislabel detection resolves");
    assert!(
        matches!(
            error,
            StreamError::Mislabeled {
                labeled: LABELED,
                framed: FRAMED,
                ..
            }
        ),
        "unexpected stream error: {error:?}",
    );
}

/// A transport half dropped before its explicit end control is reported as
/// `Truncated`, never mistaken for a clean stream end.
///
/// The end control is what distinguishes a completed stream from one cut
/// off mid-reply by a dying peer; the frames already yielded must not pass
/// as the whole reply.
#[test]
fn truncated_stream_is_reported_not_ended() {
    let (a, mut b) = memory();
    let stream = Stream::new(4).expect("stream 4 exists");
    let error = run_to_quiescence(async {
        let send = async {
            let mut sender: StreamSender<_, Unit> =
                StreamSender::new(a.connector.clone(), EPOCH, Speaker::Initiator, stream);
            sender
                .frame(reply_frame(Frame::Reaction(Reaction::Match, Flow::End)))
                .await
                .expect("frame writes");
            // Dropped without `finish`: transport end-of-stream arrives
            // with no end control ahead of it.
            drop(sender);
        };
        let receive = async {
            let (slots, mut claims) = claims();
            let (route, mut errors) = error_route();
            let driver = AcceptDriver::new(
                &mut b.acceptor,
                EPOCH,
                Speaker::Initiator,
                slots,
                route.clone(),
            );
            let mut receiver: StreamReceiver<_, Unit> =
                StreamReceiver::new(claims.take(stream), Speaker::Initiator, stream, route);
            let observe = async {
                // The complete frame ahead of the truncation still arrives.
                assert_eq!(
                    receiver.next().await,
                    Some(Frame::Reaction(Reaction::Match, Flow::End)),
                );
                tokio::select! {
                    biased;
                    frame = receiver.next() => {
                        panic!("a truncated stream yielded: {frame:?}")
                    }
                    error = errors.first() => error,
                }
            };
            tokio::select! {
                biased;
                error = observe => error,
                error = driver.run() => {
                    panic!("the truncated stream was rejected at its label: {error:?}")
                }
            }
        };
        join(send, receive).await.1
    })
    .expect("truncation detection resolves");
    // The origin is pinned in full: it is the field an operator debugging a
    // caller-built link reads first, so misattributing the speaker or the
    // logical stream is itself a regression.
    assert!(
        matches!(
            error,
            StreamError::Truncated { origin } if origin == Origin::stream(Speaker::Initiator, stream)
        ),
        "unexpected stream error: {error:?}",
    );
}

/// A frame arriving after the explicit end control is reported as
/// `AfterEnd`: a peer that keeps talking past its own end is caught.
///
/// After `End(Stream)` the receiver requires transport end-of-stream
/// before ending cleanly (`design/streaming-wire-deadlock.md` §8.10's
/// double-checked stream ends), recovering the deleted demux's
/// frame-after-end detection.
#[test]
fn frames_after_the_end_control_are_reported() {
    let (a, mut b) = memory();
    let stream = Stream::new(5).expect("stream 5 exists");
    let error = run_to_quiescence(async {
        let send = async {
            // Raw bytes: an honest `StreamSender` half-closes right after
            // its end control, so the violation is built from the codec
            // pieces directly.
            let mut tx = a.connector.connect().await.expect("stream opens");
            tx.write_all(&label(EPOCH, stream))
                .await
                .expect("label writes");
            let mut write = FrameWrite::new(Speaker::Initiator, tx);
            write
                .frame(&(stream, Frame::<Unit>::End(End::Stream)))
                .await
                .expect("the end control writes");
            write
                .frame(&(stream, Frame::<Unit>::End(End::Reply)))
                .await
                .expect("the frame beyond the end writes");
        };
        let receive = async {
            let (slots, mut claims) = claims();
            let (route, mut errors) = error_route();
            let driver = AcceptDriver::new(
                &mut b.acceptor,
                EPOCH,
                Speaker::Initiator,
                slots,
                route.clone(),
            );
            let mut receiver: StreamReceiver<_, Unit> =
                StreamReceiver::new(claims.take(stream), Speaker::Initiator, stream, route);
            let observe = async {
                tokio::select! {
                    biased;
                    frame = receiver.next() => {
                        panic!("a stream past its end yielded: {frame:?}")
                    }
                    error = errors.first() => error,
                }
            };
            tokio::select! {
                biased;
                error = observe => error,
                error = driver.run() => {
                    panic!("the overlong stream was rejected at its label: {error:?}")
                }
            }
        };
        join(send, receive).await.1
    })
    .expect("after-end detection resolves");
    // Pinned in full, origin included, like the truncation test above.
    assert!(
        matches!(
            error,
            StreamError::AfterEnd { origin } if origin == Origin::stream(Speaker::Initiator, stream)
        ),
        "unexpected stream error: {error:?}",
    );
}

/// A second transport stream bearing an already-delivered label is
/// rejected by the accept driver as `Duplicate`.
///
/// Claim slots are take-once, so the duplicate has nowhere to go; opening
/// a logical stream twice is a peer violation, terminal for the session.
#[test]
fn accept_driver_rejects_duplicate_label() {
    let (a, mut b) = memory();
    let stream = Stream::new(1).expect("stream 1 exists");
    run_to_quiescence(async {
        let send = async {
            for _ in 0..2 {
                let mut tx = a.connector.connect().await.expect("stream opens");
                tx.write_all(&label(EPOCH, stream))
                    .await
                    .expect("label writes");
                // The label stays readable after the drop; nothing more is
                // needed to reach the duplicate check.
            }
        };
        let receive = async {
            let (slots, mut claims) = claims();
            // The claim stays alive, so the first delivery succeeds and
            // only the second stream is at fault.
            let _claim = claims.take(stream);
            let (route, _errors) = error_route();
            let driver =
                AcceptDriver::new(&mut b.acceptor, EPOCH, Speaker::Initiator, slots, route);
            driver.run().await
        };
        let ((), error) = join(send, receive).await;
        // Pinned in full, origin included, like the stream-error tests.
        assert!(
            matches!(
                error,
                AcceptError::Duplicate { origin }
                    if origin == Origin::stream(Speaker::Initiator, stream)
            ),
            "unexpected accept outcome: {error:?}",
        );
    })
    .expect("duplicate rejection resolves");
}

/// A stream labeled with an index outside the logical streams is rejected
/// by the accept driver as `UnknownStream` carrying the index.
#[test]
fn accept_driver_rejects_unknown_stream_index() {
    let (a, mut b) = memory();
    run_to_quiescence(async {
        let send = async {
            let mut tx = a.connector.connect().await.expect("stream opens");
            // One past the last logical stream: no claim slot can exist.
            tx.write_all(&[EPOCH, Stream::COUNT])
                .await
                .expect("label writes");
        };
        let receive = async {
            let (slots, _claims) = claims::<tokio::io::DuplexStream>();
            let (route, _errors) = error_route();
            let driver =
                AcceptDriver::new(&mut b.acceptor, EPOCH, Speaker::Initiator, slots, route);
            driver.run().await
        };
        let ((), error) = join(send, receive).await;
        assert!(
            matches!(
                error,
                AcceptError::UnknownStream {
                    index: Stream::COUNT,
                    ..
                }
            ),
            "unexpected accept outcome: {error:?}",
        );
    })
    .expect("unknown-index rejection resolves");
}

/// A supply failure reaches the one receiver still awaiting its claim as
/// `SupplyClosed` carrying the deposited I/O cause.
///
/// This pins `design/streaming-wire-deadlock.md` §8.10's deferred-supply
/// semantics deterministically: the parked accept driver deposits the
/// transport failure, and the first `SupplyClosed` reporter — the consumer
/// that provably needed a stream — claims it as its `source`.
#[test]
fn supply_failure_reaches_the_awaiting_receiver() {
    let (a, mut b) = memory();
    let stream = Stream::new(6).expect("stream 6 exists");
    let error = run_to_quiescence(async {
        // The peer link is gone before any stream arrives, so the very
        // first accept observes the supply failure.
        drop(a);
        let (slots, mut claims) = claims();
        let (route, mut errors) = error_route();
        let driver = AcceptDriver::new(
            &mut b.acceptor,
            EPOCH,
            Speaker::Initiator,
            slots,
            route.clone(),
        );
        let mut receiver: StreamReceiver<_, Unit> =
            StreamReceiver::new(claims.take(stream), Speaker::Initiator, stream, route);
        let observe = async {
            tokio::select! {
                biased;
                frame = receiver.next() => {
                    panic!("no stream could have arrived: {frame:?}")
                }
                error = errors.first() => error,
            }
        };
        tokio::select! {
            biased;
            error = observe => error,
            error = driver.run() => {
                panic!("a supply failure is not a peer violation: {error:?}")
            }
        }
    })
    .expect("deferred supply failure resolves");
    match error {
        StreamError::SupplyClosed {
            source: Some(cause),
            ..
        } => {
            // The deposited cause is the acceptor's own transport error,
            // not a substitute minted at the reporting site.
            assert_eq!(cause.kind(), std::io::ErrorKind::UnexpectedEof);
        }
        other => panic!("unexpected stream error: {other:?}"),
    }
}

/// A supply failure after every needed stream was delivered leaves the
/// session to complete cleanly on the streams it holds.
///
/// The park path of `design/streaming-wire-deadlock.md` §8.10: a peer that
/// finished cleanly may drop its link while this side still reads, so the
/// accept driver parks on the failure instead of failing the session, and
/// nothing surfaces through the error route.
#[test]
fn supply_failure_after_delivery_lets_the_session_finish() {
    let (a, mut b) = memory();
    let stream = Stream::new(7).expect("stream 7 exists");
    run_to_quiescence(async {
        let send = async {
            let mut sender: StreamSender<_, Unit> =
                StreamSender::new(a.connector.clone(), EPOCH, Speaker::Initiator, stream);
            sender
                .frame(reply_frame(Frame::Reaction(Reaction::Match, Flow::End)))
                .await
                .expect("frame writes");
            sender
                .finish()
                .await
                .expect("finish writes the end control");
            // The peer's link drops after its clean completion; the
            // already-delivered stream must still be readable.
            drop(a);
        };
        let receive = async {
            let (slots, mut claims) = claims();
            let (route, mut errors) = error_route();
            let driver = AcceptDriver::new(
                &mut b.acceptor,
                EPOCH,
                Speaker::Initiator,
                slots,
                route.clone(),
            );
            let mut receiver: StreamReceiver<_, Unit> =
                StreamReceiver::new(claims.take(stream), Speaker::Initiator, stream, route);
            let consume = async {
                assert_eq!(
                    receiver.next().await,
                    Some(Frame::Reaction(Reaction::Match, Flow::End)),
                );
                assert_eq!(receiver.finish().await, ReceiverFinish::Clean);
            };
            tokio::select! {
                biased;
                () = consume => {}
                error = errors.first() => {
                    panic!("a parked supply failure surfaced without a claimant: {error:?}")
                }
                error = driver.run() => {
                    panic!("a supply failure is not a peer violation: {error:?}")
                }
            }
        };
        join(send, receive).await;
    })
    .expect("the parked supply failure leaves the session live");
}

/// Wrap a frame the type-level exclusion admits.
fn reply_frame(frame: Frame<Unit>) -> ReplyFrame<Unit> {
    ReplyFrame::try_from(frame).expect("not a stream-end control")
}

/// Stream-end control is excluded from reply frames at the type level.
#[test]
fn stream_end_is_not_a_reply_frame() {
    assert!(ReplyFrame::<Unit>::try_from(Frame::End(End::Stream)).is_err());
}
