//! Pair a peer's reply with one outstanding materialized query.

use crate::tree::mirror::streaming::message::Listed;

use crate::{
    Version,
    counter::widen,
    tree::{
        mirror::contained,
        mirror::streaming::{
            Backend, ErasedNode, Leaf,
            byte_set::ByteSet,
            erased::Reaction,
            ledger::{self, Charge, ReplyBound},
            materialized::{
                Error, Query, Reserved, Resolution, Resolve, SupplyLedger, Violation, violation,
            },
            stats::Recorder,
        },
        typed::{ErasedPrefix, height::Z},
    },
};

use super::charges;

/// One query's reaction loop.
///
/// The resolver pairs held children with the reply's reactions in order,
/// accumulates the scope's [`Resolution`], and reports each semantic fault as
/// its exact [`Violation`]. A query reaction returns a [`Dispute`] for the
/// caller to resolve at the next level; matches and supplies resolve locally.
///
/// The reply's one match names the later held children that also match,
/// which take no reaction. The resolver resolves each of them as matched
/// when it reaches it: before pairing the next positional reaction, before
/// a supply at a greater radix, and at the reply's end. So it infers them
/// one at a time as it advances, keeping its resolution in radix order.
pub struct Resolver<'session, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The scope being resolved.
    prefix: ErasedPrefix,
    /// Held children still awaiting a reaction, or covered by the reply's
    /// match and awaiting their turn.
    fan: std::vec::IntoIter<(u8, B::Erased)>,
    /// The held children the reply's match covers.
    covered: ByteSet,
    /// Whether the reply has carried its one match.
    matched: bool,
    /// The node those children were listed from, which the resolution
    /// carries to assembly.
    original: Option<B::Erased>,
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
    /// The bound on the reply, which sized the resolution's slots.
    bound: ReplyBound,
    /// The question's charge for its queue slot and the buffer of its
    /// children, which `fan` walks: it returns when the resolver finishes,
    /// once that buffer is gone and every child sits in a slot.
    #[expect(dead_code, reason = "held for its bytes, which return when it drops")]
    question: Charge,
    /// The question's node copies, which the resolution keeps as well.
    copies: Charge,
    /// The parts of the room the question set aside for the reply that
    /// stayed with it: the resolution takes its slots' part, and the
    /// records' part returns when the resolver is done.
    reservation: Reserved,
    /// What the reply has held so far: its reactions and the listing
    /// entries its queries carried.
    held: ReplyBound,
    /// The bytes the records the take derives from the reply's questions
    /// so far hold ([`charges::derived_record_bytes`]), which the
    /// reservation's records part covers.
    derived: usize,
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
    pub(super) listing: Vec<Listed>,
}

impl<'session, B> Resolver<'session, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Begin resolving one outstanding query.
    pub fn new(
        Query {
            prefix,
            ours,
            original,
            bound,
            charge,
            copies,
            reservation,
        }: Query<B::Erased>,
        their_version: &'session Version,
        ledger: &'session SupplyLedger,
        stats: &'session Recorder,
    ) -> Self {
        Self {
            prefix,
            fan: ours.into_iter(),
            covered: ByteSet::EMPTY,
            matched: false,
            original,
            // Sized at the reservation's slots, so the resolution holds
            // exactly what its part of the reservation prices.
            resolved: Vec::with_capacity(bound.reactions),
            their_version,
            ledger,
            stats,
            bound,
            question: charge,
            copies,
            reservation,
            held: ReplyBound::default(),
            derived: 0,
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
        self.held.reactions += 1;
        match reaction {
            Reaction::Match(later) => {
                if self.matched {
                    return violation(Violation::RepeatedMatch);
                }
                self.matched = true;
                let Some((radix, node)) = self.next_uncovered() else {
                    return violation(Violation::UnexpectedMatch);
                };
                self.resolved.push((radix, Resolve::Ready(Some(node))));
                let covered = later.set();
                let held = self
                    .fan
                    .as_slice()
                    .iter()
                    .filter(|(radix, _)| covered.contains(*radix))
                    .count();
                if held != covered.len() {
                    return violation(Violation::UnlistedCover);
                }
                self.covered = covered;
            }
            Reaction::Supply(radix, node) => {
                self.pass_covered(Some(radix));
                if self.resolved.last().is_some_and(|(last, _)| radix <= *last) {
                    return violation(Violation::InvalidSupply);
                }
                match self.fan.as_slice().first() {
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
                        let leaves = widen(node.len());
                        self.ledger.absorb(leaves)?;
                        // An absorbed supply is content this replica just
                        // learned: credit its exact live-leaf count.
                        self.stats.gained(leaves);
                        self.resolved.push((radix, Resolve::Ready(Some(node))));
                    }
                }
            }
            Reaction::Query(listing) => {
                self.held.listing += listing.len();
                self.derived += charges::derived_record_bytes(self.prefix.height(), listing.len());
                let Some((radix, node)) = self.next_uncovered() else {
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

    /// Resolve, as matched, the covered children ahead of the next one the
    /// match does not cover, and take that next child.
    fn next_uncovered(&mut self) -> Option<(u8, B::Erased)> {
        self.pass_covered(None);
        self.fan.next()
    }

    /// Resolve, as matched, the covered children at the front of the fan,
    /// stopping at the first the match does not cover or, if `before` is
    /// given, at the first at or above that radix.
    fn pass_covered(&mut self, before: Option<u8>) {
        while let Some((radix, _)) = self.fan.as_slice().first()
            && self.covered.contains(*radix)
            && before.is_none_or(|before| *radix < before)
        {
            let (radix, node) = self
                .fan
                .next()
                .expect("the fan's first child was just read");
            self.resolved.push((radix, Resolve::Ready(Some(node))));
        }
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
        self.pass_covered(None);
        if self.fan.next().is_some() {
            violation(Violation::UnfinishedReply)
        } else {
            ledger::fitted(
                self.bound,
                self.held,
                self.reservation.records.bytes(),
                self.derived,
            );
            // The question's own charge and the reservation's records part
            // return as the resolver drops here; the resolution keeps its
            // slots' part and the question's node copies.
            Ok(Resolution {
                prefix: self.prefix,
                resolved: self.resolved,
                original: self.original,
                charge: self.reservation.slots,
                copies: self.copies,
            })
        }
    }
}
