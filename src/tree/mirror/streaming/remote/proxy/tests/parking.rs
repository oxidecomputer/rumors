//! Parking: decoded chunks wait for their stage, and the scopes a chunk's
//! questions create publish when the stage takes it.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use super::harness::{join_oracle, left_initiates};
use crate::testing::Quiescence;
use crate::tree::{
    Root as TreeRoot,
    arb::{arb_divergent_pair, full_fan_behind_two_pair},
    mirror::streaming::{
        channel::{QueueKind, QueueRole, with_kind_capacity, with_observation, with_role_hold},
        testing::session::{AMPLE, completed, over_pipe},
        window::{FAN, Window, WindowConfig},
    },
    typed::height::{Height, UnderUnderRoot},
};

/// The typed height of level-3 replies, which answer level-3 questions.
const LEVEL_3: usize = UnderUnderRoot::HEIGHT - 1;

/// Scheduler rounds the test holds the level-3 stage: long enough for every
/// invited reply to arrive and park.
const HOLD_POLLS: usize = 20_000;

/// Reconcile two replicas over an ample pipe, each endpoint's walk and
/// proxy sharing that endpoint's window, and require both to succeed
/// unless the session stalled.
fn session(
    left: TreeRoot,
    right: TreeRoot,
    windows: [WindowConfig; 2],
) -> Result<(TreeRoot, TreeRoot), Quiescence> {
    over_pipe::<()>((left, right), windows, AMPLE).map(completed)
}

/// Decoding never waits on the walk: while the level-3 stage takes nothing,
/// every reply its questions invite still decodes and parks, and once the
/// stage resumes the session converges.
///
/// At the floor window the initiator asks level-3 questions until its
/// one-slot question queue stops it: `K` questions in the queue, one taken
/// by the stage that awaits its answer, and one whose chunk has left while
/// its question waits for room. Each reply parks as at least one chunk, so
/// with the stage held parking reaches at least `K + 2` chunks.
#[test]
fn every_invited_reply_parks_while_its_stage_is_held() {
    let (initiator, responder) = full_fan_behind_two_pair();
    assert!(
        left_initiates(&initiator, &responder),
        "the fixture's initiator must win the election"
    );
    let level_3 = QueueRole::new(QueueKind::ProxyResponses, LEVEL_3);
    let (result, report) = with_observation(|| {
        with_role_hold(level_3, HOLD_POLLS, || {
            session(
                initiator.clone(),
                responder.clone(),
                [WindowConfig::FLOOR; 2],
            )
        })
    });
    let (left, right) = result.expect("the held session resumes and completes");
    assert_eq!(left, right, "both sides converge");
    assert_eq!(left, join_oracle(&initiator, &responder));
    let invited = Window::FLOOR.capacity(LEVEL_3) + 2;
    let parked = report.role(level_3).high_water;
    assert!(
        parked >= invited,
        "every invited level-3 reply parked while the stage was held: {parked} of {invited}"
    );
}

/// The scope queue fed at each take completes this fixture at a fan less
/// one slot, and at a fan plus one no publish waits.
///
/// The responder takes the initiator's full-fan reply chunk by chunk, each
/// take publishing its chunk's scopes before its stage receives the chunk.
/// The scope queue's constructor, `next_scopes`, argues that a fan plus one
/// slots never make a take wait, whatever the chunking; the decoder delivers
/// each question as its frame arrives, so this session's takes publish far
/// fewer than a fan at once, and both capacities complete.
#[test]
fn scope_queue_capacity_law() {
    let (initiator, responder) = full_fan_behind_two_pair();
    assert!(
        left_initiates(&initiator, &responder),
        "the fixture's initiator must win the election"
    );
    let run = || {
        session(
            initiator.clone(),
            responder.clone(),
            [WindowConfig::FLOOR; 2],
        )
    };

    let (left, right) = with_kind_capacity(QueueKind::ProxyNextScopes, FAN - 1, run)
        .expect("FAN - 1 scope slots complete");
    assert_eq!(left, right);
    assert_eq!(left, join_oracle(&initiator, &responder));

    let (result, report) = with_observation(run);
    result.expect("the default scope queue completes");
    let scopes = report.kind(QueueKind::ProxyNextScopes);
    assert!(scopes.sends >= FAN, "a full fan of scopes was published");
    assert_eq!(scopes.blocked_send_polls, 0, "no scope publish waited");
}

/// Reconcile `left` and `right` on two different windows, in both orders,
/// and require both sessions to complete and converge on the join.
///
/// One side runs the floor window and the other a wider one, so the two
/// sides' queues and question counts differ while each bounds its own
/// session.
fn check_mismatched_windows_converge(
    left: &TreeRoot,
    right: &TreeRoot,
) -> Result<(), TestCaseError> {
    let narrow = Window::FLOOR;
    let wide = Window::uniform(8);
    let expected = join_oracle(left, right);
    for windows in [[narrow, wide], [wide, narrow]] {
        let (a, b) = session(
            left.clone(),
            right.clone(),
            windows.map(WindowConfig::Fixed),
        )
        .expect("mismatched windows stay live");
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(&a, &expected);
    }
    Ok(())
}

proptest! {
    /// Peers on different windows complete and converge in either role
    /// ([`check_mismatched_windows_converge`]).
    #[test]
    fn mismatched_windows_converge_in_either_role(
        (left, right) in arb_divergent_pair(),
    ) {
        check_mismatched_windows_converge(&left, &right)?;
    }
}
