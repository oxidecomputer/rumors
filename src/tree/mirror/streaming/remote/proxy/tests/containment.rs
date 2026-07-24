//! Version-containment enforcement over the full wire stack.

use futures::join;

use crate::link::memory_with_capacity;
use crate::testing::run_to_quiescence;
use crate::tree::arb::uncontained_supply_pair;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::{
    Root as TreeRoot,
    mirror::{
        Error as MirrorError,
        streaming::{
            Local, Root,
            materialized::{Error as MaterializedError, Handshaking, Violation},
            mirror,
            remote::Handshaking as RemoteHandshaking,
        },
    },
};

use super::TRANSPORT_CAPACITY;
use super::harness::{LeftError, RightError};

/// Drive the two-proxy topology, returning each endpoint's result instead
/// of asserting success.
async fn reconcile_results(
    a: TreeRoot<()>,
    b: TreeRoot<()>,
) -> (
    Result<TreeRoot<()>, LeftError>,
    Result<TreeRoot<()>, RightError>,
) {
    let a = Handshaking::start(Local, Root::from(a)).window(WindowConfig::FLOOR);
    let b = Handshaking::start(Local, Root::from(b)).window(WindowConfig::FLOOR);

    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    let remote_b = RemoteHandshaking::start(Local, a_link).window(WindowConfig::FLOOR);
    let remote_a = RemoteHandshaking::start(Local, b_link).window(WindowConfig::FLOOR);

    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
    (
        a.map(|(root, _control)| root.into()),
        b.map(|(_control, root)| root.into()),
    )
}

/// A supplied leaf whose version escapes the sender's declared greeting
/// version fails the session with a typed violation after crossing a real
/// link.
///
/// The enforcement holds through the frame codec and supply decoder, not
/// only in process. The receiving endpoint reports the violation from its own materialized
/// participant in either endpoint position; the sender's endpoint is left
/// to whatever its aborted transport surfaces, which is not this
/// tripwire's concern. The in-process twin is
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
                Err(MirrorError::Client(MaterializedError::Violation(
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
                Err(MirrorError::Server(MaterializedError::Violation(
                    Violation::UncontainedSupply
                ))),
            ),
            "the rejection is position-independent",
        );
    }
}
