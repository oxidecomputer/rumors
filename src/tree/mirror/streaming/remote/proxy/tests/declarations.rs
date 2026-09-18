//! Full-stack checks for the greeting's session-size declarations.
//!
//! `set_len`, `max_version_bytes`, and `target_message_size` determine role
//! election or window sizing. These tests rewrite one received declaration
//! and check how the production session responds when the traffic disagrees.

use std::convert::Infallible;

use crate::testing::{IoSide, run_to_quiescence};
use crate::tree::{
    Root as TreeRoot,
    arb::early_first_child_dispute_pair,
    mirror::streaming::{
        remote::{
            Error as RemoteError,
            adapter::DecodeError as ReplyDecodeError,
            codec::{DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind},
            streams::StreamError,
        },
        window::FAN,
    },
};

use super::harness::{self, GreetingRewrite};

/// Borrow the proxy error reported by the endpoint that heard a rewrite.
fn receiver_error<'a>(
    receiver: IoSide,
    left: &'a Result<TreeRoot, harness::EndpointFailure>,
    right: &'a Result<TreeRoot, harness::EndpointFailure>,
    declaration: &str,
) -> &'a RemoteError<Infallible> {
    harness::proxy_error(receiver, left, right).unwrap_or_else(|error| {
        panic!("{declaration} mismatch was not reported by the receiver: {error}")
    })
}

/// A divergent pair whose live set sizes differ strictly: one message
/// against four, on distinct parties, so the smaller side wins the
/// initiator election under their actual declarations.
fn uneven_pair() -> (TreeRoot, TreeRoot) {
    let (large, small) = harness::disjoint_pair(4, 1);
    (small, large)
}

/// Messages the bulk side of [`batched_uneven_pair`] originates: one more
/// than the root fan, so at least one root child holds two or more leaves
/// by pigeonhole.
const BULK_MESSAGES: usize = FAN + 1;

/// A divergent pair of one message against [`BULK_MESSAGES`], on distinct
/// parties.
///
/// The small side wins the initiator election, and the bulk side's
/// exclusive root children — at least one of which spans multiple leaves
/// — reach it as whole supplied subtrees, so the traffic toward the small
/// side includes a batched multi-record run.
fn batched_uneven_pair() -> (TreeRoot, TreeRoot) {
    let (large, small) = harness::disjoint_pair(BULK_MESSAGES, 1);
    (small, large)
}

/// A peer batching supply runs past the session minimum fails the session.
///
/// The receiver hears the bulk peer's `target_message_size` as zero,
/// so it negotiates a zero session run budget while the peer keeps
/// batching at the true exchanged minimum: the first multi-record run to
/// arrive is a frame no encoder honoring the receiver's minimum can
/// produce, and ingress rejects it as `OverbatchedRun` before buffering
/// its body — the greeting-declared budget premise enforced on the remote
/// decode path, completing the declaration matrix beside `set_len` and
/// `max_version_bytes`. Both endpoints terminate (link poisoning rides
/// any session error).
#[test]
fn understated_target_message_size_fails_the_session() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = batched_uneven_pair();
        // The receiver holds the small tree and hears the bulk side's target
        // as zero.
        let rewrite = GreetingRewrite::target_message_size(0);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            small,
            large,
            rewrite.clone(),
        ))
        .expect("an overbatched supply run must terminate both sessions, not stall them");
        assert!(rewrite.fired(), "the target size was not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "target_message_size");
        assert!(
            matches!(
                receiver_error,
                RemoteError::Stream(StreamError::Decode(CodecDecodeError {
                    kind: CodecDecodeErrorKind::OverbatchedRun { budget: 0, .. },
                    ..
                }))
            ),
            "mistyped target_message_size violation: {receiver_error:?}",
        );
        assert!(left.is_err());
        assert!(right.is_err());
    }
}

/// An overstated `target_message_size` still converges.
///
/// The session uses the smaller target, so the receiver's own value still
/// governs both encoders.
#[test]
fn overstated_target_message_size_still_converges() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = batched_uneven_pair();
        let expected = harness::join_oracle(&small, &large);
        let rewrite = GreetingRewrite::target_message_size(u64::MAX);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            small,
            large,
            rewrite.clone(),
        ))
        .expect("the session must terminate");
        assert!(rewrite.fired(), "the target size was not rewritten");
        let left = left.expect("left endpoint reconciles despite the inflated reading");
        let right = right.expect("right endpoint reconciles despite the inflated reading");
        assert_eq!(left, expected);
        assert_eq!(right, expected);
    }
}

/// A supplied version over the declared `max_version_bytes` fails the session.
///
/// The receiving side reports `OversizedVersion` at the first offending
/// record — the declared aggregate covers every version the peer's tree
/// materializes, so an arriving version over it voids the premise the
/// window solve priced the session with — and both endpoints terminate
/// (link poisoning rides any session error).
#[test]
fn understated_version_bytes_fail_the_session() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (left, right) = early_first_child_dispute_pair();
        let rewrite = GreetingRewrite::max_version_bytes(0);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left,
            right,
            matches!(receiver, IoSide::Left).then(|| rewrite.clone()),
            matches!(receiver, IoSide::Right).then(|| rewrite.clone()),
        ))
        .expect("an oversized supplied version must terminate both sessions");
        assert!(rewrite.fired(), "the version size was not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "max_version_bytes");
        assert!(matches!(
            receiver_error,
            RemoteError::Decode(ReplyDecodeError::OversizedVersion { declared: 0, .. })
        ));
        assert!(left.is_err());
        assert!(right.is_err());
    }
}

/// A supply stream past the declared `set_len` fails the session.
///
/// The declared set length bounds the occupancy priced by the window. The
/// wire decoder therefore reports `OverdrawnSupply` at the first excess
/// record, before the payload reaches the backend. The sender may already
/// have completed; this test constrains only the receiver's diagnosis.
///
/// Reducing the smaller side's declaration preserves its initiator role, so
/// its early supplies reach this check on the opening stream.
#[test]
fn understated_set_len_fails_the_session() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = uneven_pair();
        // The receiver holds the large tree and hears the small
        // (initiating) side's declared length as zero.
        let rewrite = GreetingRewrite::set_len(0);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            large,
            small,
            rewrite.clone(),
        ))
        .expect("an overdrawn supply stream must terminate both sessions");
        assert!(rewrite.fired(), "the set length was not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "set_len");
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(ReplyDecodeError::OverdrawnSupply { declared: 0 })
            ),
            "mistyped set_len violation: {receiver_error:?}",
        );
    }
}

/// A divergent pair of four messages against eight, on distinct parties:
/// the four-message side wins the initiator election, and its whole
/// exclusive content rides the opening-supply stream as one reply.
fn opening_bulk_pair() -> (TreeRoot, TreeRoot) {
    let (large, small) = harness::disjoint_pair(8, 4);
    (small, large)
}

/// A `set_len` mismatch surfacing *within* one still-open reply fails at the
/// offending record, at ingress.
///
/// The zero-declaration case above trips at a reply's first record; here
/// the heard declaration admits one leaf while the initiator's
/// opening-supply reply carries four, so the overrun surfaces
/// mid-reply. The wire decoder must reject that record before the decoded
/// subtree reaches the backend. Reducing the smaller side's declaration to a
/// nonzero value preserves its initiator role.
#[test]
fn set_len_overrun_within_one_reply_fails_at_ingress() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = opening_bulk_pair();
        // The receiver holds the large tree and hears the small
        // (initiating) side's declared length as one.
        let rewrite = GreetingRewrite::set_len(1);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            large,
            small,
            rewrite.clone(),
        ))
        .expect("a mid-reply overdrawn supply must terminate both sessions, not stall them");
        assert!(rewrite.fired(), "the set length was not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "within-one-reply set_len");
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(ReplyDecodeError::OverdrawnSupply { declared: 1 })
            ),
            "mistyped within-one-reply set_len violation: {receiver_error:?}",
        );
    }
}

/// An overstated `max_version_bytes` costs window width, not convergence.
///
/// The budget solve saturates toward its floor on huge inputs
/// (`pathological_pricing_saturates_to_the_floor` pins the solve itself), roles
/// are unaffected, and the session converges on the union.
#[test]
fn overstated_version_bytes_still_converge() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (left, right) = early_first_child_dispute_pair();
        let expected = harness::join_oracle(&left, &right);
        let rewrite = GreetingRewrite::max_version_bytes(u64::MAX);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left,
            right,
            matches!(receiver, IoSide::Left).then(|| rewrite.clone()),
            matches!(receiver, IoSide::Right).then(|| rewrite.clone()),
        ))
        .expect("the session must terminate");
        assert!(rewrite.fired(), "the version size was not rewritten");
        let left = left.expect("left endpoint reconciles despite the overstated bound");
        let right = right.expect("right endpoint reconciles despite the overstated bound");
        assert_eq!(left, expected);
        assert_eq!(right, expected);
    }
}

/// An overstated `set_len` heard from the larger side still converges.
///
/// The roles stay complementary, so the changed declaration affects only
/// the receiving side's derived capacities.
#[test]
fn overstated_set_len_from_the_bulk_side_still_converges() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = uneven_pair();
        let expected = harness::join_oracle(&small, &large);
        // The smaller side hears the larger side's set size as u64::MAX.
        let rewrite = GreetingRewrite::set_len(u64::MAX);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            small,
            large,
            rewrite.clone(),
        ))
        .expect("the session must terminate");
        assert!(rewrite.fired(), "the set length was not rewritten");
        let left = left.expect("left endpoint reconciles despite the overstated set size");
        let right = right.expect("right endpoint reconciles despite the overstated set size");
        assert_eq!(left, expected);
        assert_eq!(right, expected);
    }
}
