//! Full-stack transport-failure propagation properties.

use std::{convert::Infallible, io};

use proptest::prelude::*;

use super::{harness, injected_operation, reconcile_locally, reconcile_with_stacked_failures};
use crate::message::Message;
use crate::testing::{
    InjectedIo, IoFault, IoFaultUnit, IoOperation, IoPlan, IoReport, IoSide, run_to_quiescence,
};
use crate::tree::{
    Action, Tree,
    arb::arb_divergent_pair,
    arb::nth_party,
    mirror::{
        Error as MirrorError,
        streaming::{
            Failing, Local,
            remote::{
                CodecDecodeErrorKind, CodecEncodeErrorKind, DemuxError, EncodeLeafError,
                Error as RemoteError, MuxError,
            },
        },
    },
};

/// Find the typed injected source retained anywhere below a remote failure.
fn injected<E>(error: &RemoteError<E>) -> Option<InjectedIo> {
    let source = match error {
        RemoteError::HandshakeRead(source) | RemoteError::HandshakeWrite(source) => source,
        RemoteError::Incoming(DemuxError::Codec(error)) => match &error.kind {
            CodecDecodeErrorKind::Read { source, .. }
            | CodecDecodeErrorKind::Truncated { source, .. } => source,
            _ => return None,
        },
        RemoteError::Outgoing(MuxError::Codec(error)) => match &error.kind {
            CodecEncodeErrorKind::Write { source, .. } | CodecEncodeErrorKind::Flush(source) => {
                source
            }
            CodecEncodeErrorKind::InvalidLeaf(EncodeLeafError::Version(source))
            | CodecEncodeErrorKind::InvalidLeaf(EncodeLeafError::Message(source)) => source,
            _ => return None,
        },
        _ => return None,
    };
    injected_io(source)
}

/// A deterministic pair whose proxy backends perform real conversion work.
fn stacked_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
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
    (left.root, right.root)
}

/// Recover the custom source stored inside an ordinary I/O error.
fn injected_io(error: &io::Error) -> Option<InjectedIo> {
    error
        .get_ref()
        .and_then(|source| source.downcast_ref::<InjectedIo>())
        .copied()
}

/// Transport direction errors must enter through their corresponding driver.
fn has_expected_surface(error: &RemoteError<Infallible>, operation: IoOperation) -> bool {
    match operation {
        IoOperation::Read => matches!(
            error,
            RemoteError::HandshakeRead(_) | RemoteError::Incoming(DemuxError::Codec(_))
        ),
        IoOperation::Write | IoOperation::Flush => {
            matches!(
                error,
                RemoteError::HandshakeWrite(_) | RemoteError::Outgoing(MuxError::Codec(_))
            )
        }
    }
}

fn plan(fault: Option<IoFault>, chunk: usize, delays: Vec<u8>, buffered: bool) -> IoPlan {
    IoPlan {
        read_chunk: chunk,
        write_chunk: chunk,
        read_delays: delays.clone(),
        write_delays: delays.clone(),
        flush_delays: delays,
        hold_until_flush: buffered,
        fault,
    }
}

/// Count the successful prefix relevant to one fault threshold.
fn completed(report: IoReport, fault: IoFault) -> usize {
    match (fault.operation, fault.unit) {
        (IoOperation::Read, IoFaultUnit::Operations) => report.reads,
        (IoOperation::Read, IoFaultUnit::Bytes) => report.read_bytes,
        (IoOperation::Write, IoFaultUnit::Operations) => report.writes,
        (IoOperation::Write, IoFaultUnit::Bytes) => report.write_bytes,
        (IoOperation::Flush, _) => report.flushes,
    }
}

/// Return the selected endpoint's remote error, if that endpoint failed at
/// the transport layer expected by the harness role.
fn endpoint_error(
    outcome: &harness::Outcome,
    fail_left: bool,
) -> Result<&RemoteError<Infallible>, TestCaseError> {
    if fail_left {
        match &outcome.left {
            Err(MirrorError::Server(error)) => Ok(error),
            other => Err(TestCaseError::fail(format!(
                "left transport fault was masked: {other:?}",
            ))),
        }
    } else {
        match &outcome.right {
            Err(MirrorError::Client(error)) => Ok(error),
            other => Err(TestCaseError::fail(format!(
                "right transport fault was masked: {other:?}",
            ))),
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        max_shrink_iters: 2_048,
        ..ProptestConfig::default()
    })]

    /// Every reached read, write, or flush fault retains its typed identity
    /// and terminates both sessions; every unreached fault is behaviorally inert.
    #[test]
    fn transport_failures_are_exact_and_fail_fast(
        (left, right) in arb_divergent_pair(),
        fail_left in any::<bool>(),
        operation in prop_oneof![
            Just(IoOperation::Read),
            Just(IoOperation::Write),
            Just(IoOperation::Flush),
        ],
        after in 0usize..256,
        bytes in any::<bool>(),
        chunk in 1usize..=16,
        delays in proptest::collection::vec(0_u8..=2, 0..32),
        buffered in any::<bool>(),
    ) {
        let unit = if bytes && operation != IoOperation::Flush {
            IoFaultUnit::Bytes
        } else {
            IoFaultUnit::Operations
        };
        let fault = IoFault { operation, after, unit };
        let expected = run_to_quiescence(reconcile_locally(left.clone(), right.clone()))
            .expect("the materialized oracle should remain live");
        let clean_plan = plan(None, chunk, delays.clone(), buffered);
        let clean = run_to_quiescence(harness::reconcile(
            left.clone(),
            right.clone(),
            17,
            if fail_left { clean_plan.clone() } else { IoPlan::default() },
            if fail_left { IoPlan::default() } else { clean_plan },
        ))
        .map_err(|stopped| TestCaseError::fail(format!(
            "clean adverse transport became quiescent: {stopped:?}",
        )))?;
        prop_assert_eq!(clean.left.as_ref().ok(), Some(&expected.0));
        prop_assert_eq!(clean.right.as_ref().ok(), Some(&expected.1));
        let clean_report = if fail_left {
            clean.left_io.snapshot()
        } else {
            clean.right_io.snapshot()
        };
        let should_inject = after < completed(clean_report, fault);

        let fault_plan = plan(Some(fault), chunk, delays, buffered);
        let outcome = run_to_quiescence(harness::reconcile(
            left.clone(),
            right.clone(),
            17,
            if fail_left { fault_plan.clone() } else { IoPlan::default() },
            if fail_left { IoPlan::default() } else { fault_plan },
        ))
        .map_err(|stopped| TestCaseError::fail(format!(
            "transport fault left the session quiescent: {stopped:?}",
        )))?;
        let report = if fail_left {
            outcome.left_io.snapshot()
        } else {
            outcome.right_io.snapshot()
        };
        let expected_fault = InjectedIo {
            side: if fail_left { IoSide::Left } else { IoSide::Right },
            operation,
            after,
            unit,
        };
        prop_assert_eq!(report.injected, should_inject.then_some(expected_fault));

        if should_inject {
            let error = endpoint_error(&outcome, fail_left)?;
            prop_assert!(has_expected_surface(error, operation));
            prop_assert_eq!(injected(error), Some(expected_fault));
            prop_assert!(outcome.left.is_err());
            prop_assert!(outcome.right.is_err());

            let recovered = run_to_quiescence(harness::reconcile(
                left,
                right,
                17,
                IoPlan::default(),
                IoPlan::default(),
            ))
            .map_err(|stopped| TestCaseError::fail(format!(
                "clean recovery became quiescent: {stopped:?}",
            )))?;
            prop_assert_eq!(recovered.left.as_ref().ok(), Some(&expected.0));
            prop_assert_eq!(recovered.right.as_ref().ok(), Some(&expected.1));
        } else {
            prop_assert_eq!(outcome.left.as_ref().ok(), Some(&expected.0));
            prop_assert_eq!(outcome.right.as_ref().ok(), Some(&expected.1));
        }

    }
}

/// Every meaningful operation/unit pair has deterministic immediate-failure
/// coverage, independent of the generated reachability cases above.
#[test]
fn every_transport_fault_surface_is_reachable() {
    let variants = [
        (IoOperation::Read, IoFaultUnit::Operations),
        (IoOperation::Read, IoFaultUnit::Bytes),
        (IoOperation::Write, IoFaultUnit::Operations),
        (IoOperation::Write, IoFaultUnit::Bytes),
        (IoOperation::Flush, IoFaultUnit::Operations),
    ];

    for (operation, unit) in variants {
        let (left, right) = stacked_pair();
        let fault = IoFault {
            operation,
            after: 0,
            unit,
        };
        let outcome = run_to_quiescence(harness::reconcile(
            left,
            right,
            17,
            plan(Some(fault), usize::MAX, Vec::new(), false),
            IoPlan::default(),
        ))
        .unwrap_or_else(|stopped| panic!("{operation:?}/{unit:?} became quiescent: {stopped:?}"));
        let expected = InjectedIo {
            side: IoSide::Left,
            operation,
            after: 0,
            unit,
        };
        assert_eq!(outcome.left_io.snapshot().injected, Some(expected));
        let error = endpoint_error(&outcome, true).unwrap();
        assert!(has_expected_surface(error, operation));
        assert_eq!(injected(error), Some(expected));
        assert!(outcome.right.is_err());
    }
}

/// Backend and transport decorators compose without changing which reachable
/// layer supplies the causal failure.
#[test]
fn stacked_backend_and_transport_failures_remain_distinct() {
    let (left, right) = stacked_pair();
    let backend = Failing::after(Local, 0);
    let unreachable_io = IoPlan {
        fault: Some(IoFault {
            operation: IoOperation::Write,
            after: usize::MAX,
            unit: IoFaultUnit::Operations,
        }),
        ..IoPlan::default()
    };
    let ((left_result, right_result), io) = run_to_quiescence(reconcile_with_stacked_failures(
        left.clone(),
        right.clone(),
        backend.clone(),
        true,
        unreachable_io,
    ))
    .expect("backend-first stacked failure should terminate");
    let backend_error = match &left_result {
        Err(MirrorError::Server(error)) => error,
        other => panic!("backend error was masked: {other:?}"),
    };
    assert_eq!(
        injected_operation(backend_error),
        backend.history().first().copied(),
    );
    assert!(io.snapshot().injected.is_none());
    assert!(right_result.is_err());

    let immediate_io = IoPlan {
        fault: Some(IoFault {
            operation: IoOperation::Write,
            after: 0,
            unit: IoFaultUnit::Operations,
        }),
        ..IoPlan::default()
    };
    let ((left_result, right_result), io) = run_to_quiescence(reconcile_with_stacked_failures(
        left,
        right,
        Failing::after(Local, usize::MAX),
        true,
        immediate_io,
    ))
    .expect("transport-first stacked failure should terminate");
    let transport_error = match &left_result {
        Err(MirrorError::Server(error)) => error,
        other => panic!("transport error was masked: {other:?}"),
    };
    assert_eq!(injected(transport_error), io.snapshot().injected);
    assert!(io.snapshot().injected.is_some());
    assert!(right_result.is_err());
}
