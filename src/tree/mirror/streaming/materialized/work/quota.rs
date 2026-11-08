//! What a reply can still deliver to the chunks it parks: the one account that
//! both checks each frame of the reply and prices the room the reply still
//! needs.
//!
//! A question sets aside room for its reply's chunks before the reply exists,
//! priced at the reply's bound ([`parked_room`]). The decoder returns that room
//! as the reply shows it cannot use it, so that a reply much smaller than its
//! bound does not hold the rest of the bound until it ends. Returning room
//! early is safe only if the room left always covers every chunk the reply can
//! still park. A [`ReplyQuota`] makes that hold by construction:
//!
//! - It records what the reply has used of each limit the room is priced on:
//!   reaction slots, listing entries, the radix floor, the positions, and the
//!   one covered set.
//! - Each frame spends from it, and fails, leaving the quota as it was, if the
//!   spend would pass a limit. So no reply the decoder accepts ever uses more
//!   than the quota counted.
//! - Its price, what the reply can still deliver, is a function of those limits
//!   that no spend raises: each spend moves what it uses from what can come to
//!   what is ready, and each chunk delivered takes what was ready out of the
//!   price as it takes its own bytes out of the room.
//! - It returns the room beyond its price after each frame, and the room starts
//!   at its price, since the question priced the room as a fresh quota's price
//!   ([`parked_room`]), so the room covers the price at every moment, and each
//!   chunk's bytes come out of what the price set aside for it.

use std::marker::PhantomData;
use std::mem::size_of;

use crate::tree::mirror::streaming::{
    FAN,
    byte_set::ByteSet,
    erased::Reaction,
    holding::Holding,
    ledger::{Charge, ReplyBound, min, queued},
    message::Listed,
    remote::{Inbound, Parked},
};

use super::charges::{covered_set_price, covered_sets};

#[cfg(test)]
mod tests;

/// The most a reply can still deliver to its parked chunks: the questions its
/// queries ask, the reactions, the listing entries the queries carry, and the
/// covered set the match carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Deliverable {
    /// The queries it can still deliver: each asks one question, and every
    /// chunk but the reply's last ends at one, so the chunks it can still park
    /// number at most these and one more.
    pub(crate) questions: usize,
    /// The reaction slots those chunks can fill together: one per reaction, and
    /// one per child a counted set covers, which takes no reaction.
    pub(crate) reactions: usize,
    /// The listing entries their queries can carry together.
    pub(crate) listing: usize,
    /// The covered sets their match can carry: at most one, a reply carrying at
    /// most one match, and each covering children whose slots
    /// [`reactions`](Self::reactions) counts.
    pub(crate) sets: usize,
}

/// The bytes the parked chunks `still` describes can hold: a chunk for each
/// question, and one more, beside the frames still to arrive.
///
/// Each chunk is priced as each chunk is
/// ([`parked_bytes`](super::charges::parked_bytes)), each covered set net of
/// the slots of the children it covers ([`covered_set_price`]). Each frame
/// still to arrive takes a node in its lane from the demultiplexer
/// ([`Inbound`]) until its decoder takes it: one per reaction slot, and one
/// more for an empty reply's bare end. A supply run's permit prices its own
/// node as well, so a supply's slot overprices its last run by a node.
pub(crate) const fn parked_price<E>(still: Deliverable) -> usize {
    (still.questions + 1) * queued::<Parked<E>>()
        + still.reactions * size_of::<Reaction<E>>()
        + (still.reactions + 1) * queued::<Inbound>()
        + still.sets * covered_set_price::<E>()
        + still.listing * size_of::<Listed>()
}

/// The part of a reservation the reply's chunks take while they park: the price
/// of everything a reply within `bound` can deliver, about `listed` of the
/// asker's children.
///
/// That is the price of a fresh [`ReplyQuota`] for the same bound and
/// positions, computed by the same code ([`opening`]): the room a question sets
/// aside is exactly what its reply's quota first prices, and the reservation
/// holds no parked room its reply could never use. Before its first frame, a
/// reply can still deliver:
///
/// - a reaction slot for each the bound allows, at most a fan, one per radix;
/// - a question at most for each position the asker listed, since a query sits
///   only at one, and for each reaction slot;
/// - at most a fan of listing entries for each of those questions, since a
///   listing names each child at its own radix, and no more than the bound's
///   listing;
/// - the set of later matches its one match covers, if the asker listed a child
///   for the match and one more to cover, priced net of the slots the bound's
///   reactions count for the children it covers ([`covered_set_price`]);
/// - a chunk for each question, and one more.
///
/// The decoder charges each chunk its own part
/// ([`parked_bytes`](super::charges::parked_bytes)), which prices the chunk's
/// vectors at their capacities. It allocates each chunk's reactions, and each
/// listing a query carries, at exactly their lengths, so the chunks of a reply
/// within its bound take no more than this room together; a chunk that took
/// more would be charged in full and counted as a shortfall ([`Charge::take`]).
pub(crate) const fn parked_room<E>(bound: ReplyBound, listed: usize) -> usize {
    parked_price::<E>(opening(bound, listed))
}

/// The most a reply within `bound`, about `listed` of the asker's children, can
/// deliver before its first frame: what a fresh [`ReplyQuota`] for the same
/// bound and positions prices.
///
/// The two parts of a reservation that grow with the reply's listing are priced
/// from it, by the one formula the quota prices with ([`Left::deliverable`]):
/// the room the reply's chunks park in ([`parked_room`]) and the records its
/// questions derive ([`records_room`](super::charges::records_room)). Neither
/// holds room for listing entries beyond a fan per question the reply can ask.
pub(crate) const fn opening(bound: ReplyBound, listed: usize) -> Deliverable {
    Left::opening(bound, listed).deliverable(Ready::NONE)
}

/// What a frame asked of a [`ReplyQuota`] that it could not give.
///
/// Each names the limit the frame would pass. A conforming peer passes none, so
/// each reveals a miscounting or misordering peer at the frame that would pass
/// it, before the reply holds more than its question reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Overdraft {
    /// The reaction would take a slot beyond the bound's `bound` reaction
    /// slots.
    Reactions {
        /// The reaction slots the bound allows.
        bound: usize,
    },
    /// The query would carry listing entries beyond what the peer's grandchild
    /// count of the scope leaves, of a bound of `bound`, or more than a fan.
    Listing {
        /// The listing entries the bound allows.
        bound: usize,
    },
    /// The reaction, at `radix`, does not sit above the reply's latest one, at
    /// `after`.
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
    /// The query has no position to take: none is left, or it sits at a radix
    /// the match covers, which is no position's to take.
    UnpositionedQuery,
    /// The match has no position left to take, for itself and each child it
    /// covers.
    UnpositionedMatch,
    /// The match follows the reply's first.
    RepeatedMatch,
}

/// What a reply has taken that no delivered chunk holds yet.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ready {
    /// Their reaction slots: one per reaction, and one per child their match
    /// covers.
    pub(crate) slots: usize,
    /// The queries among them.
    pub(crate) questions: usize,
    /// The listing entries those queries carry together.
    pub(crate) listing: usize,
    /// The children their match covers: none unless the match is among them and
    /// carries a covered set.
    pub(crate) covered: usize,
}

impl Ready {
    /// Nothing ready: the state before a reply's first frame and after each
    /// chunk.
    pub(crate) const NONE: Self = Self {
        slots: 0,
        questions: 0,
        listing: 0,
        covered: 0,
    };
}

/// What a reply's later frames can still use of the limits its room is priced
/// on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Left {
    /// The reaction slots later reactions can take: what the bound leaves, and
    /// at most one per radix above the floor that the match does not cover.
    pub(crate) reactions: usize,
    /// The positions later matches and queries can take.
    pub(crate) positions: usize,
    /// The listing entries later queries can carry: what the peer's grandchild
    /// count of the scope leaves.
    pub(crate) listing: usize,
    /// Whether the reply's one match can still come.
    pub(crate) matchable: bool,
}

impl Left {
    /// What a reply bounded by `bound` can use before its first frame, against
    /// a record listing `positions` positions: every slot the bound allows, at
    /// most one per radix, every position, all the peer's count, and its match.
    const fn opening(bound: ReplyBound, positions: usize) -> Self {
        Self {
            reactions: min(bound.reactions, FAN),
            positions,
            listing: bound.listing,
            matchable: true,
        }
    }

    /// The most a reply can still deliver to parked chunks: what is `ready`,
    /// beside the most that can come within these limits.
    ///
    /// Later queries sit only at the positions left, and number no more than
    /// the later reactions; each carries at most a fan of listing entries, as
    /// the codec decodes every listing. A later match can carry one covered
    /// set, if two positions are left, for the match and a child it covers
    /// ([`covered_sets`]); the slots of the children it covers are among the
    /// later reactions'. The questions are the queries ready and those to come.
    pub(crate) const fn deliverable(self, ready: Ready) -> Deliverable {
        let questions = min(self.positions, self.reactions);
        let sets = if self.matchable {
            covered_sets(self.positions)
        } else {
            0
        };
        Deliverable {
            questions: ready.questions + questions,
            reactions: ready.slots + self.reactions,
            listing: ready.listing + min(self.listing, FAN * questions),
            sets: if ready.covered > 0 { 1 } else { 0 } + sets,
        }
    }
}

/// What one reply can still deliver to its parked chunks, which checks each of
/// the reply's frames and returns the reply's room beyond it.
///
/// The decoder creates one per reply, from the bound the record of the question
/// sets on the reply, the positions the record lists, and the room the question
/// set aside ([`parked_room`]). It reports each frame before it reads the next:
/// a match ([`matched`](Self::matched)), a query ([`query`](Self::query)), or
/// each run of a supply ([`supply`](Self::supply)), whose node's child count it
/// reports once assembled ([`supplied`](Self::supplied)). Each chunk takes its
/// bytes from the room ([`chunk`](Self::chunk)), and after each frame the room
/// beyond what the reply can still deliver returns
/// ([`release`](Self::release)).
///
/// The quota assumes nothing about the reply it does not check, so the room
/// covers what is left whatever the peer sends. It checks counts and order: the
/// record's cursor, which the decoder consults first, checks *which* position
/// each match and query takes and which positions a match covers.
#[derive(Debug)]
pub(crate) struct ReplyQuota<E> {
    /// The bound the record of the question sets on the reply.
    bound: ReplyBound,
    /// The reaction slots the reply has taken: one per reaction, and one per
    /// child its match covers.
    ///
    /// A covered child takes no reaction, but the bound counts a slot for it,
    /// the parent resolution keeps a slot for it, and the room counts on its
    /// slot to pay for the covered set ([`covered_set_price`]). Counting it
    /// keeps those slots out of the reach of a later reaction.
    slots: usize,
    /// The listing entries the reply has used of the peer's grandchild count of
    /// the scope.
    ///
    /// Those are the entries its queries carried, and the child counts of the
    /// children it matched or its match covers and of the nodes it supplied:
    /// the count includes each, and their listings never come.
    listing: usize,
    /// The radix of the reply's latest reaction: every later reaction sits at a
    /// greater one.
    floor: Option<u8>,
    /// The positions a later match or query can take: those the reply has not
    /// reached that its match does not cover.
    positions: usize,
    /// The radixes the reply's match covers.
    covered: ByteSet,
    /// Whether the reply has carried its one match.
    matched: bool,
    /// What the reply has taken that no delivered chunk holds yet.
    ready: Ready,
    /// The part of the room the question set aside that no chunk has taken and
    /// no release has returned.
    room: Charge,
    /// The handle type whose sizes price the room.
    handles: PhantomData<fn() -> E>,
}

impl<E> ReplyQuota<E> {
    /// The quota of a reply bounded by `bound` against a record listing
    /// `positions` positions for its matches and queries, its chunks parked out
    /// of `room`.
    ///
    /// Its price starts at [`parked_room`] of the same bound and positions,
    /// which is what a question sets aside: that price is this fresh quota's,
    /// which this checks in debug builds.
    pub(crate) fn new(bound: ReplyBound, positions: usize, room: Charge) -> Self {
        let quota = Self {
            bound,
            slots: 0,
            listing: 0,
            floor: None,
            positions,
            covered: ByteSet::EMPTY,
            matched: false,
            ready: Ready::NONE,
            room,
            handles: PhantomData,
        };
        debug_assert_eq!(
            quota.left(),
            Left::opening(bound, positions),
            "a fresh quota leaves what parked_room prices",
        );
        quota
    }

    /// Whether the reply has taken no reaction slot yet.
    pub(crate) fn is_untouched(&self) -> bool {
        self.slots == 0
    }

    /// Spend a match at `radix` that covers the children at `covered`, whose
    /// child counts together with the matched child's are `children`.
    ///
    /// The match takes its position, one more for each child it covers, and a
    /// slot for itself and each of them; the child counts leave the listing.
    ///
    /// # Errors
    ///
    /// Returns [`Overdraft::RepeatedMatch`] after a first match,
    /// [`Overdraft::UnpositionedMatch`] if the positions left are fewer than
    /// the match takes, [`Overdraft::Unordered`] unless `radix` sits above the
    /// reply's latest reaction, and [`Overdraft::Reactions`] if the slots pass
    /// the bound.
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
    /// Returns [`Overdraft::UnpositionedQuery`] if no position is left or the
    /// match covers `radix`, [`Overdraft::Unordered`] unless `radix` sits above
    /// the reply's latest reaction, [`Overdraft::Listing`] if the entries are
    /// more than the listing left or than a fan, and [`Overdraft::Reactions`]
    /// if the slot passes the bound.
    ///
    /// A listing names each child at its own radix, so it holds a fan at most,
    /// as the codec decodes every listing; the price counts on that, holding
    /// the listing still to come to a fan per query.
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
    /// conforming backend reports, and keeps a miscounting one from overflowing
    /// the sum.
    pub(crate) fn supplied(&mut self, children: usize) {
        self.listing += children.min(FAN);
    }

    /// Take a chunk of `bytes` out of the room, the chunk holding everything
    /// the reply has taken that no chunk holds yet, and ending the reply if
    /// `last`.
    ///
    /// The chunk takes its bytes from what the price set aside for it, so the
    /// room left still covers the price of what is left. The last chunk returns
    /// the rest of the room: nothing more can come.
    ///
    /// A chunk that does not end the reply must end at a query, as the
    /// decoder's do: the price counts one chunk per query and one more, so a
    /// chunk holding no query would take a chunk's bytes the price never set
    /// aside. Taking one panics in debug builds.
    pub(crate) fn chunk(&mut self, bytes: usize, last: bool) -> Charge {
        debug_assert!(
            last || self.ready.questions > 0,
            "a chunk that does not end its reply ends at a query",
        );
        self.ready = Ready::NONE;
        let charge = self.room.take(Holding::ParkedReplies, bytes);
        if last {
            drop(std::mem::take(&mut self.room));
        }
        charge
    }

    /// Return the room beyond what the reply can still deliver.
    ///
    /// The room covers that much at every moment, which is what lets every
    /// chunk take its whole price from it: it starts at the price, the price
    /// never grows, and each release returns only the excess.
    pub(crate) fn release(&mut self) {
        let beyond = self.room.bytes().saturating_sub(self.price());
        drop(self.room.split(Holding::ParkedReplies, beyond));
    }

    /// The bytes the reply's parked chunks can still hold.
    pub(crate) fn price(&self) -> usize {
        parked_price::<E>(self.deliverable())
    }

    /// The most the reply can still deliver to parked chunks, what it has taken
    /// that no chunk holds yet included.
    ///
    /// What is ready counts as it was taken; what can come is bounded by what
    /// the reply leaves of each limit ([`Left::deliverable`]). The slots of the
    /// children a later match covers are among the later reactions', whose
    /// bound and radixes both still count them.
    ///
    /// No term grows as the reply goes on. Each spend moves its slots,
    /// position, and entries from what can come to what is ready, and raises
    /// the floor, which takes radixes only from what can come; the match takes
    /// its covered children's radixes from what can come as it counts their
    /// slots as ready; a supplied node's child count only shrinks the listing
    /// left; and each chunk takes what is ready out of the price.
    pub(crate) fn deliverable(&self) -> Deliverable {
        self.left().deliverable(self.ready)
    }

    /// What the reply's later frames can still use of each limit.
    fn left(&self) -> Left {
        let radixes = self.floor.map_or(FAN, |floor| {
            usize::from(u8::MAX - floor) - self.covered.len_above(floor)
        });
        Left {
            reactions: (self.bound.reactions - self.slots).min(radixes),
            positions: self.positions,
            listing: self.listing_left(),
            matchable: !self.matched,
        }
    }

    /// The listing entries the reply's later queries can still carry: what the
    /// peer's grandchild count of the scope leaves beside what the reply has
    /// used of it.
    ///
    /// A conforming peer never exceeds it: its count sums its children's child
    /// counts, a matched or covered child's is this side's, and a supplied node
    /// is the peer's child or what the peer's deletion filter left of it.
    fn listing_left(&self) -> usize {
        self.bound.listing.saturating_sub(self.listing)
    }

    /// Check that a reaction at `radix` sits above the reply's latest one.
    ///
    /// A reply's reactions ascend by radix, its supplies among its positional
    /// reactions, and the price counts on that order: the reactions still to
    /// come number at most one per radix above the floor.
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
    /// child either side holds ([`ReplyBound`]): a reaction is about one such
    /// child, a covered child is one both sides hold, and no two of them share
    /// a radix.
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
