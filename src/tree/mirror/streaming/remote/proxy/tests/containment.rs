//! Version-containment enforcement over the full wire stack.

use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::network::NetworkKeys;
use crate::testing::{memory_connection, run_to_quiescence};
use crate::tree::arb::{nth_party, uncontained_supply_pair};
use crate::tree::typed::Path;
use crate::tree::{
    Action, Root as TreeRoot, Tree,
    mirror::streaming::remote::{Error, adapter::DecodeError},
};

use super::harness::{
    Backends, EndpointError, EndpointFailure, GreetingRewrite, TRANSPORT_CAPACITY, Topology, codec,
    disjoint_pair, drive, join_oracle, reconcile_rewritten_greetings,
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
        0,
    )
    .await
}

/// A supplied leaf whose version escapes the sender's declared greeting
/// version fails the session at the receiver's decoder, as
/// `UncontainedSupply` naming the escaped leaf, in either protocol
/// position.
///
/// The receiving endpoint reports the violation from its proxy, whose
/// decoder reads the escaped record, whether its materialized participant
/// is the client (left) or the server (right). The sender's endpoint is
/// left to whatever its aborted transport surfaces.
#[test]
fn uncontained_supply_is_rejected_at_the_wire() {
    for receiver_left in [true, false] {
        let (receiver, poisoned, escaped, _) = uncontained_supply_pair();
        let (left, right) = if receiver_left {
            (receiver, poisoned)
        } else {
            (poisoned, receiver)
        };
        let (left_out, right_out) = run_to_quiescence(reconcile_results(left, right))
            .expect("the rejecting session becomes quiescent");
        let receiver_out = if receiver_left { left_out } else { right_out };
        let path = <[u8; 32]>::from(escaped);
        assert!(
            matches!(
                &receiver_out,
                Err(EndpointError::Proxy(Error::Decode(DecodeError::UncontainedSupply {
                    path: actual,
                }))) if *actual == path
            ),
            "the receiver rejects the escaped leaf (receiver_left={receiver_left}): \
             {receiver_out:?}",
        );
    }
}

proptest! {
    /// A side that reads its peer's greeting version as the empty version
    /// rejects the peer's first supplied leaf as `UncontainedSupply`,
    /// naming one of the peer's leaves, in either endpoint position.
    ///
    /// The rewrite models a peer that understates its version: every leaf
    /// it holds lies outside the empty version, so its first conforming
    /// supply breaks the declaration. The replicas hold disjoint sets of
    /// different sizes, so each holds leaves the other lacks, and both
    /// sides elect the same initiator whatever version either reads.
    #[test]
    fn a_shrunken_version_declaration_is_rejected(
        deceived_messages in 1usize..=4,
        extra in 1usize..=3,
        deceived_larger in any::<bool>(),
        deceived_left in any::<bool>(),
    ) {
        let (deceived_count, peer_count) = if deceived_larger {
            (deceived_messages + extra, deceived_messages)
        } else {
            (deceived_messages, deceived_messages + extra)
        };
        let (deceived, peer) = disjoint_pair(deceived_count, peer_count);
        let peer_paths: Vec<[u8; 32]> = Tree::<()>::from_root(NetworkKeys::fixture(), peer.clone())
            .iter()
            .map(|(version, _)| <[u8; 32]>::from(Path::for_fixture_leaf(version)))
            .collect();

        let rewrite = GreetingRewrite::version(Version::new());
        let (left, right, left_hears, right_hears) = if deceived_left {
            (deceived, peer, Some(rewrite.clone()), None)
        } else {
            (peer, deceived, None, Some(rewrite.clone()))
        };
        let (left_out, right_out) =
            run_to_quiescence(reconcile_rewritten_greetings(left, right, left_hears, right_hears))
                .expect("the rejecting session becomes quiescent");
        prop_assert!(rewrite.fired(), "the peer's version was not rewritten");
        let deceived_out = if deceived_left { left_out } else { right_out };
        prop_assert!(
            matches!(
                &deceived_out,
                Err(EndpointError::Proxy(Error::Decode(DecodeError::UncontainedSupply { path })))
                    if peer_paths.contains(path)
            ),
            "the deceived side rejects the peer's supply: {:?}",
            deceived_out,
        );
    }
}

/// A `(receiver, sender, redacted)` triple: the replicas share `shared`
/// leaves inserted on one party, of which the receiver then redacted the
/// one at `choice`, which the sender still holds.
///
/// The receiver holds one leaf fewer, so it initiates in both sides'
/// elections whatever version either side reads.
fn redacted_pair(shared: usize, choice: prop::sample::Index) -> (TreeRoot, TreeRoot, Version) {
    let mut sender = Tree::<()>::default();
    sender.act(
        &nth_party(0),
        (0..shared).map(|_| Action::Insert(Message::new(()))),
    );
    let versions: Vec<Version> = sender.iter().map(|(version, _)| version.clone()).collect();
    let redacted = versions[choice.index(versions.len())].clone();
    let mut receiver = sender.clone();
    receiver.act(&nth_party(1), [Action::Forget(redacted.clone())]);
    (receiver.root, sender.root, redacted)
}

proptest! {
    /// A message the receiver redacted stays redacted when its peer
    /// supplies it again: the receiver's decoder rejects the supply as
    /// `SeenSupply`, naming the redacted leaf, in either endpoint position.
    ///
    /// Reconciled as declared, the pair converges on the join, which lacks
    /// the redacted leaf: the sender's filter drops it, because the
    /// receiver's version contains it. The sender is then made
    /// non-conforming by rewriting the receiver's greeting version, as the
    /// sender reads it, to the empty version. Its filter then keeps the
    /// redacted leaf, the one leaf the receiver lacks, and supplies it.
    #[test]
    fn a_redacted_message_supplied_again_is_rejected(
        shared in 1usize..=6,
        choice in any::<prop::sample::Index>(),
        receiver_left in any::<bool>(),
    ) {
        let (receiver, sender, redacted) = redacted_pair(shared, choice);
        let (left, right) = if receiver_left {
            (receiver.clone(), sender.clone())
        } else {
            (sender.clone(), receiver.clone())
        };

        let expected = join_oracle(&left, &right);
        let (left_out, right_out) =
            run_to_quiescence(reconcile_rewritten_greetings(left.clone(), right.clone(), None, None))
                .expect("the conforming session becomes quiescent");
        for out in [left_out, right_out] {
            let out = out.expect("the conforming session reconciles");
            let tree = Tree::<()>::from_root(NetworkKeys::fixture(), out.clone());
            prop_assert!(tree.get(&redacted).is_none(), "the redaction reaches both sides");
            prop_assert_eq!(out, expected.clone());
        }

        let rewrite = GreetingRewrite::version(Version::new());
        let sender_hears = Some(rewrite.clone());
        let (left_hears, right_hears) = if receiver_left {
            (None, sender_hears)
        } else {
            (sender_hears, None)
        };
        let (left_out, right_out) =
            run_to_quiescence(reconcile_rewritten_greetings(left, right, left_hears, right_hears))
                .expect("the rejecting session becomes quiescent");
        prop_assert!(rewrite.fired(), "the receiver's version was not rewritten");
        let receiver_out = if receiver_left { left_out } else { right_out };
        let path = <[u8; 32]>::from(Path::for_fixture_leaf(&redacted));
        prop_assert!(
            matches!(
                &receiver_out,
                Err(EndpointError::Proxy(Error::Decode(DecodeError::SeenSupply { path: actual })))
                    if *actual == path
            ),
            "the receiver rejects the redacted leaf: {:?}",
            receiver_out,
        );
    }
}
