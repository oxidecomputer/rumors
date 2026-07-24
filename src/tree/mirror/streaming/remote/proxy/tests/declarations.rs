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
};

use super::harness::{self, GreetingRewrite};

/// The observable root hash of a reconciled `tree::Root`.
fn hash_of(root: &crate::tree::Root<()>) -> [u8; 16] {
    Tree { root: root.clone() }.hash()
}

/// The expected reconciled union, computed by the in-memory join oracle.
fn union_hash(a: &crate::tree::Root<()>, b: &crate::tree::Root<()>) -> [u8; 16] {
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

/// A peer whose supplied versions exceed its greeting-declared
/// `max_version_bytes` is absorbed silently: the declaration feeds only
/// the window solve and no ingress path re-checks arriving versions
/// against it, so a session whose received declaration is rewritten to
/// zero still converges on the union while every supplied version
/// arrives over the declared bound.
#[test]
fn understated_version_bytes_are_absorbed_silently() {
    for victim_left in [false, true] {
        let (left, right) = early_first_child_dispute_pair();
        let expected = union_hash(&left, &right);
        let rewrite = GreetingRewrite::max_version_bytes(0);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left,
            right,
            victim_left.then_some(rewrite),
            (!victim_left).then_some(rewrite),
        ))
        .expect("the session must terminate");
        let left = left.expect("left endpoint reconciles despite the understated bound");
        let right = right.expect("right endpoint reconciles despite the understated bound");
        assert_eq!(hash_of(&left), expected);
        assert_eq!(hash_of(&right), expected);
    }
}

/// An absurdly overstated `max_version_bytes` declaration costs only
/// window width, never the session: the budget solve saturates toward its
/// floor on huge inputs (`pathological_pricing_saturates_to_the_floor`
/// pins the solve itself), roles are unaffected, and the session
/// converges on the union.
#[test]
fn overstated_version_bytes_still_converge() {
    for victim_left in [false, true] {
        let (left, right) = early_first_child_dispute_pair();
        let expected = union_hash(&left, &right);
        let rewrite = GreetingRewrite::max_version_bytes(u64::MAX);
        let (left, right) = run_to_quiescence(harness::reconcile_rewritten_greetings(
            left,
            right,
            victim_left.then_some(rewrite),
            (!victim_left).then_some(rewrite),
        ))
        .expect("the session must terminate");
        let left = left.expect("left endpoint reconciles despite the overstated bound");
        let right = right.expect("right endpoint reconciles despite the overstated bound");
        assert_eq!(hash_of(&left), expected);
        assert_eq!(hash_of(&right), expected);
    }
}

/// An absurdly overstated `set_len` heard from the *larger* side leaves
/// the roles complementary — the smaller side initiates against the honest
/// pair and against the lie alike — so the lie distorts only the victim's
/// derived capacities and the session converges on the union.
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
