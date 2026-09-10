//! Merge two trees in memory, honoring deletions by causal version.
//!
//! This is the behavioral oracle for wire reconciliation. At each path:
//!
//! - An absent pair stays absent.
//! - A one-sided subtree is filtered by [`Unknown::unknown`] against the
//!   absent side's causal ceiling. A leaf that side has seen but no longer
//!   holds was deleted there.
//! - Equal nodes retain our handle, including its cached hash and bounds.
//! - Differing nodes merge their sorted child lists and recurse. Reassembly
//!   through [`Node::branch`] compresses singleton branches again.
//!
//! Equality checks shared ownership first, then Merkle hashes. A divergent
//! branch visits at most 256 children; equal subtrees need no descent.

use itertools::{EitherOrBoth, Itertools};

use crate::Version;

use super::typed::*;
use super::unknown::Unknown;
use height::{Height, Root, S, Z};

/// Merge the roots using their causal ceilings to recognize deletions.
///
/// Set `changed` if the result gains or loses a leaf relative to `a`; never
/// clear it. The walk detects gains by a surviving subtree and losses by a
/// reduced leaf count, without hashing solely to check for change. Each path
/// can only gain or lose, so changes cannot cancel across recursive calls.
pub fn join(
    a: Option<Node<Root>>,
    b: Option<Node<Root>>,
    a_version: &Version,
    b_version: &Version,
    changed: &mut bool,
) -> Option<Node<Root>> {
    // Tests can interrupt the walk before any candidate is published.
    #[cfg(test)]
    crate::tree::panic_injection::fire_if_armed();

    Join::join(a, b, a_version, b_version, changed)
}

/// One height of the merge, preserving [`join`]'s change-detection contract.
pub trait Join: Unknown {
    /// Merge the nodes at this height and record any change from `a`.
    fn join(
        a: Option<Node<Self>>,
        b: Option<Node<Self>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> Option<Node<Self>>;
}

/// Merge branches, filtering one-sided children and retaining equal ones.
impl<H: Join> Join for S<H>
where
    S<H>: Height + Unknown,
{
    /// Reconcile this level and rebuild its surviving children in order.
    fn join(
        a: Option<Node<S<H>>>,
        b: Option<Node<S<H>>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> Option<Node<S<H>>> {
        // A later fuse step exercises unwinding after some children were merged.
        #[cfg(test)]
        crate::tree::panic_injection::fire_if_armed();

        match (a, b) {
            (None, None) => None,
            // Filtering can only remove our leaves, so a smaller count means
            // a change. Any surviving leaf from their one-sided subtree is new.
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
                // Equal paths commit the same versions, hence the same messages
                // under the unique-version contract. Keep our cached node.
                if ours == theirs {
                    return Some(ours);
                }

                // Consume both sorted child lists. Equal children keep their
                // existing handles; one-sided children still need deletion
                // filtering against the absent side's causal ceiling.
                let ours = ours.into_children();
                let theirs = theirs.into_children();
                // Reserving the larger side avoids growth for mostly overlapping
                // fans. Disjoint additions can grow this up to 256 children.
                let mut merged = Children::with_capacity(ours.len().max(theirs.len()));
                for pair in ours.into_iter().merge_join_by(theirs, |a, b| a.0.cmp(&b.0)) {
                    let (radix, ours, theirs) = match pair {
                        EitherOrBoth::Both((radix, ours), (_, theirs)) => {
                            if ours == theirs {
                                merged.push(radix, ours);
                                continue;
                            }
                            (radix, Some(ours), Some(theirs))
                        }
                        EitherOrBoth::Left((radix, ours)) => (radix, Some(ours), None),
                        EitherOrBoth::Right((radix, theirs)) => (radix, None, Some(theirs)),
                    };
                    if let Some(child) = Join::join(ours, theirs, a_version, b_version, changed) {
                        // The merge yields unique ascending radices, so append
                        // without a search or shifting existing entries.
                        merged.push(radix, child);
                    }
                }

                Node::branch(merged)
            }
        }
    }
}

/// Resolve leaves that survive the branch-level equality checks.
impl Join for Z {
    /// Filter a one-sided leaf; paired leaves must have pruned above.
    fn join(
        a: Option<Node<Z>>,
        b: Option<Node<Z>>,
        a_version: &Version,
        b_version: &Version,
        changed: &mut bool,
    ) -> Option<Node<Z>> {
        match (a, b) {
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
            // Same-path leaves hash equally and prune above. Ingestion checks
            // version reuse; join trusts that distinct versions have distinct paths.
            (Some(_), Some(_)) => {
                unreachable!("same-position leaves hash equally and prune above")
            }
        }
    }
}

#[cfg(test)]
mod tests;
