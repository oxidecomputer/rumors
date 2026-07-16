//! End-to-end successful transport-adversity properties.

use proptest::prelude::*;

use super::{harness, reconcile_locally};
use crate::message::Message;
use crate::tree::{
    Action, Tree,
    arb::arb_divergent_pair,
    arb::nth_party,
    mirror::streaming::testing::{IoPlan, run_to_quiescence},
};

fn plan(read_chunk: usize, write_chunk: usize, delays: Vec<u8>, hold_until_flush: bool) -> IoPlan {
    IoPlan {
        read_chunk,
        write_chunk,
        read_delays: delays.clone(),
        write_delays: delays.clone(),
        flush_delays: delays,
        hold_until_flush,
        fault: None,
    }
}

/// A one-byte pipe whose writers publish only on flush still completes a full
/// divergent session, not merely the preamble shared with the old protocol.
#[test]
fn flush_only_one_byte_transport_reconciles() {
    let mut left = Tree::new();
    left.act(
        &nth_party(0),
        (0..8).map(|_| Action::Insert(Message::new(()))),
    );
    let mut right = Tree::new();
    right.act(
        &nth_party(1),
        (0..8).map(|_| Action::Insert(Message::new(()))),
    );
    let expected = run_to_quiescence(reconcile_locally(left.root.clone(), right.root.clone()))
        .expect("materialized oracle should remain live");
    let flush_only = plan(1, 1, vec![1; 512], true);
    let outcome = run_to_quiescence(harness::reconcile(
        left.root,
        right.root,
        1,
        flush_only.clone(),
        flush_only,
    ))
    .expect("flush-only one-byte transport should remain live");
    assert_eq!(outcome.left.unwrap(), expected.0);
    assert_eq!(outcome.right.unwrap(), expected.1);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        max_shrink_iters: 2_048,
        ..ProptestConfig::default()
    })]

    /// Arbitrary successful fragmentation, delay, capacity, and flush
    /// buffering preserve the exact materialized reconciliation result.
    #[test]
    fn successful_io_adversity_matches_materialized(
        (left, right) in arb_divergent_pair(),
        capacity in 1usize..=64,
        left_read_chunk in 1usize..=16,
        left_write_chunk in 1usize..=16,
        right_read_chunk in 1usize..=16,
        right_write_chunk in 1usize..=16,
        left_delays in proptest::collection::vec(0_u8..=2, 0..64),
        right_delays in proptest::collection::vec(0_u8..=2, 0..64),
        left_buffered in any::<bool>(),
        right_buffered in any::<bool>(),
    ) {
        let expected = run_to_quiescence(reconcile_locally(left.clone(), right.clone()))
            .expect("the materialized oracle should remain live");
        let outcome = run_to_quiescence(harness::reconcile(
            left,
            right,
            capacity,
            plan(left_read_chunk, left_write_chunk, left_delays, left_buffered),
            plan(right_read_chunk, right_write_chunk, right_delays, right_buffered),
        ))
        .map_err(|stopped| TestCaseError::fail(format!(
            "successful transport became quiescent: {stopped:?}",
        )))?;

        prop_assert_eq!(outcome.left.as_ref().ok(), Some(&expected.0));
        prop_assert_eq!(outcome.right.as_ref().ok(), Some(&expected.1));
        for (report, read_chunk, write_chunk) in [
            (outcome.left_io.snapshot(), left_read_chunk, left_write_chunk),
            (outcome.right_io.snapshot(), right_read_chunk, right_write_chunk),
        ] {
            prop_assert!(report.reads > 0);
            prop_assert!(report.writes > 0);
            prop_assert!(report.flushes > 0);
            prop_assert!(report.largest_read <= read_chunk);
            prop_assert!(report.largest_write <= write_chunk);
        }
    }
}
