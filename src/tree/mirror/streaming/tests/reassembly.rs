//! What reassembly keeps of the nodes a session leaves unchanged.

use before::Party;
use proptest::collection::vec;
use proptest::prelude::*;

use super::LocalSession;
use crate::message::Message;
use crate::tree::arb::{arb_wide_divergent_pair, roots_at_depth};
use crate::tree::{Action, Root, Tree, typed::Path};

/// Replicas where one, `ahead`, has seen everything the other, `behind`,
/// has, returned as `(ahead, behind)`.
///
/// `behind` is `ahead` as it stood after a prefix of its actions. `ahead`
/// then inserts more and may redact any leaf it holds, so its set contains
/// `behind`'s when it redacted none of `behind`'s leaves, and its version
/// covers everything `behind` holds either way. With a drawn depth, both
/// sides' leaves are placed below a shared prefix of that many bytes
/// ([`roots_at_depth`]), so every scope the session explores sits on a
/// compressed path.
fn arb_contained_pair() -> impl Strategy<Value = (Root, Root)> {
    let placement = prop_oneof![Just(None), (0usize..31).prop_map(Some)];
    (0..40usize, 0..40usize, vec(any::<bool>(), 0..80), placement).prop_map(
        |(shared, later, redact, placement)| {
            let party = Party::seed();
            let mut tree = Tree::<()>::new();
            tree.act(
                &party,
                (0..shared).map(|_| Action::Insert(Message::new(()))),
            );
            let behind = tree.root.clone();
            tree.act(&party, (0..later).map(|_| Action::Insert(Message::new(()))));
            let keys: Vec<_> = tree.iter().map(|(v, _)| Path::for_leaf(v)).collect();
            tree.act(
                &party,
                keys.iter()
                    .zip(redact)
                    .filter_map(|(key, forget)| forget.then_some(Action::Forget(*key))),
            );
            match placement {
                None => (tree.root, behind),
                Some(depth) => {
                    let [ahead, behind] = roots_at_depth([tree.root, behind], depth);
                    (ahead, behind)
                }
            }
        },
    )
}

proptest! {
    /// A session leaves a side that learns nothing holding the very root it
    /// started with, whichever side initiates, at a budget of zero and at an
    /// unlimited one.
    ///
    /// The side whose version covers everything the other holds gains no
    /// leaf and loses none, so every scope it reassembles gets back the
    /// children it listed there. Reassembly must then return the node it
    /// held, scope by scope up to the root, keeping its hash and bounds;
    /// a rebuilt copy would have publication compute them again. The
    /// placed pairs put every explored scope on a compressed path, whose
    /// listed child is a peeled copy rather than a stored node.
    #[test]
    fn a_side_that_learns_nothing_keeps_its_root(
        (ahead, behind) in arb_contained_pair(),
        unlimited in any::<bool>(),
    ) {
        let budget = if unlimited { usize::MAX } else { 0 };
        for ahead_connects in [true, false] {
            let (client, server) = if ahead_connects {
                (ahead.clone(), behind.clone())
            } else {
                (behind.clone(), ahead.clone())
            };
            let (client, server) = LocalSession::new(client, server).budget(budget).run().sides();
            let reconciled = if ahead_connects { client } else { server };
            match (&ahead.node, &reconciled.node) {
                (Some(prior), Some(kept)) => prop_assert!(
                    kept.ptr_eq(prior),
                    "the side that learned nothing holds a rebuilt root",
                ),
                (None, None) => {}
                (prior, kept) => prop_assert!(
                    false,
                    "the side that learned nothing changed: {prior:?} became {kept:?}",
                ),
            }
        }
    }
}

/// Compute every memo of `root`'s tree, as publication does before a
/// replica serves it.
fn warmed(root: Root) -> Root {
    if let Some(node) = &root.node {
        node.hash();
        node.version_bytes();
    }
    root
}

proptest! {
    /// Every node of both reconciled trees holds the bounds and encoded
    /// version size a direct walk computes, whichever side initiates, at a
    /// budget of zero and at an unlimited one.
    ///
    /// Reassembly seeds a rebuilt node's bounds from the node it replaces,
    /// and the join oracle, which compares hashes, cannot see a wrong bound.
    /// The inputs are warm, as published trees are, so seeding has bounds
    /// to start from; the pairs redact, so rebuilt nodes lose children as
    /// well as gain them.
    #[test]
    fn reconciled_trees_hold_exact_memos(
        (a, b) in prop_oneof![arb_wide_divergent_pair(), arb_contained_pair()],
        unlimited in any::<bool>(),
    ) {
        let budget = if unlimited { usize::MAX } else { 0 };
        let (a, b) = (warmed(a), warmed(b));
        for (client, server) in [(a.clone(), b.clone()), (b, a)] {
            let (client, server) = LocalSession::new(client, server).budget(budget).run().sides();
            for side in [client, server] {
                let drifted = side.node.and_then(|node| node.into_untyped().mismemoized());
                prop_assert!(drifted.is_none(), "memos drifted: {drifted:?}");
            }
        }
    }
}
