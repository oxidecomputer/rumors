use std::{mem, pin::Pin};

use futures::{Stream, StreamExt, future::try_join, stream};

use crate::{
    counter::widen,
    message::PayloadCodec,
    tree::{
        mirror::streaming::{
            Backend, ErasedNode, FAN, Leaf,
            backend::BoxNodeStream,
            channel::{QueueKind, QueueRole, Receiver, Sender, channel},
            erased::{Reaction as ProtocolReaction, Reply, ops},
            holding::{Holding, held_by, holding},
            ledger::{Charge, ReplyBound},
            materialized::{Deliverable, SupplyLedger, parked_bytes, room_beyond},
            message::{Carried, Grandchildren, Listed},
        },
        typed::{
            ErasedPrefix, Path, Prefix,
            height::{Height, Z},
        },
    },
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    error::BoundedPart,
    error::{DecodeError, ScopeError},
    scope::{ReplyLevel, Scope},
};

/// Leaf records each reply decoder holds at once at most: a full fan in its
/// leaves' edge and the one in its reader's hand.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const SUPPLY_RECORDS_PER_STREAM: usize = FAN + 1;

/// The most bytes one session's decode fans hold at once on the in-memory
/// backend.
///
/// Each of the [`STREAM_COUNT`](super::super::STREAM_COUNT) logical reply streams a session decodes
/// holds at most [`SUPPLY_RECORDS_PER_STREAM`] decoded leaves, each a
/// [`DecodedLeaf`] whose handle keeps nothing resident beyond it on the
/// in-memory backend. A full fan permits one parent group's worth of
/// read-ahead, reducing wakeups and overlapping parsing with slower
/// assembly; capacity one still makes progress.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const SUPPLY_DECODE_ENVELOPE_BYTES: usize = super::super::STREAM_COUNT
    * SUPPLY_RECORDS_PER_STREAM
    * mem::size_of::<DecodedLeaf<<crate::tree::mirror::streaming::Local as Backend>::Node<Z>>>();

/// One item on a [`LeafEdge`]'s way to assembly.
pub(crate) enum DecodedLeaf<N> {
    /// A decoded leaf at its path.
    Leaf(Prefix<Z>, N),
    /// The end of a group of consecutive supplies: the assembler finishes
    /// the group's last node and waits for the next group.
    Break,
    /// The end of the reply: the assembler returns.
    End,
}

/// The edges between wire decoding and leaf assembly, which every reply one
/// decode pump decodes reuses: decoded leaves one way, assembled supplies
/// the other.
///
/// A pump decodes its replies one after another, so one pair of edges
/// serves them all. A reply whose decode fails can leave either edge
/// holding the rest of that reply's items, so a failed decode ends the
/// edges' use; the pump fails the session on it.
pub struct LeafEdge<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The reader's half of the leaves' edge.
    leaves: Sender<DecodedLeaf<B::Node<Z>>>,
    /// The assembler's half of the leaves' edge.
    assembler: Receiver<DecodedLeaf<B::Node<Z>>>,
    /// The assembler's half of the supplies' edge.
    supplies: Sender<(ErasedPrefix, B::Erased)>,
    /// The reader's half of the supplies' edge.
    assembled: Receiver<(ErasedPrefix, B::Erased)>,
}

impl<B> LeafEdge<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Create a leaves' edge of [`FAN`] slots.
    ///
    /// A full fan amortizes reader and assembler wakeups and lets parsing
    /// run ahead when assembly is slower. Progress needs only one slot, which
    /// the tests' one-slot edge establishes, so the fan is a throughput choice,
    /// a fixed amount per logical stream whatever the budget, not a liveness
    /// requirement.
    pub fn new() -> Self {
        Self::with_capacity(FAN)
    }

    /// Create a one-slot leaves' edge.
    ///
    /// Test-only because production fixes the capacity at [`FAN`] for
    /// bounded read-ahead. One slot, the smallest capacity a protocol edge
    /// accepts, establishes whether progress depends on that performance
    /// choice.
    #[cfg(test)]
    pub(super) fn one_slot() -> Self {
        Self::with_capacity(1)
    }

    /// Create a leaves' edge of `capacity` slots.
    ///
    /// The supplies' edge always holds [`FAN`] nodes, which liveness needs:
    /// the reader drains it only when a group of supplies ends, and until
    /// then the assembler sends one node per run in the group. A reply's
    /// leaves ascend strictly by path, so its runs have distinct radices and
    /// a group holds at most `FAN` runs; the assembler therefore never waits
    /// on a reader that waits on it.
    fn with_capacity(capacity: usize) -> Self {
        let (leaves, assembler) = channel(
            QueueRole::new(QueueKind::DecodedLeaves, Z::HEIGHT),
            capacity,
        );
        let (supplies, assembled) =
            channel(QueueRole::new(QueueKind::AssembledSupplies, Z::HEIGHT), FAN);
        Self {
            leaves,
            assembler,
            supplies,
            assembled,
        }
    }
}

impl<B> Default for LeafEdge<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Create a leaves' edge of [`FAN`] slots, as [`new`](Self::new).
    fn default() -> Self {
        Self::new()
    }
}

/// Replay the initiator's distinguished opening question from the root-fan
/// listing its greeting carried.
///
/// No wire frame exists for the opening: the greeting decode already
/// validated the listing's canonical order, so synthesizing the one-query
/// reply and its root scope is infallible. An empty listing replays an empty
/// opening `Query` — the empty-tree initiator's "send everything".
pub fn opening_reply<E>(listing: Vec<Listed>) -> (Reply<E>, Scope) {
    // The local responder answers against this record and never decodes
    // a reply with it, so no tighter bound on such a reply is needed.
    let scope = Scope::opening(&listing, Grandchildren::MAX);
    (
        Reply {
            reactions: vec![ProtocolReaction::Query(listing)],
            charge: Charge::default(),
            last: true,
            carried: Carried::Nothing,
        },
        scope,
    )
}

/// Decode one reply at `level` through `edge`, validating the positions of
/// the questions it asks against `scope`, the record of the question it
/// answers, and hand its reactions to `deliver` in chunks as they complete.
///
/// A reaction completes at its frame, or for a supply once its run of
/// leaves ends: at the next positional reaction, at a run of another
/// radix, or at the reply's end. A chunk ends at each question the reply
/// asks but its last, delivered with the reactions completed before it as
/// soon as its frame decodes, so a question the peer must answer never
/// waits behind frames the peer may send only after that answer. Where a
/// reply's chunks end therefore depends only on the reply, never on when
/// its frames arrive: a reply arrives in exactly one more chunk than it
/// asks questions before its last reaction, and no chunk holds more than
/// one question. The last chunk is marked, and holds the reply's last
/// reaction unless the reply is empty. Each chunk's reactions are
/// allocated at exactly their count.
///
/// `room` is the room the question set aside for the reply's chunks. Each
/// chunk leaves charged its own part of it ([`parked_bytes`]), which
/// returns once the chunk's stage has used it. As the reply narrows what
/// it can still deliver, the room beyond that returns at once
/// ([`room_beyond`]): the room always covers every chunk the reply can
/// still park, so no chunk falls short of it, and none of it outlives the
/// reply.
///
/// At [`ReplyLevel::Leaf`] only an empty request for the leaf is a valid
/// query. Validation reads the record through a cursor and builds nothing:
/// the scopes the reply's questions create are derived when each chunk is
/// taken ([`derive_questions`](super::derive_questions)), from the same
/// record.
///
/// The reply's supplied leaves cross `edge` to assembly while the reply is
/// read. After an error the edge may hold items of the failed reply, so a
/// caller decodes nothing further through it.
// The edge, the session's decode limits, the reply's level, scope, and
// frames, and the chunks' destination are separate inputs; the pump
// supplies the limits from its ingress.
#[allow(clippy::too_many_arguments)]
pub async fn decode_reply<B, F>(
    edge: &mut LeafEdge<B>,
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    level: ReplyLevel,
    scope: &Scope,
    frames: &mut F,
    codec: PayloadCodec,
    room: Charge,
    deliver: &mut (impl FnMut(Reply<B::Erased>) + Send),
) -> Result<(), DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    // The reply's supplied runs group into nodes one level under the
    // scope's parent: the scope's own children height.
    let children_height = scope.parent().height() - 1;
    let LeafEdge {
        leaves,
        assembler,
        supplies,
        assembled,
    } = edge;
    let read = Reader::<B>::new(
        version_bytes,
        &ledger,
        level,
        scope,
        leaves,
        assembled,
        codec,
        room,
    )
    .read(frames, deliver);
    let assemble = holding(
        Holding::Assembly,
        assemble_groups::<B>(backend, children_height, assembler, supplies),
    );
    // The assembler returns only after the reader's closing `End`, so the
    // reader never waits on an assembler that has finished. A failure on
    // either side drops the other, which may be waiting on its edge.
    try_join(read, assemble).await?;
    Ok(())
}

/// Decode one reply as [`decode_reply`] does, joining its chunks into one
/// last chunk: the one chunk itself when the reply asks no question before
/// its last reaction, so its allocation is the decoder's own.
// The arguments are `decode_reply`'s, but for the chunks' destination.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub async fn decode_whole_reply<B, F>(
    edge: &mut LeafEdge<B>,
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    level: ReplyLevel,
    scope: &Scope,
    frames: &mut F,
    codec: PayloadCodec,
) -> Result<Reply<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    let mut chunks = Vec::new();
    decode_reply(
        edge,
        backend,
        version_bytes,
        ledger,
        level,
        scope,
        frames,
        codec,
        Charge::default(),
        &mut |chunk: Reply<B::Erased>| chunks.push(chunk),
    )
    .await?;
    if chunks.len() == 1 {
        return Ok(chunks.remove(0));
    }
    let count = chunks.iter().map(|chunk| chunk.reactions.len()).sum();
    let mut reactions = Vec::with_capacity(count);
    for chunk in chunks {
        reactions.extend(chunk.reactions);
    }
    Ok(Reply {
        reactions,
        charge: Charge::default(),
        last: true,
        carried: Carried::Nothing,
    })
}

/// Assemble one reply's groups of supplied leaves into complete nodes at the
/// scope's children `height`, sending each node back to the reader, until
/// the reply's closing [`DecodedLeaf::End`].
async fn assemble_groups<B>(
    backend: B,
    height: usize,
    leaves: &mut Receiver<DecodedLeaf<B::Node<Z>>>,
    supplies: &mut Sender<(ErasedPrefix, B::Erased)>,
) -> Result<(), DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    loop {
        let first = match leaves.recv().await.expect(EDGE_OPEN) {
            DecodedLeaf::End => return Ok(()),
            // An empty group assembles nothing.
            DecodedLeaf::Break => continue,
            DecodedLeaf::Leaf(prefix, leaf) => (prefix, leaf),
        };
        let mut group = assembly(backend.clone(), height, first, leaves);
        while let Some(node) = group.next().await {
            let node = node.map_err(DecodeError::Backend)?;
            supplies.send(node).await.expect(EDGE_OPEN);
        }
    }
}

/// Stream one group's decoded leaves, from its `first`, through the
/// backend's node assembler.
///
/// The edge accounts for decoded leaves awaiting the backend. Any further
/// buffering used to construct a node belongs to that backend; `Local`, for
/// example, keeps only the open right spine of the subtree it is building.
///
/// The stream ends at the group's closing [`DecodedLeaf::Break`], and is
/// fused, so the assembler cannot read past it into the next group's leaves.
fn assembly<'a, B>(
    backend: B,
    height: usize,
    first: (Prefix<Z>, B::Node<Z>),
    leaves: &'a mut Receiver<DecodedLeaf<B::Node<Z>>>,
) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send + 'a>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let rest = stream::poll_fn(move |cx| {
        leaves
            .poll_next_unpin(cx)
            .map(|item| match item.expect(EDGE_OPEN) {
                DecodedLeaf::Leaf(prefix, leaf) => Some(Ok((prefix, leaf))),
                DecodedLeaf::Break => None,
                DecodedLeaf::End => {
                    unreachable!("the reader closes a group with `Break` before ending its reply")
                }
            })
    });
    let group = stream::once(async move { Ok(first) }).chain(rest).fuse();
    #[cfg(test)]
    let group = group.inspect(|_| fan_probe::on_recv());
    let group: BoxNodeStream<'a, B, Z> = Box::pin(group);
    ops::assemble(backend, height, group)
}

/// Why a send or receive on a [`LeafEdge`] cannot fail: the edge owns both
/// halves of each of its channels, and the reader and assembler use them
/// only while the edge is borrowed for their reply.
const EDGE_OPEN: &str = "the edge holds both halves of its channels while a reply uses them";

/// One reply's reader: its validation state, and the reactions it has read
/// but not yet delivered.
struct Reader<'a, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The peer's remaining declared supply allowance.
    ledger: &'a SupplyLedger,
    /// How the reply's queries pair with positions.
    level: ReplyLevel,
    /// The record of the question the reply answers.
    scope: &'a Scope,
    /// The bound the record sets on the reply.
    bound: ReplyBound,
    /// The listing entries the reply's queries have carried so far.
    listed: usize,
    /// The reactions the reply has carried so far, delivered or not.
    reactions: usize,
    /// Ordering and scope state for supplied leaves.
    supplies: SupplyRuns,
    /// The leaves' edge to assembly.
    leaves: &'a mut Sender<DecodedLeaf<B::Node<Z>>>,
    /// The supplies' edge back from assembly.
    assembled: &'a mut Receiver<(ErasedPrefix, B::Erased)>,
    /// The codec used to decode supplied messages.
    codec: PayloadCodec,
    /// The complete reactions not yet delivered, in wire order.
    ready: Vec<ProtocolReaction<B::Erased>>,
    /// The queries among the ready reactions.
    ready_questions: usize,
    /// The listing entries the ready queries carry together.
    ready_listing: usize,
    /// The runs of the open group of supplies, each awaiting its node.
    group: Vec<(u8, ErasedPrefix)>,
    /// The record's positions the reply has not reached.
    positions_left: usize,
    /// The radix of the reply's latest reaction: every later reaction sits
    /// at a greater one.
    after: Option<u8>,
    /// The child counts of the nodes the reply has supplied: each is a
    /// child of the peer's whose count its grandchild count of the scope
    /// includes, and whose listing therefore never arrives.
    supplied_children: usize,
    /// This side's child counts of the children the reply has matched:
    /// each matched child is identical on both sides, so the peer's
    /// grandchild count of the scope includes the same count for it, and
    /// its listing never arrives.
    matched_children: usize,
    /// The part of the room the question set aside for the reply's chunks
    /// that the chunks delivered so far did not take.
    room: Charge,
}

impl<'a, B> Reader<'a, B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Begin reading a reply against `scope`, its chunks charged out of
    /// `room`.
    // The inputs are `decode_reply`'s, but for the frames and the chunks'
    // destination, which `read` takes.
    #[allow(clippy::too_many_arguments)]
    fn new(
        version_bytes: u64,
        ledger: &'a SupplyLedger,
        level: ReplyLevel,
        scope: &'a Scope,
        leaves: &'a mut Sender<DecodedLeaf<B::Node<Z>>>,
        assembled: &'a mut Receiver<(ErasedPrefix, B::Erased)>,
        codec: PayloadCodec,
        room: Charge,
    ) -> Self {
        Self {
            ledger,
            level,
            scope,
            bound: scope.reply_bound(),
            listed: 0,
            reactions: 0,
            supplies: SupplyRuns::new(version_bytes),
            leaves,
            assembled,
            codec,
            ready: Vec::new(),
            ready_questions: 0,
            ready_listing: 0,
            group: Vec::new(),
            positions_left: scope.positions().remaining(),
            after: None,
            supplied_children: 0,
            matched_children: 0,
            room,
        }
    }

    /// Read and validate the reply while sending its supplied leaves to
    /// assembly, delivering its reactions in chunks, then close the reply
    /// on the edge with [`DecodedLeaf::End`].
    async fn read<F>(
        mut self,
        frames: &mut F,
        deliver: &mut (impl FnMut(Reply<B::Erased>) + Send),
    ) -> Result<(), DecodeError<B::Error>>
    where
        F: Stream<Item = Frame> + Unpin,
    {
        let mut positions = self.scope.positions();
        loop {
            let Some(frame) = frames.next().await else {
                return Err(DecodeError::TruncatedReply);
            };
            let (reaction, flow) = match frame {
                Frame::Reaction(reaction, flow) => (reaction, flow),
                Frame::End(End::Reply) if self.reactions == 0 => break,
                Frame::End(End::Stream) => return Err(DecodeError::UnexpectedStreamEnd),
                Frame::End(_) => return Err(DecodeError::BareEndAfterReaction),
            };

            match reaction {
                WireReaction::Match => {
                    self.close_group().await;
                    // Reject the overrun at its frame rather than retaining an
                    // unbounded reply for later walk validation.
                    let children = positions.upcoming_children();
                    let (prefix, _) = positions.next().ok_or(ScopeError::UnpositionedMatch)?;
                    self.reach(&prefix);
                    self.matched_children += children;
                    self.count()?;
                    self.ready.push(ProtocolReaction::Match);
                }
                WireReaction::Query(listing) => {
                    self.close_group().await;
                    // Validate the question's position now, at its frame;
                    // the scope it creates is derived at the take.
                    let (prefix, _) = self.level.check(&mut positions, &listing)?;
                    self.reach(&prefix);
                    self.listed += listing.len();
                    if self.listed > self.bound.listing {
                        return Err(DecodeError::ExceedsBound {
                            part: BoundedPart::Listing,
                            bound: self.bound.listing,
                        });
                    }
                    self.count()?;
                    self.ready_questions += 1;
                    self.ready_listing += listing.len();
                    self.ready.push(ProtocolReaction::Query(listing));
                    // The chunk ends at its question, unless the question
                    // ends the reply, whose last chunk then carries it.
                    if flow == Flow::Continue {
                        deliver(self.chunk(false));
                    }
                }
                WireReaction::Supply(records) => {
                    // The codec rejects empty runs. Keep the assertion for
                    // frames constructed directly inside the crate.
                    debug_assert!(
                        !records.is_empty(),
                        "the codec never yields an empty supply run",
                    );
                    // Decode records directly into assembly; retain only one
                    // placeholder for each version-derived supply run.
                    // Decoded leaves become replica content or wait in the
                    // decode fan, so the session-budget meter counts their
                    // construction there.
                    let mut records = records.records(self.codec);
                    while let Some(record) = held_by(Holding::DecodedLeaves, || records.next()) {
                        let (version, message) = record.map_err(DecodeError::Record)?;
                        let (leaf_prefix, run) = self
                            .supplies
                            .observe::<B::Error>(self.scope.parent(), &version)?;
                        if let Some(run) = run {
                            self.after = Some(run.0);
                            self.group.push(run);
                            self.count()?;
                        }
                        // Charge the peer's declared set length before the
                        // payload enters backend custody.
                        self.ledger
                            .charge(1)
                            .map_err(|declared| DecodeError::OverdrawnSupply { declared })?;
                        let leaf = holding(
                            Holding::DecodedLeaves,
                            <B::Node<Z> as Leaf>::leaf(version, message),
                        )
                        .await
                        .map_err(DecodeError::Backend)?;
                        #[cfg(test)]
                        fan_probe::on_send();
                        self.leaves
                            .send(DecodedLeaf::Leaf(leaf_prefix, leaf))
                            .await
                            .expect(EDGE_OPEN);
                    }
                }
            }

            self.release();
            if flow == Flow::End {
                break;
            }
        }
        self.close_group().await;
        self.leaves.send(DecodedLeaf::End).await.expect(EDGE_OPEN);
        deliver(self.chunk(true));
        Ok(())
    }

    /// Count one more reaction, failing unless the reply still fits its
    /// bound.
    ///
    /// A peer that lists a child with its true grandchild count cannot
    /// exceed the bound its own count sets, so a reply over it reveals a
    /// miscounting peer at the reaction that overruns, before the reply
    /// holds more than its question reserved.
    fn count(&mut self) -> Result<(), DecodeError<B::Error>> {
        self.reactions += 1;
        if self.reactions > self.bound.reactions {
            return Err(DecodeError::ExceedsBound {
                part: BoundedPart::Reactions,
                bound: self.bound.reactions,
            });
        }
        Ok(())
    }

    /// End the open group of supplies, if any, and make its runs' nodes
    /// ready in run order.
    ///
    /// The assembler finishes a group's last node only once the group
    /// ends, so a supply is complete only then; ending the group at every
    /// positional reaction keeps a question behind a supply from waiting on
    /// frames still to come.
    async fn close_group(&mut self) {
        self.supplies.interrupt();
        if self.group.is_empty() {
            return;
        }
        self.leaves.send(DecodedLeaf::Break).await.expect(EDGE_OPEN);
        for (radix, prefix) in mem::take(&mut self.group) {
            let (actual, node) = self.assembled.recv().await.expect(EDGE_OPEN);
            assert_eq!(
                actual, prefix,
                "assembly yields each run's node at its version-derived prefix, in run order",
            );
            // A supplied node has at most a fan of children, so the clamp
            // changes nothing a conforming backend reports, and keeps a
            // miscounting one from overflowing the sum.
            self.supplied_children += node.child_count().min(FAN);
            self.ready.push(ProtocolReaction::Supply(radix, node));
        }
    }

    /// Note that the reply has reached the record's position at `prefix`.
    fn reach(&mut self, prefix: &ErasedPrefix) {
        self.positions_left -= 1;
        self.after = prefix.as_bytes().last().copied();
    }

    /// The most the reply can still deliver to parked chunks, its ready
    /// reactions and open group included.
    ///
    /// Later reactions number at most what the bound leaves and one per
    /// radix above the latest; later queries sit only at positions not yet
    /// reached. Later listing entries number at most what the peer's
    /// grandchild count of the scope leaves, less the child counts of the
    /// children it matched and of the nodes it supplied, which that count
    /// includes and whose listings never come, and at most a fan per later
    /// query. Every non-last chunk
    /// holds a query, so the chunks number at most the queries to come and
    /// one more.
    ///
    /// No term grows as the reply goes on: each reaction moves from what
    /// can come to what is ready, and each chunk delivered takes its
    /// reactions out of what is ready and its own price out of the room.
    fn deliverable(&self) -> Deliverable {
        let radices =
            usize::from(u8::MAX) + 1 - self.after.map_or(0, |radix| usize::from(radix) + 1);
        let reactions = (self.bound.reactions - self.reactions).min(radices);
        let questions = self.positions_left.min(reactions);
        let listing = self
            .bound
            .listing
            .saturating_sub(self.listed + self.matched_children + self.supplied_children)
            .min(FAN * questions);
        Deliverable {
            chunks: self.ready_questions + questions + 1,
            reactions: self.ready.len() + self.group.len() + reactions,
            listing: self.ready_listing + listing,
        }
    }

    /// Return the room beyond what the reply can still deliver.
    ///
    /// The room covers that much at every moment, which is what lets every
    /// chunk take its whole price from it: it starts at the reply's bound,
    /// what the reply can still deliver never grows, and each release
    /// returns only the excess.
    fn release(&mut self) {
        let beyond = room_beyond::<B::Erased>(self.room.bytes(), self.deliverable());
        drop(self.room.split(Holding::ParkedReplies, beyond));
    }

    /// The ready reactions as one chunk, allocated at exactly their count:
    /// a chunk can park, and the reservation for its reply prices each
    /// reaction at its size, which growth slack would exceed.
    fn chunk(&mut self, last: bool) -> Reply<B::Erased> {
        self.ready_questions = 0;
        self.ready_listing = 0;
        let mut chunk = Reply {
            reactions: self.ready.drain(..).collect(),
            charge: Charge::default(),
            last,
            carried: Carried::Nothing,
        };
        chunk.charge = self.room.take(Holding::ParkedReplies, parked_bytes(&chunk));
        if last {
            // The reply is whole: nothing more can come.
            drop(std::mem::take(&mut self.room));
        } else {
            self.release();
        }
        chunk
    }
}

/// Validate and group the supplied leaves within one reply.
struct SupplyRuns {
    /// The peer's greeting-declared `max_version_bytes`, covering every
    /// version its tree materializes and so every version it may supply.
    version_bytes: u64,
    /// Last leaf path, used to enforce strict wire order.
    previous_leaf: Option<Prefix<Z>>,
    /// Prefix of the supply run currently receiving leaves.
    current: Option<ErasedPrefix>,
    /// Last run radix, retained across positional reactions.
    previous_radix: Option<u8>,
}

impl SupplyRuns {
    /// Begin a reply with no observed supply run.
    fn new(version_bytes: u64) -> Self {
        Self {
            version_bytes,
            previous_leaf: None,
            current: None,
            previous_radix: None,
        }
    }

    /// End the current supply run when a positional reaction intervenes.
    fn interrupt(&mut self) {
        self.current = None;
    }

    /// Validate one supplied leaf and identify the start of a new run.
    ///
    /// The run boundary sits one level under `expected_parent`: the
    /// supplied leaf's path must extend the parent prefix, and the byte
    /// after it is the run's radix.
    fn observe<E>(
        &mut self,
        expected_parent: ErasedPrefix,
        version: &crate::Version,
    ) -> Result<(Prefix<Z>, Option<(u8, ErasedPrefix)>), DecodeError<E>> {
        // The declared aggregate covers every version the peer's tree
        // materializes, so every version it supplies must encode within
        // it; one arriving over the declaration shows the declaration
        // false, and fails the session before the record is admitted.
        let actual = version.as_bytes().len();
        if widen(actual) > self.version_bytes {
            return Err(DecodeError::OversizedVersion {
                declared: self.version_bytes,
                actual,
            });
        }
        let path = Path::for_leaf(version);
        let leaf_prefix = Prefix::<Z>::containing(&path);
        let path_bytes = <[u8; 32]>::from(path);
        let parent_len = expected_parent.as_bytes().len();
        if &path_bytes[..parent_len] != expected_parent.as_bytes() {
            return Err(DecodeError::LeafOutsideScope {
                expected: expected_parent.as_bytes().to_vec(),
                actual: path.into(),
            });
        }
        let radix = path_bytes[parent_len];
        let node_prefix = expected_parent.push(radix);
        if let Some(previous) = self
            .previous_leaf
            .filter(|previous| *previous >= leaf_prefix)
        {
            return Err(DecodeError::LeafOrder {
                previous: previous.into(),
                current: path.into(),
            });
        }
        self.previous_leaf = Some(leaf_prefix);

        let run = if self.current != Some(node_prefix) {
            if let Some(previous) = self.previous_radix.filter(|previous| *previous == radix) {
                return Err(DecodeError::SupplyOrder { previous, radix });
            }
            self.current = Some(node_prefix);
            self.previous_radix = Some(radix);
            Some((radix, node_prefix))
        } else {
            None
        };
        Ok((leaf_prefix, run))
    }
}

/// Test-gated occupancy probe for the reader/assembler fan channels.
///
/// Counts the decoded leaf records resident between the reader's send
/// ([`Reader::read`]) and the assembler's pull, and the peak of that
/// count. The adapter tests drive [`decode`] on a current-thread runtime and
/// each channel is FIFO with
/// one producer and one consumer, so a thread-local counter mirrors the
/// occupancy exactly: incremented before the reader awaits the send (the record
/// in the reader's hand is resident), decremented when the assembler's stream
/// yields the record.
#[cfg(test)]
pub(super) mod fan_probe {
    use std::cell::Cell;

    // Clippy's `missing_const_for_thread_local` can reject const-block
    // initializers on targets that lower `thread_local!` through fallback TLS.
    // The allow keeps `-D warnings` clean on those targets.
    thread_local! {
        #[allow(clippy::missing_const_for_thread_local)]
        static RESIDENT: Cell<usize> = const { Cell::new(0) };
        #[allow(clippy::missing_const_for_thread_local)]
        static PEAK: Cell<usize> = const { Cell::new(0) };
    }

    /// Zero the counters before a measured decode.
    pub(in super::super) fn reset() {
        RESIDENT.with(|cell| cell.set(0));
        PEAK.with(|cell| cell.set(0));
    }

    pub(super) fn on_send() {
        let resident = RESIDENT.with(|cell| {
            cell.set(cell.get() + 1);
            cell.get()
        });
        PEAK.with(|cell| cell.set(cell.get().max(resident)));
    }

    pub(super) fn on_recv() {
        RESIDENT.with(|cell| cell.set(cell.get().saturating_sub(1)));
    }

    /// The peak resident record count since the last [`reset`].
    pub(in super::super) fn peak() -> usize {
        PEAK.with(Cell::get)
    }
}
