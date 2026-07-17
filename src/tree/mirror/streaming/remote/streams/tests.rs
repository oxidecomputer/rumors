//! Unit tests for lazy stream establishment, labeling, and claim routing.

use futures::{StreamExt, future::join};

use crate::link::{Acceptor, memory};
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::remote::codec::{End, Flow, Frame, Reaction, Speaker, Stream};

use super::{
    AcceptDriver, AcceptError, ReceiverFinish, ReplyFrame, StreamReceiver, StreamSender, claims,
    error_route, label,
};

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

/// Wrap a frame the type-level exclusion admits.
fn reply_frame(frame: Frame<Unit>) -> ReplyFrame<Unit> {
    ReplyFrame::try_from(frame).expect("not a stream-end control")
}

/// Stream-end control is excluded from reply frames at the type level.
#[test]
fn stream_end_is_not_a_reply_frame() {
    assert!(ReplyFrame::<Unit>::try_from(Frame::End(End::Stream)).is_err());
}
