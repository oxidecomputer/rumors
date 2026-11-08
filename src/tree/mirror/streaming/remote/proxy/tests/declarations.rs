//! Full-stack checks for the greeting's session-size declarations.
//!
//! `set_len`, `max_version_bytes`, and `target_message_size` determine role
//! election, the receiver's ingress checks, and the session's run target.
//! These tests rewrite one received declaration
//! and check how the production session responds when the traffic disagrees.

use std::convert::Infallible;

use crate::testing::{IoSide, run_to_quiescence};
use crate::tree::{
    Root as TreeRoot,
    arb::early_first_child_dispute_pair,
    mirror::streaming::{
        FAN,
        remote::{
            Error as RemoteError,
            adapter::{BoundedPart, DecodeError as ReplyDecodeError},
            codec::{DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind},
            pipe::PipeError,
        },
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
/// `max_version_bytes`. Both endpoints terminate.
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
                RemoteError::Pipe(PipeError::Decode(CodecDecodeError {
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
/// materializes, so an arriving version over it shows the declaration
/// false — and both endpoints terminate.
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
/// The declared set length bounds the records the peer can supply. The
/// wire decoder therefore reports `OverdrawnSupply` at the first excess
/// record, before the payload reaches the backend. The sender may already
/// have completed; this test constrains only the receiver's diagnosis.
///
/// Reducing the smaller side's declaration preserves its initiator role, so
/// its supplies answering the responder's requests reach this check.
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

/// A `set_len` mismatch surfacing *within* one still-open reply fails at the
/// offending record, at ingress.
///
/// The zero-declaration case above trips at a reply's first record. Here the
/// initiator hears the bulk responder's declaration as two, while the
/// responder's opening reply supplies all of its exclusive root children at
/// once, so the overrun surfaces at the third record, mid-reply. The wire
/// decoder must reject that record before the decoded subtree reaches the
/// backend. A declaration of two still exceeds the initiator's one message,
/// so the election is unchanged.
#[test]
fn set_len_overrun_within_one_reply_fails_at_ingress() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = batched_uneven_pair();
        // The receiver holds the small tree and hears the bulk
        // (responding) side's declared length as two.
        let rewrite = GreetingRewrite::set_len(2);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            small,
            large,
            rewrite.clone(),
        ))
        .expect("a mid-reply overdrawn supply must terminate both sessions, not stall them");
        assert!(rewrite.fired(), "the set length was not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "within-one-reply set_len");
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(ReplyDecodeError::OverdrawnSupply { declared: 2 })
            ),
            "mistyped within-one-reply set_len violation: {receiver_error:?}",
        );
    }
}

/// A peer that declares `max_version_bytes` as `u64::MAX` still converges on
/// the union.
///
/// The declaration is an upper bound, so overstating it rejects no
/// conforming version, and the session accepts the extreme value. Roles are
/// unaffected.
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

/// A responder's root reply over the grandchild count its greeting declared
/// fails the session at the initiator's decoder.
///
/// The initiator hears the bulk responder's root count as one, so it
/// expects a reply of at most its own root children plus one reaction. The
/// responder's reply holds its four exclusive root children beside the
/// initiator's one, so the decoder rejects the reply at the reaction that
/// overruns, before it parks.
#[test]
fn understated_root_grandchildren_fail_the_opening_reply() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = uneven_pair();
        let rewrite = GreetingRewrite::grandchild_count(1);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            small,
            large,
            rewrite.clone(),
        ))
        .expect("an over-bound reply must terminate both sessions");
        assert!(rewrite.fired(), "the root count was not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "grandchildren");
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(ReplyDecodeError::ExceedsBound {
                    part: BoundedPart::Reactions,
                    bound: 2,
                })
            ),
            "mistyped root count violation: {receiver_error:?}",
        );
    }
}

/// A root count declared above the truth only enlarges the bound, so the
/// session completes with the honest join.
#[test]
fn overstated_root_grandchildren_complete_the_session() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = uneven_pair();
        let expected = harness::join_oracle(&small, &large);
        let rewrite = GreetingRewrite::grandchild_count(65_536);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            small,
            large,
            rewrite.clone(),
        ))
        .expect("an overstated count must not stall the session");
        assert!(rewrite.fired(), "the root count was not rewritten");
        assert_eq!(left.expect("the left session completes"), expected);
        assert_eq!(right.expect("the right session completes"), expected);
    }
}

/// Messages each side of [`listing_pair`] holds: enough that every root
/// child either side holds has several children on each side.
const LISTED_MESSAGES: usize = 2_000;

/// A divergent pair of [`LISTED_MESSAGES`] messages against one fewer, on
/// distinct parties: the smaller side initiates, and its greeting's root
/// listing names every root child with a count well above one.
fn listing_pair() -> (TreeRoot, TreeRoot) {
    let (large, small) = harness::disjoint_pair(LISTED_MESSAGES, LISTED_MESSAGES - 1);
    (small, large)
}

/// An initiator's reply about a root child over the grandchild count its
/// greeting listed for that child fails the session at the responder's
/// decoder.
///
/// The responder hears every count in the initiator's root listing as one,
/// so each question it asks about a disputed root child expects at most
/// one listing entry. With about eight messages under each root child on
/// each side, the initiator's replies list its children of every disputed
/// grandchild, so the first such reply overruns its listing bound.
#[test]
fn understated_listing_grandchildren_fail_a_reply() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = listing_pair();
        let rewrite = GreetingRewrite::listing_grandchildren(1);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            large,
            small,
            rewrite.clone(),
        ))
        .expect("an over-bound reply must terminate both sessions");
        assert!(rewrite.fired(), "the listing counts were not rewritten");
        let receiver_error = receiver_error(receiver, &left, &right, "listing grandchildren");
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(ReplyDecodeError::ExceedsBound { .. })
            ),
            "mistyped listing count violation: {receiver_error:?}",
        );
    }
}

/// Listing counts declared above the truth only enlarge the bounds, so the
/// session completes with the honest join.
#[test]
fn overstated_listing_grandchildren_complete_the_session() {
    for receiver in [IoSide::Left, IoSide::Right] {
        let (small, large) = listing_pair();
        let expected = harness::join_oracle(&small, &large);
        let rewrite = GreetingRewrite::listing_grandchildren(65_536);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_for(
            receiver,
            large,
            small,
            rewrite.clone(),
        ))
        .expect("overstated counts must not stall the session");
        assert!(rewrite.fired(), "the listing counts were not rewritten");
        assert_eq!(left.expect("the left session completes"), expected);
        assert_eq!(right.expect("the right session completes"), expected);
    }
}
