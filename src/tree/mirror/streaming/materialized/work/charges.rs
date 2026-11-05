//! What the walk's items hold, priced for its endpoint's ledger, and the
//! most each unit of admission can hold before it is built.
//!
//! Each price counts the allocations an item keeps while it waits where
//! the walk leaves it: its node in the channel it waits in
//! ([`queued`]) and the buffers of the vectors it owns, at their
//! capacities ([`buffer`]). A reservation is priced, before the reply it
//! covers exists, at the bound the peer's grandchild count sets
//! ([`ReplyBound`]), in the same element sizes the reply's items have
//! when they arrive.
//!
//! A reservation is three charges, one per place the reply's items take.
//! The query keeps two: the records part, which returns when the stage
//! has finished the reply, and the parent resolution's part, which moves
//! into that resolution and returns at its assembly. The third, the room
//! the reply's chunks park in ([`room`]), travels with the question's
//! record to the decoder, which charges each chunk its own part as it
//! decodes ([`parked_bytes`]) and returns whatever exceeds what the reply
//! can still deliver as soon as the reply shows it ([`room_beyond`]):
//! whatever the bound allowed beyond what the reply holds returns as the
//! reply narrows it, and each chunk's part as soon as its stage has used
//! it.

use std::mem::size_of;

use crate::tree::mirror::streaming::{
    Backend, ErasedNode, FAN, Leaf,
    erased::{Reaction, Reply},
    holding::Holding,
    ledger::{Charge, ReplyBound, Unit, buffer, queued, shared},
    materialized::{Error, Query, Reserved, Resolution, Resolve},
    message::{Carried, Grandchildren, Listed},
    remote::{Outstanding, Parked, RecordedChild, Scope},
};
use crate::tree::typed::{
    ErasedPrefix,
    height::{Height, Root, Z},
};

use super::LeafRequest;
#[cfg(any(test, feature = "test-internals"))]
use super::answer::{Planned, Question};

/// The bytes an outgoing reply holds while it waits for its consumer: its
/// slot in the response queue, its reactions, and the listing each query
/// carries.
fn reply_bytes<B>(reply: &Reply<B::Erased>) -> usize
where
    B: Backend<Node<Z>: Leaf>,
{
    let listings: usize = reply
        .reactions
        .iter()
        .map(|reaction| match reaction {
            Reaction::Query(listing) => buffer(listing),
            Reaction::Match | Reaction::Supply(..) => 0,
        })
        .sum();
    queued::<Result<Reply<B::Erased>, Error<B::Error>>>() + buffer(&reply.reactions) + listings
}

/// `reactions` as a whole outgoing reply, one last chunk, built out of
/// `grant`.
pub(super) fn reply<B>(grant: &mut Charge, reactions: Vec<Reaction<B::Erased>>) -> Reply<B::Erased>
where
    B: Backend<Node<Z>: Leaf>,
{
    chunk::<B>(grant, reactions, true, Carried::Nothing)
}

/// `reactions` as one outgoing chunk, ending its reply if `last`, built out
/// of `grant`, and carrying `carried` for its encoder: what the question it
/// ends at sets aside, if any.
pub(super) fn chunk<B>(
    grant: &mut Charge,
    reactions: Vec<Reaction<B::Erased>>,
    last: bool,
    carried: Carried,
) -> Reply<B::Erased>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut reply = Reply {
        reactions,
        charge: Charge::default(),
        last,
        carried,
    };
    reply.charge = grant.take(Holding::OutgoingReplies, reply_bytes::<B>(&reply));
    reply
}

/// The most a chunk of `reactions` reactions holds, its queries listing at
/// most `listing` entries together, before it is built.
fn chunk_bound<E, Err>(reactions: usize, listing: usize) -> usize {
    queued::<Result<Reply<E>, Error<Err>>>()
        + reactions * size_of::<Reaction<E>>()
        + listing * size_of::<Listed>()
}

/// A resolution of `resolved` at `prefix`, built out of `grant` for its
/// queue slot and slots, and keeping `copies`, the node copies exploding
/// its node made, until its assembly.
pub(super) fn resolution<E>(
    grant: &mut Charge,
    prefix: ErasedPrefix,
    resolved: Vec<(u8, Resolve<E>)>,
    copies: Charge,
) -> Resolution<E> {
    let bytes = queued::<Resolution<E>>() + buffer(&resolved);
    Resolution {
        prefix,
        resolved,
        charge: grant.take(Holding::Resolutions, bytes),
        copies,
    }
}

/// The part of a reservation the parent resolution takes: the resolution
/// the stage builds from the reply, a slot for each reaction the bound
/// allows, in its queue slot.
pub(crate) fn parent_resolution<E>(bound: ReplyBound) -> usize {
    queued::<Resolution<E>>() + bound.reactions * size_of::<(u8, Resolve<E>)>()
}

/// The part of a reservation the reply's chunks take while they park.
///
/// They are at most one chunk more than the reply's questions, which number
/// at most `listed.min(bound.reactions)` since a query sits only at a
/// position the asker listed, and they hold the reply's reactions and
/// listing entries.
///
/// The decoder charges each chunk its own part ([`parked_bytes`]), which
/// prices the chunk's vectors at their capacities. It allocates each
/// chunk's reactions, and each listing a query carries, at exactly their
/// lengths, so the chunks of a reply within its bound take no more than
/// this room together; a chunk that took more would be charged in full
/// and counted as a shortfall ([`Charge::take`]).
pub(crate) fn parked_room<E>(bound: ReplyBound, listed: usize) -> usize {
    parked_price::<E>(Deliverable {
        chunks: listed.min(bound.reactions) + 1,
        reactions: bound.reactions,
        listing: bound.listing,
    })
}

/// The most a reply can still deliver to its parked chunks: the chunks,
/// their reactions, and the listing entries their queries carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Deliverable {
    /// The chunks it can still park.
    pub(crate) chunks: usize,
    /// The reactions those chunks can hold together.
    pub(crate) reactions: usize,
    /// The listing entries their queries can carry together.
    pub(crate) listing: usize,
}

/// The bytes the parked chunks `still` describes can hold, priced as each
/// chunk is ([`parked_bytes`]).
pub(crate) fn parked_price<E>(still: Deliverable) -> usize {
    still.chunks * queued::<Parked<E>>()
        + still.reactions * size_of::<Reaction<E>>()
        + still.listing * size_of::<Listed>()
}

/// The bytes of a reply's room, `room`, beyond what the reply can still
/// deliver, `still`: what the room no longer needs to hold.
///
/// The decoder releases exactly this as the reply narrows what it can
/// still deliver, so the room always covers what is left and the receive
/// path never waits.
pub(crate) fn room_beyond<E>(room: usize, still: Deliverable) -> usize {
    room.saturating_sub(parked_price::<E>(still))
}

/// The bytes a decoded chunk holds while it parks: its node in the parking
/// queue, its reactions, and the listing each of its queries carries, every
/// vector at its capacity.
///
/// The decoder takes this from the room the question set aside for the
/// chunk's reply ([`room`]) as the chunk decodes, and the chunk returns it
/// once its stage has used the chunk.
pub(crate) fn parked_bytes<E>(chunk: &Reply<E>) -> usize {
    let listings: usize = chunk
        .reactions
        .iter()
        .map(|reaction| match reaction {
            Reaction::Query(listing) => buffer(listing),
            Reaction::Match | Reaction::Supply(..) => 0,
        })
        .sum();
    queued::<Parked<E>>() + buffer(&chunk.reactions) + listings
}

/// The part of a reservation the records the take derives take: one queue
/// slot per question the reply can ask, beside a recorded child per entry
/// of the reply's listings.
pub(crate) fn records_room(bound: ReplyBound, listed: usize) -> usize {
    listed.min(bound.reactions) * queued::<Scope>() + bound.listing * size_of::<RecordedChild>()
}

/// The room a question listing `listed` children sets aside for the peer's
/// reply bounded by `bound`: the reply's chunks parked, the records its take
/// derives, and the parent resolution the stage builds.
pub(crate) fn reservation<E>(bound: ReplyBound, listed: usize) -> usize {
    parked_room::<E>(bound, listed) + records_room(bound, listed) + parent_resolution::<E>(bound)
}

/// The parts of the room, built out of `grant`, a question listing
/// `listed` children sets aside for the reply `bound` bounds that its query
/// keeps, each charged to the place its items take.
fn reserve<E>(grant: &mut Charge, bound: ReplyBound, listed: usize) -> Reserved {
    Reserved {
        records: grant.take(Holding::QuestionRecords, records_room(bound, listed)),
        slots: grant.take(Holding::Resolutions, parent_resolution::<E>(bound)),
    }
}

/// The room, built out of `grant`, the chunks of the reply `bound` bounds
/// take while they park, for a question listing `listed` children
/// ([`parked_room`]).
///
/// The chunk ending at the question carries it to the encoder, which
/// publishes it with the question's record ([`Carried::Question`]).
pub(super) fn room<E>(grant: &mut Charge, bound: ReplyBound, listed: usize) -> Charge {
    grant.take(Holding::ParkedReplies, parked_room::<E>(bound, listed))
}

/// The bytes the record of a question listing `listed` children holds.
///
/// The record waits in the decoder's queue beside its charge and its
/// reply's room ([`Outstanding`]), then in the allocation the decoder
/// shares with the take until the take reads its reply's last chunk, so it
/// is priced at the larger of the two, beside one radix and count per
/// listed child.
pub(crate) fn record_bytes(listed: usize) -> usize {
    queued::<Outstanding>().max(shared::<Scope>()) + listed * size_of::<RecordedChild>()
}

/// The charge, built out of `grant`, for the record of a question listing
/// `listed` children, which the chunk carrying the question hands its
/// encoder ([`record_bytes`]).
pub(super) fn record(grant: &mut Charge, listed: usize) -> Charge {
    grant.take(Holding::QuestionRecords, record_bytes(listed))
}

/// The question about `prefix` that lists `ours`, whose reply `bound`
/// bounds, built out of `grant`.
///
/// It is charged for what it holds, `copies` of node copies listing its
/// children made included, and for the parts of its reply's reservation it
/// keeps ([`reserve`]).
pub(super) fn query<E>(
    grant: &mut Charge,
    prefix: ErasedPrefix,
    ours: Vec<(u8, E)>,
    bound: ReplyBound,
    copies: usize,
) -> Query<E> {
    let held = queued::<Query<E>>() + buffer(&ours);
    Query {
        prefix,
        charge: grant.take(Holding::Questions, held),
        copies: grant.take(Holding::ExplodedNodes, copies),
        reservation: reserve::<E>(grant, bound, ours.len()),
        ours,
        bound,
    }
}

/// The charges, built out of `grant`, for one leaf request's queue slot,
/// the parts of its reply's reservation it keeps, and the room its reply,
/// one supplied leaf, parks in ([`room`]).
pub(super) fn leaf_request<E>(grant: &mut Charge) -> (Charge, Reserved, Charge) {
    (
        grant.take(Holding::Questions, queued::<LeafRequest>()),
        reserve::<E>(grant, ReplyBound::LEAF, 0),
        room::<E>(grant, ReplyBound::LEAF, 0),
    )
}

/// The most an answer's own unit holds, before its scope is explored.
///
/// That is the copies exploding its scope makes, `copies`, its resolution,
/// with a slot for each child either side holds, and its reactions that ask
/// nothing, one per such child at most. This side holds `ours` children of
/// the scope and the peer listed `theirs`; their union is at most their sum
/// and, since each sits at its own radix, at most a fan
/// ([`answer_children`]).
pub(crate) fn answer_bound<E, Err>(ours: usize, theirs: usize, copies: usize) -> usize {
    let children = answer_children(ours, theirs);
    // A chunk's reactions keep one slot spare for a question
    // ([`Chunks::next_part`](super::answer::Chunks::next_part)).
    copies
        + queued::<Resolution<E>>()
        + children * size_of::<(u8, Resolve<E>)>()
        + chunk_bound::<E, Err>(children + 1, 0)
}

/// The most children an answer's scope can hold when this side holds
/// `ours` of them and the peer listed `theirs`: their sum, at most a fan.
///
/// The sum is exact when the two sides share no child of the scope and
/// counts each shared child twice otherwise. The exact union needs this
/// side's radices, which only exploding the node lists, and the answer's
/// grant prices that explosion, so it is admitted before them.
pub(crate) fn answer_children(ours: usize, theirs: usize) -> usize {
    (ours + theirs).min(FAN)
}

/// The most a question's unit holds, before it is built.
///
/// That is a query listing at most `listed` of this side's children, the
/// copies exploding the child asked about makes, `copies`, the reservation
/// for the reply that the peer's count of the child, `theirs`, bounds at
/// the child's typed `height`, the chunk ending at the question behind
/// `leading` reactions, and the record.
pub(crate) fn question_bound<E, Err>(
    height: usize,
    theirs: Grandchildren,
    copies: usize,
    leading: usize,
    listed: usize,
) -> usize {
    queued::<Query<E>>()
        + listed * size_of::<(u8, E)>()
        + copies
        + reservation::<E>(ReplyBound::of(height, listed, theirs), listed)
        + chunk_bound::<E, Err>(leading + 1, listed)
        + record_bytes(listed)
}

/// The most a leaf request's unit holds, before it is built: the request
/// with the room for its reply, the chunk ending at it behind `leading`
/// reactions, and its record.
pub(crate) fn leaf_question_bound<E, Err>(leading: usize) -> usize {
    queued::<LeafRequest>()
        + reservation::<E>(ReplyBound::LEAF, 0)
        + chunk_bound::<E, Err>(leading + 1, 0)
        + record_bytes(0)
}

/// The most a whole reply of at most `reactions` reactions that asks
/// nothing holds, with `copies` of node copies its building makes: the unit
/// of an answer that supplies a whole node, or a leaf.
pub(crate) fn supply_bound<E, Err>(reactions: usize, copies: usize) -> usize {
    copies + chunk_bound::<E, Err>(reactions, 0)
}

/// The most the grant of a unit of `unit` kind concerning a scope `depth`
/// bytes deep holds, in a walk over nodes `E` whose backend fails with
/// `Err`: a bound on what that depth's floor slot for the kind holds.
///
/// Each bound grows with each of its arguments, so each slot is bounded at
/// the largest arguments the walk passes a unit of its kind at its depth:
///
/// - The one question about the root is the opening question, which no
///   reaction leads in its chunk.
/// - Every other question lists at most a fan of children, about a node
///   whose peer counts at most [`Grandchildren::MAX`] grandchildren, behind
///   at most two fans of reactions, the children this side and the peer's
///   listing can hold between them. A question about a leaf is a leaf
///   request.
/// - An answer explores its scope, against a listing of at most a fan of
///   the peer's children, or supplies at most a fan of children; an answer
///   about a leaf supplies that one leaf.
/// - Every explosion, of the scope an answer explores or of the child a
///   question asks about, copies at most
///   [`ErasedNode::MAX_EXPLOSION_BYTES`].
fn slot_bound<E: ErasedNode, Err>(unit: Unit, depth: usize) -> usize {
    let copies = E::MAX_EXPLOSION_BYTES;
    let leading = 2 * FAN;
    let height = Root::HEIGHT - depth;
    match unit {
        Unit::Question if depth == 0 => {
            question_bound::<E, Err>(height, Grandchildren::MAX, copies, 0, FAN)
        }
        Unit::Question if height == 0 => leaf_question_bound::<E, Err>(leading),
        Unit::Question => {
            question_bound::<E, Err>(height, Grandchildren::MAX, copies, leading, FAN)
        }
        Unit::Answer if height == 0 => supply_bound::<E, Err>(1, 0),
        Unit::Answer => {
            answer_bound::<E, Err>(FAN, FAN, copies).max(supply_bound::<E, Err>(FAN, copies))
        }
    }
}

/// The *floor allowance*: the most a session's floor slots can hold at
/// once, in a walk over nodes `E` whose backend fails with `Err`, which a
/// session sets aside out of its memory budget for them.
///
/// A floor slot holds one grant of its kind and depth at a time, its own
/// unit's or what it carries for units admitted against the budget, and
/// frees only once every charge built out of that grant is released, as
/// the ledger admits units
/// ([`Ledger::admit`](crate::tree::mirror::streaming::ledger::Ledger::admit)),
/// so a set of slots holds at most the sum of their largest grants
/// ([`slot_bound`]).
///
/// Each side of a session uses half its slots. A question about a scope is
/// asked by an answer about the scope's parent, on the same side, and
/// answered on the other side, whose answers about the scope's children
/// ask its next questions. The initiator asks the opening question, about
/// the root at depth 0, so it asks only about scopes at even depths and
/// answers only at odd ones; the responder asks at odd depths and answers
/// at even ones. A ledger is built before its session's handshake settles
/// which side it is, so the allowance is the larger of the two sides'
/// sums.
///
/// On the in-memory backend on 64-bit targets that is about 34 MiB, almost
/// all of it the listings of the replies one question per depth can invite,
/// a fan of fans of entries each.
pub(crate) fn floor_allowance<E: ErasedNode, Err>() -> usize {
    // Indexed by side: the initiator's sum, then the responder's.
    let mut sides = [0, 0];
    for depth in 0..=Root::HEIGHT {
        let asker = depth % 2;
        sides[asker] += slot_bound::<E, Err>(Unit::Question, depth);
        sides[1 - asker] += slot_bound::<E, Err>(Unit::Answer, depth);
    }
    sides[0].max(sides[1])
}

/// The most one logical stream's stage, encoder, and decoder hold for
/// `place` beyond its ledger charges at once.
///
/// These are the buffers they fill while they build items they charge only
/// once built, or keep a moment after their charge returns. Every term
/// counts requested bytes, as the ledger's charges and the allocation meter
/// both do, so the allocator's rounding enters neither side of the
/// comparison it allows for.
///
/// Each term is computed from the types it covers, so it follows them:
///
/// - Every channel keeps the node of the last item taken from it as its
///   queue's stub until the channel drops, after that item's charge has
///   returned: one node of the place's item type per channel of the place.
///   A stream's questions travel one channel; its resolutions two, the
///   child and the parent ones; its outgoing replies one; its parked chunks
///   one; its records two, the decoder's queue and the take's scopes.
/// - [`Holding::Resolutions`]: an answer's plan, one entry per child either
///   side holds, at most two fans, in a vector at most twice its length.
/// - [`Holding::OutgoingReplies`]: a chunk's reactions, allocated before
///   its question is admitted with one slot spare, at most two fans and
///   one; and a question's listing, a fan of entries, built before the
///   chunk carrying it is charged.
/// - [`Holding::ParkedReplies`]: a decoder's ready reactions and its open
///   group's runs, a fan of each, in vectors at most twice their length.
/// - [`Holding::QuestionRecords`]: a take's derived scopes for one chunk, a
///   fan, in a vector at most twice its length; and the record an encoder
///   still holds for the reply it writes after its asker's reservation
///   returned, a scope with a fan of radices and counts.
///
/// The exploded nodes' place is charged before each explosion the walk
/// makes; the greeting's copy of the root, made before any unit is
/// admitted, is the one copy no unit charges.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn scratch<E, Err>(place: Holding) -> usize {
    match place {
        Holding::Questions => queued::<Query<E>>().max(queued::<LeafRequest>()),
        Holding::Resolutions => {
            2 * queued::<Resolution<E>>() + 2 * 2 * FAN * size_of::<Planned<E, Question<E>>>()
        }
        Holding::OutgoingReplies => {
            queued::<Result<Reply<E>, Error<Err>>>()
                + (2 * FAN + 1) * size_of::<Reaction<E>>()
                + FAN * size_of::<Listed>()
        }
        Holding::ParkedReplies => {
            queued::<Parked<E>>()
                + 2 * FAN * (size_of::<Reaction<E>>() + size_of::<(u8, ErasedPrefix)>())
        }
        Holding::QuestionRecords => {
            queued::<Outstanding>()
                + queued::<Scope>()
                + 2 * FAN * size_of::<Scope>()
                + size_of::<Scope>()
                + FAN * size_of::<RecordedChild>()
        }
        _ => 0,
    }
}
