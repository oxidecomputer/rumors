//! What a reply can still deliver to the chunks it parks: the one account
//! that both checks each frame of the reply and prices the room the reply
//! still needs.
//!
//! A question sets aside room for its reply's chunks before the reply
//! exists, priced at the reply's bound ([`parked_room`]). The decoder
//! returns that room as the reply shows it cannot use it, so that a reply
//! much smaller than its bound does not hold the rest of the bound until it
//! ends. Returning room early is safe only if the room left always covers
//! every chunk the reply can still park. An [`Allowance`] makes that hold
//! by construction:
//!
//! - It records what the reply has used of each limit the room is priced
//!   on: reaction slots, listing entries, the radix floor, the positions,
//!   and the one covered set.
//! - Each frame spends from it, and fails, leaving the allowance as it was,
//!   if the spend would pass a limit. So no reply the decoder accepts ever
//!   uses more than the allowance counted.
//! - Its price, what the reply can still deliver, is a function of those
//!   limits that no spend raises: each spend moves what it uses from what
//!   can come to what is ready, and each chunk delivered takes what was
//!   ready out of the price as it takes its own bytes out of the room.
//! - It returns the room beyond its price after each frame, and the room
//!   starts at least at its price, so the room covers the price at every
//!   moment, and each chunk's bytes come out of what the price set aside
//!   for it.

use std::marker::PhantomData;
use std::mem::size_of;

use crate::tree::mirror::streaming::{
    FAN,
    byte_set::ByteSet,
    erased::Reaction,
    holding::Holding,
    ledger::{Charge, ReplyBound, min, queued},
    message::Listed,
    remote::Parked,
};

use super::charges::{covered_set_price, covered_sets};

#[cfg(test)]
mod tests;

/// The most a reply can still deliver to its parked chunks: the chunks,
/// their reactions, the listing entries their queries carry, and the
/// covered set their match carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Deliverable {
    /// The chunks it can still park.
    pub(crate) chunks: usize,
    /// The reaction slots those chunks can fill together: one per reaction,
    /// and one per child a counted set covers, which takes no reaction.
    pub(crate) reactions: usize,
    /// The listing entries their queries can carry together.
    pub(crate) listing: usize,
    /// The covered sets their match can carry: at most one, a reply
    /// carrying at most one match, and each covering children whose slots
    /// [`reactions`](Self::reactions) counts.
    pub(crate) sets: usize,
}

/// The bytes the parked chunks `still` describes can hold, priced as each
/// chunk is ([`parked_bytes`](super::charges::parked_bytes)), each covered
/// set net of the slots of the children it covers ([`covered_set_price`]).
pub(crate) const fn parked_price<E>(still: Deliverable) -> usize {
    still.chunks * queued::<Parked<E>>()
        + still.reactions * size_of::<Reaction<E>>()
        + still.sets * covered_set_price::<E>()
        + still.listing * size_of::<Listed>()
}

/// The part of a reservation the reply's chunks take while they park: the
/// price of everything a reply within `bound` can deliver, about `listed`
/// of the asker's children.
///
/// The chunks are at most one more than the reply's questions, which number
/// at most `listed.min(bound.reactions)` since a query sits only at a
/// position the asker listed, and they hold the reply's reactions and
/// listing entries, and the set of later matches its one match covers,
/// which needs a listed child for the match and one more to cover. The
/// bound's reactions count a slot for each child the set covers, so the
/// set is priced net of those slots ([`covered_set_price`]).
///
/// An [`Allowance`] for the same bound and positions starts at a price no
/// greater than this: the same terms, but for the listing, which it also
/// holds to a fan per question that can still come. The two are equal
/// unless the bound allows more listing entries than that, and the
/// allowance's first release returns the difference.
///
/// The decoder charges each chunk its own part
/// ([`parked_bytes`](super::charges::parked_bytes)), which prices the
/// chunk's vectors at their capacities. It allocates each chunk's
/// reactions, and each listing a query carries, at exactly their lengths,
/// so the chunks of a reply within its bound take no more than this room
/// together; a chunk that took more would be charged in full and counted
/// as a shortfall ([`Charge::take`]).
pub(crate) const fn parked_room<E>(bound: ReplyBound, listed: usize) -> usize {
    parked_price::<E>(Deliverable {
        chunks: min(listed, bound.reactions) + 1,
        reactions: bound.reactions,
        listing: bound.listing,
        sets: covered_sets(listed),
    })
}

/// What a frame asked of an [`Allowance`] that it could not give.
///
/// Each names the limit the frame would pass. A conforming peer passes
/// none, so each reveals a miscounting or misordering peer at the frame
/// that would pass it, before the reply holds more than its question
/// reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Overdraft {
    /// The reaction would take a slot beyond the bound's `bound` reaction
    /// slots.
    Reactions {
        /// The reaction slots the bound allows.
        bound: usize,
    },
    /// The query would carry listing entries beyond what the peer's
    /// grandchild count of the scope leaves, of a bound of `bound`, or more
    /// than a fan.
    Listing {
        /// The listing entries the bound allows.
        bound: usize,
    },
    /// The reaction, at `radix`, does not sit above the reply's latest
    /// one, at `after`.
    Unordered {
        /// The radix of the reply's latest reaction.
        after: u8,
        /// The radix of the reaction out of order.
        radix: u8,
    },
    /// The supply sits at `radix`, which the reply's match covers.
    CoveredSupply {
        /// The radix supplied.
        radix: u8,
    },
    /// The query has no position to take: none is left, or it sits at a
    /// radix the match covers, which is no position's to take.
    UnpositionedQuery,
    /// The match has no position left to take, for itself and each child
    /// it covers.
    UnpositionedMatch,
    /// The match follows the reply's first.
    RepeatedMatch,
}

/// What a reply has taken that no delivered chunk holds yet.
#[derive(Clone, Copy, Debug, Default)]
struct Ready {
    /// Their reaction slots: one per reaction, and one per child their
    /// match covers.
    slots: usize,
    /// The queries among them.
    questions: usize,
    /// The listing entries those queries carry together.
    listing: usize,
    /// The children their match covers: none unless the match is among
    /// them and carries a covered set.
    covered: usize,
}

/// What one reply can still deliver to its parked chunks, which checks each
/// of the reply's frames and returns the reply's room beyond it.
///
/// The decoder creates one per reply, from the bound the record of the
/// question sets on the reply, the positions the record lists, and the
/// room the question set aside ([`parked_room`]). It reports each frame
/// before it reads the next: a match ([`matched`](Self::matched)), a query
/// ([`query`](Self::query)), or each run of a supply
/// ([`supply`](Self::supply)), whose node's child count it reports once
/// assembled ([`supplied`](Self::supplied)). Each chunk takes its bytes
/// from the room ([`chunk`](Self::chunk)), and after each frame the room
/// beyond what the reply can still deliver returns
/// ([`release`](Self::release)).
///
/// The allowance assumes nothing about the reply it does not check, so the
/// room covers what is left whatever the peer sends. It checks counts and
/// order: the record's cursor, which the decoder consults first, checks
/// *which* position each match and query takes and which positions a match
/// covers.
#[derive(Debug)]
pub(crate) struct Allowance<E> {
    /// The bound the record of the question sets on the reply.
    bound: ReplyBound,
    /// The reaction slots the reply has taken: one per reaction, and one per
    /// child its match covers.
    ///
    /// A covered child takes no reaction, but the bound counts a slot for
    /// it, the parent resolution keeps a slot for it, and the room counts
    /// on its slot to pay for the covered set ([`covered_set_price`]).
    /// Counting it keeps those slots out of the reach of a later reaction.
    slots: usize,
    /// The listing entries the reply has used of the peer's grandchild
    /// count of the scope.
    ///
    /// Those are the entries its queries carried, and the child counts of
    /// the children it matched or its match covers and of the nodes it
    /// supplied: the count includes each, and their listings never come.
    listing: usize,
    /// The radix of the reply's latest reaction: every later reaction sits
    /// at a greater one.
    floor: Option<u8>,
    /// The positions a later match or query can take: those the reply has
    /// not reached that its match does not cover.
    positions: usize,
    /// The radixes the reply's match covers.
    covered: ByteSet,
    /// Whether the reply has carried its one match.
    matched: bool,
    /// What the reply has taken that no delivered chunk holds yet.
    ready: Ready,
    /// The part of the room the question set aside that no chunk has taken
    /// and no release has returned.
    room: Charge,
    /// The handle type whose sizes price the room.
    handles: PhantomData<fn() -> E>,
}

impl<E> Allowance<E> {
    /// The allowance of a reply bounded by `bound` against a record listing
    /// `positions` positions for its matches and queries, its chunks parked
    /// out of `room`.
    ///
    /// Its price starts at most at [`parked_room`] of the same bound and
    /// positions, which is what a question sets aside.
    pub(crate) fn new(bound: ReplyBound, positions: usize, room: Charge) -> Self {
        Self {
            bound,
            slots: 0,
            listing: 0,
            floor: None,
            positions,
            covered: ByteSet::EMPTY,
            matched: false,
            ready: Ready::default(),
            room,
            handles: PhantomData,
        }
    }

    /// Whether the reply has taken no reaction slot yet.
    pub(crate) fn is_untouched(&self) -> bool {
        self.slots == 0
    }

    /// Spend a match at `radix` that covers the children at `covered`,
    /// whose child counts together with the matched child's are
    /// `children`.
    ///
    /// The match takes its position, one more for each child it covers,
    /// and a slot for itself and each of them; the child counts leave the
    /// listing.
    ///
    /// # Errors
    ///
    /// Returns [`Overdraft::RepeatedMatch`] after a first match,
    /// [`Overdraft::UnpositionedMatch`] if the positions left are fewer than
    /// the match takes, [`Overdraft::Unordered`] unless `radix` sits above
    /// the reply's latest reaction, and [`Overdraft::Reactions`] if the slots
    /// pass the bound.
    pub(crate) fn matched(
        &mut self,
        radix: u8,
        covered: ByteSet,
        children: usize,
    ) -> Result<(), Overdraft> {
        if self.matched {
            return Err(Overdraft::RepeatedMatch);
        }
        let taken = 1 + covered.len();
        let positions = self
            .positions
            .checked_sub(taken)
            .ok_or(Overdraft::UnpositionedMatch)?;
        self.check_order(radix)?;
        let slots = self.check_slots(taken)?;
        self.floor = Some(radix);
        self.slots = slots;
        self.positions = positions;
        self.covered = covered;
        self.matched = true;
        self.listing += children;
        self.ready.slots += taken;
        self.ready.covered = covered.len();
        Ok(())
    }

    /// Spend a query at `radix` listing `entries` children.
    ///
    /// The query takes its position and a slot, and its entries leave the
    /// listing.
    ///
    /// # Errors
    ///
    /// Returns [`Overdraft::UnpositionedQuery`] if no position is left or
    /// the match covers `radix`, [`Overdraft::Unordered`] unless `radix`
    /// sits above the reply's latest reaction, [`Overdraft::Listing`] if the
    /// entries are more than the listing left or than a fan, and
    /// [`Overdraft::Reactions`] if the slot passes the bound.
    ///
    /// A listing names each child at its own radix, so it holds a fan at
    /// most, as the codec decodes every listing; the price counts on that,
    /// holding the listing still to come to a fan per query.
    pub(crate) fn query(&mut self, radix: u8, entries: usize) -> Result<(), Overdraft> {
        let positions = self
            .positions
            .checked_sub(1)
            .filter(|_| !self.covered.contains(radix))
            .ok_or(Overdraft::UnpositionedQuery)?;
        self.check_order(radix)?;
        if entries > self.listing_left().min(FAN) {
            return Err(Overdraft::Listing {
                bound: self.bound.listing,
            });
        }
        let slots = self.check_slots(1)?;
        self.floor = Some(radix);
        self.slots = slots;
        self.positions = positions;
        self.listing += entries;
        self.ready.slots += 1;
        self.ready.questions += 1;
        self.ready.listing += entries;
        Ok(())
    }

    /// Spend the start of a supply's run at `radix`.
    ///
    /// The run takes a slot and no position: a supply names its own radix.
    ///
    /// # Errors
    ///
    /// Returns [`Overdraft::CoveredSupply`] if the match covers `radix`,
    /// [`Overdraft::Unordered`] unless `radix` sits above the reply's latest
    /// reaction, and [`Overdraft::Reactions`] if the slot passes the bound.
    pub(crate) fn supply(&mut self, radix: u8) -> Result<(), Overdraft> {
        if self.covered.contains(radix) {
            return Err(Overdraft::CoveredSupply { radix });
        }
        self.check_order(radix)?;
        let slots = self.check_slots(1)?;
        self.floor = Some(radix);
        self.slots = slots;
        self.ready.slots += 1;
        Ok(())
    }

    /// Note that a supplied node holds `children` children, which leave the
    /// listing.
    ///
    /// A node has at most a fan of children, so the clamp changes nothing a
    /// conforming backend reports, and keeps a miscounting one from
    /// overflowing the sum.
    pub(crate) fn supplied(&mut self, children: usize) {
        self.listing += children.min(FAN);
    }

    /// Take a chunk of `bytes` out of the room, the chunk holding everything
    /// the reply has taken that no chunk holds yet, and ending the reply if
    /// `last`.
    ///
    /// The chunk takes its bytes from what the price set aside for it, so
    /// the room left still covers the price of what is left. The last chunk
    /// returns the rest of the room: nothing more can come.
    ///
    /// A chunk that does not end the reply must end at a query, as the
    /// decoder's do: the price counts one chunk per query and one more, so
    /// a chunk holding no query would take a chunk's bytes the price never
    /// set aside. Taking one panics in debug builds.
    pub(crate) fn chunk(&mut self, bytes: usize, last: bool) -> Charge {
        debug_assert!(
            last || self.ready.questions > 0,
            "a chunk that does not end its reply ends at a query",
        );
        self.ready = Ready::default();
        let charge = self.room.take(Holding::ParkedReplies, bytes);
        if last {
            drop(std::mem::take(&mut self.room));
        }
        charge
    }

    /// Return the room beyond what the reply can still deliver.
    ///
    /// The room covers that much at every moment, which is what lets every
    /// chunk take its whole price from it: it starts at least at the price,
    /// the price never grows, and each release returns only the excess.
    pub(crate) fn release(&mut self) {
        let beyond = self.room.bytes().saturating_sub(self.price());
        drop(self.room.split(Holding::ParkedReplies, beyond));
    }

    /// The bytes the reply's parked chunks can still hold.
    pub(crate) fn price(&self) -> usize {
        parked_price::<E>(self.deliverable())
    }

    /// The most the reply can still deliver to parked chunks, what it has
    /// taken that no chunk holds yet included.
    ///
    /// What is ready counts as it was taken. What can come is bounded four
    /// ways:
    ///
    /// - Later reactions number at most the slots the bound leaves, and one
    ///   per radix above the floor that the match does not cover.
    /// - Later queries sit only at the positions left, and number no more
    ///   than the later reactions.
    /// - Later listing entries number at most what the peer's count leaves
    ///   and a fan per later query.
    /// - A later match can carry one covered set, if two positions are
    ///   left, for the match and a child it covers; the slots of the
    ///   children it covers are among the later reactions', whose bound and
    ///   radixes both still count them.
    ///
    /// Every non-last chunk ends at a query, so the chunks number at most
    /// the queries ready and to come, and one more.
    ///
    /// No term grows as the reply goes on. Each spend moves its slots,
    /// position, and entries from what can come to what is ready, and
    /// raises the floor, which takes radixes only from what can come; the
    /// match takes its covered children's radixes from what can come as it
    /// counts their slots as ready; a supplied node's child count only
    /// shrinks the listing left; and each chunk takes what is ready out of
    /// the price.
    pub(crate) fn deliverable(&self) -> Deliverable {
        let radixes = self.floor.map_or(FAN, |floor| {
            usize::from(u8::MAX - floor) - self.covered.len_above(floor)
        });
        let reactions = (self.bound.reactions - self.slots).min(radixes);
        let questions = self.positions.min(reactions);
        let listing = self.listing_left().min(FAN * questions);
        let sets = usize::from(!self.matched && self.positions >= 2);
        Deliverable {
            chunks: self.ready.questions + questions + 1,
            reactions: self.ready.slots + reactions,
            listing: self.ready.listing + listing,
            sets: usize::from(self.ready.covered > 0) + sets,
        }
    }

    /// The listing entries the reply's later queries can still carry: what
    /// the peer's grandchild count of the scope leaves beside what the
    /// reply has used of it.
    ///
    /// A conforming peer never exceeds it: its count sums its children's
    /// child counts, a matched or covered child's is this side's, and a
    /// supplied node is the peer's child or what the peer's deletion filter
    /// left of it.
    fn listing_left(&self) -> usize {
        self.bound.listing.saturating_sub(self.listing)
    }

    /// Check that a reaction at `radix` sits above the reply's latest one.
    ///
    /// A reply's reactions ascend by radix, its supplies among its
    /// positional reactions, and the price counts on that order: the
    /// reactions still to come number at most one per radix above the
    /// floor.
    fn check_order(&self, radix: u8) -> Result<(), Overdraft> {
        match self.floor {
            Some(after) if radix <= after => Err(Overdraft::Unordered { after, radix }),
            _ => Ok(()),
        }
    }

    /// The reaction slots taken once `slots` more are, if the bound still
    /// allows them.
    ///
    /// A conforming peer never exceeds the bound, which allows one slot per
    /// child either side holds ([`ReplyBound`]): a reaction is about one
    /// such child, a covered child is one both sides hold, and no two of
    /// them share a radix.
    fn check_slots(&self, slots: usize) -> Result<usize, Overdraft> {
        let taken = self.slots + slots;
        if taken > self.bound.reactions {
            return Err(Overdraft::Reactions {
                bound: self.bound.reactions,
            });
        }
        Ok(taken)
    }
}
