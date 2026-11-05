//! Connected-session error routing and greeting validation.

use std::convert::Infallible;

use proptest::prelude::*;

use super::{
    fixtures::{LeafOrder, full_depth_comb_pair, one_sided_pair},
    floor_start, streaming_mirror_sides,
};
use crate::default;
use crate::testing::run_to_quiescence;
use crate::tree::mirror::{
    Error as MirrorError,
    streaming::{
        FAN, Failing, Failure, Fault, Faulting, GreetingLie, Local, Miscount, Misreport,
        ReplyCorruption, Root as StreamingRoot,
        channel::{with_observation, with_schedule},
        materialized::{Error as MaterializedError, Handshaking, Start, Violation},
        message::CountError,
        mirror as drive_streaming,
    },
};
use crate::tree::{
    Root,
    arb::{arb_wide_divergent_pair, leaf_parent_dispute_pair},
    typed::{
        Hash, HashKey,
        height::{self, Height as _},
    },
};

/// The digest of `root`'s live set under the fixture keys, which the
/// streaming test sessions run under, or `None` for an empty root.
fn live_hash(root: &Root) -> Option<Hash> {
    root.node
        .as_ref()
        .map(|node| node.hash(&HashKey::fixture()))
}

/// A `Failing<Local>` endpoint at a budget of zero over `root`, its nodes
/// wrapped for the failing backend, which misreports the root's count if
/// it misreports at the root's height.
fn failing_start(backend: Failing<Local>, root: Root) -> Handshaking<Failing<Local>, Start> {
    let root = StreamingRoot {
        ceiling: root.ceiling,
        root: root.node.map(|node| backend.root(node.into())),
    };
    Handshaking::start(
        backend,
        root,
        Some(crate::network::NetworkKeys::fixture()),
        default::TARGET_MESSAGE_SIZE as u64,
    )
    .budget(0)
}

/// The last ordinary walk reply in the full-depth fixture.
const LAST_WALK_REPLY: usize = 15;

/// An in-process session failure, with the same error type on each side.
type ConnectedError = MirrorError<MaterializedError<Infallible>, MaterializedError<Infallible>>;

/// Run one reply corruption at one ordinary walk phase and orientation.
fn connected_violation(
    corruption: ReplyCorruption,
    steps: usize,
    fault_client: bool,
) -> Result<(), ConnectedError> {
    let (client_root, server_root) = full_depth_comb_pair(2, LeafOrder::Interleaved);
    if fault_client {
        let client = Faulting::new(
            floor_start(client_root),
            steps,
            Some(Fault::Reply(corruption)),
        );
        let server = floor_start(server_root);
        run_to_quiescence(drive_streaming(client, server))
            .expect("the reversed connected driver must surface the fault, not stall")
            .map(|_| ())
    } else {
        let client = floor_start(client_root);
        let server = Faulting::new(
            floor_start(server_root),
            steps,
            Some(Fault::Reply(corruption)),
        );
        run_to_quiescence(drive_streaming(client, server))
            .expect("the connected driver must surface the fault, not stall")
            .map(|_| ())
    }
}

/// Every scripted reply corruption is detected at every ordinary walk phase,
/// in either orientation, and attributed to its receiver.
#[test]
fn connected_violations_are_classified_exhaustively() {
    for &corruption in ReplyCorruption::ALL {
        for steps in 0..=LAST_WALK_REPLY {
            for fault_client in [false, true] {
                let violation = corruption.violation();
                let result = connected_violation(corruption, steps, fault_client);
                let actual = match (fault_client, result) {
                    (false, Err(MirrorError::Client(MaterializedError::Violation(actual))))
                    | (true, Err(MirrorError::Server(MaterializedError::Violation(actual)))) => {
                        actual
                    }
                    (_, Err(other)) => panic!(
                        "{corruption:?} at phase {steps}, fault_client={fault_client} was \
                         misclassified: {other:?}"
                    ),
                    (_, Ok(())) => panic!(
                        "{corruption:?} at phase {steps}, fault_client={fault_client} completed"
                    ),
                };
                assert_eq!(actual, violation);
            }
        }
    }
}

proptest! {
    /// Every reached materialized backend failure terminates the session and
    /// survives sibling cancellation with its exact operation identity.
    ///
    /// The wide generator is what puts failures on the response-stream
    /// path: a stage loop explodes nodes only under disputed scopes, and
    /// the small generator's few leaves rarely collide into one.
    #[test]
    fn materialized_backend_failures_are_fail_fast(
        (client_root, server_root) in arb_wide_divergent_pair(),
        operations in 0usize..32,
        fail_client in any::<bool>(),
        schedule in proptest::collection::vec(0_u8..=2, 0..128),
    ) {
        let failing = Failing::after(Local, operations);
        let client_backend = if fail_client {
            failing.clone()
        } else {
            Failing::after(Local, usize::MAX)
        };
        let server_backend = if fail_client {
            Failing::after(Local, usize::MAX)
        } else {
            failing.clone()
        };
        let client = failing_start(client_backend, client_root);
        let server = failing_start(server_backend, server_root);
        let result = with_schedule(schedule, || {
            run_to_quiescence(drive_streaming(client, server))
        })
            .map_err(|stopped| TestCaseError::fail(format!(
                "backend failure left materialized reconciliation quiescent: {stopped:?}",
            )))?;
        let history = failing.history();

        if let Some(expected) = history.get(operations).copied() {
            let actual = match result {
                Err(MirrorError::Client(MaterializedError::Backend(
                    Failure::Injected(operation),
                ))) if fail_client => Some(operation),
                Err(MirrorError::Server(MaterializedError::Backend(
                    Failure::Injected(operation),
                ))) if !fail_client => Some(operation),
                other => return Err(TestCaseError::fail(format!(
                    "materialized backend failure was masked: {other:?}",
                ))),
            };
            prop_assert_eq!(actual, Some(expected));
        } else {
            prop_assert!(result.is_ok(), "session failed without injection: {result:?}");
        }
    }
}

/// The typed height of the root's children, which every greeting lists.
const ROOT_CHILDREN_HEIGHT: usize = 31;

proptest! {
    /// A grandchild count above 65,536, which a backend reading corrupted
    /// storage could report, fails the session with that count on the side
    /// whose backend reported it, and neither panics nor stalls.
    ///
    /// The count is misreported for every node one side's backend lists as
    /// a child at one height, or for its root at the root's height. The
    /// session reads a node's count only to list it, or for the root to
    /// send it in its greeting, and then fails at once, so it fails exactly
    /// when it read a misreported count and completes otherwise. Every
    /// greeting carries the root's count and lists the root's children, so
    /// a misreported count at either height always fails the session when
    /// that side holds a root; deeper heights fail only where a disputed
    /// scope's question lists them.
    #[test]
    fn out_of_range_grandchild_counts_fail_the_session(
        (client_root, server_root) in arb_wide_divergent_pair(),
        height in 1..=height::Root::HEIGHT,
        grandchildren in prop_oneof![Just(65_537usize), 65_537usize..=1 << 24, Just(usize::MAX)],
        miscount_client in any::<bool>(),
        schedule in proptest::collection::vec(0_u8..=2, 0..128),
    ) {
        let miscounting = Failing::miscounting(Local, Miscount {
            height,
            report: Misreport::Grandchildren(grandchildren),
        });
        let sound = Failing::after(Local, usize::MAX);
        let holds_root = if miscount_client {
            client_root.node.is_some()
        } else {
            server_root.node.is_some()
        };
        let (client_backend, server_backend) = if miscount_client {
            (miscounting.clone(), sound)
        } else {
            (sound, miscounting.clone())
        };
        let client = failing_start(client_backend, client_root);
        let server = failing_start(server_backend, server_root);
        let result = with_schedule(schedule, || {
            run_to_quiescence(drive_streaming(client, server))
        })
            .map_err(|stopped| TestCaseError::fail(format!(
                "a misreported count left reconciliation quiescent: {stopped:?}",
            )))?;
        let read = miscounting.miscounts_read();
        match result {
            Err(MirrorError::Client(MaterializedError::Count(error))) if miscount_client => {
                prop_assert_eq!(error, CountError::Grandchildren(grandchildren));
                prop_assert!(read > 0, "the session failed on a count it never read");
            }
            Err(MirrorError::Server(MaterializedError::Count(error))) if !miscount_client => {
                prop_assert_eq!(error, CountError::Grandchildren(grandchildren));
                prop_assert!(read > 0, "the session failed on a count it never read");
            }
            Ok(_) => prop_assert_eq!(read, 0, "the session completed past a misreported count"),
            Err(other) => return Err(TestCaseError::fail(format!(
                "a misreported count was misattributed: {other:?}",
            ))),
        }
        if height >= ROOT_CHILDREN_HEIGHT && holds_root {
            prop_assert!(read > 0, "the greeting must carry the root's count and list its children");
        }
    }
}

proptest! {
    /// A child count above a fan, which a backend reading corrupted storage
    /// could report, fails the session with that count on the side whose
    /// backend reported it, and neither panics nor stalls.
    ///
    /// The count is misreported for every node one side's backend lists as
    /// a child at one height, or for its root at the root's height. A
    /// session reads a node's child count to price an answer about it, to
    /// list it, and to explode it, and fails at the listing or the
    /// explosion, before it sizes anything by the count. So it completes or
    /// fails with exactly that count on that side, and fails only once it
    /// has read a misreported count. It always fails when that side holds a
    /// root and the height is the root's or its children's, since every
    /// greeting explodes the root and lists its children. It cannot require
    /// failure whenever a misreported count is read: a supply answer prices
    /// a node by its count and explodes it only if the peer lacks some of
    /// its leaves.
    #[test]
    fn out_of_range_child_counts_fail_the_session(
        (client_root, server_root) in arb_wide_divergent_pair(),
        height in 1..=height::Root::HEIGHT,
        children in prop_oneof![Just(FAN + 1), FAN + 1..=1 << 24, Just(usize::MAX)],
        miscount_client in any::<bool>(),
        schedule in proptest::collection::vec(0_u8..=2, 0..128),
    ) {
        let miscounting = Failing::miscounting(Local, Miscount {
            height,
            report: Misreport::Children(children),
        });
        let sound = Failing::after(Local, usize::MAX);
        let holds_root = if miscount_client {
            client_root.node.is_some()
        } else {
            server_root.node.is_some()
        };
        let (client_backend, server_backend) = if miscount_client {
            (miscounting.clone(), sound)
        } else {
            (sound, miscounting.clone())
        };
        let client = failing_start(client_backend, client_root);
        let server = failing_start(server_backend, server_root);
        let result = with_schedule(schedule, || {
            run_to_quiescence(drive_streaming(client, server))
        })
            .map_err(|stopped| TestCaseError::fail(format!(
                "a misreported child count left reconciliation quiescent: {stopped:?}",
            )))?;
        let read = miscounting.miscounts_read();
        let failed = match result {
            Err(MirrorError::Client(MaterializedError::Count(error))) if miscount_client => {
                prop_assert_eq!(error, CountError::Children(children));
                true
            }
            Err(MirrorError::Server(MaterializedError::Count(error))) if !miscount_client => {
                prop_assert_eq!(error, CountError::Children(children));
                true
            }
            Ok(_) => false,
            Err(other) => return Err(TestCaseError::fail(format!(
                "a misreported child count was misattributed: {other:?}",
            ))),
        };
        if failed {
            prop_assert!(read > 0, "the session failed on a count it never read");
        }
        if height >= ROOT_CHILDREN_HEIGHT && holds_root {
            prop_assert!(failed, "the greeting must explode the root and list its children");
        }
    }
}

/// Every greeting lie is classified in both orientations.
///
/// Under-declarations fail with their exact violation. Over-declarations
/// preserve the reconciled content; an inflated version also widens history,
/// so only a size inflation preserves the honest run's ceiling.
#[test]
fn greeting_lies_are_classified_exhaustively() {
    for &lie in GreetingLie::ALL {
        for fault_client in [false, true] {
            let (client_root, server_root) = full_depth_comb_pair(2, LeafOrder::Interleaved);
            let expected = match lie {
                // The zero declaration trips at the first absorbed supply;
                // the one-leaf declaration admits supply first and trips on
                // the ledger's accumulation; both land as the same
                // violation, from opposite ends of the allowance.
                GreetingLie::ShrunkenSetLen | GreetingLie::UnderdeclaredSetLen => {
                    Some(Violation::OverdrawnSupply)
                }
                GreetingLie::InflatedSetLen | GreetingLie::InflatedVersion => None,
            };

            let client = floor_start(client_root.clone());
            let server = floor_start(server_root.clone());
            let result = if fault_client {
                let faulting = Faulting::new(client, 0, Some(Fault::Greeting(lie)));
                run_to_quiescence(drive_streaming(faulting, server))
            } else {
                let faulting = Faulting::new(server, 0, Some(Fault::Greeting(lie)));
                run_to_quiescence(drive_streaming(client, faulting))
            }
            .expect("a greeting-lied session must terminate, not stall");

            match (expected, result) {
                (Some(violation), Err(error)) => {
                    // The deceived side raises the violation; the driver
                    // reports errors by the side that raised them.
                    match (fault_client, error) {
                        (true, MirrorError::Server(MaterializedError::Violation(actual)))
                        | (false, MirrorError::Client(MaterializedError::Violation(actual))) => {
                            assert_eq!(actual, violation);
                        }
                        (_, other) => panic!("misrouted or mistyped greeting-lie error: {other:?}"),
                    }
                }
                (None, Ok((ours, theirs))) => {
                    let (ours, theirs): (Root, Root) = (ours.into(), theirs.into());
                    let (base_client, base_server) =
                        streaming_mirror_sides(client_root, server_root);
                    assert_eq!(live_hash(&ours), live_hash(&base_client));
                    assert_eq!(live_hash(&theirs), live_hash(&base_server));
                    if lie == GreetingLie::InflatedSetLen {
                        assert_eq!(&ours.ceiling, &base_client.ceiling);
                        assert_eq!(&theirs.ceiling, &base_server.ceiling);
                    }
                }
                (Some(violation), Ok(_)) => {
                    panic!("undetected greeting lie {lie:?}: expected {violation:?}")
                }
                (None, Err(error)) => panic!("benign greeting lie {lie:?} faulted: {error:?}"),
            }
        }
    }
}

/// A malformed query in the final leaf reply is detected on either protocol
/// side and attributed to the receiver that rejects it.
#[test]
fn terminal_reply_violation_is_detected_on_both_sides() {
    let (client_root, server_root, _) = leaf_parent_dispute_pair();
    let corruption = ReplyCorruption::UnexpectedQuery;

    let client = floor_start(client_root.clone());
    let server = Faulting::new(
        floor_start(server_root.clone()),
        16,
        Some(Fault::Reply(corruption)),
    );
    let error = run_to_quiescence(drive_streaming(client, server))
        .expect("the terminal violation must not stall")
        .expect_err("the terminal violation must abort the session");
    assert!(matches!(
        error,
        MirrorError::Client(MaterializedError::Violation(Violation::UnexpectedQuery))
    ));

    let client = Faulting::new(floor_start(client_root), 16, Some(Fault::Reply(corruption)));
    let server = floor_start(server_root);
    let error = run_to_quiescence(drive_streaming(client, server))
        .expect("the reversed terminal violation must not stall")
        .expect_err("the reversed terminal violation must abort the session");
    assert!(matches!(
        error,
        MirrorError::Server(MaterializedError::Violation(Violation::UnexpectedQuery))
    ));
}

/// Equal versions return both connected states' outputs without opening the
/// descent.
#[test]
fn equal_versions_return_outputs_without_descent() {
    let (_, root) = one_sided_pair(&[(0x20, 2, 1)]);
    let ((ours, theirs), report) =
        with_observation(|| streaming_mirror_sides(root.clone(), root.clone()));

    assert_eq!(ours, root);
    assert_eq!(theirs, root);
    assert_eq!(
        report.roles().count(),
        0,
        "the equal-version path must not construct descent queues",
    );
}

/// Semantic and source-failure decorators can be nested without erasing which
/// layer aborted the session.
#[test]
fn semantic_and_backend_failure_layers_compose() {
    let (client_root, server_root) = one_sided_pair(&[(0x20, 1, 1)]);
    let backend = Failing::after(Local, usize::MAX);
    let client = failing_start(backend.clone(), client_root);
    let server = failing_start(backend, server_root);
    let server = Faulting::new(
        server,
        0,
        Some(Fault::Reply(ReplyCorruption::UnexpectedQuery)),
    );
    let error = run_to_quiescence(drive_streaming(client, server))
        .expect("the stacked session must terminate")
        .expect_err("the semantic decorator must fault");
    assert!(matches!(
        error,
        MirrorError::Client(MaterializedError::Violation(Violation::UnexpectedQuery))
    ));

    let (client_root, server_root) = one_sided_pair(&[(0x20, 1, 1)]);
    let backend = Failing::after(Local, 0);
    let client = failing_start(backend.clone(), client_root);
    let server = failing_start(backend, server_root);
    let server = Faulting::new(server, 0, None);
    let error = run_to_quiescence(drive_streaming(client, server))
        .expect("the stacked session must terminate")
        .expect_err("the backend decorator must fault");
    assert!(matches!(
        error,
        MirrorError::Client(MaterializedError::Backend(Failure::Injected(_)))
            | MirrorError::Server(MaterializedError::Backend(Failure::Injected(_)))
    ));
}
