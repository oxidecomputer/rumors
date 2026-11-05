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
    byte_set::ByteSet,
    erased::{Reaction, Reply},
    holding::Holding,
    ledger::{Charge, ReplyBound, Unit, buffer, max, min, queued, shared},
    materialized::{Error, Query, Reserved, Resolution, Resolve},
    message::{Carried, Grandchildren, LATER_MATCHES_HEAP_BYTES, Listed},
    remote::{Outstanding, Parked, RecordedChild, Scope},
};
use crate::tree::typed::{
    ErasedPrefix,
    height::{self, Height, Z},
    untyped,
};

use super::LeafRequest;
use super::answer::{Held, Planned, Question};

/// The bytes the reactions own beyond their own slots: the listing each
/// query carries, at its capacity, and the set a match carries of the later
/// children it covers.
fn owned_bytes<E>(reactions: &[Reaction<E>]) -> usize {
    reactions
        .iter()
        .map(|reaction| match reaction {
            Reaction::Query(listing) => buffer(listing),
            Reaction::Match(later) => later.heap_bytes(),
            Reaction::Supply(..) => 0,
        })
        .sum()
}

/// The bytes an outgoing reply holds while it waits for its consumer: its
/// slot in the response queue, its reactions, and what they own
/// ([`owned_bytes`]).
fn reply_bytes<B>(reply: &Reply<B::Erased>) -> usize
where
    B: Backend<Node<Z>: Leaf>,
{
    queued::<Result<Reply<B::Erased>, Error<B::Error>>>()
        + buffer(&reply.reactions)
        + owned_bytes(&reply.reactions)
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

/// The most a chunk holding `held` reactions and covered sets holds, its
/// queries listing at most `listing` entries together, before it is built.
const fn chunk_bound<E, Err>(held: Held, listing: usize) -> usize {
    queued::<Result<Reply<E>, Error<Err>>>()
        + held.reactions * size_of::<Reaction<E>>()
        + held.sets * LATER_MATCHES_HEAP_BYTES
        + listing * size_of::<Listed>()
}

/// What `reactions` hold before their chunk is built: their count, and the
/// covered sets among them.
pub(super) fn held<E>(reactions: &[Reaction<E>]) -> Held {
    Held {
        reactions: reactions.len(),
        sets: reactions
            .iter()
            .filter(|reaction| matches!(reaction, Reaction::Match(later) if later.get().is_some()))
            .count(),
    }
}

/// `held` with one slot spare for the question its chunk ends at, as every
/// chunk's reactions keep one ([`Chunks::next_part`](super::answer::Chunks::next_part)).
const fn with_question(held: Held) -> Held {
    Held {
        reactions: held.reactions + 1,
        sets: held.sets,
    }
}

/// A resolution of `resolved` at `prefix`, exploded from `original`, built
/// out of `grant` for its queue slot and slots.
pub(super) fn resolution<E>(
    grant: &mut Charge,
    prefix: ErasedPrefix,
    original: Option<E>,
    resolved: Vec<(u8, Resolve<E>)>,
) -> Resolution<E> {
    let bytes = queued::<Resolution<E>>() + buffer(&resolved);
    Resolution {
        prefix,
        resolved,
        original,
        charge: grant.take(Holding::Resolutions, bytes),
    }
}

/// The part of a reservation the parent resolution takes: the resolution
/// the stage builds from the reply, a slot for each reaction the bound
/// allows, in its queue slot.
pub(crate) const fn parent_resolution<E>(bound: ReplyBound) -> usize {
    queued::<Resolution<E>>() + bound.reactions * size_of::<(u8, Resolve<E>)>()
}

/// The part of a reservation the reply's chunks take while they park.
///
/// They are at most one chunk more than the reply's questions, which number
/// at most `listed.min(bound.reactions)` since a query sits only at a
/// position the asker listed, and they hold the reply's reactions and
/// listing entries, and the set of later matches its one match covers,
/// which needs a listed child for the match and one more to cover.
///
/// The decoder charges each chunk its own part ([`parked_bytes`]), which
/// prices the chunk's vectors at their capacities. It allocates each
/// chunk's reactions, and each listing a query carries, at exactly their
/// lengths, so the chunks of a reply within its bound take no more than
/// this room together; a chunk that took more would be charged in full
/// and counted as a shortfall ([`Charge::take`]).
pub(crate) const fn parked_room<E>(bound: ReplyBound, listed: usize) -> usize {
    parked_price::<E>(Deliverable {
        chunks: min(listed, bound.reactions) + 1,
        reactions: bound.reactions,
        listing: bound.listing,
        sets: covered_sets(listed),
    })
}

/// The covered sets a reply about `listed` of the asker's children can
/// carry: its one match's, if it can match one child and cover another.
const fn covered_sets(listed: usize) -> usize {
    if listed >= 2 { 1 } else { 0 }
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
    /// The covered sets their match can carry: at most one, a reply
    /// carrying at most one match.
    pub(crate) sets: usize,
}

/// The bytes the parked chunks `still` describes can hold, priced as each
/// chunk is ([`parked_bytes`]).
pub(crate) const fn parked_price<E>(still: Deliverable) -> usize {
    still.chunks * queued::<Parked<E>>()
        + still.reactions * size_of::<Reaction<E>>()
        + still.sets * LATER_MATCHES_HEAP_BYTES
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

/// The bytes a decoded chunk holds while it parks.
///
/// That is its node in the parking queue, its reactions, and what they own
/// ([`owned_bytes`]): the listing each of its queries carries and the set
/// its match covers, every vector at its capacity.
///
/// The decoder takes this from the room the question set aside for the
/// chunk's reply ([`room`]) as the chunk decodes, and the chunk returns it
/// once its stage has used the chunk.
pub(crate) fn parked_bytes<E>(chunk: &Reply<E>) -> usize {
    queued::<Parked<E>>() + buffer(&chunk.reactions) + owned_bytes(&chunk.reactions)
}

/// The bytes the take's record of one question a reply asks holds while it
/// waits, the question listing `listing` entries in a reply about a node
/// at `height`.
///
/// That is its node in the channel it waits in, and one recorded child per
/// listing entry, or, at height 1, a leaf's parent, the one leaf the
/// question requests, its listing being empty.
///
/// The take derives each record, a [`Scope`], at exactly that many
/// children.
pub(crate) const fn derived_record_bytes(height: usize, listing: usize) -> usize {
    let recorded = if height == 1 { 1 } else { listing };
    queued::<Scope>() + recorded * size_of::<RecordedChild>()
}

/// The part of a reservation the records the take derives take: the most
/// [`derived_record_bytes`] can sum to over a reply to a question about a
/// node at `height` that lists `listed` children, bounded by `bound`.
///
/// That is one queue slot per question the reply can ask, beside a
/// recorded child per entry of the reply's listings, or, about a leaf's
/// parent, one per question: the leaf it requests.
pub(crate) const fn records_room(height: usize, bound: ReplyBound, listed: usize) -> usize {
    let questions = min(listed, bound.reactions);
    let recorded = if height == 1 {
        questions
    } else {
        bound.listing
    };
    questions * queued::<Scope>() + recorded * size_of::<RecordedChild>()
}

/// The room a question about a node at `height` listing `listed` children
/// sets aside for the peer's reply bounded by `bound`.
///
/// That is the reply's chunks parked, the records its take derives, and
/// the parent resolution the stage builds.
pub(crate) const fn reservation<E>(height: usize, bound: ReplyBound, listed: usize) -> usize {
    parked_room::<E>(bound, listed)
        + records_room(height, bound, listed)
        + parent_resolution::<E>(bound)
}

/// The parts of the room, built out of `grant`, a question about a node at
/// `height` listing `listed` children sets aside for the reply `bound`
/// bounds that its query keeps, each charged to the place its items take.
fn reserve<E>(grant: &mut Charge, height: usize, bound: ReplyBound, listed: usize) -> Reserved {
    Reserved {
        records: grant.take(
            Holding::QuestionRecords,
            records_room(height, bound, listed),
        ),
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
pub(crate) const fn record_bytes(listed: usize) -> usize {
    max(queued::<Outstanding>(), shared::<Scope>()) + listed * size_of::<RecordedChild>()
}

/// The charge, built out of `grant`, for the record of a question listing
/// `listed` children, which the chunk carrying the question hands its
/// encoder ([`record_bytes`]).
pub(super) fn record(grant: &mut Charge, listed: usize) -> Charge {
    grant.take(Holding::QuestionRecords, record_bytes(listed))
}

/// The question about `prefix` that lists `ours`, the children of
/// `original`, whose reply `bound` bounds, built out of `grant`.
///
/// It is charged for what it holds and for the parts of its reply's
/// reservation it keeps ([`reserve`]). Its handle to `original` sits inline in its queue
/// slot and allocates nothing: it keeps alive a node the walk already held,
/// the one it listed `ours` from.
pub(super) fn query<E>(
    grant: &mut Charge,
    prefix: ErasedPrefix,
    original: Option<E>,
    ours: Vec<(u8, E)>,
    bound: ReplyBound,
) -> Query<E> {
    let held = queued::<Query<E>>() + buffer(&ours);
    Query {
        prefix,
        charge: grant.take(Holding::Questions, held),
        reservation: reserve::<E>(grant, prefix.height(), bound, ours.len()),
        ours,
        original,
        bound,
    }
}

/// The charges, built out of `grant`, for one leaf request's queue slot,
/// the parts of its reply's reservation it keeps, and the room its reply,
/// one supplied leaf, parks in ([`room`]).
pub(super) fn leaf_request<E>(grant: &mut Charge) -> (Charge, Reserved, Charge) {
    (
        grant.take(Holding::Questions, queued::<LeafRequest>()),
        reserve::<E>(grant, 0, ReplyBound::LEAF, 0),
        room::<E>(grant, ReplyBound::LEAF, 0),
    )
}

/// The most an answer's own unit holds, before its scope is explored.
///
/// That is its resolution, with a slot for each of the `children` either side holds, its plan, with
/// an entry for each such child at most, and its reactions that ask
/// nothing, one per such child at most ([`answer_children`] counts them),
/// with the set of later matches its match covers, if the `shared` children
/// both sides hold, the only ones it can match, number two or more
/// ([`shared_children`] counts them).
///
/// A plan entry is priced at an internal answer's: a leaf parent's, which
/// plans each question by its radix alone, is no larger.
pub(crate) const fn answer_bound<E, Err>(children: usize, shared: usize) -> usize {
    // A chunk's reactions keep one slot spare for a question
    // ([`Chunks::next_part`](super::answer::Chunks::next_part)).
    queued::<Resolution<E>>()
        + children * size_of::<(u8, Resolve<E>)>()
        + children * size_of::<Planned<E, Question<E>>>()
        + chunk_bound::<E, Err>(
            with_question(Held {
                reactions: children,
                sets: covered_sets(shared),
            }),
            0,
        )
}

/// A child a supply answer lists is no larger than the supply reaction its
/// chunk charges for it, for the in-memory backend's nodes.
///
/// That is the premise of the session-budget suite's floor on outgoing
/// replies' charges (`testing::OUTGOING_REPLIES_FLOOR_DIVISOR`).
const _: () = assert!(
    size_of::<(u8, untyped::Node)>() <= size_of::<Reaction<untyped::Node>>(),
    "a listed child must be no larger than its supply reaction",
);

/// A leaf parent's plan entry is no larger than an internal answer's, at
/// which [`answer_bound`] prices both, for the in-memory backend's nodes.
const _: () = assert!(
    size_of::<Planned<untyped::Node, u8>>()
        <= size_of::<Planned<untyped::Node, Question<untyped::Node>>>(),
    "a leaf parent's plan entry must fit an internal answer's price",
);

/// What an answer's grant must still hold once its resolution and plan are
/// built.
///
/// That is the reply's last chunk if it asks nothing, holding `unasked`
/// reactions and covered sets
/// ([`Chunks::unasked_last`](super::answer::Chunks::unasked_last)), and
/// nothing if it ends at a question, whose own grant builds it.
///
/// The chunk's reactions keep one slot spare, as every chunk's do.
pub(super) fn unasked_chunk_bound<E, Err>(unasked: Option<Held>) -> usize {
    unasked.map_or(0, |held| chunk_bound::<E, Err>(with_question(held), 0))
}

/// The children both sides hold of an answer's scope, when this side holds
/// the children at `ours` and the peer listed `theirs`: the children the
/// answer can match, exact.
///
/// Each side's children sit at distinct radices, so the children either
/// holds ([`answer_children`]) count each shared child once.
pub(crate) fn shared_children(ours: ByteSet, theirs: &[Listed]) -> usize {
    ours.len() + theirs.len() - answer_children(ours, theirs)
}

/// The children an answer's scope holds when this side holds the children
/// at `ours` and the peer listed `theirs`: the size of their union, exact,
/// and so at most a fan.
///
/// A child both sides hold sits at one radix, so it counts once. The walk
/// reads this side's radices from the backend before it explodes the node
/// ([`ErasedNode::child_radices`]), since the answer's grant prices that
/// explosion.
pub(crate) fn answer_children(ours: ByteSet, theirs: &[Listed]) -> usize {
    ours.union(theirs.iter().map(|listed| listed.radix).collect())
        .len()
}

/// The most a question's unit holds, before it is built.
///
/// That is a query listing at most `listed` of this side's children, the
/// reservation
/// for the reply that the peer's count of the child, `theirs`, bounds at
/// the child's typed `height`, the chunk ending at the question behind the
/// `leading` reactions and covered sets, and the record.
pub(crate) const fn question_bound<E, Err>(
    height: usize,
    theirs: Grandchildren,
    leading: Held,
    listed: usize,
) -> usize {
    queued::<Query<E>>()
        + listed * size_of::<(u8, E)>()
        + reservation::<E>(height, ReplyBound::of(height, listed, theirs), listed)
        + chunk_bound::<E, Err>(with_question(leading), listed)
        + record_bytes(listed)
}

/// The most a leaf request's unit holds, before it is built: the request
/// with the room for its reply, the chunk ending at it behind the `leading`
/// reactions and covered sets, and its record.
pub(crate) const fn leaf_question_bound<E, Err>(leading: Held) -> usize {
    queued::<LeafRequest>()
        + reservation::<E>(0, ReplyBound::LEAF, 0)
        + chunk_bound::<E, Err>(with_question(leading), 0)
        + record_bytes(0)
}

/// The need of a question whose reply lies within `bound`: the bytes
/// processing the reply is predicted to admit one level below the node it
/// is about.
///
/// The question's claim in the invitation share weighs it
/// ([`Ledger::invite`](crate::tree::mirror::streaming::ledger::Ledger::invite)).
///
/// Processing a reply builds an answer one level below the node for each
/// query the reply carries, each answering at least the children its query
/// lists, and the listing entries of all its queries together are at most
/// `bound.listing`. What those answers take beyond their entries depends
/// on this side's children of the node's children, which the walk reads
/// only as it builds the question, after
/// admitting it. Until then only the type maximum bounds the answers from
/// above, so the prediction is the least they can be granted when the
/// reply lists every entry its bound allows: as few answers as can hold the
/// entries, a fan each, each answering only the children its entries name. It is a lower bound on what any such reply's
/// processing admits there.
///
/// The questions processing sends two levels down are not part of it: each
/// claims its own room and need at its own depth when it is asked, so no
/// byte counts toward two depths' claims. They are priced by the peer's
/// counts of the grandchildren, which only the reply carries.
///
/// A reply that lists nothing, about a leaf's parent or answering a request,
/// is predicted to need nothing; the bound already says so
/// ([`ReplyBound::of`]), so the node's height adds nothing. The prediction falls short of what
/// processing keeps wherever this side holds children the peer lacks, and
/// exceeds it wherever the reply lists less
/// than its bound allows or the answers return room they did not use; the
/// share's pace corrects it from what the answers keep.
pub(crate) const fn processing_need<E, Err>(bound: ReplyBound) -> usize {
    let entries = bound.listing;
    if entries == 0 {
        return 0;
    }
    // An answer's bound grows by the same bytes with each child it answers.
    let answer = answer_bound::<E, Err>(0, 0);
    let per_child = answer_bound::<E, Err>(1, 0) - answer;
    entries.div_ceil(FAN) * answer + entries * per_child
}

/// The most a whole reply of at most `reactions` reactions that asks
/// nothing holds: the unit of an answer that supplies a whole node, or a
/// leaf.
pub(crate) const fn supply_bound<E, Err>(reactions: usize) -> usize {
    chunk_bound::<E, Err>(Held { reactions, sets: 0 }, 0)
}

/// The most any unit's grant holds, in a walk over nodes `E` whose backend
/// fails with `Err`: a bound on every charge, since every charge is a grant
/// or a part of one.
///
/// Each bound above grows with each of its arguments, so each kind of unit
/// is bounded at the largest arguments the walk can pass it:
///
/// - Each question lists at most a fan of children
///   ([`ErasedNode::child_count`]), and the peer's count of the child it
///   asks about is at most [`Grandchildren::MAX`], 65,536, so its reply is
///   bounded at a fan of reactions and 65,536 listing entries. Any height
///   above the leaves' parents bounds a reply that much; the root's is one.
/// - A chunk's reactions ahead of its question are entries of its answer's
///   plan, one per child either side holds. Each side lists at most a fan,
///   as this side's nodes hold and as the peer's listings decode, in
///   strictly ascending radices, so at most two fans lead a question, and
///   the answer's one match among them may carry a covered set.
/// - An answer's scope holds at most a fan of children, and a supplying
///   answer supplies at most a fan.
///
/// The question at those arguments dominates, at about 1.6 MiB on the
/// in-memory backend, under a thousandth of `u32::MAX`: its listing entries
/// are the one term that grows past a fan. The walk asserts at its construction that this fits the four
/// bytes a charge counts in ([`assert_grants_fit`]).
pub(crate) const fn largest_grant<E: ErasedNode, Err>() -> usize {
    let leading = Held {
        reactions: 2 * FAN,
        sets: 1,
    };
    let question = question_bound::<E, Err>(height::Root::HEIGHT, Grandchildren::MAX, leading, FAN);
    let leaf_question = leaf_question_bound::<E, Err>(leading);
    let answer = answer_bound::<E, Err>(FAN, FAN);
    let supply = supply_bound::<E, Err>(FAN);
    max(max(question, leaf_question), max(answer, supply))
}

/// The most the grant of a unit of `unit` kind concerning a scope `depth`
/// bytes deep holds, in a walk over nodes `E` whose backend fails with
/// `Err`: a bound on what that depth's floor slot for the kind holds.
///
/// Each bound grows with each of its arguments, so each slot is bounded at
/// the largest arguments the walk passes a unit of its kind at its depth,
/// as [`largest_grant`] argues them:
///
/// - The one question about the root is the opening question, which no
///   reaction leads in its chunk.
/// - Every other question lists at most a fan of children, about a node
///   whose peer counts at most [`Grandchildren::MAX`] grandchildren, behind
///   at most two fans of reactions, one of them a match that may carry a
///   covered set. A question about a leaf is a leaf request.
/// - An answer explores its scope, at most a fan of children either side
///   holds, or supplies at most a fan of them; an answer about a leaf
///   supplies that one leaf.
const fn slot_bound<E: ErasedNode, Err>(unit: Unit, depth: usize) -> usize {
    let leading = Held {
        reactions: 2 * FAN,
        sets: 1,
    };
    let height = height::Root::HEIGHT - depth;
    match unit {
        Unit::Question if depth == 0 => {
            let unled = Held {
                reactions: 0,
                sets: 0,
            };
            question_bound::<E, Err>(height, Grandchildren::MAX, unled, FAN)
        }
        Unit::Question if height == 0 => leaf_question_bound::<E, Err>(leading),
        Unit::Question => question_bound::<E, Err>(height, Grandchildren::MAX, leading, FAN),
        Unit::Answer if height == 0 => supply_bound::<E, Err>(1),
        Unit::Answer => max(
            answer_bound::<E, Err>(FAN, FAN),
            supply_bound::<E, Err>(FAN),
        ),
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
/// On the in-memory backend on 64-bit targets that is about 26 MiB, almost
/// all of it the listings of the replies one question per depth can invite,
/// a fan of fans of entries each.
pub(crate) const fn floor_allowance<E: ErasedNode, Err>() -> usize {
    // Indexed by side: the initiator's sum, then the responder's.
    let mut sides = [0, 0];
    let mut depth = 0;
    while depth <= height::Root::HEIGHT {
        let asker = depth % 2;
        sides[asker] += slot_bound::<E, Err>(Unit::Question, depth);
        sides[1 - asker] += slot_bound::<E, Err>(Unit::Answer, depth);
        depth += 1;
    }
    max(sides[0], sides[1])
}

/// Check, as a walk over nodes `E` is built, that every grant it asks for
/// fits the four bytes a charge counts in ([`largest_grant`]).
///
/// Called in a constant, so a backend whose grants could outgrow them fails
/// to compile.
pub(crate) const fn assert_grants_fit<E: ErasedNode, Err>() {
    assert!(
        largest_grant::<E, Err>() <= u32::MAX as usize,
        "every unit's grant must fit a charge's four-byte count",
    );
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
/// - [`Holding::OutgoingReplies`]: a chunk's reactions, allocated before
///   its question is admitted with one slot spare, at most two fans and
///   one, with the set of later matches the answer's match carries; and a
///   question's listing, a fan of entries, built before the chunk carrying
///   it is charged.
/// - [`Holding::ParkedReplies`]: a decoder's ready reactions and its open
///   group's runs, a fan of each, in vectors at most twice their length.
/// - [`Holding::QuestionRecords`]: a take's derived scopes for one chunk, a
///   fan, in a vector at most twice its length; and the record an encoder
///   still holds for the reply it writes after its asker's reservation
///   returned, a scope with a fan of radices and counts.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn scratch<E, Err>(place: Holding) -> usize {
    match place {
        Holding::Questions => queued::<Query<E>>().max(queued::<LeafRequest>()),
        Holding::Resolutions => 2 * queued::<Resolution<E>>(),
        Holding::OutgoingReplies => {
            queued::<Result<Reply<E>, Error<Err>>>()
                + (2 * FAN + 1) * size_of::<Reaction<E>>()
                + LATER_MATCHES_HEAP_BYTES
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
