//! Version-containment enforcement over the full wire stack.

use crate::testing::{memory_connection, run_to_quiescence};
use crate::tree::arb::uncontained_supply_pair;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::{
    Root as TreeRoot,
    mirror::streaming::materialized::{Error as MaterializedError, Violation},
};

use super::harness::{
    Backends, EndpointError, EndpointFailure, TRANSPORT_CAPACITY, Topology, codec, drive,
};

/// Drive the two-proxy topology in which the right endpoint's materialized
/// participant is the protocol server, returning each endpoint's result
/// instead of asserting success.
async fn reconcile_results(
    a: TreeRoot,
    b: TreeRoot,
) -> (
    Result<TreeRoot, EndpointFailure>,
    Result<TreeRoot, EndpointFailure>,
) {
    let (a_end, b_end) = memory_connection(TRANSPORT_CAPACITY);
    drive(
        Topology::RightProxyConnects,
        Backends::local(),
        a,
        b,
        (a_end.read, a_end.write),
        (b_end.read, b_end.write),
        codec::<()>(),
        WindowConfig::FLOOR,
    )
    .await
}

/// A supplied leaf whose version escapes the sender's declared greeting
/// version fails the session with a typed violation after crossing a real
/// pipe.
///
/// The enforcement holds through the frame codec and supply decoder, not
/// only in process. The receiving endpoint reports the violation from its
/// own materialized participant in either protocol position (the topology
/// makes the left one the client and the right one the server); the
/// sender's endpoint is left to whatever its aborted transport surfaces,
/// which is not this tripwire's concern. The in-process twin is
/// `uncontained_supply_is_rejected_by_streaming`.
#[test]
fn uncontained_supply_is_rejected_at_the_wire() {
    // Receiving side in the left endpoint position: its materialized
    // participant is the mirror's client.
    {
        let (receiver, poisoned, _, _) = uncontained_supply_pair();
        let (receiver_out, _poisoned_out) =
            run_to_quiescence(reconcile_results(receiver, poisoned))
                .expect("the rejecting session becomes quiescent");
        assert!(
            matches!(
                receiver_out,
                Err(EndpointError::Local(MaterializedError::Violation(
                    Violation::UncontainedSupply
                ))),
            ),
            "the receiving side rejects the escaped leaf over the wire",
        );
    }

    // Receiving side in the right endpoint position: its materialized
    // participant is the mirror's server.
    {
        let (receiver, poisoned, _, _) = uncontained_supply_pair();
        let (_poisoned_out, receiver_out) =
            run_to_quiescence(reconcile_results(poisoned, receiver))
                .expect("the rejecting session becomes quiescent");
        assert!(
            matches!(
                receiver_out,
                Err(EndpointError::Local(MaterializedError::Violation(
                    Violation::UncontainedSupply
                ))),
            ),
            "the rejection is position-independent",
        );
    }
}
