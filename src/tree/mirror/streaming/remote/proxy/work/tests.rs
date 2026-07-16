use std::future;

use tokio::io::{duplex, split};

use super::Work;
use crate::tree::mirror::streaming::{
    Failing, Failure, Local, Operation,
    remote::{adapter::EncodeError, codec::Speaker, proxy::Error, session::Drivers},
    testing::run_to_quiescence,
};

/// A pump error cancels parked peers and retains its original error identity.
#[test]
fn pump_failure_preempts_parked_pumps() {
    let (transport, _peer) = duplex(1);
    let (read, write) = split(transport);
    let (drivers, incoming, outgoing): (Drivers<_, _, ()>, _, _) =
        Drivers::new(Speaker::Initiator, read, write);
    let mut work = Work::new(Failing::after(Local, usize::MAX), drivers);

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

    // The protocol owns these endpoints in production. Keeping them alive
    // ensures no transport closure can accidentally win the error race.
    let _incoming = incoming;
    let _outgoing = outgoing;
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
