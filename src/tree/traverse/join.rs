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
//! - **both have it, hashes differ**: explode both one level and merge-walk
//!   the two ascending radix fans in lockstep, recursing only into the
//!   radixes whose child subtrees differ — children equal by pointer or by
//!   content hash carry over verbatim through the shared structure — and
//!   reassembling with [`Node::branch`] (which re-compresses singletons and
//!   recomputes the joined branch version).
//!
//! The merge walk enumerates each *divergent* branch's full fan (≤ 256
//! entries) rather than only its changed radixes; equal subtrees still
//! prune by pointer-or-hash before any descent, so a small delta against a
//! large shared tree costs work proportional to the delta at the tree
//! level, with a per-divergent-node constant bounded by the fan.

use crate::Version;

use super::typed::*;
use super::unknown::Unknown;
use height::{Height, Root, S, Z};

/// Merges two trees rooted at `a` and `b` into one.
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

/// The inductive step of the merge, implemented per [`Height`]; see the
/// module docs for the four-case analysis each level performs.
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

                // Differing subtrees: descend one level, merge-walking the
                // two ascending radix fans in lockstep and recursing only
                // into the radixes that actually diverge. A child equal on
                // both sides — by `Node`'s `ptr_eq`-or-hash equality, the
                // same short-circuit the node-level check above uses —
                // carries over verbatim: nothing is learned across an equal
                // subtree. One-sided radixes recurse too: the asymmetric
                // arms above filter them against the absent side's version,
                // which is where deletion honoring drops what that side
                // redacted.
                //
                // The walk reads both fans directly rather than diffing the
                // two maps against each other: the merged map starts from
                // *ours* and only divergent radixes are rewritten, so every
                // shared child persists by structural sharing.
                let ours = ours.into_children();
                let theirs = theirs.into_children();

                let mut merged = ours.clone();
                let mut ours = ours.iter().peekable();
                let mut theirs = theirs.iter().peekable();
                loop {
                    let (radix, our_child, their_child) = match (ours.peek(), theirs.peek()) {
                        (None, None) => break,
                        (Some((radix, _)), None) => {
                            (*radix, ours.next().map(|(_, child)| child), None)
                        }
                        (None, Some((radix, _))) => {
                            (*radix, None, theirs.next().map(|(_, child)| child))
                        }
                        (Some((ours_radix, _)), Some((theirs_radix, _))) => {
                            let radix = (*ours_radix).min(*theirs_radix);
                            (
                                radix,
                                ours.next_if(|(r, _)| *r == radix).map(|(_, child)| child),
                                theirs.next_if(|(r, _)| *r == radix).map(|(_, child)| child),
                            )
                        }
                    };

                    if let (Some(our_child), Some(their_child)) = (&our_child, &their_child)
                        && our_child == their_child
                    {
                        continue;
                    }

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
