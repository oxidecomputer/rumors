//! Connected-session error routing and greeting validation.

use proptest::prelude::*;

use super::{
    fixtures::{LeafOrder, full_depth_comb_pair, one_sided_pair},
    floor_start, streaming_mirror_sides,
};
use crate::DEFAULT_TARGET_MESSAGE_SIZE;
use crate::testing::run_to_quiescence;
use crate::tree::arb::arb_wide_divergent_pair;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::mirror::{
    Error as MirrorError,
    streaming::{
        Failing, FailingNode, Failure, Fault, Faulting, GreetingLie, Local, Root as StreamingRoot,
        materialized::{
            Error as MaterializedError, Handshaking, Start, Violation,
            channel::{with_observation, with_schedule},
        },
        mirror as drive_streaming,
    },
};

/// A `Failing<Local>` endpoint at the floor window over `root`, its nodes
/// wrapped for the failing backend.
fn failing_start(
    backend: Failing<Local>,
    root: crate::tree::Root,
) -> Handshaking<Failing<Local>, Start> {
    let root = StreamingRoot {
        ceiling: root.ceiling,
        root: root.root.map(FailingNode::new),
    };
    Handshaking::start(backend, root, DEFAULT_TARGET_MESSAGE_SIZE as u64)
        .window(WindowConfig::FLOOR)
}

/// The connected abort suite's injected faults: every reply-shaped
/// violation [`Faulting`] can script.
///
/// All but one are structural. The content shape is a structurally legal
/// supply whose version escapes the declared greeting version; the escape
/// rides a party no fixture ticks ([`Faulting`]'s injection machinery),
/// so the supplied ceiling is *incomparable* with the declared version:
/// the containment predicate's hard case crosses the connected driver end
/// to end, not only the dominating regime the deterministic tripwires
/// build.
fn arb_connected_violation() -> impl Strategy<Value = Violation> {
    prop_oneof![
        Just(Violation::UnaskedReply),
        Just(Violation::UnansweredQuery),
        Just(Violation::UnfinishedReply),
        Just(Violation::UnexpectedMatch),
        Just(Violation::UnexpectedQuery),
        Just(Violation::UnexpectedSupply),
        Just(Violation::InvalidSupply),
        Just(Violation::UncontainedSupply),
    ]
}

/// Every greeting lie the harness can tell ([`GreetingLie`]): both
/// detectable under-declarations and both tolerated over-declarations.
fn arb_greeting_lie() -> impl Strategy<Value = GreetingLie> {
    prop_oneof![
        Just(GreetingLie::ShrunkenSetLen),
        Just(GreetingLie::UnderdeclaredSetLen),
        Just(GreetingLie::InflatedSetLen),
        Just(GreetingLie::ShrunkenVersion),
        Just(GreetingLie::InflatedVersion),
    ]
}

proptest! {
    /// The connected driver returns the detected reply violation on the correct side.
    #[test]
    fn connected_violation_aborts_with_its_error(
        violation in arb_connected_violation(),
        server_steps in 0usize..=15,
        client_steps in 0usize..=15,
    ) {
        let (client_root, server_root) =
            full_depth_comb_pair(2, LeafOrder::Interleaved);
        let local = floor_start(client_root.clone());
        let honest_server = floor_start(server_root.clone());
        let faulting_server =
            Faulting::new(honest_server, server_steps, Some(Fault::Reply(violation)));
        let result = run_to_quiescence(drive_streaming(local, faulting_server))
            .expect("the connected driver must surface the fault, not stall");
        match result {
            Err(MirrorError::Client(MaterializedError::Violation(actual))) => {
                prop_assert_eq!(actual, violation);
            }
            Err(other) => prop_assert!(false, "unexpected driver error: {other:?}"),
            Ok(_) => prop_assert!(false, "the faulting counterparty unexpectedly completed"),
        }

        // Reversing the handshake sides also reverses initiator order: the
        // driver's frame-relative error is flipped back to the original client.
        let honest_client = floor_start(client_root);
        let faulting_client =
            Faulting::new(honest_client, client_steps, Some(Fault::Reply(violation)));
        let local = floor_start(server_root);
        let result = run_to_quiescence(drive_streaming(faulting_client, local))
            .expect("the reversed connected driver must surface the fault, not stall");
        match result {
            Err(MirrorError::Server(MaterializedError::Violation(actual))) => {
                prop_assert_eq!(actual, violation);
            }
            Err(other) => prop_assert!(false, "unexpected reversed driver error: {other:?}"),
            Ok(_) => prop_assert!(
                false,
                "the reversed faulting counterparty unexpectedly completed"
            ),
        }
    }

    /// Under-declared greetings return the expected violation in either orientation.
    ///
    /// Over-declarations preserve the reconciled content; an inflated version
    /// also widens the resulting history, so only a size inflation preserves
    /// the honest run's ceiling.
    #[test]
    fn greeting_lies_classify_exactly(
        lie in arb_greeting_lie(),
        fault_client in any::<bool>(),
    ) {
        let (client_root, server_root) =
            full_depth_comb_pair(2, LeafOrder::Interleaved);
        let expected = match lie {
            // The zero declaration trips at the first absorbed supply;
            // the one-leaf declaration admits supply first and trips on
            // the ledger's accumulation — both land as the same
            // violation, from opposite ends of the allowance.
            GreetingLie::ShrunkenSetLen | GreetingLie::UnderdeclaredSetLen => {
                Some(Violation::OverdrawnSupply)
            }
            GreetingLie::ShrunkenVersion => Some(Violation::UncontainedSupply),
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
                        prop_assert_eq!(actual, violation);
                    }
                    (_, other) => {
                        return Err(TestCaseError::fail(format!(
                            "misrouted or mistyped greeting-lie error: {other:?}",
                        )));
                    }
                }
            }
            (None, Ok((ours, theirs))) => {
                let (ours, theirs): (crate::tree::Root, crate::tree::Root) =
                    (ours.into(), theirs.into());
                let (base_client, base_server) =
                    streaming_mirror_sides(client_root, server_root);
                prop_assert_eq!(&ours.root, &base_client.root);
                prop_assert_eq!(&theirs.root, &base_server.root);
                if lie == GreetingLie::InflatedSetLen {
                    prop_assert_eq!(&ours.ceiling, &base_client.ceiling);
                    prop_assert_eq!(&theirs.ceiling, &base_server.ceiling);
                }
            }
            (Some(violation), Ok(_)) => {
                return Err(TestCaseError::fail(format!(
                    "undetected greeting lie {lie:?}: expected {violation:?}",
                )));
            }
            (None, Err(error)) => {
                return Err(TestCaseError::fail(format!(
                    "benign greeting lie {lie:?} faulted: {error:?}",
                )));
            }
        }
    }

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
    let server = Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery)));
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
