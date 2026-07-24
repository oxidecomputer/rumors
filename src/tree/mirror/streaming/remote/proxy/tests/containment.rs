//! Version-containment ingestion behavior over the full wire stack.

use futures::join;

use crate::link::memory_with_capacity;
use crate::testing::run_to_quiescence;
use crate::tree::arb::uncontained_supply_pair;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::{
    Root as TreeRoot,
    mirror::streaming::{
        Local, Root,
        materialized::Handshaking,
        mirror,
        remote::Handshaking as RemoteHandshaking,
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

/// Pins the wire half of the streaming ingestion hole this branch closes: a
/// supplied leaf whose version escapes the sender's declared greeting
/// version decodes and is absorbed on the receiving side.
///
/// The frame codec and the supply decoder validate leaf scope and ordering
/// but never version containment against the greeting, so the escaped leaf
/// crosses a real link intact; the in-process twin
/// (`uncontained_supply_is_absorbed_by_streaming`) pins the same absorption
/// without the wire, and the alternating oracle's twin pins the downstream
/// immortality legs.
#[test]
fn uncontained_supply_crosses_the_wire() {
    let (victim, poisoned, path, _escaped) = uncontained_supply_pair();
    let key = crate::tree::Key::from(path);
    let (victim_out, poisoned_out) = run_to_quiescence(reconcile_results(victim, poisoned))
        .expect("the session becomes quiescent");
    let victim_out = victim_out.expect("the victim's session completes");
    let _ = poisoned_out.expect("the sender's session completes");
    assert!(
        victim_out
            .root
            .as_ref()
            .is_some_and(|root| root.get(key.as_bytes()).is_some()),
        "the escaped leaf decodes and is absorbed over the wire",
    );
}
