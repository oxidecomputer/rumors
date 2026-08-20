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

/// One query's reaction loop: pairs the held children against the reply's
/// reactions in order, accumulating the scope's [`Resolution`] and reporting
/// each counterparty fault as its exact [`Violation`].
pub struct Resolver<'v, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    prefix: ErasedPrefix,
    fan: Peekable<std::vec::IntoIter<(u8, B::Erased)>>,
    resolved: Vec<(u8, Resolve<B::Erased>)>,
    /// The peer's declared greeting version: every supplied subtree's
    /// ceiling must be contained in it
    /// ([`Violation::UncontainedSupply`]).
    their_version: &'v Version,
    /// The peer's declared-set-length ledger: every absorbed supply
    /// charges its exact live-leaf count
    /// ([`Violation::OverdrawnSupply`]).
    ledger: &'v SupplyLedger,
    /// The session's stats recorder: each absorbed supply credits its
    /// exact live-leaf count as
    /// [`messages_gained`](crate::SessionStats::messages_gained).
    stats: Recorder,
}

impl<'v, B> Resolver<'v, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    pub fn new(
        Query { prefix, ours }: Query<B::Erased>,
        their_version: &'v Version,
        ledger: &'v SupplyLedger,
        stats: Recorder,
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

    #[allow(clippy::type_complexity)]
    pub fn react(
        &mut self,
        reaction: Reaction<B::Erased>,
    ) -> Result<Option<(ErasedPrefix, u8, B::Erased, Vec<(u8, Hash)>)>, Error<B::Error>> {
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
                return Ok(Some((self.prefix, radix, node, listing)));
            }
        }

        Ok(None)
    }

    pub fn ready(&mut self, radix: u8, node: Option<B::Erased>) {
        self.resolved.push((radix, Resolve::Ready(node)));
    }

    pub fn pending(&mut self, radix: u8) {
        self.resolved.push((radix, Resolve::Pending));
    }

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
