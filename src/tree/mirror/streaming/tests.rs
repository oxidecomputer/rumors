//! Honest-peer behavior and the shared in-memory driver harness.
//!
//! Capacity/scheduling stress lives in [`capacity`], connected abort and
//! lifecycle checks in [`faults`], and deterministic tree builders in
//! [`fixtures`].

use std::{convert::Infallible, future};

use proptest::prelude::*;

use super::driver::try_join_mapped;
use crate::testing::run_to_quiescence;
use crate::tree::arb::{
    arb_divergent_pair, arb_tree_root, leaf_parent_dispute_pair, leaf_parent_redaction_pair,
};
use crate::tree::mirror::alternating;
use crate::tree::mirror::streaming::backend::with_local_schedule;
use crate::tree::mirror::streaming::materialized::channel::with_schedule;
use crate::tree::mirror::streaming::materialized::progress::with_trace;
use crate::tree::mirror::streaming::{
    Local, Root as StreamingRoot, materialized::Handshaking, mirror as drive_streaming,
};
use crate::tree::{Root, mirror::Error as MirrorError};

mod capacity;
mod faults;
mod fixtures;

/// Either terminal error preempts a peer which can no longer make progress.
#[test]
fn terminal_errors_preempt_parked_peers() {
    let left = try_join_mapped(
        future::ready(Err::<(), _>("left")),
        MirrorError::<&str, Infallible>::Client,
        future::pending::<Result<(), Infallible>>(),
        MirrorError::Server,
    );
    assert!(matches!(
        run_to_quiescence(left),
        Ok(Err(MirrorError::Client("left")))
    ));

    let right = try_join_mapped(
        future::pending::<Result<(), Infallible>>(),
        MirrorError::Client,
        future::ready(Err::<(), _>("right")),
        MirrorError::<Infallible, &str>::Server,
    );
    assert!(matches!(
        run_to_quiescence(right),
        Ok(Err(MirrorError::Server("right")))
    ));
}

/// Reconcile `a` and `b` through the streaming local backend, returning both
/// sides' reconciled roots in argument order, with no convergence assertion.
fn streaming_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    streaming_mirror_sides_with_schedule(a, b, Vec::new())
}

/// Reconcile under an explicit, shrinkable channel-poll schedule.
fn streaming_mirror_sides_with_schedule(
    a: Root<()>,
    b: Root<()>,
    schedule: Vec<u8>,
) -> (Root<()>, Root<()>) {
    streaming_mirror_sides_with_schedules(a, b, schedule, Vec::new())
}

/// Reconcile under independent channel and Local-backend poll schedules.
fn streaming_mirror_sides_with_schedules(
    a: Root<()>,
    b: Root<()>,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> (Root<()>, Root<()>) {
    let (a, b): (StreamingRoot<Local, ()>, StreamingRoot<Local, ()>) = (a.into(), b.into());
    let client = Handshaking::start(Local, a.clone());
    let server = Handshaking::start(Local, b.clone());
    let (result, trace) = with_trace(|| {
        with_schedule(channel_schedule, || {
            with_local_schedule(backend_schedule, || {
                run_to_quiescence(drive_streaming(client, server))
            })
        })
    });
    let (ours, theirs) = result
        .expect("streaming mirror became quiescent before completion")
        // `Local` is infallible, so the session's only inhabited errors are
        // violations — which two honest local endpoints must never speak.
        .expect("local mirror speaks no violations");
    trace.assert_valid();
    (ours.into(), theirs.into())
}

/// Reconcile `a` and `b` through the streaming local backend, asserting the
/// two sides converge to the same root, and return it.
fn streaming_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides(a, b);
    assert_eq!(ours, theirs, "streaming endpoints should converge");
    ours
}

/// Reconcile under an explicit channel-poll schedule, asserting convergence.
fn scheduled_streaming_mirror(a: Root<()>, b: Root<()>, schedule: Vec<u8>) -> Root<()> {
    let (ours, theirs) = streaming_mirror_sides_with_schedule(a, b, schedule);
    assert_eq!(
        ours, theirs,
        "scheduled streaming endpoints should converge"
    );
    ours
}

/// Reconcile under independent channel and Local-backend poll schedules.
fn fully_scheduled_streaming_mirror(
    a: Root<()>,
    b: Root<()>,
    channel_schedule: Vec<u8>,
    backend_schedule: Vec<u8>,
) -> Root<()> {
    let (ours, theirs) =
        streaming_mirror_sides_with_schedules(a, b, channel_schedule, backend_schedule);
    assert_eq!(
        ours, theirs,
        "fully scheduled streaming endpoints should converge"
    );
    ours
}

/// Reconcile `a` and `b` through the alternating implementation — the
/// behavioral oracle the streaming protocol must reproduce exactly —
/// returning both sides' roots in argument order, with no convergence
/// assertion.
fn alternating_mirror_sides(a: Root<()>, b: Root<()>) -> (Root<()>, Root<()>) {
    pollster::block_on(async {
        let local_a = alternating::local::Exchange::start(a);
        let local_b = alternating::local::Exchange::start(b);
        match alternating::mirror(local_a, local_b).await {
            Err(e) => match e {},
            Ok(pair) => pair,
        }
    })
}

/// Reconcile `a` and `b` through the alternating oracle, asserting the two
/// sides converge to the same root, and return it.
fn alternating_mirror(a: Root<()>, b: Root<()>) -> Root<()> {
    let (ours, theirs) = alternating_mirror_sides(a, b);
    assert_eq!(ours, theirs, "oracle endpoints should converge");
    ours
}

/// Generated relationships for the independent alternating oracle.
///
/// The union covers shared-history divergence (including redactions and
/// matched subtrees), independent party histories (including empty bootstrap
/// shapes), and the equal-version short circuit.
fn arb_oracle_pair() -> impl Strategy<Value = (Root<()>, Root<()>)> {
    prop_oneof![
        4 => arb_divergent_pair(),
        2 => (arb_tree_root(0, 0..=8), arb_tree_root(1, 0..=8)),
        1 => arb_tree_root(0, 0..=8).prop_map(|root| (root.clone(), root)),
    ]
}

/// A dispute that survives to leaf-parent height — both sides hold the same
/// `S<Z>` prefix with different leaf sets — converges to the union.
///
/// The responder's closing `uncertain` lists its leaves, and the leaf-height
/// `Closing`/`Complete` words carry the difference in both directions.
#[test]
fn converges_on_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_dispute_pair();
    assert_eq!(
        streaming_mirror(a, b),
        expected,
        "both sides should hold the union",
    );
}

/// A leaf redacted on one side under a disputed leaf-parent must disappear
/// from the other side too: the closing request for it prunes against the
/// redactor's version and drops on both sides instead of shipping.
#[test]
fn honors_redaction_under_leaf_parent_dispute() {
    let (a, b, expected) = leaf_parent_redaction_pair();
    for (left, right) in [(a.clone(), b.clone()), (b, a)] {
        for (channel_schedule, backend_schedule) in [
            (Vec::new(), Vec::new()),
            (
                vec![2; 2_048],
                (0..2_048).map(|step| (step % 3) as u8).collect(),
            ),
        ] {
            assert_eq!(
                fully_scheduled_streaming_mirror(
                    left.clone(),
                    right.clone(),
                    channel_schedule,
                    backend_schedule,
                ),
                expected,
                "the redacted leaf should survive nowhere",
            );
        }
    }
}

proptest! {
    /// Streaming and the alternating oracle agree in both orientations.
    ///
    /// Across every generated causal relationship, both implementations
    /// return the same two roots and converge their endpoints. This is the
    /// design-of-record property tying selectable V1 to default V2 semantics.
    #[test]
    fn streaming_matches_alternating_oracle((a, b) in arb_oracle_pair()) {
        for (left, right) in [(a.clone(), b.clone()), (b, a)] {
            let expected = alternating_mirror_sides(left.clone(), right.clone());
            let actual = streaming_mirror_sides(left, right);
            prop_assert_eq!(&actual, &expected);
            prop_assert_eq!(&actual.0, &actual.1);
            prop_assert_eq!(&expected.0, &expected.1);
        }
    }
}
