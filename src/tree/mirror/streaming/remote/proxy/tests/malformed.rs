//! Full-stack rejection of peer-controlled malformed frames.

use super::harness::{self, FrameMutation, FrameSelector, Script};
use crate::message::Message;
use crate::tree::{
    Action, Tree,
    arb::nth_party,
    mirror::{
        Error as MirrorError,
        streaming::{
            remote::{CodecDecodeErrorKind, DecodeSignalError, DemuxError, Error as RemoteError},
            testing::run_to_quiescence,
        },
    },
};

/// Construct a small divergent pair which necessarily opens the descent.
fn divergent_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    let mut left = Tree::new();
    left.act(&nth_party(0), [Action::Insert(Message::new(()))]);
    let mut right = Tree::new();
    right.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    (left.root, right.root)
}

/// Construct enough independent paths to guarantee a multi-child query.
fn broad_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    let mut left = Tree::new();
    left.act(
        &nth_party(0),
        (0..16).map(|_| Action::Insert(Message::new(()))),
    );
    let mut right = Tree::new();
    right.act(
        &nth_party(1),
        (0..16).map(|_| Action::Insert(Message::new(()))),
    );
    (left.root, right.root)
}

/// Extract the reserved signal byte from a full incoming error chain.
fn reserved_signal(error: &RemoteError<std::convert::Infallible>) -> Option<u8> {
    let RemoteError::Incoming(DemuxError::Codec(error)) = error else {
        return None;
    };
    let CodecDecodeErrorKind::InvalidSignal(DecodeSignalError::Reserved(signal)) = &error.kind
    else {
        return None;
    };
    Some(signal.byte())
}

/// Borrow the remote error detected opposite the corrupt writer.
fn receiving_error<'a>(
    corrupt_left: bool,
    left: &'a Result<crate::tree::Root<()>, harness::LeftError>,
    right: &'a Result<crate::tree::Root<()>, harness::RightError>,
) -> &'a RemoteError<std::convert::Infallible> {
    if corrupt_left {
        match right {
            Err(MirrorError::Client(error)) => error,
            other => panic!("receiving right proxy did not report the fault: {other:?}"),
        }
    } else {
        match left {
            Err(MirrorError::Server(error)) => error,
            other => panic!("receiving left proxy did not report the fault: {other:?}"),
        }
    }
}

/// A reserved signal injected in either physical direction is reported
/// exactly by its receiving proxy, while the other endpoint also terminates.
#[test]
fn reserved_signals_propagate_through_the_full_proxy() {
    for corrupt_left in [false, true] {
        let (left, right) = divergent_pair();
        let script = Script::new(FrameSelector::First, FrameMutation::Signal(u8::MAX));
        let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
            left,
            right,
            corrupt_left.then(|| script.clone()),
            (!corrupt_left).then(|| script.clone()),
        ))
        .expect("a malformed signal must terminate both sessions");
        assert!(script.fired(), "the malformed signal was never injected");

        let actual = reserved_signal(receiving_error(corrupt_left, &left_result, &right_result));
        assert_eq!(actual, Some(u8::MAX));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}

/// A syntactically valid signal in the initiator's forbidden opening phase is
/// retained as a typed placement failure through the proxy.
#[test]
fn phase_invalid_signal_propagates_through_the_full_proxy() {
    const OPENING_MATCH_CONTINUE_SIGNAL: u8 = 0;

    let (left, right) = divergent_pair();
    let corrupt_left = right.ceiling.as_bytes() < left.ceiling.as_bytes();
    let script = Script::new(
        FrameSelector::First,
        FrameMutation::Signal(OPENING_MATCH_CONTINUE_SIGNAL),
    );
    let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
        left,
        right,
        corrupt_left.then(|| script.clone()),
        (!corrupt_left).then(|| script.clone()),
    ))
    .expect("phase-invalid signal must terminate both sessions");
    assert!(script.fired());
    let error = receiving_error(corrupt_left, &left_result, &right_result);
    assert!(matches!(
        error,
        RemoteError::Incoming(DemuxError::Codec(error))
            if matches!(
                error.kind,
                CodecDecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(_))
            )
    ));
    assert!(left_result.is_err());
    assert!(right_result.is_err());
}

/// Canonical query ordering is enforced when corruption occurs inside an
/// otherwise honest, live proxy session.
#[test]
fn unordered_query_propagates_through_the_full_proxy() {
    for corrupt_left in [false, true] {
        let (left, right) = broad_pair();
        let script = Script::new(FrameSelector::Query, FrameMutation::UnorderQuery);
        let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
            left,
            right,
            corrupt_left.then(|| script.clone()),
            (!corrupt_left).then(|| script.clone()),
        ))
        .expect("unordered query must terminate both sessions");
        assert!(script.fired(), "no nonempty query reached the mutator");
        assert!(matches!(
            receiving_error(corrupt_left, &left_result, &right_result),
            RemoteError::Incoming(DemuxError::Codec(error))
                if matches!(error.kind, CodecDecodeErrorKind::QueryOutOfOrder(_))
        ));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}

/// A second reply manufactured after an honest reply has consumed the final
/// scope reaches the proxy's reply-accounting check.
#[test]
fn duplicated_reply_is_rejected_as_unasked() {
    let (left, right) = divergent_pair();
    let script = Script::new(FrameSelector::EndingReaction, FrameMutation::Duplicate);
    let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
        left,
        right,
        Some(script.clone()),
        None,
    ))
    .expect("duplicated final reply must terminate both sessions");
    assert!(script.fired(), "no ending reaction reached the mutator");
    assert!(matches!(
        receiving_error(true, &left_result, &right_result),
        RemoteError::UnaskedReply
    ));
    assert!(left_result.is_err());
    assert!(right_result.is_err());
}

/// Duplicating a stream-end frame is rejected as traffic after closure rather
/// than being mistaken for a second clean end.
#[test]
fn duplicate_stream_end_is_rejected_by_the_session() {
    const STREAM_END_STATE: u8 = 9;

    for corrupt_left in [false, true] {
        let (left, right) = divergent_pair();
        let script = Script::new(
            FrameSelector::State(STREAM_END_STATE),
            FrameMutation::Duplicate,
        );
        let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
            left,
            right,
            corrupt_left.then(|| script.clone()),
            (!corrupt_left).then(|| script.clone()),
        ))
        .expect("duplicate stream end must terminate both sessions");
        assert!(script.fired(), "no stream-end frame reached the mutator");
        assert!(matches!(
            receiving_error(corrupt_left, &left_result, &right_result),
            RemoteError::Incoming(DemuxError::FrameAfterEnd { .. })
        ));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}
