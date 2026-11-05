//! What the walk's items hold, priced for its endpoint's ledger.
//!
//! Each price counts the allocations an item keeps while it waits where
//! the walk leaves it: its node in the channel it waits in
//! ([`queued`]) and the buffers of the vectors it owns, at their
//! capacities ([`buffer`]). A reservation is priced, before the reply it
//! covers exists, at the bound the peer's grandchild count sets
//! ([`ReplyBound`]), in the same element sizes the reply's items have
//! when they arrive.

use std::mem::size_of;

use crate::tree::mirror::streaming::{
    Backend, Leaf,
    erased::{Reaction, Reply},
    holding::Holding,
    ledger::{Charge, Ledger, ReplyBound, buffer, queued},
    materialized::{Error, Query, Resolution, Resolve},
    message::{Grandchildren, Listed},
    remote::{Parked, Scope},
};
use crate::tree::typed::{ErasedPrefix, height::Z};

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

/// `reactions` as a whole outgoing reply, one last chunk, charged to
/// `ledger` for what it holds.
pub(super) fn reply<B>(ledger: &Ledger, reactions: Vec<Reaction<B::Erased>>) -> Reply<B::Erased>
where
    B: Backend<Node<Z>: Leaf>,
{
    chunk::<B>(ledger, reactions, true)
}

/// `reactions` as one outgoing chunk, ending its reply if `last`, charged
/// to `ledger` for what it holds.
pub(super) fn chunk<B>(
    ledger: &Ledger,
    reactions: Vec<Reaction<B::Erased>>,
    last: bool,
) -> Reply<B::Erased>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut reply = Reply {
        reactions,
        charge: Charge::default(),
        last,
    };
    reply.charge = ledger.charge(Holding::OutgoingReplies, reply_bytes::<B>(&reply));
    reply
}

/// A resolution of `resolved` at `prefix`, charged to `ledger` for its
/// queue slot and slots, and keeping `kept` until its assembly.
pub(super) fn resolution<E>(
    ledger: &Ledger,
    prefix: ErasedPrefix,
    resolved: Vec<(u8, Resolve<E>)>,
    kept: Charge,
) -> Resolution<E> {
    let bytes = queued::<Resolution<E>>() + buffer(&resolved);
    Resolution {
        prefix,
        resolved,
        charge: ledger.charge(Holding::Resolutions, bytes),
        kept,
    }
}

/// The part of a reservation the parent resolution takes: the resolution
/// the stage builds from the reply, a slot for each reaction the bound
/// allows, in its queue slot.
pub(crate) fn parent_resolution<E>(bound: ReplyBound) -> usize {
    queued::<Resolution<E>>() + bound.reactions * size_of::<(u8, Resolve<E>)>()
}

/// The room a question listing `listed` children sets aside for the peer's
/// reply bounded by `bound`: the reply's chunks parked, the records its take
/// derives from its queries, and the parent resolution the stage builds.
///
/// A query sits only at a position the asker listed, so the reply asks at
/// most `listed.min(bound.reactions)` questions; the decoder parks at most
/// one more chunk than that, and the records together hold at most the
/// reply's listing entries.
pub(crate) fn reservation<E>(bound: ReplyBound, listed: usize) -> usize {
    let questions = listed.min(bound.reactions);
    let parked = (questions + 1) * queued::<Parked<E>>()
        + bound.reactions * size_of::<Reaction<E>>()
        + bound.listing * size_of::<Listed>();
    let records = questions * queued::<Scope>() + bound.listing * size_of::<(u8, Grandchildren)>();
    parked + records + parent_resolution::<E>(bound)
}

/// The question about `prefix` that lists `ours`, whose reply `bound`
/// bounds, charged to `ledger` for what it holds, `copies` of node copies
/// listing its children made included, and for its reply's reservation.
pub(super) fn query<E>(
    ledger: &Ledger,
    prefix: ErasedPrefix,
    ours: Vec<(u8, E)>,
    bound: ReplyBound,
    copies: usize,
) -> Query<E> {
    let held = queued::<Query<E>>() + buffer(&ours) + copies;
    let room = reservation::<E>(bound, ours.len());
    Query {
        prefix,
        ours,
        bound,
        charge: ledger.charge(Holding::Questions, held),
        reservation: ledger.charge(Holding::ParkedReplies, room),
    }
}

/// The charge for one leaf request in its queue, with the room its reply,
/// one supplied leaf, sets aside.
pub(super) fn leaf_request<E>(ledger: &Ledger, request: usize) -> Charge {
    let bytes = request + reservation::<E>(ReplyBound::LEAF, 0);
    ledger.charge(Holding::ParkedReplies, bytes)
}
