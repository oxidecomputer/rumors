use std::collections::VecDeque;
use std::convert::Infallible;

use futures::StreamExt;
use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::arb_tree_root;
use crate::tree::mirror::Error;
use crate::tree::mirror::streaming::Local;
use crate::tree::mirror::streaming::remote::Violation;
use crate::tree::typed::{Node, Path, Prefix, height};

use super::{Leaves, decode, encode};

/// A scripted leaf source: a queue of leaves, ending cleanly (the run's
/// terminator) or by truncation.
struct Feed {
    leaves: VecDeque<(Version, Message<()>)>,
    terminated: bool,
}

impl Leaves<()> for Feed {
    async fn next(&mut self) -> Result<Option<(Version, Message<()>)>, Violation> {
        match self.leaves.pop_front() {
            Some(leaf) => Ok(Some(leaf)),
            None if self.terminated => Ok(None),
            None => Err(Violation::Truncated),
        }
    }
}

/// Encode `node` as a whole tree root and collect its leaf run (the `Local`
/// backend cannot fail, so the stream is effectively infallible).
fn run(node: &Node<(), height::Root>) -> VecDeque<(Version, Message<()>)> {
    pollster::block_on(
        encode::<Local, (), height::Root>(Local, Prefix::new(), node.clone())
            .map(|leaf| leaf.unwrap_or_else(|error| match error {}))
            .collect(),
    )
}

/// What decoding through `Local` yields: the placed node, or the violation
/// or (uninhabited) backend fault that stopped it.
type Decoded<H> = Result<(Prefix<H>, Node<(), H>), Error<Violation, Infallible>>;

/// Decode one subtree at height `H` from `feed` through `Local`.
fn decoded<H: super::Convert>(feed: &mut Feed) -> Decoded<H> {
    pollster::block_on(decode(&Local, feed))
}

/// Project a decode failure onto the wire violation it must be.
fn violation(error: Error<Violation, Infallible>) -> Violation {
    match error {
        Error::Client(violation) => violation,
        Error::Server(never) => match never {},
    }
}

proptest! {
    /// Encoding a subtree's leaves and decoding them back reassembles the
    /// identical node at the identical place: the flat leaf run, with paths
    /// derived rather than transmitted, is a faithful transport.
    #[test]
    fn round_trips(root in arb_tree_root(0, 0..=64)) {
        if let Some(node) = root.root {
            let leaves = run(&node);
            prop_assert_eq!(leaves.len(), node.len(), "one item per leaf");
            let mut feed = Feed { leaves, terminated: true };
            let (prefix, rebuilt) =
                decoded::<height::Root>(&mut feed).expect("honest runs decode");
            prop_assert_eq!(prefix, Prefix::new());
            prop_assert_eq!(rebuilt, node);
            prop_assert!(
                feed.leaves.is_empty(),
                "decode consumed exactly one subtree",
            );
        }
    }

    /// A run that dies before its terminator is rejected as truncated, not
    /// misread as a smaller complete subtree.
    #[test]
    fn rejects_truncation(root in arb_tree_root(0, 1..=8)) {
        let node = root.root.expect("at least one leaf means a root exists");
        let mut feed = Feed { leaves: run(&node), terminated: false };
        prop_assert_eq!(
            violation(
                decoded::<height::Root>(&mut feed)
                    .map(|_| ())
                    .expect_err("truncated run must not decode"),
            ),
            Violation::Truncated,
        );
    }

    /// Derived paths that fail to strictly ascend — here, adjacent leaves
    /// swapped, and a duplicated leaf — are rejected before they can violate
    /// the assembly contract.
    #[test]
    fn rejects_disordered_leaves(root in arb_tree_root(0, 2..=16)) {
        let node = root.root.expect("two leaves mean a root exists");
        let mut swapped = run(&node);
        swapped.swap(0, 1);
        let mut feed = Feed { leaves: swapped, terminated: true };
        prop_assert_eq!(
            violation(
                decoded::<height::Root>(&mut feed)
                    .map(|_| ())
                    .expect_err("swapped leaves must not decode"),
            ),
            Violation::LeafOrder,
        );

        let mut duplicated = run(&node);
        let first = duplicated[0].clone();
        duplicated.push_front(first);
        let mut feed = Feed { leaves: duplicated, terminated: true };
        prop_assert_eq!(
            violation(
                decoded::<height::Root>(&mut feed)
                    .map(|_| ())
                    .expect_err("duplicated leaf must not decode"),
            ),
            Violation::LeafOrder,
        );
    }

    /// A leaf whose derived path escapes the subtree named by the run's
    /// first leaf is rejected: placement is self-certifying.
    #[test]
    fn rejects_misplaced_leaves(root in arb_tree_root(0, 2..=16)) {
        let node = root.root.expect("two leaves mean a root exists");
        let leaves = run(&node);
        // Two leaves already diverging in their first path byte cannot
        // belong to one under-root subtree. Random paths make adjacent
        // first-byte collisions common enough to matter, so scan for a
        // diverging pair rather than assuming the first two.
        let paths: Vec<[u8; 32]> = leaves
            .iter()
            .map(|(version, message)| Path::for_leaf(version, message.bytes()).into())
            .collect();
        let Some(at) = paths.windows(2).position(|pair| pair[0][0] != pair[1][0]) else {
            // All sampled leaves share a first byte; nothing to misplace.
            return Ok(());
        };
        let pair = VecDeque::from([leaves[at].clone(), leaves[at + 1].clone()]);
        let mut feed = Feed { leaves: pair, terminated: true };
        prop_assert_eq!(
            violation(
                decoded::<height::UnderRoot>(&mut feed)
                    .map(|_| ())
                    .expect_err("leaves of two subtrees must not decode as one"),
            ),
            Violation::Misplaced,
        );
    }
}

/// A run with no leaves at all is rejected rather than assembled into
/// nothing.
#[test]
fn rejects_empty_subtree() {
    let mut feed = Feed {
        leaves: VecDeque::new(),
        terminated: true,
    };
    assert_eq!(
        violation(
            decoded::<height::Root>(&mut feed)
                .map(|_| ())
                .expect_err("an empty run must not decode"),
        ),
        Violation::EmptySubtree,
    );
}
