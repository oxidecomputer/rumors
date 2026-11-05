//! The in-flight count is reached: a fixture drives one level's outstanding
//! questions to exactly `K(h) + FAN + 1`.

use super::LocalSession;
use super::skeleton::{Party, client_role};
use crate::tree::{
    arb::full_fan_behind_two_pair,
    mirror::streaming::window::{FAN, Window, WindowConfig},
    typed::height::{Height, UnderRoot, UnderUnderRoot},
};

/// The typed height of level-3 questions, which list depth-3 children.
const LEVEL_3: usize = UnderUnderRoot::HEIGHT - 1;

/// A level reaches the in-flight count exactly at the floor window.
///
/// The session runs at the floor window, set explicitly, because only at
/// one queue slot does this fixture exercise every term of the count. The
/// initiator's first internal stage hands on the two-question reply; with
/// one slot, the level-3 stage has dequeued one question and awaits its
/// answer while the queue holds the other. The stage then hands on the
/// full-fan reply before the first answer arrives: `1 + 1 + FAN` questions
/// outstanding. At a wider window both questions fit in the queue, and the
/// same fixture's `2 + FAN` falls short of that window's count without any
/// question dequeued. Reaching the bound at the floor shows that the meter
/// counts every term the count names, so the bound cannot be tightened.
#[test]
fn a_full_fan_behind_a_dequeued_question_reaches_the_count() {
    let (initiator, responder) = full_fan_behind_two_pair();
    assert_eq!(
        client_role(&initiator, &responder),
        Party::I,
        "the smaller tree must initiate"
    );
    let mut outcome = LocalSession::new(initiator, responder)
        .window(WindowConfig::FLOOR)
        .trace()
        .run();
    let trace = outcome
        .trace
        .take()
        .expect("the session was asked to record its trace");
    trace.assert_valid();
    let (ours, theirs) = outcome.sides();
    assert_eq!(ours, theirs, "sanity: the session converges");

    let high_water = trace.outstanding_high_water();
    let initiator_work = high_water
        .keys()
        .find_map(|&(work, height)| (height == UnderRoot::HEIGHT).then_some(work))
        .expect("the initiator asks the root question");
    assert_eq!(
        high_water[&(initiator_work, LEVEL_3)],
        Window::FLOOR.capacity(LEVEL_3) + FAN + 1
    );
}
