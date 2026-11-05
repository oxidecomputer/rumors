//! Sessions over one duplex byte stream: evidence that sharing one pipe does
//! not deadlock the walk, and the demonstration that the harness would catch
//! it if it did.
//!
//! Every session here runs both endpoints' walks and proxies over one
//! in-memory connection, two `tokio::io::duplex` pipes, under the
//! closed-world poller, which reports a stall whenever every task waits with
//! no wake arranged.
//!
//! - **The demonstration.** A cycle of waits stalls a session over one
//!   pipe, and the poller reports it: here a take, on the walk's side of
//!   parking, waits on its encoder through a scope queue two slots short of
//!   a fan. A wait of the receive path on the walk would close a cycle of
//!   the same kind. Parking driven past its capacity fails the session
//!   instead of waiting. At the derived capacities, the
//!   dispute-before-supplies fixture (one dispute ahead of six whole-subtree
//!   supplies, which puts supplies on the pipe ahead of frames their stage
//!   needs first) completes and matches the in-memory join.
//! - **The pipe regime.** At the one-slot window and a one-byte pipe, every
//!   wait anywhere in the session is a full handoff, so a new wait of the
//!   receive path on the walk shows up as a stall. Scheduled divergent
//!   sessions, the capacity stress fixtures, the wire suites' stress
//!   fixtures, peers on mismatched windows, and peers on different budgets
//!   must all complete.
//!
//! Several fixtures place leaves at chosen paths, and the
//! wire decoder derives each received leaf's path from its version. Their
//! sessions therefore run under [`Path::with_leaf_paths`] with every leaf of
//! both trees, so that derivation agrees with the placement on both sides.

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::*;

use super::dispute_before_supplies::dispute_before_supplies_trees;
use super::fixtures::{
    LeafOrder, full_depth_comb_pair, grown, one_sided_pair, path_at, pyramid_pair, rooted,
};
use super::skeleton::{Party, client_role};
use super::{GENERATED_SCHEDULE_MAX_LEN, join_oracle, standard_schedules};
use crate::testing::Quiescence;
use crate::tree::Root;
use crate::tree::arb::{
    arb_divergent_pair, early_first_child_dispute_pair, full_fan_behind_two_pair,
    leaf_parent_dispute_pair,
};
use crate::tree::mirror::Error as MirrorError;
use crate::tree::mirror::streaming::backend::with_local_schedule;
use crate::tree::mirror::streaming::channel::{
    QueueKind, QueueRole, with_kind_capacity, with_observation, with_role_hold, with_schedule,
};
use crate::tree::mirror::streaming::remote::Error as RemoteError;
use crate::tree::mirror::streaming::testing::session::{AMPLE, Endpoint, over_pipe};
use crate::tree::mirror::streaming::window::{FAN, Window, WindowConfig};
use crate::tree::typed::height::{Height, UnderRoot, UnderUnderRoot};
use crate::tree::typed::{Node as TreeNode, Path};
use crate::{Version, causally};

/// Bytes each direction of the pipe buffers in the regime's sessions: one,
/// so every byte crosses in its own handoff.
const ONE_BYTE: usize = 1;

/// The typed height of level-2 replies: the dispute-before-supplies
/// fixture's supplies park at this height on the side that requested them.
const LEVEL_2: usize = UnderRoot::HEIGHT - 1;

/// The typed height of level-3 replies, which answer level-3 questions.
const LEVEL_3: usize = UnderUnderRoot::HEIGHT - 1;

/// The uniform window of the wide peer in the mismatched-window sessions.
const WIDE: Window = Window::uniform(8);

/// Scheduler rounds the wide-peer test holds the level-3 stage: long enough
/// for every invited reply to arrive and park.
const HOLD_POLLS: usize = 20_000;

/// Every leaf's version and placed path, across both trees of a pair.
fn leaf_paths(pair: &(Root, Root)) -> Vec<(Version, Path)> {
    let mut leaves = BTreeMap::new();
    for root in [&pair.0, &pair.1] {
        for (path, leaf) in TreeNode::range_owned(root.node.as_ref(), causally::all()) {
            leaves.insert(
                leaf.version().as_bytes().to_vec(),
                (leaf.version().clone(), Path::from(path)),
            );
        }
    }
    leaves.into_values().collect()
}

/// [`over_pipe`] for a pair whose leaves sit at chosen paths, with leaf-path
/// derivation following that placement for the session.
fn placed_over_pipe(
    pair: (Root, Root),
    windows: [WindowConfig; 2],
    buffer: usize,
) -> Result<(Endpoint, Endpoint), Quiescence> {
    let paths = leaf_paths(&pair);
    Path::with_leaf_paths(paths, || over_pipe::<()>(pair, windows, buffer))
}

/// Reconcile a pair over a one-byte pipe at `windows` under the given
/// channel and backend poll schedules, and require both endpoints to
/// complete holding the join of their trees.
///
/// `placed` runs the session under the pair's leaf-path placement.
fn assert_completes_over_one_byte(
    pair: (Root, Root),
    placed: bool,
    windows: [WindowConfig; 2],
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> Result<(), TestCaseError> {
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let outcome = with_schedule(channel_schedule, || {
        with_local_schedule(backend_schedule, || {
            if placed {
                placed_over_pipe(pair, windows, ONE_BYTE)
            } else {
                over_pipe::<()>(pair, windows, ONE_BYTE)
            }
        })
    });
    let (a, b) = outcome.map_err(|stopped| {
        TestCaseError::fail(format!(
            "a session over a one-byte pipe stopped: {stopped:?}"
        ))
    })?;
    let (a, b) = match (a, b) {
        (Ok(a), Ok(b)) => (a, b),
        (a, b) => {
            return Err(TestCaseError::fail(format!(
                "a session over a one-byte pipe failed: A {:?}, B {:?}",
                a.err(),
                b.err()
            )));
        }
    };
    prop_assert_eq!(&a, &expected);
    prop_assert_eq!(&b, &expected);
    Ok(())
}

/// The harness detects a stall over one pipe: a take that waits on the
/// encoder stalls the session, and the closed-world poller reports it.
///
/// The initiator of the full-fan fixture answers with a full-fan reply.
/// When the responder's stage takes it, the take publishes 256 scopes
/// before the stage receives the reply. The encoder draining those scopes
/// holds one while it awaits that scope's answer, which the stage produces
/// only after the take; with the scope queue capped at `FAN − 2` slots the
/// take can never finish, a cycle through the stage's take and its encoder
/// alone, both on the walk's side of parking. The pipe has room for
/// everything either side writes, so nothing but that cycle can hold the
/// session. The proxy's `scope_queue_capacity_law` covers the capacities at
/// which this session completes.
#[test]
fn harness_detects_a_stall_over_one_pipe() {
    let outcome = with_kind_capacity(QueueKind::ProxyNextScopes, FAN - 2, || {
        over_pipe::<()>(full_fan_behind_two_pair(), [WindowConfig::FLOOR; 2], AMPLE)
    });
    assert!(
        matches!(outcome, Err(Quiescence::Stalled)),
        "a take waiting on its encoder must stall the session: {outcome:?}"
    );
}

/// Parking driven past its capacity fails the session over one pipe
/// instead of waiting.
///
/// The fixture is one dispute ahead of six whole-subtree supplies, which
/// answer the responder's empty queries as six level-2 replies. With every
/// walk queue at one slot, the responder's stage takes the disputed reply
/// and the first supply, then blocks publishing the second supply's
/// resolution until the dispute's chain resolves, every level down to a
/// leaf. The supplies are written as soon as they are encoded, long before
/// that chain can finish, so with parking capped at one slot the third
/// supply parks and the fourth finds parking full. The decoder fails at
/// once, reporting the derived capacity, and both sessions end rather than
/// stall.
#[test]
fn parking_at_one_slot_fails_fast_over_one_pipe() {
    let (supplier, chain) = dispute_before_supplies_trees();
    // Parking holds every reply the window can invite, or every question
    // the height has at all: 256 level-2 questions, one per root child.
    let invited = Window::FLOOR.capacity(LEVEL_2) + FAN + 1;
    let population = FAN;
    let (supplier_out, chain_out) = with_kind_capacity(QueueKind::ProxyResponses, 1, || {
        placed_over_pipe((supplier, chain), [WindowConfig::FLOOR; 2], AMPLE)
    })
    .expect("parking fails instead of waiting, so the session cannot stall");
    assert!(
        matches!(
            &chain_out,
            Err(MirrorError::Server(RemoteError::ParkingOverflow { height, capacity }))
                if *height == LEVEL_2 && *capacity == invited.min(population)
        ),
        "the requesting side's parking overflows at level 2: {:?}",
        chain_out.err()
    );
    assert!(
        supplier_out.is_err(),
        "the supplier's session ends with its counterparty's"
    );
}

/// At the derived capacities, the dispute-before-supplies fixture completes
/// over one pipe and both sides hold the in-memory join.
///
/// This is the same session as the one-slot parking case, with every queue
/// at the capacity the session derives, the walk's at the one-slot window.
#[test]
fn derived_capacities_complete_dispute_before_supplies_over_one_pipe() {
    let pair = dispute_before_supplies_trees();
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let (supplier_out, chain_out) = placed_over_pipe(pair, [WindowConfig::FLOOR; 2], AMPLE)
        .expect("the dispute-before-supplies fixture must not stall over one pipe");
    assert_eq!(supplier_out.expect("the supplier completes"), expected);
    assert_eq!(chain_out.expect("the requesting side completes"), expected);
}

/// Two trees, initiator first, whose initiator answers one disputed root
/// child with a reply carrying nine level-3 questions and the next with a
/// reply carrying a full fan of them.
///
/// Both sides hold root children 0 and 1, on concurrent parties. Under 0 the
/// responder lists nine depth-2 prefixes and under 1 all 256; the initiator
/// holds one leaf under each, at a depth-3 path the responder lacks, so
/// every listed prefix is a question. The initiator holds two leaves against
/// the responder's 265, so it wins the smaller-set election. Paths are
/// placed, so a session must run under the placement.
fn nine_then_full_fan_pair() -> (Root, Root) {
    let initiator = rooted(grown(
        None,
        1,
        1,
        &(),
        &[path_at(&[0, 0, 0xFF]), path_at(&[1, 0, 0xFF])],
    ));
    let listed: Vec<Path> = (0..9_u8)
        .map(|radix| path_at(&[0, radix]))
        .chain((0..=u8::MAX).map(|radix| path_at(&[1, radix])))
        .collect();
    let responder = rooted(grown(None, 2, 1, &(), &listed));
    (initiator, responder)
}

/// A wide peer parks more replies than its narrow counterparty's window
/// could size parking for, so each side sizes parking from its own window.
///
/// The wide initiator's level-3 stage is held. With eight slots its walk
/// records all nine questions of the first reply, one dequeued and eight
/// queued, and then hands on the full-fan reply: `8 + 1 + FAN` questions
/// outstanding, every one answered and parked while the stage waits. That
/// is more than the `1 + 1 + FAN` the narrow responder's window yields, so
/// parking sized from the peer's window, or from the smaller window, would
/// overflow here; the session instead completes with the wide side's
/// parking full to its own count.
#[test]
fn a_wide_peer_parks_past_its_narrow_peers_count() {
    let pair = nine_then_full_fan_pair();
    assert_eq!(
        client_role(&pair.0, &pair.1),
        Party::I,
        "the smaller tree must initiate"
    );
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let level_3 = QueueRole::new(QueueKind::ProxyResponses, LEVEL_3);
    let (outcome, report) = with_observation(|| {
        with_role_hold(level_3, HOLD_POLLS, || {
            placed_over_pipe(
                pair,
                [WindowConfig::Fixed(WIDE), WindowConfig::FLOOR],
                AMPLE,
            )
        })
    });
    let (initiator, responder) = outcome.expect("the held session resumes and completes");
    assert_eq!(initiator.expect("the wide initiator completes"), expected);
    assert_eq!(responder.expect("the narrow responder completes"), expected);

    let own_count = WIDE.capacity(LEVEL_3) + FAN + 1;
    let narrow_count = Window::FLOOR.capacity(LEVEL_3) + FAN + 1;
    assert!(
        narrow_count < own_count,
        "the windows must yield different counts"
    );
    let parked = report.role(level_3);
    assert_eq!(parked.effective_capacity, own_count);
    assert_eq!(
        parked.high_water, own_count,
        "every invited level-3 reply parked while the stage was held"
    );
}

/// The dispute-before-supplies fixture completes over a one-byte pipe under
/// every standard poll schedule, in both orientations.
#[test]
fn dispute_before_supplies_completes_over_a_one_byte_pipe() {
    let (supplier, chain) = dispute_before_supplies_trees();
    for pair in [(supplier.clone(), chain.clone()), (chain, supplier)] {
        for (channel_schedule, backend_schedule) in standard_schedules() {
            assert_completes_over_one_byte(
                pair.clone(),
                true,
                [WindowConfig::FLOOR; 2],
                channel_schedule,
                backend_schedule,
            )
            .unwrap();
        }
    }
}

/// Every capacity stress fixture completes over a one-byte pipe at the
/// one-slot window, in both orientations and under every standard poll
/// schedule.
///
/// The fixtures stress the walk's one-slot and fan-sized queues: a full fan
/// of disputed root children, a full fan below four disputed parents, the
/// fan-plus-one and double-fan aggregates, a multiplying pyramid, and a comb
/// reaching every trie height.
#[test]
fn capacity_stress_fixtures_complete_over_a_one_byte_pipe() {
    let cases = [
        pyramid_pair(&[FAN], 1, LeafOrder::Outside),
        pyramid_pair(&[4, FAN], 1, LeafOrder::Reversed),
        one_sided_pair(&[(0x00, FAN - 2, 1), (0x01, 1, 1)]),
        one_sided_pair(&[(0x00, FAN - 1, 1), (0x01, FAN - 1, 1)]),
        pyramid_pair(&[8, 4, 4, 2], 2, LeafOrder::Interleaved),
        full_depth_comb_pair(2, LeafOrder::Interleaved),
    ];
    for (a, b) in cases {
        for pair in [(a.clone(), b.clone()), (b, a)] {
            for (channel_schedule, backend_schedule) in standard_schedules() {
                assert_completes_over_one_byte(
                    pair.clone(),
                    true,
                    [WindowConfig::FLOOR; 2],
                    channel_schedule,
                    backend_schedule,
                )
                .unwrap();
            }
        }
    }
}

/// The wire suites' own stress fixtures complete over a one-byte pipe at the
/// one-slot window, in both orientations and under every standard poll
/// schedule.
///
/// A full-fan reply behind two disputes and a dispute ahead of
/// whole-subtree supplies are version-addressed; a dispute surviving to the
/// leaf parents is placed.
#[test]
fn wire_stress_fixtures_complete_over_a_one_byte_pipe() {
    let (leaf_left, leaf_right, _) = leaf_parent_dispute_pair();
    let cases = [
        (full_fan_behind_two_pair(), false),
        (early_first_child_dispute_pair(), false),
        ((leaf_left, leaf_right), true),
    ];
    for ((a, b), placed) in cases {
        for pair in [(a.clone(), b.clone()), (b, a)] {
            for (channel_schedule, backend_schedule) in standard_schedules() {
                assert_completes_over_one_byte(
                    pair.clone(),
                    placed,
                    [WindowConfig::FLOOR; 2],
                    channel_schedule,
                    backend_schedule,
                )
                .unwrap();
            }
        }
    }
}

proptest! {
    /// Arbitrary divergent sessions complete over a one-byte pipe at the
    /// one-slot window under arbitrary channel and backend poll schedules,
    /// and both sides hold the join.
    #[test]
    fn scheduled_sessions_complete_over_a_one_byte_pipe(
        pair in arb_divergent_pair(),
        channel_schedule in vec(0_u8..=2, 0..GENERATED_SCHEDULE_MAX_LEN),
        backend_schedule in vec(0_u8..=2, 0..GENERATED_SCHEDULE_MAX_LEN),
    ) {
        assert_completes_over_one_byte(
            pair,
            false,
            [WindowConfig::FLOOR; 2],
            channel_schedule,
            backend_schedule,
        )?;
    }

    /// Peers on mismatched windows complete over a one-byte pipe in either
    /// role.
    ///
    /// Each side sizes parking from its own window, since a reply travels
    /// only in answer to its receiver's own question. This test holds the
    /// mismatch to the one-byte regime; that each side's parking takes its
    /// own window's capacity is judged by the proxy's
    /// `mismatched_windows_size_parking_by_each_sides_window`, and that a
    /// wide side needs it by `a_wide_peer_parks_past_its_narrow_peers_count`.
    #[test]
    fn mismatched_windows_complete_over_a_one_byte_pipe(pair in arb_divergent_pair()) {
        let narrow = WindowConfig::FLOOR;
        let wide = WindowConfig::Fixed(Window::uniform(8));
        for windows in [[narrow, wide], [wide, narrow]] {
            assert_completes_over_one_byte(pair.clone(), false, windows, Vec::new(), Vec::new())?;
        }
    }

    /// Peers on different memory budgets complete over a one-byte pipe,
    /// whichever side holds the smaller budget.
    ///
    /// Each side sizes its window from the smaller of the two exchanged
    /// budgets with its own backend's node price.
    #[test]
    fn different_budgets_complete_over_a_one_byte_pipe(
        pair in arb_divergent_pair(),
        budgets in prop::sample::subsequence(vec![0_usize, 1 << 16, 1 << 20, 1 << 29], 2),
    ) {
        let [small, large] = [budgets[0], budgets[1]].map(WindowConfig::Budget);
        for windows in [[small, large], [large, small]] {
            assert_completes_over_one_byte(pair.clone(), false, windows, Vec::new(), Vec::new())?;
        }
    }
}
