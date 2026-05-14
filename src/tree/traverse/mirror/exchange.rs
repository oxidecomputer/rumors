use std::convert::Infallible;

use imbl::{OrdMap, OrdSet};
use itertools::{EitherOrBoth, Itertools};

use crate::{
    Key, Message, Version,
    tree::{
        traverse::unknown::Unknown,
        typed::{
            Levels, Node, Prefix,
            height::{Height, Pred, Root, S, Z},
            levels::{Below, Top},
        },
    },
};

use super::message;

/// The height just under the root, i.e. 31.
type UnderRoot = <Root as Pred>::Pred;

/// An in-progress mirror synchronization.
pub struct Exchange<'v, P, F, L>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    levels: L,
    other_version: &'v Version<P>,
    with_message: F,
}

/// The initiator's start of the protocol.
pub fn initiator<P, F, T>(
    node: Option<Node<P, T, Root>>,
    other_version: &Version<P>,
    with_message: F,
) -> (message::Start, Exchange<'_, P, F, Top<P, T>>)
where
    F: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    (
        message::Start {
            root: Node::root_hash(&node),
        },
        Exchange {
            levels: Node::levels(node),
            other_version,
            with_message,
        },
    )
}

/// The responder's start of the protocol.
pub fn responder<P, T, F>(
    node: Option<Node<P, T, Root>>,
    other_version: &Version<P>,
    with_message: F,
    message::Start { root }: message::Start,
) -> (
    message::Exchange<P, T, UnderRoot>,
    Result<Exchange<'_, P, F, Below<P, T, UnderRoot, Top<P, T>>>, Option<Node<P, T, Root>>>,
)
where
    F: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    // If the initiator has the same root hash, our trees must be equal, so
    // there's no need to continue the protocol; however, we need to signal back
    // to the initiator that we're finished.
    if root == Node::root_hash(&node) {
        return (message::Exchange::default(), Err(node));
    }

    // If the root hashes mismatch, we need to explode out the root node into
    // the level below it and send those hashes back to the initiator.
    let levels = Node::levels(None).down(
        node.map(|n| {
            n.into_children()
                .into_iter()
                .map(|(radix, child)| (Prefix::new().push(radix), child))
                .collect()
        })
        .unwrap_or_default(),
    );
    (
        message::Exchange {
            providing: Default::default(), // Can't have been asked for anything yet
            requested: Default::default(), // Can't have learned about an unknown child yet
            // We're uncertain about all the children of the root node, which
            // are all at this level:
            uncertain: levels
                .level()
                .into_iter()
                .map(|(prefix, child)| (prefix.clone(), child.hash()))
                .collect(),
        },
        Ok(Exchange {
            levels,
            other_version,
            with_message,
        }),
    )
}

impl<'v, P, F, L> Exchange<'v, P, F, L>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    /// The symmetric middle of the protocol.
    pub fn exchange<H, T>(
        mut self,
        message::Exchange {
            providing,
            requested,
            uncertain,
        }: message::Exchange<P, T, S<H>>,
    ) -> (
        message::Exchange<P, T, H>,
        Result<Exchange<'v, P, F, Below<P, T, H, Below<P, T, S<H>, L>>>, Option<Node<P, T, Root>>>,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown,
        T: Clone,
    {
        // The current level of our traversal through the tree:
        let level = self.levels.level_mut();

        // 1. Insert all the provided previously-unknown nodes into our level.
        for (prefix, node) in providing {
            level.insert(prefix, node);
        }

        // 2. Look up all the requested prefixes and split them into all their
        // children (filtered by those children being unknown relative to the
        // other's version), to slot into the other's tree.
        let mut providing = OrdMap::default();
        for prefix in requested {
            if let Some(node) = level.remove(&prefix) {
                // Filter children by whether they are unknown to the other
                // party: we should take note of this filtration locally as
                // well, because the other party lacking a node means that
                // anything strictly causally prior to their version which we
                // still have was deleted already on their side.
                if let Some(node) = Unknown::unknown(
                    Some(node),
                    prefix.clone(),
                    &self.other_version,
                    &mut self.with_message,
                ) {
                    // Re-insert the filtered node back into the level.
                    level.insert(prefix.clone(), node.clone());

                    // Explode the node into its children and return them to the
                    // other party as requested.
                    for (radix, child) in node.into_children() {
                        providing.insert(prefix.clone().push(radix), child);
                    }
                }
            } else {
                // The counterparty should only request prefixes that we
                // provided to it; if not, either the counterparty is
                // misbehaving, or we are.
                #[cfg(debug_assertions)]
                panic!("counterparty requested unknown prefix {:?}", prefix);
            }
        }

        // 3. Partition the uncertain hashes into:
        //
        //   (a) those whose prefixes are unknown to us (request entire subtree),
        //   (b) those whose prefixes are known and hashes match (don't touch), and
        //   (c) those whose prefixes are known and hashes don't match (uncertain).
        //
        // For nodes about which we are uncertain (i.e. nodes which we've
        // learned are *not* on the disjoint or matching frontier), we send all
        // the children hashes to the other party so they can make the same
        // determination, two levels down from their previous position.
        let mut requested = OrdSet::default();
        let mut below = OrdMap::default();
        let mut below_below = OrdMap::default();

        // Chunk the uncertain prefixes by which parent they belong to, and pull
        // the parent for modification only once per parent prefix:
        for (parent_prefix, chunk) in uncertain
            .into_iter()
            .map(|(prefix, hash)| {
                let (parent_prefix, radix) = prefix.pop();
                (parent_prefix, radix, hash)
            })
            .chunk_by(|(parent_prefix, _, _)| parent_prefix.clone())
            .into_iter()
        {
            // For each parent, process the uncertain prefixes which pass through it:
            if let Some(parent) = level.remove(&parent_prefix) {
                // Across the intersection of the uncertain hashes and the
                // children of the parent:
                for point in parent
                    .into_children()
                    .into_iter()
                    .merge_join_by(chunk, |(child_radix, _), (_, hash_radix, _)| {
                        child_radix.cmp(hash_radix)
                    })
                {
                    use EitherOrBoth::*;
                    match point {
                        Left((child_radix, child)) => {
                            // We have a child the counterparty did not list as
                            // uncertain: they cannot have it, since they
                            // enumerate all of their children's hashes when
                            // they are uncertain about a parent. Filter the
                            // node against their version to honor any
                            // deletions, then send the surviving subtree to
                            // them via `providing` (and retain a copy below).
                            let child_prefix = parent_prefix.clone().push(child_radix);
                            if let Some(child) = Unknown::unknown(
                                Some(child),
                                child_prefix.clone(),
                                self.other_version,
                                &mut self.with_message,
                            ) {
                                providing.insert(child_prefix.clone(), child.clone());
                                below.insert(child_prefix, child);
                            }
                        }
                        Both((child_radix, child), (parent_prefix, _, hash)) => {
                            let child_prefix = parent_prefix.push(child_radix);
                            if child.hash() == hash {
                                // The hashes match so there's no more work to
                                // do; keep this child unmodified
                                below.insert(child_prefix, child);
                            } else {
                                // The hashes don't match, so we need to further
                                // process this child; insert its children at
                                // the absolute bottom-most level
                                for (grandchild_radix, grandchild) in child.into_children() {
                                    let grandchild_prefix =
                                        child_prefix.clone().push(grandchild_radix);
                                    below_below.insert(grandchild_prefix, grandchild);
                                }
                            }
                        }
                        Right((parent_prefix, hash_radix, _)) => {
                            // The counterparty was uncertain about this child,
                            // but we don't have it at all; this means we need
                            // to request it from them
                            requested.insert(parent_prefix.push(hash_radix));
                        }
                    }
                }
            } else {
                // We don't have the parent at all. This is only reachable when
                // the counterparty enumerated children of a prefix without
                // knowing whether we had it -- which, in a correct protocol
                // run, only happens on the responder's opening message (where
                // it lists every child of its root unconditionally). The
                // initiator processes that message as its very first
                // `exchange`, at which point its level is still at `Top`
                // (`Height = Root`); any other call site reaching this branch
                // indicates a misbehaving counterparty or a protocol bug.
                debug_assert_eq!(
                    <L::Height as Height>::HEIGHT,
                    <Root as Height>::HEIGHT,
                    "counterparty indicated uncertainty about unknown parent \
                    prefix {:?} outside of the initiator's first round",
                    parent_prefix,
                );
                for (parent, hash_radix, _) in chunk {
                    requested.insert(parent.push(hash_radix));
                }
            }
        }

        // The new bottom of our level is two below where we started:
        let levels = self.levels.down(below).down(below_below);
        let uncertain: OrdMap<_, _> = levels
            .level()
            .iter()
            .map(|(prefix, node)| (prefix.clone(), node.hash()))
            .collect();

        // Determine whether we're finished, by examining our situation
        let next = if uncertain.is_empty() && requested.is_empty() {
            // If we are no longer uncertain about any children and we have not
            // requested any that we're aware we're lacking, then we must be
            // finished. At this point, the bottom-most level we just created
            // should by definition be empty (but it is not the case conversely
            // that if the level is empty, we're finished -- consider the
            // situation when uncertain is empty but requested is not!).
            Err(levels.collapse())
        } else {
            // Otherwise, we should continue onwards!
            Ok(Exchange {
                levels,
                with_message: self.with_message,
                other_version: self.other_version,
            })
        };

        // The message we send to our counterparty (unconditionally regardless
        // of whether we're ourselves finished internally):
        let message = message::Exchange {
            providing,
            requested,
            uncertain,
        };

        (message, next)
    }

    /// The responder's end of the protocol.
    pub fn complete_responder<T>(
        mut self,
        message::Exchange {
            providing,
            requested,
            uncertain,
        }: message::Exchange<P, T, Z>,
    ) -> (
        message::Complete<P, T>,
        Result<Infallible, Option<Node<P, T, Root>>>,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = S<Z>>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        // At the final stage, the initiator cannot be uncertain about a leaf:
        // either it has it (in which case the hash is the same: 0xff...), or it
        // doesn't (in which case it should be requested, not uncertain). No
        // well-behaved initiator will ever specify a non-empty uncertain.
        debug_assert!(
            uncertain.is_empty(),
            "initiator supplied non-empty uncertain set: {:?}",
            uncertain
        );

        // The current level of our traversal through the tree:
        let level = self.levels.level_mut();

        // 1. Insert all the provided previously-unknown nodes into our level.
        for (prefix, node) in providing {
            level.insert(prefix, node);
        }

        // 2. Look up all the requested prefixes and split them into all their
        // children (filtered by those children being unknown relative to the
        // other's version), to slot into the other's tree.
        let mut providing = OrdMap::default();
        for prefix in requested {
            if let Some(node) = level.remove(&prefix) {
                // Filter children by whether they are unknown to the other
                // party: we should take note of this filtration locally as
                // well, because the other party lacking a node means that
                // anything strictly causally prior to their version which we
                // still have was deleted already on their side.
                if let Some(node) = Unknown::unknown(
                    Some(node),
                    prefix.clone(),
                    &self.other_version,
                    &mut self.with_message,
                ) {
                    // Re-insert the filtered node back into the level.
                    level.insert(prefix.clone(), node.clone());

                    // Explode the node into its children and return them to the
                    // other party as requested.
                    for (radix, child) in node.into_children() {
                        providing.insert(prefix.clone().push(radix), child);
                    }
                }
            } else {
                // The counterparty should only request prefixes that we
                // provided to it; if not, either the counterparty is
                // misbehaving, or we are.
                #[cfg(debug_assertions)]
                panic!("counterparty requested unknown prefix {:?}", prefix);
            }
        }

        (message::Complete { providing }, Err(self.levels.collapse()))
    }

    /// The initiator's end of the protocol.
    pub fn complete_initiator<T>(
        mut self,
        message::Complete { providing }: message::Complete<P, T>,
    ) -> Result<Infallible, Option<Node<P, T, Root>>>
    where
        L: Levels<P, T, Height = Z>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        // The current level of our traversal through the tree:
        let level = self.levels.level_mut();

        // 1. Insert all the provided previously-unknown nodes into our level.
        for (prefix, node) in providing {
            level.insert(prefix, node);
        }

        Err(self.levels.collapse())
    }
}
