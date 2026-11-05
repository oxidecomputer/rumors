use std::{mem, pin::Pin};

use futures::{Stream, StreamExt, future::try_join, stream};

use crate::{
    Version,
    counter::widen,
    message::PayloadCodec,
    tree::{
        mirror::contained,
        mirror::streaming::{
            Backend, ErasedNode, FAN, Leaf,
            backend::BoxNodeStream,
            channel::{QueueKind, QueueRole, Receiver, Sender, channel},
            erased::{Reaction as ProtocolReaction, Reply, ops},
            holding::{Holding, held_by, holding},
            ledger::Charge,
            materialized::{Allowance, SupplyLedger, parked_bytes},
            message::{Carried, Grandchildren, Listed},
        },
        typed::{
            ErasedPrefix, Path, PathKey, Prefix,
            height::{Height, Z},
        },
    },
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    super::pipe::{Inbound, WirePermit},
    error::DecodeError,
    scope::{ReplyLevel, Scope},
};

/// What a reply decoder reads its frames as: a frame, and the permit from
/// the receive pool it holds for a supply run's bytes.
///
/// The pipe's lanes carry [`Inbound`] items; tests that drive a decoder
/// directly hand it bare frames, which hold no permit.
pub trait IncomingFrame {
    /// The frame and its permit.
    fn into_parts(self) -> (Frame, WirePermit);
}

/// A bare frame holds no permit.
impl IncomingFrame for Frame {
    fn into_parts(self) -> (Frame, WirePermit) {
        (self, WirePermit::default())
    }
}

/// A frame from the pipe, with its permit.
impl IncomingFrame for Inbound {
    fn into_parts(self) -> (Frame, WirePermit) {
        (self.frame, self.permit)
    }
}

/// Leaf records each reply decoder holds at once at most: a full fan in its
/// leaves' edge and the one in its reader's hand.
pub(crate) const SUPPLY_RECORDS_PER_STREAM: usize = FAN + 1;

/// The most bytes one session's decode fans hold at once, for leaves whose
/// handles are `N`: the *decode-ahead* a session's memory budget sets aside.
///
/// Each of the [`STREAM_COUNT`](super::super::STREAM_COUNT) logical reply
/// streams a session decodes holds at most [`SUPPLY_RECORDS_PER_STREAM`]
/// decoded leaves, each a [`DecodedLeaf`], counted at twice its size: the
/// most a channel node takes beside its item. A full fan permits one parent
/// group's worth of read-ahead, reducing wakeups and overlapping parsing
/// with slower assembly; capacity one still makes progress.
///
/// The leaf a [`DecodedLeaf`] carries is replica content from the moment
/// it is built, so only its handle counts here. On the in-memory backend
/// the handle keeps nothing resident beyond the node.
pub(crate) const fn decode_ahead_bytes<N>() -> usize {
    2 * super::super::STREAM_COUNT * SUPPLY_RECORDS_PER_STREAM * mem::size_of::<DecodedLeaf<N>>()
}

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
    /// a fixed amount per logical stream that the fixed allowance sets aside
    /// ([`decode_ahead_bytes`]), not a liveness requirement.
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
/// it can still deliver, the room beyond that returns at once: the reply's
/// [`Allowance`] checks every frame against what the room was priced on,
/// so the room always covers every chunk the reply can still park, no
/// chunk falls short of it, and none of it outlives the reply.
///
/// At [`ReplyLevel::Leaf`] only an empty request for the leaf is a valid
/// query. Validation reads the record through a cursor and builds nothing:
/// the scopes the reply's questions create are derived when each chunk is
/// taken ([`derive_questions`](super::derive_questions)), from the same
/// record.
///
/// Each supplied record is checked against `bounds` before it is placed
/// ([`SupplyBounds`]). The reply's supplied leaves cross `edge` to assembly
/// while the reply is read. After an error the edge may hold items of the
/// failed reply, so a caller decodes nothing further through it.
///
/// A supply run's permit from the receive pool ([`IncomingFrame`]) returns
/// as soon as the run's last record is copied into its leaf, before the
/// decoder waits for its next frame: that frame may be a run the reader
/// can read only once a permit returns.
// The edge, the session's decode limits, the reply's level, scope, and
// frames, and the chunks' destination are separate inputs; the pump
// supplies the limits from its ingress.
#[allow(clippy::too_many_arguments)]
pub async fn decode_reply<B, F>(
    edge: &mut LeafEdge<B>,
    backend: B,
    bounds: &SupplyBounds,
    ledger: SupplyLedger,
    level: ReplyLevel,
    scope: &Scope,
    frames: &mut F,
    codec: PayloadCodec,
    path_key: Option<&PathKey>,
    room: Charge,
    deliver: &mut (impl FnMut(Reply<B::Erased>) + Send),
) -> Result<(), DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item: IncomingFrame> + Unpin,
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
        bounds, &ledger, level, scope, leaves, assembled, codec, path_key, room,
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
    bounds: &SupplyBounds,
    ledger: SupplyLedger,
    level: ReplyLevel,
    scope: &Scope,
    frames: &mut F,
    codec: PayloadCodec,
    path_key: Option<&PathKey>,
) -> Result<Reply<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item: IncomingFrame> + Unpin,
{
    let mut chunks = Vec::new();
    decode_reply(
        edge,
        backend,
        bounds,
        ledger,
        level,
        scope,
        frames,
        codec,
        path_key,
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
    /// Ordering and scope state for supplied leaves.
    supplies: SupplyRuns<'a>,
    /// The leaves' edge to assembly.
    leaves: &'a mut Sender<DecodedLeaf<B::Node<Z>>>,
    /// The supplies' edge back from assembly.
    assembled: &'a mut Receiver<(ErasedPrefix, B::Erased)>,
    /// The codec used to decode supplied messages.
    codec: PayloadCodec,
    /// The keyed hash that places supplied leaves; `None` admits no supply.
    path_key: Option<&'a PathKey>,
    /// The complete reactions not yet delivered, in wire order.
    ready: Vec<ProtocolReaction<B::Erased>>,
    /// The runs of the open group of supplies, each awaiting its node.
    group: Vec<(u8, ErasedPrefix)>,
    /// What the reply can still deliver, which checks each frame and
    /// holds the part of the room no chunk has taken and no release has
    /// returned.
    allowance: Allowance<B::Erased>,
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
        bounds: &'a SupplyBounds,
        ledger: &'a SupplyLedger,
        level: ReplyLevel,
        scope: &'a Scope,
        leaves: &'a mut Sender<DecodedLeaf<B::Node<Z>>>,
        assembled: &'a mut Receiver<(ErasedPrefix, B::Erased)>,
        codec: PayloadCodec,
        path_key: Option<&'a PathKey>,
        room: Charge,
    ) -> Self {
        Self {
            ledger,
            level,
            scope,
            supplies: SupplyRuns::new(bounds),
            leaves,
            assembled,
            codec,
            path_key,
            ready: Vec::new(),
            group: Vec::new(),
            allowance: Allowance::new(scope.reply_bound(), scope.positions().remaining(), room),
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
        F: Stream<Item: IncomingFrame> + Unpin,
    {
        let mut positions = self.scope.positions();
        loop {
            let Some(item) = frames.next().await else {
                return Err(DecodeError::TruncatedReply);
            };
            let (frame, permit) = item.into_parts();
            let (reaction, flow) = match frame {
                Frame::Reaction(reaction, flow) => (reaction, flow),
                Frame::End(End::Reply) if self.allowance.is_untouched() => {
                    positions.end()?;
                    break;
                }
                Frame::End(End::Stream) => return Err(DecodeError::UnexpectedStreamEnd),
                Frame::End(_) => return Err(DecodeError::BareEndAfterReaction),
            };

            match reaction {
                WireReaction::Match(later) => {
                    self.close_group().await;
                    // Reject the overrun, a second match, or a cover of
                    // anything but later listed children at its frame,
                    // rather than retaining a reply for later walk
                    // validation.
                    let matched = positions.matched(&later)?;
                    self.allowance.matched(
                        radix(&matched.prefix),
                        later.set(),
                        matched.children,
                    )?;
                    self.ready.push(ProtocolReaction::Match(later));
                }
                WireReaction::Query(listing) => {
                    self.close_group().await;
                    // Validate the question's position now, at its frame;
                    // the scope it creates is derived at the take.
                    let (prefix, _) = self.level.check(&mut positions, &listing)?;
                    self.allowance.query(radix(&prefix), listing.len())?;
                    self.ready.push(ProtocolReaction::Query(listing));
                    // The chunk ends at its question, unless the question
                    // ends the reply, whose last chunk then carries it.
                    if flow == Flow::Continue {
                        deliver(self.chunk(false));
                    }
                }
                WireReaction::Supply(run) => {
                    // The codec rejects empty runs. Keep the assertion for
                    // frames constructed directly inside the crate.
                    debug_assert!(
                        !run.is_empty(),
                        "the codec never yields an empty supply run"
                    );
                    // Decode records directly into assembly; retain only one
                    // placeholder for each version-derived supply run.
                    // Decoded leaves become replica content or wait in the
                    // decode fan, so the session-budget meter counts their
                    // construction there.
                    let mut records = run.records(self.codec);
                    while let Some(record) = held_by(Holding::DecodedLeaves, || records.next()) {
                        let (version, message) = record.map_err(DecodeError::Record)?;
                        let (leaf_prefix, run) = self.supplies.observe::<B::Error>(
                            self.scope.parent(),
                            &version,
                            self.path_key,
                        )?;
                        if let Some(run) = run {
                            self.allowance.supply(run.0)?;
                            self.group.push(run);
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
                    // Every record is copied out of the run, so its bytes
                    // and its permit return now.
                    drop(records);
                    drop(run);
                    drop(permit);
                }
            }

            self.allowance.release();
            if flow == Flow::End {
                // The end is never inferred, only checked: every listed
                // child after the last frame must be covered.
                positions.end()?;
                break;
            }
        }
        self.close_group().await;
        self.leaves.send(DecodedLeaf::End).await.expect(EDGE_OPEN);
        deliver(self.chunk(true));
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
            self.allowance.supplied(node.child_count(&actual));
            self.ready.push(ProtocolReaction::Supply(radix, node));
        }
    }

    /// The ready reactions as one chunk, allocated at exactly their count:
    /// a chunk can park, and the reservation for its reply prices each
    /// reaction at its size, which growth slack would exceed.
    ///
    /// A chunk that does not end the reply leaves the room beyond what the
    /// reply can still deliver for the caller to release, as it does after
    /// every frame.
    fn chunk(&mut self, last: bool) -> Reply<B::Erased> {
        let mut chunk = Reply {
            reactions: self.ready.drain(..).collect(),
            charge: Charge::default(),
            last,
            carried: Carried::Nothing,
        };
        chunk.charge = self.allowance.chunk(parked_bytes(&chunk), last);
        chunk
    }
}

/// The radix of the listed child at `prefix`: the byte that extends its
/// parent's prefix to it.
fn radix(prefix: &ErasedPrefix) -> u8 {
    *prefix
        .as_bytes()
        .last()
        .expect("a listed child's prefix extends its parent's by its radix")
}

/// What every supplied leaf's version must satisfy, read from the
/// session's two greetings.
///
/// A peer supplies only leaves its own tree holds, and only those this
/// side's greeting version does not contain: its filter drops every leaf
/// this side already holds or has redacted. So each supplied version
///
/// - encodes within the peer's declared `max_version_bytes`, which covers
///   every version the peer's tree materializes
///   ([`DecodeError::OversizedVersion`]);
/// - is contained in the peer's own greeting version, which joins every
///   version its tree holds ([`DecodeError::UncontainedSupply`]); and
/// - is not contained in this side's greeting version
///   ([`DecodeError::SeenSupply`]).
///
/// A conforming peer never breaks any of these, so a breach is a
/// conformance failure, and the decoder fails the session at the offending
/// record, before its payload takes backend custody. Absorbing either kind
/// of version breach would make a redaction fail to hold. A leaf outside
/// the peer's version lies outside the ceiling this side adopts for the
/// session, so no later redaction this side makes can cover it. A leaf
/// inside this side's version is one this side holds or redacted, and
/// redaction leaves no tombstone: it is honored only by that version.
#[derive(Clone, Debug)]
pub struct SupplyBounds {
    /// The peer's greeting version.
    declared: Version,
    /// The peer's greeting-declared `max_version_bytes`.
    version_bytes: u64,
    /// This side's own greeting version, the one the peer filters its
    /// supplies against.
    seen: Version,
}

impl SupplyBounds {
    /// Bound the session's supplies by the peer's greeting version and
    /// declared `max_version_bytes`, and by this side's own greeting
    /// version, `seen`.
    pub fn new(declared: Version, version_bytes: u64, seen: Version) -> Self {
        Self {
            declared,
            version_bytes,
            seen,
        }
    }

    /// Check one supplied leaf's version against the bounds, naming the
    /// leaf by its keyed `path` when its version breaks one.
    ///
    /// The size check comes first, so the comparisons read only a version
    /// within the declaration. Neither comparison allocates. Containment is
    /// a partial order, so a version incomparable with the peer's is as
    /// uncontained as one above it.
    fn check<E>(&self, version: &Version, path: Path) -> Result<(), DecodeError<E>> {
        // The declared aggregate covers every version the peer's tree
        // materializes, so every version it supplies must encode within
        // it; one arriving over the declaration shows the declaration
        // false.
        let actual = version.as_bytes().len();
        if widen(actual) > self.version_bytes {
            return Err(DecodeError::OversizedVersion {
                declared: self.version_bytes,
                actual,
            });
        }
        if !contained(version, &self.declared) {
            return Err(DecodeError::UncontainedSupply { path: path.into() });
        }
        if contained(version, &self.seen) {
            return Err(DecodeError::SeenSupply { path: path.into() });
        }
        Ok(())
    }
}

/// Validate and group the supplied leaves within one reply.
struct SupplyRuns<'a> {
    /// What every supplied leaf's version must satisfy.
    bounds: &'a SupplyBounds,
    /// Last leaf path, used to enforce strict wire order.
    previous_leaf: Option<Prefix<Z>>,
    /// Prefix of the supply run currently receiving leaves.
    current: Option<ErasedPrefix>,
}

impl<'a> SupplyRuns<'a> {
    /// Begin a reply with no observed supply run.
    fn new(bounds: &'a SupplyBounds) -> Self {
        Self {
            bounds,
            previous_leaf: None,
            current: None,
        }
    }

    /// End the current supply run when a positional reaction intervenes.
    fn interrupt(&mut self) {
        self.current = None;
    }

    /// Validate one supplied leaf and identify the start of a new run.
    ///
    /// The leaf's version must lie within the session's [`SupplyBounds`],
    /// checked before the leaf is placed. The run boundary sits one level
    /// under `expected_parent`: the supplied leaf's path must extend the
    /// parent prefix, and the byte after it is the run's radix.
    fn observe<E>(
        &mut self,
        expected_parent: ErasedPrefix,
        version: &Version,
        path_key: Option<&PathKey>,
    ) -> Result<(Prefix<Z>, Option<(u8, ErasedPrefix)>), DecodeError<E>> {
        let Some(path_key) = path_key else {
            return Err(DecodeError::UnkeyedSupply);
        };
        let path = Path::for_leaf(path_key, version);
        self.bounds.check(version, path)?;
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

        // A run that resumes after a positional reaction starts anew here,
        // and the reply's allowance refuses it: its radix does not sit above
        // the reaction that interrupted it.
        let run = if self.current != Some(node_prefix) {
            self.current = Some(node_prefix);
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
