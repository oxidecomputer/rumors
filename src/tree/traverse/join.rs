//! A direct, in-memory merge of two trees by a single simultaneous recursion
//! over both, inductive over the height.
//!
//! This is the local-only counterpart to the [`mirror`](super::mirror)
//! protocol: where the mirror reconciles two replicas by exchanging messages
//! (and so must serialize, run a zipper, and build the union on both sides),
//! `join` walks the two trees in lockstep in one process and builds the
//! merged union once. It is observationally identical to mirroring two local
//! trees, producing the same merged [`Root`](crate::tree::Root), because it
//! delegates all version filtering to the same [`Unknown`] traversal the
//! mirror uses.
//!
//! For each pair of nodes at a path the recursion distinguishes four cases:
//!
//! - **neither side has it**: nothing.
//! - **only one side has it**: hand the whole subtree to [`Unknown::unknown`],
//!   filtered against the *other* side's version vector. Survivors are the
//!   subtree the other side learns; anything causally `<=` the other side's
//!   version was deleted there (the version vector is the entire deletion
//!   mechanism; there are no tombstones) and is dropped.
//! - **both have it, hashes equal**: the subtrees are identical (content
//!   addressing makes equal hash ⟹ equal content, versions included), so keep
//!   one verbatim.
//! - **both have it, hashes differ**: explode both one level and recurse only
//!   into the radixes whose child subtrees differ (an [`OrdMap::diff`] that
//!   prunes the shared ones by pointer), reassembling with [`Node::branch`]
//!   (which re-compresses singletons and recomputes the joined branch version).
//!
//! [`OrdMap::diff`]: imbl::OrdMap::diff

use crate::version::Version;

use super::typed::*;
use super::unknown::Unknown;
use height::{Height, Root, S, Z};

/// Merge two trees rooted at `a` and `b` into one.
///
/// `a_version` / `b_version` are the two roots' version vectors, used to honor
/// deletions (a node one side lacks while its version is `<=` that side's vector
/// was deleted there, and is dropped).
pub fn join<T>(
    a: Option<Node<T, Root>>,
    b: Option<Node<T, Root>>,
    a_version: &Version,
    b_version: &Version,
) -> Option<Node<T, Root>>
where
    T: Send + Sync,
{
    Join::join(a, b, a_version, b_version)
}

pub trait Join: Unknown {
    fn join<T>(
        a: Option<Node<T, Self>>,
        b: Option<Node<T, Self>>,
        a_version: &Version,
        b_version: &Version,
    ) -> Option<Node<T, Self>>
    where
        T: Send + Sync;
}

impl<H: Join> Join for S<H>
where
    S<H>: Height + Unknown,
{
    fn join<T>(
        a: Option<Node<T, S<H>>>,
        b: Option<Node<T, S<H>>>,
        a_version: &Version,
        b_version: &Version,
    ) -> Option<Node<T, S<H>>>
    where
        T: Send + Sync,
    {
        match (a, b) {
            (None, None) => None,
            // Asymmetric cases: a subtree one side holds and the other lacks.
            // Filter it against the *other* side's version vector to honor
            // deletions: causally-known subtrees the other side lacks were
            // deleted there, and drop out.
            (Some(ours), None) => Unknown::unknown(Some(ours), b_version),
            (None, Some(theirs)) => Unknown::unknown(Some(theirs), a_version),
            (Some(ours), Some(theirs)) => {
                // Identical subtrees: keep one. Equality short-circuits on
                // shared backing (the common case for forked trees, hash-free)
                // and otherwise on the content hash ⟹ equal content (content
                // addressing). Either way there is nothing to learn on either
                // side.
                if ours == theirs {
                    return Some(ours);
                }

                // Differing subtrees: descend one level, but only into the
                // radixes that actually diverge. `OrdMap::diff` walks both
                // persistent B-trees in lockstep and prunes whole spans that are
                // pointer-equal — the shared backing a fork leaves behind — so it
                // yields exactly the changed children, in ascending-radix order,
                // without enumerating the full radix union or probing the
                // unchanged children. A small delta against a large shared tree
                // therefore costs work proportional to the delta, not to the
                // fan-out. (`diff` classifies a child as unchanged via `Node`'s
                // `PartialEq`, which is the same `ptr_eq`-or-hash short-circuit
                // the node-level equality above uses: nothing is learned across
                // an equal subtree, so it carries over verbatim.)
                //
                // Collect the divergent radixes first — cloning only those few
                // children — so we don't hold `diff`'s borrow of `ours` /
                // `theirs` across the recursive rewrite of the merged map.
                let ours = ours.into_children();
                let theirs = theirs.into_children();

                // Start the merged map from *ours* (moved — `diff`'s borrow has
                // ended) and rewrite only the divergent radixes; every shared
                // child carries over verbatim by structural sharing.
                let mut merged = ours.clone();
                for (radix, our_child, their_child) in ours.diff_owned(&theirs) {
                    match Join::join(our_child, their_child, a_version, b_version) {
                        Some(child) => {
                            merged.insert(radix, child);
                        }
                        None => {
                            merged.remove(&radix);
                        }
                    }
                }

                Node::branch(merged)
            }
        }
    }
}

impl Join for Z {
    fn join<T>(
        a: Option<Node<T, Z>>,
        b: Option<Node<T, Z>>,
        a_version: &Version,
        b_version: &Version,
    ) -> Option<Node<T, Z>>
    where
        T: Send + Sync,
    {
        match (a, b) {
            (None, None) => None,
            (Some(ours), None) => Unknown::unknown(Some(ours), b_version),
            (None, Some(theirs)) => Unknown::unknown(Some(theirs), a_version),
            // Two leaves at the same path are the same leaf: the path is the
            // content-addressed hash of (version, value) (see
            // `Path::for_leaf`), so identical paths carry identical contents.
            // Keep one.
            (Some(ours), Some(_)) => Some(ours),
        }
    }
}

#[cfg(test)]
mod tests;
