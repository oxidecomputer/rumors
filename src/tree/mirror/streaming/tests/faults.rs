//! Connected-session abort routing and lifecycle atomicity.

use proptest::prelude::*;

use super::{
    fixtures::{LeafOrder, full_depth_comb_pair, one_sided_pair},
    streaming_mirror_sides,
};
use crate::testing::run_to_quiescence;
use crate::tree::arb::arb_divergent_pair;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::mirror::{
    Error as MirrorError,
    streaming::{
        Failing, FailingNode, Failure, Fault, Faulting, GreetingLie, Local, Root as StreamingRoot,
        materialized::{
            Error as MaterializedError, Handshaking, Violation,
            channel::{with_observation, with_schedule},
        },
        mirror as drive_streaming,
    },
};

fn failing_root(root: crate::tree::Root<()>) -> StreamingRoot<Failing<Local>, ()> {
    StreamingRoot {
        ceiling: root.ceiling,
        root: root.root.map(FailingNode::new),
    }
}

/// The connected abort suite's injected faults: one structural shape
/// (a reply reaction the pairing never asked for) and one content shape
/// (a structurally legal supply whose version escapes the declared
/// greeting version).
///
/// The escape rides a party no fixture ticks
/// ([`Faulting`]'s injection machinery), so the supplied ceiling is
/// *incomparable* with the declared version: the containment
/// predicate's hard case crosses the connected driver end to end, not
/// only the dominating regime the deterministic tripwires build.
fn arb_connected_violation() -> impl Strategy<Value = Violation> {
    prop_oneof![
        Just(Violation::UnexpectedQuery),
        Just(Violation::UncontainedSupply),
    ]
}

/// Every greeting lie the harness can tell ([`GreetingLie`]): both
/// detectable under-declarations and both tolerated over-declarations.
fn arb_greeting_lie() -> impl Strategy<Value = GreetingLie> {
    prop_oneof![
        Just(GreetingLie::ShrunkenSetLen),
        Just(GreetingLie::InflatedSetLen),
        Just(GreetingLie::ShrunkenVersion),
        Just(GreetingLie::InflatedVersion),
    ]
}

proptest! {
    /// A genuine malformed reply crosses the fully connected driver as its
    /// detected violation while both materialized input roots remain untouched.
    #[test]
    fn connected_violation_aborts_without_mutating_root(
        violation in arb_connected_violation(),
        server_steps in 0usize..=15,
        client_steps in 0usize..=15,
    ) {
        let (client_root, server_root) =
            full_depth_comb_pair(2, LeafOrder::Interleaved);
        let before = (client_root.clone(), server_root.clone());
        let local = Handshaking::start(Local, StreamingRoot::from(client_root.clone())).window(WindowConfig::FLOOR);
        let honest_server = Handshaking::start(Local, StreamingRoot::from(server_root.clone())).window(WindowConfig::FLOOR);
        let faulting_server = Faulting::new(honest_server, server_steps, Some(Fault::Reply(violation)));
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
        let honest_client = Handshaking::start(Local, StreamingRoot::from(client_root.clone())).window(WindowConfig::FLOOR);
        let faulting_client = Faulting::new(honest_client, client_steps, Some(Fault::Reply(violation)));
        let local = Handshaking::start(Local, StreamingRoot::from(server_root.clone())).window(WindowConfig::FLOOR);
        let result = run_to_quiescence(drive_streaming(faulting_client, local))
            .expect("the reversed connected driver must surface the fault, not stall");
        match result {
            Err(MirrorError::Server(MaterializedError::Violation(actual))) => {
                prop_assert_eq!(actual, violation);
            }
            Err(other) => prop_assert!(false, "unexpected reversed driver error: {other:?}"),
            Ok(_) => prop_assert!(false, "the reversed faulting counterparty unexpectedly completed"),
        }

        prop_assert_eq!((client_root, server_root), before);
    }

    /// Every greeting lie classifies exactly against an honest counterparty,
    /// in both orientations.
    ///
    /// The under-declarations are detected by the deceived side as their
    /// named violation — through an entirely honest supply stream, the path
    /// the reply-corruption faults never take — with both input roots
    /// untouched. The over-declarations are tolerated: a declaration is an
    /// upper premise, so the session completes and reconciles the same
    /// content an honest run does (an inflated declared version widens the
    /// deceived side's output ceiling by exactly the inflation, which
    /// `ours | declared` absorbs by definition, so the ceiling comparison
    /// is asserted only where the lie leaves it untouched).
    #[test]
    fn greeting_lies_classify_exactly(
        lie in arb_greeting_lie(),
        fault_client in any::<bool>(),
    ) {
        let (client_root, server_root) =
            full_depth_comb_pair(2, LeafOrder::Interleaved);
        let before = (client_root.clone(), server_root.clone());
        let expected = match lie {
            GreetingLie::ShrunkenSetLen => Some(Violation::OverdrawnSupply),
            GreetingLie::ShrunkenVersion => Some(Violation::UncontainedSupply),
            GreetingLie::InflatedSetLen | GreetingLie::InflatedVersion => None,
        };

        let client = Handshaking::start(Local, StreamingRoot::from(client_root.clone()))
            .window(WindowConfig::FLOOR);
        let server = Handshaking::start(Local, StreamingRoot::from(server_root.clone()))
            .window(WindowConfig::FLOOR);
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
                let (ours, theirs): (crate::tree::Root<()>, crate::tree::Root<()>) =
                    (ours.into(), theirs.into());
                let (base_client, base_server) =
                    streaming_mirror_sides(client_root.clone(), server_root.clone());
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

        prop_assert_eq!((client_root, server_root), before);
    }

    /// Every reached materialized backend failure terminates the session and
    /// survives sibling cancellation with its exact operation identity.
    #[test]
    fn materialized_backend_failures_are_fail_fast(
        (client_root, server_root) in arb_divergent_pair(),
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
        let client = Handshaking::start(client_backend, failing_root(client_root)).window(WindowConfig::FLOOR);
        let server = Handshaking::start(server_backend, failing_root(server_root)).window(WindowConfig::FLOOR);
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
    let client =
        Handshaking::start(backend.clone(), failing_root(client_root)).window(WindowConfig::FLOOR);
    let server = Handshaking::start(backend, failing_root(server_root)).window(WindowConfig::FLOOR);
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
    let client =
        Handshaking::start(backend.clone(), failing_root(client_root)).window(WindowConfig::FLOOR);
    let server = Handshaking::start(backend, failing_root(server_root)).window(WindowConfig::FLOOR);
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
