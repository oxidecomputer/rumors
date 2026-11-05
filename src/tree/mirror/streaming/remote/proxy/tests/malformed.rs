//! Full-stack rejection of malformed frames from a non-conforming peer.

use std::convert::Infallible;

use super::harness::{self, EndpointFailure, FrameMutation, FrameSelector, Script};
use crate::testing::{IoSide, run_to_quiescence};
use crate::tree::{
    Root as TreeRoot,
    arb::{early_first_child_dispute_pair, leaf_parent_dispute_pair},
    mirror::streaming::remote::{
        Error as RemoteError,
        codec::{
            DecodeErrorKind as CodecDecodeErrorKind, DecodeSignalError, End, Flow, ListingIssue,
            Signal, Stream,
        },
        pipe::PipeError,
    },
};

/// A wire-valid pair whose first root child is deeply disputed on both sides.
///
/// Depth matters here: the corruptions below hit an *early* frame of the
/// corrupt side, and with exchanges still owed at lower levels, the corruptor
/// provably cannot complete once its receiver aborts — so both sessions must
/// fail, not just the receiving one. (The opening question's listing rides the
/// greeting, never a data frame, so a corrupt side's first data frame is its
/// first *reply*.) The divergent branching dispute also guarantees nonempty
/// queries in both directions, which the unordered-query mutation needs to
/// find.
fn deep_pair() -> (TreeRoot, TreeRoot) {
    early_first_child_dispute_pair()
}

/// Extract the reserved state code from a full incoming error chain.
fn reserved_state(error: &RemoteError<Infallible>) -> Option<u64> {
    let RemoteError::Pipe(PipeError::Decode(error)) = error else {
        return None;
    };
    let CodecDecodeErrorKind::InvalidSignal(DecodeSignalError::State { state, .. }) = &error.kind
    else {
        return None;
    };
    Some(*state)
}

/// Borrow the remote error detected opposite the corrupt writer.
fn receiving_error<'a>(
    writer: IoSide,
    left: &'a Result<TreeRoot, EndpointFailure>,
    right: &'a Result<TreeRoot, EndpointFailure>,
) -> &'a RemoteError<Infallible> {
    let receiver = match writer {
        IoSide::Left => IoSide::Right,
        IoSide::Right => IoSide::Left,
    };
    harness::proxy_error(receiver, left, right)
        .unwrap_or_else(|error| panic!("receiver did not report the malformed frame: {error}"))
}

/// A reserved state code injected in either physical direction is
/// reported exactly by its receiving proxy, while the other endpoint also
/// terminates.
#[test]
fn reserved_signals_propagate_through_the_full_proxy() {
    for writer in [IoSide::Left, IoSide::Right] {
        let (left, right) = deep_pair();
        let script = Script::new(FrameSelector::First, FrameMutation::State(u8::MAX));
        let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
            left,
            right,
            matches!(writer, IoSide::Left).then(|| script.clone()),
            matches!(writer, IoSide::Right).then(|| script.clone()),
        ))
        .expect("a malformed signal must terminate both sessions");
        assert!(script.fired(), "the malformed signal was never injected");

        let actual = reserved_state(receiving_error(writer, &left_result, &right_result));
        assert_eq!(actual, Some(u64::from(u8::MAX)));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}

/// A signal placed in a forbidden stream phase is a typed placement failure.
///
/// The injected state is a continuing `Query` aimed at the initiator's
/// leaf-parent stream, whose grammar admits no nonempty query (nothing lies
/// below a leaf to ask about), and the receiving proxy retains the exact
/// placement rejection through the full stack. The corrupt side must be the
/// elected *initiator*, the only speaker on that stream, and the pair's
/// dispute must reach the leaf parents so that the stream carries a frame.
#[test]
fn phase_invalid_signal_propagates_through_the_full_proxy() {
    let (left, right, _) = leaf_parent_dispute_pair();
    let writer = if harness::left_initiates(&left, &right) {
        IoSide::Left
    } else {
        IoSide::Right
    };
    let script = Script::new(
        FrameSelector::Stream(Stream::MAX),
        FrameMutation::State(Signal::Query(Flow::Continue).state()),
    );
    let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
        left,
        right,
        matches!(writer, IoSide::Left).then(|| script.clone()),
        matches!(writer, IoSide::Right).then(|| script.clone()),
    ))
    .expect("phase-invalid signal must terminate both sessions");
    assert!(script.fired());
    let error = receiving_error(writer, &left_result, &right_result);
    assert!(matches!(
        error,
        RemoteError::Pipe(PipeError::Decode(error))
            if matches!(
                error.kind,
                CodecDecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(_))
            )
    ));
    assert!(left_result.is_err());
    assert!(right_result.is_err());
}

/// Canonical query ordering is enforced when corruption occurs inside an
/// otherwise conforming, live proxy session.
///
/// The corrupt physical side is arranged to be the elected *responder*:
/// with the opening question riding the greeting rather than a wire
/// frame, the responder's disputed-child listing is the one query frame
/// every divergent session still carries.
#[test]
fn unordered_query_propagates_through_the_full_proxy() {
    for writer in [IoSide::Left, IoSide::Right] {
        let (a, b) = deep_pair();
        let (initiator, responder) = harness::order_by_election(a, b);
        let (left, right) = match writer {
            IoSide::Left => (responder, initiator),
            IoSide::Right => (initiator, responder),
        };
        let script = Script::new(FrameSelector::Query, FrameMutation::UnorderQuery);
        let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
            left,
            right,
            matches!(writer, IoSide::Left).then(|| script.clone()),
            matches!(writer, IoSide::Right).then(|| script.clone()),
        ))
        .expect("unordered query must terminate both sessions");
        assert!(script.fired(), "no nonempty query reached the mutator");
        assert!(matches!(
            receiving_error(writer, &left_result, &right_result),
            RemoteError::Pipe(PipeError::Decode(error))
                if matches!(
                    error.kind,
                    CodecDecodeErrorKind::InvalidListing(ListingIssue::Order(_))
                )
        ));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}

/// A second reply manufactured after a valid reply has consumed the final
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
        receiving_error(IoSide::Left, &left_result, &right_result),
        RemoteError::UnaskedReply
    ));
    assert!(left_result.is_err());
    assert!(right_result.is_err());
}

/// A repeated end-of-stream frame is a frame on a logical stream that has
/// already ended, and the receiver rejects it as unasked, under varied
/// chunk sizes and scheduling; neither session parks.
///
/// The repeated frame is its side's first stream end, and frames the
/// receiver still needs follow it on the pipe, so the receiver's
/// demultiplexer always reaches it before its session can complete.
#[test]
fn a_repeated_stream_end_is_rejected_as_unasked() {
    use crate::testing::{IoPlan, memory_connection, wrap_io};
    use proptest::prelude::*;

    let (left, right) = deep_pair();
    // Finding this deep hash geometry is expensive; vary delivery over
    // clones of one fixture, without repeating that search for each case.
    proptest!(|(
        append_left in any::<bool>(),
        read_chunk in 1usize..64,
        write_chunk in 1usize..64,
        delays in proptest::collection::vec(0u8..=2, 0..32),
    )| {
        let script = Script::new(
            FrameSelector::State(Signal::End(End::Stream).state()),
            FrameMutation::Duplicate,
        );
        let plan = IoPlan {
            read_chunk,
            write_chunk,
            read_delays: delays.clone(),
            write_delays: delays,
            ..IoPlan::default()
        };
        let (left_end, right_end) = memory_connection(harness::TRANSPORT_CAPACITY);
        // Adversity below the mutation wrapper, where the repeated bytes
        // really enter the transport.
        let (left_read, left_write, _) =
            wrap_io(IoSide::Left, plan.clone(), left_end.read, left_end.write);
        let (right_read, right_write, _) =
            wrap_io(IoSide::Right, plan, right_end.read, right_end.write);
        let left_script = append_left.then(|| script.clone());
        let right_script = (!append_left).then(|| script.clone());
        let (left_result, right_result) = run_to_quiescence(harness::drive(
            harness::Topology::Production,
            harness::Backends::local(),
            left.clone(),
            right.clone(),
            (left_read, harness::ScriptedWrite::new(left_write, left_script)),
            (right_read, harness::ScriptedWrite::new(right_write, right_script)),
            harness::codec::<()>(),
            0,
        )).expect("a repeated stream end must not park either session");
        prop_assert!(script.fired());
        let writer = if append_left { IoSide::Left } else { IoSide::Right };
        let error = receiving_error(writer, &left_result, &right_result);
        prop_assert!(
            matches!(error, RemoteError::Pipe(PipeError::Unasked { .. })),
            "a frame after its stream's end surfaced as {error:?}"
        );
        prop_assert!(left_result.is_err());
        prop_assert!(right_result.is_err());
    });
}
