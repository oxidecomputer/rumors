//! Full-stack rejection of peer-controlled malformed frames.

use super::harness::{self, FrameMutation, FrameSelector, Script};
use crate::testing::run_to_quiescence;
use crate::tree::{
    arb::early_first_child_dispute_pair,
    mirror::{
        Error as MirrorError,
        streaming::remote::{
            CodecDecodeErrorKind, DecodeSignalError, Error as RemoteError, StreamError,
        },
    },
};

/// An honestly built, wire-valid pair whose first root child is deeply
/// disputed on both sides.
///
/// Depth matters here: the corruptions below hit an *early* frame of the
/// corrupt side, and with exchanges still owed at lower levels, the
/// corruptor provably cannot complete once its receiver aborts — so both
/// sessions must fail, not just the receiving one. (The opening question no
/// longer crosses as a frame — its listing rides the greeting — so a
/// corrupt side's first data frame is its first *reply*.) The divergent
/// branching dispute also guarantees nonempty queries in both directions,
/// which the unordered-query mutation needs to find.
fn deep_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    early_first_child_dispute_pair()
}

/// Extract the reserved signal byte from a full incoming error chain.
fn reserved_signal(error: &RemoteError<std::convert::Infallible>) -> Option<u8> {
    let RemoteError::Stream(StreamError::Decode(error)) = error else {
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
        let (left, right) = deep_pair();
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

/// A signal placed in a forbidden stream phase is a typed placement failure.
///
/// The injected byte is a continuing `Match` aimed at the opening-question
/// stream — which carries no frames at all now that the opening rides the
/// greeting — and the receiving proxy retains the exact placement rejection
/// through the full stack.
#[test]
fn phase_invalid_signal_propagates_through_the_full_proxy() {
    const OPENING_MATCH_CONTINUE_SIGNAL: u8 = 0;

    let (left, right) = deep_pair();
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
        RemoteError::Stream(StreamError::Decode(error))
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
///
/// The corrupt physical side is arranged to be the elected *responder*
/// (the lesser canonical version): with the opening question riding the
/// greeting rather than a wire frame, the responder's disputed-child
/// listing is the one query frame every divergent session still carries.
#[test]
fn unordered_query_propagates_through_the_full_proxy() {
    for corrupt_left in [false, true] {
        let (a, b) = deep_pair();
        let (greater, lesser) = if a.ceiling.as_bytes() > b.ceiling.as_bytes() {
            (a, b)
        } else {
            (b, a)
        };
        let (left, right) = if corrupt_left {
            (lesser, greater)
        } else {
            (greater, lesser)
        };
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
            RemoteError::Stream(StreamError::Decode(error))
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
    let (left, right) = deep_pair();
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
        let (left, right) = deep_pair();
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
            RemoteError::Stream(StreamError::AfterEnd { .. })
        ));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}
