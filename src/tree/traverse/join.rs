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
//! - **both have it, hashes equal**: the subtrees hold the same version
//!   set (hashes commit shape and versions), hence the same messages;
//!   keep one verbatim.
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

/// Crate-internal: two live leaves met at one tree path while disagreeing
/// on version or payload.
///
/// Never user-visible, because no input can produce it: paths are
/// full-width hashes of versions, live leaf versions never exceed the
/// ceiling a fresh tick strictly dominates, and ingestion enforces
/// containment — so a collision requires a bug in this crate or a
/// full-width hash collision (off-model). The traversals return it as a
/// typed error so tests can construct and observe the detector directly;
/// the public seams (`Batch`'s drop commit, the gossip commit) `expect` it
/// away as the invariant breach it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("two distinct leaves collided at one tree path: a version was reused")]
pub struct LeafCollision {
    /// The 32-byte version-derived path naming one colliding leaf: the
    /// resident leaf's in the merge walk, the incoming insert's in the
    /// apply walk.
    pub path: [u8; 32],
}

/// Merges two trees rooted at `a` and `b` into one.
///
/// `a_version` / `b_version` are the two roots' version vectors, used to honor
/// deletions (a node one side lacks while its version is `<=` that side's vector
/// was deleted there, and is dropped).
///
/// `changed` is set — never cleared — iff the merged result's content differs
/// from `a`'s: some leaf was gained from `b`, or some leaf of `a` was dropped
/// by deletion honoring. The recursion decides this exactly, with no hashing:
/// a gain is a subtree of `b` surviving the deletion filter where `a` held
/// nothing, and a drop moves a node's exact memoized leaf count. Gains and
/// drops live at distinct version-addressed paths and each is monotone at its
/// path, so they cannot cancel: an untouched flag really means the merged
/// tree is `a`, content-identical, equal root hash.
///
/// # Errors
///
/// [`LeafCollision`] if two leaves meet at one path while disagreeing on
/// version or payload (unreachable from any input; see [`LeafCollision`]).
/// On `Err`, `changed` may have been set but nothing has been published:
/// the caller's commit point is never reached.
pub fn join<T>(
    a: Option<Node<T, Root>>,
    b: Option<Node<T, Root>>,
    a_version: &Version,
    b_version: &Version,
    changed: &mut bool,
) -> Result<Option<Node<T, Root>>, LeafCollision>
where
    T: Send + Sync,
{
    // Test-only unwind source for the panic-atomicity pin: the merge walk
    // is the fallible region of `Tree::join`'s commit section, and its
    // entry burns the first fuse step (each branch-level step below burns
    // one more).
    #[cfg(test)]
    crate::tree::panic_injection::fire_if_armed();

    Join::join(a, b, a_version, b_version, changed)
}

/// The inductive step of the merge, implemented per [`Height`]; see the
/// module docs for the four-case analysis each level performs.
///
/// Each step upholds the [`join`] free function's `changed` contract: set
/// on any gain from `b` or any deletion-honoring drop from `a`, left
/// alone when the result is content-identical to `a`.
pub trait Join: Unknown {
    fn join<T>(
        a: Option<Node<T, Self>>,
        b: Option<Node<T, Self>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> Result<Option<Node<T, Self>>, LeafCollision>
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
        changed: &mut bool,
    ) -> Result<Option<Node<T, S<H>>>, LeafCollision>
    where
        T: Send + Sync,
    {
        // Test-only unwind source, continued: every branch-level merge step
        // burns one fuse step, so a fuse armed past the entry unwinds only
        // after earlier steps completed real merge work.
        #[cfg(test)]
        crate::tree::panic_injection::fire_if_armed();

        Ok(match (a, b) {
            (None, None) => None,
            // Asymmetric cases: a subtree one side holds and the other lacks.
            // Filter it against the *other* side's version vector to honor
            // deletions: causally-known subtrees the other side lacks were
            // deleted there, and drop out.
            //
            // On our side, the filter only ever *removes* leaves, so its
            // memoized leaf count is an exact change detector: the count
            // moved iff some leaf of ours was dropped. On their side, any
            // survivor at all is a gain (we held nothing here).
            (Some(ours), None) => {
                let leaves = ours.len();
                let kept = Unknown::unknown(Some(ours), b_version);
                *changed |= kept.as_ref().map_or(0, Node::len) != leaves;
                kept
            }
            (None, Some(theirs)) => {
                let gained = Unknown::unknown(Some(theirs), a_version);
                *changed |= gained.is_some();
                gained
            }
            (Some(ours), Some(theirs)) => {
                // Identical subtrees: keep one. Equality short-circuits on
                // shared backing (the common case for forked trees,
                // hash-free), else on the Merkle hash — same version set, hence
                // same messages: nothing to learn on either side.
                if ours == theirs {
                    return Ok(Some(ours));
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

                    match Join::join(our_child, their_child, a_version, b_version, changed)? {
                        Some(child) => {
                            merged.insert(radix, child);
                        }
                        None => {
                            merged.remove(radix);
                        }
                    }
                }

                Node::branch(merged)
            }
        })
    }
}

impl Join for Z {
    fn join<T>(
        a: Option<Node<T, Z>>,
        b: Option<Node<T, Z>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> Result<Option<Node<T, Z>>, LeafCollision>
    where
        T: Send + Sync,
    {
        Ok(match (a, b) {
            (None, None) => None,
            // The leaf-level base of the asymmetric arms' change detection:
            // our leaf dropped by deletion honoring is a change, and their
            // leaf surviving the filter is a gain.
            (Some(ours), None) => {
                let kept = Unknown::unknown(Some(ours), b_version);
                *changed |= kept.is_none();
                kept
            }
            (None, Some(theirs)) => {
                let gained = Unknown::unknown(Some(theirs), a_version);
                *changed |= gained.is_some();
                gained
            }
            // Two leaves at one path are the same leaf: the path
            // is the full-width hash of the version (`Path::for_leaf`), so
            // one path is one version, and one version is one message.
            // Verify both legs instead of assuming them; a mismatch is a
            // `LeafCollision`, unreachable except through a crate bug or an
            // off-model hash collision (see `LeafCollision`), and halting
            // beats silently keeping a side. (A same-version pair with
            // different payloads digests equal and prunes above — digests
            // are content-blind by design, a modeled trade.)
            (Some(ours), Some(theirs)) => {
                if ours.ceiling() != theirs.ceiling()
                    || ours.message().as_slice() != theirs.message().as_slice()
                {
                    return Err(LeafCollision {
                        path: Path::for_leaf(ours.ceiling()).into(),
                    });
                }
                Some(ours)
            }
        })
    }
}

#[cfg(test)]
mod tests;
