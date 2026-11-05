//! Parking: decoded replies wait for their stage in a queue sized by the
//! in-flight count, and the scopes a reply's questions create publish when
//! the stage takes it.

use futures::join;
use proptest::prelude::*;

use super::harness::{codec, join_oracle, left_initiates};
use crate::DEFAULT_TARGET_MESSAGE_SIZE;
use crate::link::memory_with_capacity;
use crate::testing::{Quiescence, run_to_quiescence};
use crate::tree::{
    Root as TreeRoot,
    arb::{arb_divergent_pair, full_fan_behind_two_pair},
    mirror::streaming::{
        Local, Root,
        channel::{QueueKind, QueueRole, with_kind_capacity, with_observation, with_role_hold},
        materialized::Handshaking,
        mirror,
        remote::{Handshaking as RemoteHandshaking, codec::Speaker, parking_census::with_census},
        window::{FAN, Window, WindowConfig},
    },
    typed::height::{Height, UnderUnderRoot},
};

/// Bytes each transport stream buffers: enough that transport backpressure
/// never holds a stage in these sessions.
const TRANSPORT: usize = 1 << 20;

/// The typed height of level-3 replies, which answer level-3 questions.
const LEVEL_3: usize = UnderUnderRoot::HEIGHT - 1;

/// Scheduler rounds the tightness test holds the level-3 stage: long enough
/// for every invited reply to arrive and park.
const HOLD_POLLS: usize = 20_000;

/// Reconcile two replicas over a memory link, each endpoint's walk and proxy
/// sharing that endpoint's window.
fn session(
    left: TreeRoot,
    right: TreeRoot,
    windows: [WindowConfig; 2],
) -> Result<(TreeRoot, TreeRoot), Quiescence> {
    let (left_link, right_link) = memory_with_capacity(TRANSPORT);
    let [left_window, right_window] = windows;
    run_to_quiescence(async move {
        let size = DEFAULT_TARGET_MESSAGE_SIZE as u64;
        let left_walk = Handshaking::start(Local, Root::from(left), size).window(left_window);
        let right_walk = Handshaking::start(Local, Root::from(right), size).window(right_window);
        let left_proxy =
            RemoteHandshaking::start(Local, left_link, codec::<()>()).window(left_window);
        let right_proxy =
            RemoteHandshaking::start(Local, right_link, codec::<()>()).window(right_window);
        let (left, right) = join!(
            Box::pin(mirror(left_walk, left_proxy)),
            Box::pin(mirror(right_walk, right_proxy)),
        );
        (
            left.expect("left session failed").0.into(),
            right.expect("right session failed").0.into(),
        )
    })
}

/// Parking holds exactly the in-flight count, so its capacity cannot shrink.
///
/// At the floor window the initiator's level-3 questions reach
/// `K + FAN + 1` outstanding. Holding the level-3 stage's takes lets every
/// one of their replies arrive and park; the high-water mark is then that
/// count, and releasing the stage lets the session converge.
#[test]
fn parking_holds_exactly_the_in_flight_count() {
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
    let parked = report.role(level_3);
    assert_eq!(
        parked.effective_capacity,
        Window::FLOOR.capacity(LEVEL_3) + FAN + 1
    );
    assert_eq!(
        parked.high_water,
        Window::FLOOR.capacity(LEVEL_3) + FAN + 1,
        "every invited level-3 reply parked while the stage was held"
    );
}

/// The scope queue fed at each take completes this fixture at a fan less
/// one slot, stalls at a fan less two, and at a fan plus one no publish
/// waits.
///
/// The responder takes the initiator's full-fan reply and publishes 256
/// scopes before its stage receives the reply. The encoder draining them
/// dequeues at most two meanwhile, the earlier answer's scope and the
/// take's first (the scope queue's constructor, `next_scopes`, gives the
/// argument). On this fixture the encoder is free to dequeue both, so
/// `FAN − 1` slots complete, possibly after waits. At `FAN − 2` the take
/// can never finish, and the session stalls.
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

    let short = with_kind_capacity(QueueKind::ProxyNextScopes, FAN - 2, run);
    assert!(
        matches!(short, Err(Quiescence::Stalled)),
        "FAN - 2 scope slots must stall on a full-fan reply: {short:?}"
    );

    let (left, right) = with_kind_capacity(QueueKind::ProxyNextScopes, FAN - 1, run)
        .expect("FAN - 1 scope slots complete");
    assert_eq!(left, right);
    assert_eq!(left, join_oracle(&initiator, &responder));

    let (result, report) = with_observation(run);
    result.expect("the default scope queue completes");
    let scopes = report.kind(QueueKind::ProxyNextScopes);
    assert!(scopes.high_water >= FAN, "a full fan of scopes was queued");
    assert_eq!(scopes.blocked_send_polls, 0, "no scope publish waited");
}

/// The parking capacity a side on `window` derives at `height`, recomputed
/// from the in-flight count.
///
/// The count is the window's queue, one dequeued question, and one reply's
/// fan, or the height's whole question population, `256^(31 − h)`, where
/// that is smaller.
fn own_parking(window: Window, height: usize) -> usize {
    let invited = window.capacity(height) + FAN + 1;
    let exponent = u32::try_from(31 - height).expect("a parking height is at most 31");
    match FAN.checked_pow(exponent) {
        Some(population) => population.min(invited),
        None => invited,
    }
}

proptest! {
    /// Peers on different windows complete and converge in either role,
    /// and each side sizes every parking queue from its own window.
    ///
    /// A reply travels only in answer to its receiver's own question, so a
    /// wide peer's many questions park at the wide peer, and the narrow
    /// peer's count and parking already cover what arrives toward it. The
    /// parking census reports each queue's derived capacity and the side
    /// that owns it, which the test holds to that side's window: sizing from
    /// the peer's window, or from the smaller or larger of the two, gives a
    /// different capacity wherever the two windows' counts differ.
    #[test]
    fn mismatched_windows_size_parking_by_each_sides_window(
        (left, right) in arb_divergent_pair(),
    ) {
        let narrow = Window::FLOOR;
        let wide = Window::uniform(8);
        let expected = join_oracle(&left, &right);
        // Equal ceilings converge in the greeting exchange, electing no
        // roles and parking nothing.
        let initiator_side = (left.ceiling != right.ceiling)
            .then(|| if left_initiates(&left, &right) { 0 } else { 1 });
        for windows in [[narrow, wide], [wide, narrow]] {
            let (result, peaks) = with_census(|| {
                session(left.clone(), right.clone(), windows.map(WindowConfig::Fixed))
            });
            let (a, b) = result.expect("mismatched windows stay live");
            prop_assert_eq!(&a, &b);
            prop_assert_eq!(&a, &expected);
            let Some(initiator_side) = initiator_side else {
                prop_assert!(peaks.is_empty(), "a converged session parks nothing");
                continue;
            };
            prop_assert!(!peaks.is_empty(), "a divergent session parks its opening reply");
            for peak in peaks {
                let side = match peak.owner {
                    Speaker::Initiator => initiator_side,
                    Speaker::Responder => 1 - initiator_side,
                };
                prop_assert_eq!(
                    peak.capacity,
                    own_parking(windows[side], peak.height),
                    "{:?} parking at height {} is sized by another window",
                    peak.owner,
                    peak.height,
                );
            }
        }
    }
}
