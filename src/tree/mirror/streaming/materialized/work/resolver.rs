use std::iter::Peekable;

use crate::{
    Version,
    tree::{
        mirror::contained,
        mirror::streaming::{
            Backend, Leaf, Node,
            materialized::{Error, Query, Resolution, Resolve, Violation, violation},
            message::Reaction,
            stats::Recorder,
        },
        typed::{
            Hash, Prefix,
            height::{Height, S, Z},
        },
    },
};

/// One query's reaction loop: pairs the held children against the reply's
/// reactions in order, accumulating the scope's [`Resolution`] and reporting
/// each counterparty fault as its exact [`Violation`].
pub struct Resolver<'v, B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    prefix: Prefix<S<H>>,
    fan: Peekable<std::vec::IntoIter<(u8, B::Node<H>)>>,
    resolved: Vec<(u8, Resolve<B, T, H>)>,
    /// The peer's declared greeting version: every supplied subtree's
    /// ceiling must be contained in it
    /// ([`Violation::UncontainedSupply`]).
    their_version: &'v Version,
    /// The session's stats recorder: each absorbed supply credits its
    /// exact live-leaf count as
    /// [`messages_gained`](crate::SessionStats::messages_gained).
    stats: Recorder,
}

impl<'v, B, T, H> Resolver<'v, B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    pub fn new(
        Query { prefix, ours }: Query<B, T, H>,
        their_version: &'v Version,
        stats: Recorder,
    ) -> Self {
        Self {
            prefix,
            fan: ours.into_iter().peekable(),
            resolved: Vec::new(),
            their_version,
            stats,
        }
    }

    pub fn react(
        &mut self,
        reaction: Reaction<B, T, H>,
    ) -> Result<Option<(Prefix<S<H>>, u8, B::Node<H>, Vec<(u8, Hash)>)>, Error<B::Error>> {
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

    pub fn ready(&mut self, radix: u8, node: Option<B::Node<H>>) {
        self.resolved.push((radix, Resolve::Ready(node)));
    }

    pub fn pending(&mut self, radix: u8) {
        self.resolved.push((radix, Resolve::Pending));
    }

    pub fn finish(mut self) -> Result<Resolution<B, T, H>, Error<B::Error>> {
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
