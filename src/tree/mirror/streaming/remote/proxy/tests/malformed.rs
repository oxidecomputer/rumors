//! Full-stack rejection of peer-controlled malformed frames.

use super::harness::{self, EndpointError, EndpointFailure, FrameMutation, FrameSelector, Script};
use crate::link::{Acceptor, Done, Link, MemoryAcceptor, MemoryConnector, MemoryLink};
use crate::testing::run_to_quiescence;
use crate::tree::{
    arb::early_first_child_dispute_pair,
    mirror::streaming::remote::{
        Error as RemoteError,
        codec::{DecodeErrorKind as CodecDecodeErrorKind, DecodeSignalError, ListingIssue},
        streams::StreamError,
    },
};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, DuplexStream};

/// An honestly built, wire-valid pair whose first root child is deeply
/// disputed on both sides.
///
/// Depth matters here: the corruptions below hit an *early* frame of the
/// corrupt side, and with exchanges still owed at lower levels, the
/// corruptor provably cannot complete once its receiver aborts — so both
/// sessions must fail, not just the receiving one. (The opening question's
/// listing rides the greeting, never a data frame, so a corrupt side's
/// first data frame is its first *reply*.) The divergent
/// branching dispute also guarantees nonempty queries in both directions,
/// which the unordered-query mutation needs to find.
fn deep_pair() -> (crate::tree::Root, crate::tree::Root) {
    early_first_child_dispute_pair()
}

/// Extract the reserved state code from a full incoming error chain.
fn reserved_state(error: &RemoteError<std::convert::Infallible>) -> Option<u64> {
    let RemoteError::Stream(StreamError::Decode(error)) = error else {
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
    corrupt_left: bool,
    left: &'a Result<crate::tree::Root, EndpointFailure>,
    right: &'a Result<crate::tree::Root, EndpointFailure>,
) -> &'a RemoteError<std::convert::Infallible> {
    let (side, receiving) = if corrupt_left {
        ("right", right)
    } else {
        ("left", left)
    };
    match receiving {
        Err(EndpointError::Proxy(error)) => error,
        other => panic!("receiving {side} proxy did not report the fault: {other:?}"),
    }
}

/// A reserved state code injected in either physical direction is
/// reported exactly by its receiving proxy, while the other endpoint also
/// terminates.
#[test]
fn reserved_signals_propagate_through_the_full_proxy() {
    for corrupt_left in [false, true] {
        let (left, right) = deep_pair();
        let script = Script::new(FrameSelector::First, FrameMutation::State(u8::MAX));
        let (left_result, right_result) = run_to_quiescence(harness::reconcile_scripted(
            left,
            right,
            corrupt_left.then(|| script.clone()),
            (!corrupt_left).then(|| script.clone()),
        ))
        .expect("a malformed signal must terminate both sessions");
        assert!(script.fired(), "the malformed signal was never injected");

        let actual = reserved_state(receiving_error(corrupt_left, &left_result, &right_result));
        assert_eq!(actual, Some(u64::from(u8::MAX)));
        assert!(left_result.is_err());
        assert!(right_result.is_err());
    }
}

/// A signal placed in a forbidden stream phase is a typed placement failure.
///
/// The injected state is a continuing `Match` aimed at the opening-supply
/// stream — whose grammar admits only supplies and ends — and the
/// receiving proxy retains the exact placement rejection through the full
/// stack. The corrupt side must be the elected *initiator*: its first
/// data frame is an opening supply, the stream the mutated state must
/// land in.
#[test]
fn phase_invalid_signal_propagates_through_the_full_proxy() {
    const MATCH_CONTINUE_STATE: u8 = 0;

    let (left, right) = deep_pair();
    let corrupt_left = harness::left_initiates(&left, &right);
    let script = Script::new(
        FrameSelector::First,
        FrameMutation::State(MATCH_CONTINUE_STATE),
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
/// The corrupt physical side is arranged to be the elected *responder*:
/// with the opening question riding the greeting rather than a wire
/// frame, the responder's disputed-child listing is the one query frame
/// every divergent session still carries.
#[test]
fn unordered_query_propagates_through_the_full_proxy() {
    for corrupt_left in [false, true] {
        let (a, b) = deep_pair();
        let (initiator, responder) = if harness::left_initiates(&a, &b) {
            (a, b)
        } else {
            (b, a)
        };
        let (left, right) = if corrupt_left {
            (responder, initiator)
        } else {
            (initiator, responder)
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
                if matches!(
                    error.kind,
                    CodecDecodeErrorKind::InvalidListing(ListingIssue::Order(_))
                )
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

/// Receive halves returned at protocol completion, kept alive for inspection.
type CompletedReads = Arc<Mutex<Vec<DuplexStream>>>;

/// A memory acceptor that keeps completed halves instead of closing them.
struct RetainReads {
    inner: MemoryAcceptor,
    completed: CompletedReads,
}

/// Leave completed halves open for the next owner, as a pooling link does.
impl Acceptor for RetainReads {
    type Rx = DuplexStream;

    /// Save the receive half only when the protocol calls its completion callback.
    async fn accept(&mut self) -> std::io::Result<(Self::Rx, Done<Self::Rx>)> {
        let (rx, _) = self.inner.accept().await?;
        let completed = self.completed.clone();
        Ok((rx, Done::new(move |rx| completed.lock().unwrap().push(rx))))
    }
}

/// Retain received streams without changing their bytes or delivery behavior.
fn retain_reads(
    link: MemoryLink,
    completed: CompletedReads,
) -> Link<DuplexStream, DuplexStream, MemoryConnector, RetainReads> {
    link.map_transport(|control_read, control_write, connector, acceptor| {
        (
            control_read,
            control_write,
            connector,
            RetainReads {
                inner: acceptor,
                completed,
            },
        )
    })
}

/// Completing a stream leaves its trailing frame unread under varied
/// chunk sizes and scheduling. Both replicas still reach the join.
#[test]
fn bytes_past_the_stream_end_are_never_read() {
    use crate::link::memory_with_capacity;
    use crate::testing::{IoPlan, IoSide, wrap_link};
    use crate::tree::mirror::streaming::window::WindowConfig;
    use proptest::prelude::*;

    let (left, right) = deep_pair();
    let mut expected = crate::tree::Tree::<()>::from_root(left.clone());
    expected.join(crate::tree::Tree::from_root(right.clone()));
    // Finding this deep hash geometry is expensive; vary delivery over
    // clones of one fixture, without repeating that search for each case.
    proptest!(|(
        append_left in any::<bool>(),
        read_chunk in 1usize..64,
        write_chunk in 1usize..64,
        delays in proptest::collection::vec(0u8..=2, 0..32),
    )| {
        let script = Script::new(FrameSelector::State(9), FrameMutation::Duplicate);
        let plan = IoPlan {
            read_chunk,
            write_chunk,
            read_delays: delays.clone(),
            write_delays: delays,
            ..IoPlan::default()
        };
        let (left_link, right_link) = memory_with_capacity(37);
        let completed = CompletedReads::default();
        let left_link = retain_reads(left_link, completed.clone());
        let right_link = retain_reads(right_link, completed.clone());
        // Count below the mutation wrapper, where the duplicated bytes really
        // enter the transport. Counting above it would miss those bytes.
        let (left_link, left_io) = wrap_link(IoSide::Left, plan.clone(), left_link);
        let (right_link, right_io) = wrap_link(IoSide::Right, plan, right_link);
        let (left_result, right_result) = run_to_quiescence(harness::drive(
            harness::Topology::Production,
            harness::Backends::local(),
            left.clone(),
            right.clone(),
            harness::scripted(left_link, append_left.then(|| script.clone())),
            harness::scripted(right_link, (!append_left).then(|| script.clone())),
            harness::codec::<()>(),
            WindowConfig::FLOOR,
        )).expect("a frame beyond stream end must not affect progress");
        prop_assert!(script.fired());
        prop_assert_eq!(&left_result.unwrap(), &expected.root);
        prop_assert_eq!(&right_result.unwrap(), &expected.root);

        let (sender, receiver) = if append_left {
            (left_io.snapshot(), right_io.snapshot())
        } else {
            (right_io.snapshot(), left_io.snapshot())
        };
        // End(Stream) is [stream, state]: one byte each for its array head,
        // stream index, and state. Exactly those three extra bytes must remain.
        prop_assert_eq!(sender.write_bytes, receiver.read_bytes + 3);
        prop_assert_eq!(receiver.write_bytes, sender.read_bytes);

        // These are the actual halves returned through Done, below the byte
        // counters. Each writer has finished, so reading their remainder ends.
        let halves = std::mem::take(&mut *completed.lock().unwrap());
        let remaining = run_to_quiescence(async {
            let mut bytes = Vec::new();
            for mut rx in halves {
                rx.read_to_end(&mut bytes).await.unwrap();
            }
            bytes
        }).expect("completed streams have no unfinished writers");
        prop_assert_eq!(remaining.len(), 3);
        prop_assert_eq!(remaining[0], 0x82);
        prop_assert!(remaining[1] < crate::link::STREAM_COUNT as u8);
        prop_assert_eq!(remaining[2], 9);
    });
}
