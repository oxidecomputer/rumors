//! Pair a peer's reply with one outstanding materialized query.

use std::iter::Peekable;

use crate::{
    Version,
    tree::{
        mirror::contained,
        mirror::streaming::{
            Backend, ErasedNode, Leaf,
            erased::Reaction,
            materialized::{Error, Query, Resolution, Resolve, SupplyLedger, Violation, violation},
            stats::Recorder,
        },
        typed::{ErasedPrefix, Hash, height::Z},
    },
};

/// One query's reaction loop.
///
/// The resolver pairs held children with the reply's reactions in order,
/// accumulates the scope's [`Resolution`], and reports each semantic fault as
/// its exact [`Violation`]. A query reaction returns a [`Dispute`] for the
/// caller to resolve at the next level; matches and supplies resolve locally.
pub struct Resolver<'session, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The scope being resolved.
    prefix: ErasedPrefix,
    /// Held children still awaiting a reaction.
    fan: Peekable<std::vec::IntoIter<(u8, B::Erased)>>,
    /// Child resolutions accumulated in radix order.
    resolved: Vec<(u8, Resolve<B::Erased>)>,
    /// The peer's declared greeting version: every supplied subtree's
    /// ceiling must be contained in it
    /// ([`Violation::UncontainedSupply`]).
    their_version: &'session Version,
    /// The peer's declared-set-length ledger: every absorbed supply
    /// charges its exact live-leaf count
    /// ([`Violation::OverdrawnSupply`]).
    ledger: &'session SupplyLedger,
    /// The session's stats recorder: each absorbed supply credits its
    /// exact live-leaf count as
    /// [`messages_gained`](crate::SessionStats::messages_gained).
    stats: &'session Recorder,
}

/// A held child that the peer reports as different.
pub struct Dispute<E> {
    /// The disputed child's full prefix.
    pub(super) child_prefix: ErasedPrefix,
    /// The child's radix in its parent resolution.
    pub(super) radix: u8,
    /// This replica's child node.
    pub(super) node: E,
    /// The peer's listing for the child.
    pub(super) listing: Vec<(u8, Hash)>,
}

impl<'session, B> Resolver<'session, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Begin resolving one outstanding query.
    pub fn new(
        Query { prefix, ours }: Query<B::Erased>,
        their_version: &'session Version,
        ledger: &'session SupplyLedger,
        stats: &'session Recorder,
    ) -> Self {
        Self {
            prefix,
            fan: ours.into_iter().peekable(),
            resolved: Vec::new(),
            their_version,
            ledger,
            stats,
        }
    }

    /// Apply one reaction.
    ///
    /// Matches and supplies resolve a child immediately and return `None`.
    /// A query returns the disputed child for the caller to answer. Any
    /// reaction that does not fit the outstanding fan returns its exact
    /// protocol violation.
    pub fn react(
        &mut self,
        reaction: Reaction<B::Erased>,
    ) -> Result<Option<Dispute<B::Erased>>, Error<B::Error>> {
        match reaction {
            Reaction::Match => {
                let Some((radix, node)) = self.fan.next() else {
                    return violation(Violation::UnexpectedMatch);
                };
                self.resolved.push((radix, Resolve::Ready(Some(node))));
            }
            Reaction::Supply(radix, node) => {
                if self.resolved.last().is_some_and(|(last, _)| radix <= *last) {
                    return violation(Violation::InvalidSupply);
                }
                match self.fan.peek() {
                    Some((next, _)) if radix == *next => {
                        return violation(Violation::UnexpectedSupply);
                    }
                    Some((next, _)) if radix > *next => {
                        return violation(Violation::InvalidSupply);
                    }
                    // A structurally valid supply still owes the content
                    // check: its ceiling is a memoized bound, so the cost
                    // is one read per supplied subtree — at worst the
                    // memo's first forcing, linear in the nodes received.
                    _ if !contained(node.span().hi(), self.their_version) => {
                        return violation(Violation::UncontainedSupply);
                    }
                    _ => {
                        self.ledger.absorb(node.len() as u64)?;
                        // An absorbed supply is content this replica just
                        // learned: credit its exact live-leaf count.
                        self.stats.gained(node.len() as u64);
                        self.resolved.push((radix, Resolve::Ready(Some(node))));
                    }
                }
            }
            Reaction::Query(listing) => {
                let Some((radix, node)) = self.fan.next() else {
                    return violation(Violation::UnexpectedQuery);
                };
                return Ok(Some(Dispute {
                    child_prefix: self.prefix.push(radix),
                    radix,
                    node,
                    listing,
                }));
            }
        }

        Ok(None)
    }

    /// Record a child resolved immediately by the caller.
    pub fn ready(&mut self, radix: u8, node: Option<B::Erased>) {
        self.resolved.push((radix, Resolve::Ready(node)));
    }

    /// Reserve a child whose resolution will arrive from the next level.
    pub fn pending(&mut self, radix: u8) {
        self.resolved.push((radix, Resolve::Pending));
    }

    /// Finish the scope after verifying that every held child was answered.
    pub fn finish(mut self) -> Result<Resolution<B::Erased>, Error<B::Error>> {
        if self.fan.next().is_some() {
            violation(Violation::UnfinishedReply)
        } else {
            Ok(Resolution {
                prefix: self.prefix,
                resolved: self.resolved,
            })
        }
    }
}
