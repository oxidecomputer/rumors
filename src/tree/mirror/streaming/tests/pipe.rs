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
//!   pipe, and the poller reports it: here a decoder's assembler and its
//!   reader each wait on the other, through one-slot edges between them. A
//!   wait of the receive path on the walk would close a cycle of the same
//!   kind. At its queues' own capacities and a budget of zero, the
//!   dispute-before-supplies fixture (one dispute ahead of six whole-subtree
//!   supplies, which puts supplies on the pipe ahead of frames their stage
//!   needs first) completes and matches the in-memory join.
//! - **The pipe regime.** At a budget of zero and a one-byte pipe, every
//!   wait anywhere in the session is a full handoff, so a new wait of the
//!   receive path on the walk shows up as a stall. Scheduled divergent
//!   sessions, the capacity stress fixtures, the wire suites' stress
//!   fixtures, a peer at a budget of zero against an unlimited one, and
//!   peers on different budgets must all complete.
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
use super::{GENERATED_SCHEDULE_MAX_LEN, at_one_slot, join_oracle, standard_schedules};
use crate::testing::Quiescence;
use crate::tree::Root;
use crate::tree::arb::{
    arb_divergent_pair, early_first_child_dispute_pair, full_fan_behind_two_pair,
    leaf_parent_dispute_pair,
};
use crate::tree::mirror::streaming::FAN;
use crate::tree::mirror::streaming::backend::with_local_schedule;
use crate::tree::mirror::streaming::channel::{QueueKind, with_kind_capacity, with_schedule};
use crate::tree::mirror::streaming::protocol::split::with_split_replies;
use crate::tree::mirror::streaming::testing::session::{AMPLE, Endpoint, over_pipe};
use crate::tree::typed::{Node as TreeNode, Path};
use crate::{Version, causally};

/// Bytes each direction of the pipe buffers in the regime's sessions: one,
/// so every byte crosses in its own handoff.
const ONE_BYTE: usize = 1;

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
pub(super) fn placed_over_pipe(
    pair: (Root, Root),
    budgets: [usize; 2],
    buffer: usize,
) -> Result<(Endpoint, Endpoint), Quiescence> {
    let paths = leaf_paths(&pair);
    Path::with_leaf_paths(paths, || over_pipe::<()>(pair, budgets, buffer))
}

/// Reconcile a pair over a one-byte pipe at `budgets` under the given
/// channel and backend poll schedules, and require both endpoints to
/// complete holding the join of their trees.
///
/// `placed` runs the session under the pair's leaf-path placement.
fn assert_completes_over_one_byte(
    pair: (Root, Root),
    placed: bool,
    budgets: [usize; 2],
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> Result<(), TestCaseError> {
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let outcome = with_schedule(channel_schedule, || {
        with_local_schedule(backend_schedule, || {
            if placed {
                placed_over_pipe(pair, budgets, ONE_BYTE)
            } else {
                over_pipe::<()>(pair, budgets, ONE_BYTE)
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

/// Two trees, initiator first, whose responder answers the opening with a
/// dispute ahead of one group of three supplied root children, the last of
/// five leaves.
///
/// Both hold root child 0, under which each holds a leaf the other lacks,
/// so it disputes; only the responder holds root children 1, 2, and 3,
/// which its root reply supplies in radix order behind the dispute. The initiator holds one leaf against
/// the responder's eight, so it wins the smaller-set election. Paths are
/// placed, so a session must run under the placement.
fn grouped_supplies_pair() -> (Root, Root) {
    let initiator = rooted(grown(None, 1, 1, &(), &[path_at(&[0, 1])]));
    let mut held = vec![path_at(&[0, 2]), path_at(&[1]), path_at(&[2])];
    held.extend((1..=5_u8).map(|leaf| path_at(&[3, leaf])));
    let responder = rooted(grown(None, 2, 1, &(), &held));
    (initiator, responder)
}

/// The harness detects a stall over one pipe: a decoder whose supplies'
/// edge cannot hold a group's nodes stalls the session, and the
/// closed-world poller reports it.
///
/// The initiator decodes the responder's root reply, whose three supplies
/// form one group. A reader takes a group's nodes back only when the group
/// ends, so with the supplies' edge at one slot the assembler waits to hand
/// back the second node, which it finishes on the third run's first leaf,
/// while the reader, with the leaves' edge at one slot, waits to hand it
/// the third run's later leaves: a cycle within one decoder, which the
/// edge's fan of supply slots rules out. The pipe has room for everything
/// either side writes, so nothing but that cycle can hold the session.
#[test]
fn harness_detects_a_stall_over_one_pipe() {
    let pair = grouped_supplies_pair();
    assert_eq!(
        client_role(&pair.0, &pair.1),
        Party::I,
        "the smaller tree must initiate"
    );
    let outcome = with_kind_capacity(QueueKind::AssembledSupplies, 1, || {
        with_kind_capacity(QueueKind::DecodedLeaves, 1, || {
            placed_over_pipe(pair, [0; 2], AMPLE)
        })
    });
    assert!(
        matches!(outcome, Err(Quiescence::Stalled)),
        "a decoder whose assembler waits on its reader must stall the session: {outcome:?}"
    );
}

/// The grouped-supplies session completes with the leaves' edge at one
/// slot, and both sides hold the join: the supplies' edge at its fan breaks
/// the cycle the stall control above builds.
#[test]
fn grouped_supplies_complete_over_a_one_slot_leaf_edge() {
    let pair = grouped_supplies_pair();
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let (initiator, responder) = with_kind_capacity(QueueKind::DecodedLeaves, 1, || {
        placed_over_pipe(pair, [0; 2], AMPLE)
    })
    .expect("a fan of supply slots must not stall");
    assert_eq!(initiator.expect("the initiator completes"), expected);
    assert_eq!(responder.expect("the responder completes"), expected);
}

/// At a budget of zero, the dispute-before-supplies fixture completes over
/// one pipe and both sides hold the in-memory join.
///
/// Every queue runs at its own capacity, and the walk asks one question at
/// a time, so the requesting side's stage blocks on the dispute's chain
/// while the supplies arrive and park.
#[test]
fn derived_capacities_complete_dispute_before_supplies_over_one_pipe() {
    let pair = dispute_before_supplies_trees();
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let (supplier_out, chain_out) = placed_over_pipe(pair, [0; 2], AMPLE)
        .expect("the dispute-before-supplies fixture must not stall over one pipe");
    assert_eq!(supplier_out.expect("the supplier completes"), expected);
    assert_eq!(chain_out.expect("the requesting side completes"), expected);
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
                [0; 2],
                channel_schedule,
                backend_schedule,
            )
            .unwrap();
        }
    }
}

/// The capacity stress fixtures, which stress the walk's one-slot and
/// fan-sized queues.
///
/// They are a full fan of disputed root children, a full fan below four
/// disputed parents, the fan-plus-one and double-fan aggregates, a
/// multiplying pyramid, and a comb reaching every trie height.
fn capacity_stress_fixtures() -> [(Root, Root); 6] {
    [
        pyramid_pair(&[FAN], 1, LeafOrder::Outside),
        pyramid_pair(&[4, FAN], 1, LeafOrder::Reversed),
        one_sided_pair(&[(0x00, FAN - 2, 1), (0x01, 1, 1)]),
        one_sided_pair(&[(0x00, FAN - 1, 1), (0x01, FAN - 1, 1)]),
        pyramid_pair(&[8, 4, 4, 2], 2, LeafOrder::Interleaved),
        full_depth_comb_pair(2, LeafOrder::Interleaved),
    ]
}

/// Require each capacity stress fixture to complete over a one-byte pipe
/// at the budget of zero, in both orientations and under every standard
/// poll schedule.
fn assert_capacity_stress_fixtures_complete() {
    for (a, b) in capacity_stress_fixtures() {
        for pair in [(a.clone(), b.clone()), (b, a)] {
            for (channel_schedule, backend_schedule) in standard_schedules() {
                assert_completes_over_one_byte(
                    pair.clone(),
                    true,
                    [0; 2],
                    channel_schedule,
                    backend_schedule,
                )
                .unwrap();
            }
        }
    }
}

/// Every capacity stress fixture ([`capacity_stress_fixtures`]) completes
/// over a one-byte pipe at the budget of zero, in both orientations and
/// under every standard poll schedule.
#[test]
fn capacity_stress_fixtures_complete_over_a_one_byte_pipe() {
    assert_capacity_stress_fixtures_complete();
}

/// Every capacity stress fixture completes over a one-byte pipe with every
/// edge that needs only one slot capped at one.
///
/// Those edges ([`ONE_SLOT_KINDS`](super::ONE_SLOT_KINDS)) include the
/// pipe's lanes. Each fixture runs at the budget of zero, in both
/// orientations and under every standard poll schedule.
///
/// Those edges have a second slot only so that a send into an empty edge
/// does not park; the pipe's argument, and the walk's, rest on one. At the
/// documented capacities the second slot could hide a wait the arguments
/// miss, so this runs the fixtures at the floor the arguments state.
#[test]
fn capacity_stress_fixtures_complete_over_a_one_byte_pipe_at_one_slot() {
    at_one_slot(
        &[
            QueueKind::OutgoingResponses,
            QueueKind::PipeOutgoing,
            QueueKind::PipeIncoming,
        ],
        assert_capacity_stress_fixtures_complete,
    );
}

/// Every capacity and wire stress fixture completes over a one-byte pipe at
/// a budget of zero with every reply split at every reaction boundary,
/// in both orientations.
///
/// Each stage then reads every reply one reaction at a time, each encoder
/// releases a reply's frames chunk by chunk, and each decoder delivers
/// each question as its frame decodes, across fixtures whose replies carry full fans of questions and long
/// runs of supplies.
#[test]
fn stress_fixtures_complete_with_split_replies_over_a_one_byte_pipe() {
    let (leaf_left, leaf_right, _) = leaf_parent_dispute_pair();
    let cases = [
        (pyramid_pair(&[FAN], 1, LeafOrder::Outside), true),
        (
            one_sided_pair(&[(0x00, FAN - 1, 1), (0x01, FAN - 1, 1)]),
            true,
        ),
        (full_depth_comb_pair(2, LeafOrder::Interleaved), true),
        (full_fan_behind_two_pair(), false),
        (early_first_child_dispute_pair(), false),
        ((leaf_left, leaf_right), true),
        (dispute_before_supplies_trees(), true),
    ];
    for ((a, b), placed) in cases {
        for pair in [(a.clone(), b.clone()), (b, a)] {
            with_split_replies(|| {
                assert_completes_over_one_byte(pair, placed, [0; 2], Vec::new(), Vec::new())
            })
            .unwrap();
        }
    }
}

/// The wire suites' own stress fixtures complete over a one-byte pipe at the
/// budget of zero, in both orientations and under every standard poll
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
                    [0; 2],
                    channel_schedule,
                    backend_schedule,
                )
                .unwrap();
            }
        }
    }
}

proptest! {
    /// Arbitrary divergent sessions complete over a one-byte pipe at a
    /// budget of zero under arbitrary channel and backend poll schedules,
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
            [0; 2],
            channel_schedule,
            backend_schedule,
        )?;
    }

    /// A peer at a budget of zero and a peer at an unlimited one complete
    /// over a one-byte pipe in either role.
    ///
    /// This test holds the mismatch to the one-byte regime; the proxy's
    /// `mismatched_budgets_converge_in_either_role` holds it over an ample
    /// pipe.
    #[test]
    fn zero_and_unlimited_budgets_complete_over_a_one_byte_pipe(pair in arb_divergent_pair()) {
        for budgets in [[0, usize::MAX], [usize::MAX, 0]] {
            assert_completes_over_one_byte(pair.clone(), false, budgets, Vec::new(), Vec::new())?;
        }
    }

    /// Peers on different memory budgets complete over a one-byte pipe,
    /// whichever side holds the smaller budget.
    ///
    /// Each side admits its work against its own budget.
    #[test]
    fn different_budgets_complete_over_a_one_byte_pipe(
        pair in arb_divergent_pair(),
        budgets in prop::sample::subsequence(vec![0_usize, 1 << 16, 1 << 20, 1 << 29], 2),
    ) {
        let [small, large] = [budgets[0], budgets[1]];
        for budgets in [[small, large], [large, small]] {
            assert_completes_over_one_byte(pair.clone(), false, budgets, Vec::new(), Vec::new())?;
        }
    }
}
