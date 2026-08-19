//! Full-stack sessions whose greeting declarations disagree with the
//! traffic behind them.
//!
//! The greeting's size words — `set_len`, `max_version_bytes` — are
//! peer-declared inputs to the window solve and the role election. These
//! sessions rewrite one word of the greeting a side *receives*
//! ([`harness::GreetingRewrite`]), so that side negotiates against a
//! declaration the peer's actual traffic does not honor: the buggy-peer
//! regime, exercised as a conformance tripwire (an authorized peer already
//! holds write authority, so none of this is a security boundary). Each
//! test pins what a lied declaration costs the receiving side.

use crate::message::Message;
use crate::testing::run_to_quiescence;
use crate::tree::{
    Action, Tree,
    arb::{early_first_child_dispute_pair, nth_party},
    mirror::{
        Error as MirrorError,
        streaming::{
            remote::{
                CodecDecodeError, CodecDecodeErrorKind, DecodeError, Error as RemoteError,
                StreamError,
            },
            window::FAN,
        },
    },
    typed::hash::MERKLE_HASH_LEN,
};

use super::harness::{self, GreetingRewrite};

/// The observable root hash of a reconciled `tree::Root`.
fn hash_of(root: &crate::tree::Root<()>) -> [u8; MERKLE_HASH_LEN] {
    Tree { root: root.clone() }.hash()
}

/// The expected reconciled union, computed by the in-memory join oracle.
fn union_hash(a: &crate::tree::Root<()>, b: &crate::tree::Root<()>) -> [u8; MERKLE_HASH_LEN] {
    let mut union = Tree { root: a.clone() };
    union.join(Tree { root: b.clone() });
    union.hash()
}

/// A divergent pair whose live set sizes differ strictly: one message
/// against four, on distinct parties, so the smaller side wins the
/// initiator election under honest declarations.
fn uneven_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    let mut small = Tree::new();
    small.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let mut large = Tree::new();
    large.act(
        &nth_party(0),
        (0..4).map(|_| Action::Insert(Message::new(()))),
    );
    (small.root, large.root)
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
/// side includes a genuinely batched multi-record run.
fn batched_uneven_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    let mut small = Tree::new();
    small.act(&nth_party(1), [Action::Insert(Message::new(()))]);
    let mut large = Tree::new();
    large.act(
        &nth_party(0),
        (0..BULK_MESSAGES).map(|_| Action::Insert(Message::new(()))),
    );
    (small.root, large.root)
}

/// A peer batching supply runs past the session minimum fails the session typed.
///
/// The deceived side hears the bulk peer's `target_message_size` as zero,
/// so it negotiates a zero session run budget while the peer keeps
/// batching at the true exchanged minimum: the first multi-record run to
/// arrive is a frame no encoder honoring the deceived side's minimum can
/// produce, and ingress rejects it as `OverbatchedRun` before buffering
/// its body — the greeting-declared budget premise enforced on the remote
/// decode path, completing the declaration matrix beside `set_len` and
/// `max_version_bytes`. Both endpoints terminate (link poisoning rides
/// any session error).
#[test]
fn understated_target_message_size_fails_the_session() {
    for receiver_left in [false, true] {
        let (small, large) = batched_uneven_pair();
        // The deceived side holds the small tree and hears the bulk
        // (supplying) side's target as zero.
        let rewrite = GreetingRewrite::target_message_size(0);
        let ((left, right), hears) = if receiver_left {
            ((small, large), (Some(rewrite), None))
        } else {
            ((large, small), (None, Some(rewrite)))
        };
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left, right, hears.0, hears.1,
        ))
        .expect("an overbatched supply run must terminate both sessions, not stall them");
        let receiver_error = if receiver_left {
            match &left {
                Err(MirrorError::Server(error)) => error,
                other => panic!(
                    "undetected target_message_size lie: the left proxy did not \
                     report the violation: {other:?}"
                ),
            }
        } else {
            match &right {
                Err(MirrorError::Client(error)) => error,
                other => panic!(
                    "undetected target_message_size lie: the right proxy did not \
                     report the violation: {other:?}"
                ),
            }
        };
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

/// An absurdly inflated `target_message_size` reading costs nothing.
///
/// The session budget is the minimum of the two targets, so the deceived
/// side's own target still governs both encoders exactly as an honest run
/// does, and the session converges on the union — the no-false-positive
/// dual of the understated lie.
#[test]
fn overstated_target_message_size_still_converges() {
    for receiver_left in [false, true] {
        let (small, large) = batched_uneven_pair();
        let expected = union_hash(&small, &large);
        let rewrite = GreetingRewrite::target_message_size(u64::MAX);
        let ((left, right), hears) = if receiver_left {
            ((small, large), (Some(rewrite), None))
        } else {
            ((large, small), (None, Some(rewrite)))
        };
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left, right, hears.0, hears.1,
        ))
        .expect("the session must terminate");
        let left = left.expect("left endpoint reconciles despite the inflated reading");
        let right = right.expect("right endpoint reconciles despite the inflated reading");
        assert_eq!(hash_of(&left), expected);
        assert_eq!(hash_of(&right), expected);
    }
}

/// A supplied version over the declared `max_version_bytes` fails the session typed.
///
/// The receiving side reports `OversizedVersion` at the first offending
/// record — the declared aggregate covers every version the peer's tree
/// materializes, so an arriving version over it voids the premise the
/// window solve priced the session with — and both endpoints terminate
/// (link poisoning rides any session error).
#[test]
fn understated_version_bytes_fail_the_session() {
    for receiver_left in [false, true] {
        let (left, right) = early_first_child_dispute_pair();
        let rewrite = GreetingRewrite::max_version_bytes(0);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left,
            right,
            receiver_left.then_some(rewrite),
            (!receiver_left).then_some(rewrite),
        ))
        .expect("an oversized supplied version must terminate both sessions");
        let receiver_error = if receiver_left {
            match &left {
                Err(MirrorError::Server(error)) => error,
                other => panic!("the left proxy did not report the violation: {other:?}"),
            }
        } else {
            match &right {
                Err(MirrorError::Client(error)) => error,
                other => panic!("the right proxy did not report the violation: {other:?}"),
            }
        };
        assert!(matches!(
            receiver_error,
            RemoteError::Decode(DecodeError::OversizedVersion { declared: 0, .. })
        ));
        assert!(left.is_err());
        assert!(right.is_err());
    }
}

/// A supply stream past the declared `set_len` fails the session typed.
///
/// The dual of the oversized-version guard, completing the declaration
/// matrix: the declared set length is a premise of the window solve's
/// occupancy envelopes and per-slot pricing, so honest supplies overrunning
/// it void what the window priced. The receiving side's wire decoder
/// reports `OverdrawnSupply` at the first record past the declaration,
/// before the payload takes backend custody — the walk's own ledger still
/// stands behind it for the in-process stack, but on the wire the ingress
/// charge fires first. The peer's endpoint is left to whatever its
/// schedule surfaces — here it may even complete, having already
/// reconciled before the deceived side's late ingestion tripped — which
/// is not this tripwire's concern (the containment wire test draws the
/// same line).
///
/// The rewrite shrinks the heard length of the honestly-smaller side, so
/// the role election stays complementary — a real under-declaring peer
/// elects from its own declared value, so only election-preserving
/// rewrites model one. The smaller side therefore still initiates; its
/// early supplies ride the opening stream and trip the deceived side's
/// ingress charge at their first record.
#[test]
fn understated_set_len_fails_the_session() {
    for receiver_left in [false, true] {
        let (small, large) = uneven_pair();
        // The receiver holds the large tree and hears the small
        // (initiating) side's declared length as zero.
        let rewrite = GreetingRewrite::set_len(0);
        let ((left, right), hears) = if receiver_left {
            ((large, small), (Some(rewrite), None))
        } else {
            ((small, large), (None, Some(rewrite)))
        };
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left, right, hears.0, hears.1,
        ))
        .expect("an overdrawn supply stream must terminate both sessions");
        let receiver_error = if receiver_left {
            match &left {
                Err(MirrorError::Server(error)) => error,
                other => panic!(
                    "undetected set_len lie: the left proxy did not report \
                     the violation: {other:?}"
                ),
            }
        } else {
            match &right {
                Err(MirrorError::Client(error)) => error,
                other => panic!(
                    "undetected set_len lie: the right proxy did not report \
                     the violation: {other:?}"
                ),
            }
        };
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(DecodeError::OverdrawnSupply { declared: 0 })
            ),
            "mistyped set_len violation: {receiver_error:?}",
        );
    }
}

/// A divergent pair of four messages against eight, on distinct parties:
/// the four-message side wins the initiator election, and its whole
/// exclusive content rides the opening-supply stream as one reply.
fn opening_bulk_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    let mut small = Tree::new();
    small.act(
        &nth_party(1),
        (0..4).map(|_| Action::Insert(Message::new(()))),
    );
    let mut large = Tree::new();
    large.act(
        &nth_party(0),
        (0..8).map(|_| Action::Insert(Message::new(()))),
    );
    (small.root, large.root)
}

/// A `set_len` lie surfacing *within* one still-open reply fails at the
/// offending record, at ingress.
///
/// The zero-declaration case above trips at a reply's first record; here
/// the heard declaration admits one leaf while the initiator's
/// opening-supply reply carries four, so the overrun surfaces
/// mid-reply. Only the wire decoder can detect it there — the walk's
/// ledger charges at absorption, after a decoded subtree materializes —
/// so the receiving side must report the ingress rejection carrying the
/// declaration it enforced, never absorb the reply whole first. The
/// rewrite shrinks the heard length of the honestly-smaller side to a
/// nonzero value below its traffic, preserving the role election.
#[test]
fn set_len_overrun_within_one_reply_fails_at_ingress() {
    for receiver_left in [false, true] {
        let (small, large) = opening_bulk_pair();
        // The receiver holds the large tree and hears the small
        // (initiating) side's declared length as one.
        let rewrite = GreetingRewrite::set_len(1);
        let ((left, right), hears) = if receiver_left {
            ((large, small), (Some(rewrite), None))
        } else {
            ((small, large), (None, Some(rewrite)))
        };
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left, right, hears.0, hears.1,
        ))
        .expect("a mid-reply overdrawn supply must terminate both sessions, not stall them");
        let receiver_error = if receiver_left {
            match &left {
                Err(MirrorError::Server(error)) => error,
                other => panic!(
                    "undetected within-one-reply set_len lie: the left proxy \
                     did not report the violation: {other:?}"
                ),
            }
        } else {
            match &right {
                Err(MirrorError::Client(error)) => error,
                other => panic!(
                    "undetected within-one-reply set_len lie: the right proxy \
                     did not report the violation: {other:?}"
                ),
            }
        };
        assert!(
            matches!(
                receiver_error,
                RemoteError::Decode(DecodeError::OverdrawnSupply { declared: 1 })
            ),
            "mistyped within-one-reply set_len violation: {receiver_error:?}",
        );
    }
}

/// An absurdly overstated `max_version_bytes` costs window width, never the session.
///
/// The budget solve saturates toward its floor on huge inputs
/// (`pathological_pricing_saturates_to_the_floor` pins the solve
/// itself), roles are unaffected, and the session converges on the
/// union.
#[test]
fn overstated_version_bytes_still_converge() {
    for receiver_left in [false, true] {
        let (left, right) = early_first_child_dispute_pair();
        let expected = union_hash(&left, &right);
        let rewrite = GreetingRewrite::max_version_bytes(u64::MAX);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left,
            right,
            receiver_left.then_some(rewrite),
            (!receiver_left).then_some(rewrite),
        ))
        .expect("the session must terminate");
        let left = left.expect("left endpoint reconciles despite the overstated bound");
        let right = right.expect("right endpoint reconciles despite the overstated bound");
        assert_eq!(hash_of(&left), expected);
        assert_eq!(hash_of(&right), expected);
    }
}

/// An absurdly overstated `set_len` heard from the larger side still converges.
///
/// The roles stay complementary — the smaller side initiates against
/// the honest pair and against the lie alike — so the lie distorts only
/// the receiving side's derived capacities.
#[test]
fn overstated_set_len_from_the_bulk_side_still_converges() {
    for small_left in [false, true] {
        let (small, large) = uneven_pair();
        let expected = union_hash(&small, &large);
        // The smaller side hears the larger side's set size as u64::MAX.
        let rewrite = GreetingRewrite::set_len(u64::MAX);
        let ((left, right), hears) = if small_left {
            ((small, large), (Some(rewrite), None))
        } else {
            ((large, small), (None, Some(rewrite)))
        };
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left, right, hears.0, hears.1,
        ))
        .expect("the session must terminate");
        let left = left.expect("left endpoint reconciles despite the overstated set size");
        let right = right.expect("right endpoint reconciles despite the overstated set size");
        assert_eq!(hash_of(&left), expected);
        assert_eq!(hash_of(&right), expected);
    }
}
