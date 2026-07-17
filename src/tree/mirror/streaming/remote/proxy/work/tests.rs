use std::future;

use super::{Physical, Work};
use crate::link::memory;
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::{
    Failing, Failure, Local, Operation,
    remote::{
        adapter::EncodeError,
        codec::Speaker,
        proxy::Error,
        streams::{AcceptDriver, claims, error_route},
    },
};

/// A pump error cancels parked peers and retains its original error identity.
#[test]
fn pump_failure_preempts_parked_pumps() {
    let (link, _peer) = memory();
    let parts = link.into_parts();
    let (control_read, control_write, acceptor, epoch) = (
        parts.control_read,
        parts.control_write,
        parts.acceptor,
        parts.epoch,
    );
    let (slots, claims) = claims();
    let (route, errors) = error_route();
    let accept = AcceptDriver::new(acceptor, epoch, Speaker::Responder, slots, route);
    let mut work: Work<_, (), _, _, _> = Work::new(
        Failing::after(Local, usize::MAX),
        Physical {
            control_read,
            control_write,
            accept,
            errors,
        },
    );

    // Poll the parked task first so this specifically exercises fail-fast
    // aggregation rather than relying on the error being the first item.
    for _ in 0..31 {
        work.spawn(future::pending());
    }
    work.spawn(async {
        Err(Error::Encode(EncodeError::Backend(Failure::Injected(
            Operation::Children { height: 1 },
        ))))
    });

    // The protocol owns the claim table in production. Keeping it alive
    // ensures no stream-layer closure can accidentally win the error race.
    let _claims = claims;
    let result = run_to_quiescence(work.execute(future::pending::<Result<(), _>>()));
    let error = result
        .expect("a pump failure must terminate the work executor")
        .expect_err("the injected pump must fail");

    assert!(matches!(
        error,
        Error::Encode(EncodeError::Backend(Failure::Injected(
            Operation::Children { height: 1 },
        )))
    ));
}
