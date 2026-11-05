use std::pin::Pin;

use futures::{Stream, StreamExt, future::try_join, stream};

use crate::{
    counter::widen,
    message::PayloadCodec,
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            backend::BoxNodeStream,
            channel::{QueueKind, QueueRole, Receiver, Sender, channel},
            erased::{Reaction as ProtocolReaction, Reply, ops},
            materialized::SupplyLedger,
            window::FAN,
        },
        typed::{
            ErasedPrefix, Hash, Path, Prefix,
            height::{Height, Z},
        },
    },
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    error::{DecodeError, ScopeError},
    scope::{ReplyLevel, Scope},
};

/// One reconstructed reply and any questions it asks next.
pub struct Decoded<E> {
    /// The reconstructed in-memory reply.
    pub reply: Reply<E>,
    /// Lower scopes created by the reply's queries, in wire order.
    pub questions: Vec<Scope>,
}

/// One item on a [`LeafEdge`]: a decoded leaf on its way to assembly, or
/// `None`, which closes the reply the preceding leaves belong to.
pub(crate) type DecodedLeaf<N, E> = Option<Result<(Prefix<Z>, N), E>>;

/// The bounded edge from wire decoding to leaf assembly, which every reply
/// one decode pump decodes reuses.
///
/// A pump decodes its replies one after another, so one edge serves them
/// all: the reader closes each reply with a `None` after its last leaf, and
/// the assembler stops there. A reply whose decode fails leaves the edge
/// holding the rest of that reply's leaves, so a failed decode ends the
/// edge's use; the pump fails the session on it.
pub struct LeafEdge<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The reader's half.
    sender: Sender<DecodedLeaf<B::Node<Z>, B::Error>>,
    /// The assembler's half.
    receiver: Receiver<DecodedLeaf<B::Node<Z>, B::Error>>,
}

impl<B> LeafEdge<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Create an edge of [`FAN`] slots.
    ///
    /// A full fan amortizes reader and assembler wakeups and lets parsing
    /// run ahead when assembly is slower. Progress needs only one slot, which
    /// the tests' one-slot edge establishes, so the fan is a throughput choice
    /// whose maximum residency the window charges, not a liveness
    /// requirement.
    pub fn new() -> Self {
        Self::with_capacity(FAN)
    }

    /// Create a one-slot edge.
    ///
    /// Test-only because production fixes the capacity at [`FAN`] for
    /// bounded read-ahead. One slot, the smallest capacity a protocol edge
    /// accepts, establishes whether progress depends on that performance
    /// choice.
    #[cfg(test)]
    pub(super) fn one_slot() -> Self {
        Self::with_capacity(1)
    }

    /// Create an edge of `capacity` slots.
    fn with_capacity(capacity: usize) -> Self {
        let (sender, receiver) = channel(
            QueueRole::new(QueueKind::DecodedLeaves, Z::HEIGHT),
            capacity,
        );
        Self { sender, receiver }
    }
}

impl<B> Default for LeafEdge<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Create an edge of [`FAN`] slots, as [`new`](Self::new).
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
pub fn opening_reply<E>(listing: Vec<(u8, Hash)>) -> (Reply<E>, Scope) {
    let scope = Scope::opening(&listing);
    (
        Reply {
            reactions: vec![ProtocolReaction::Query(listing)],
        },
        scope,
    )
}

/// Decode one reply at `level` through `edge` and derive the questions it
/// asks, validating each question's position against `scope`, the record
/// of the question the reply answers.
///
/// At [`ReplyLevel::Leaf`] only an empty request for the leaf is a valid
/// query.
///
/// The reply's supplied leaves cross `edge` to assembly while the reply is
/// read. After an error the edge may hold leaves of the failed reply, so a
/// caller decodes nothing further through it.
// The edge, the session's decode limits, and the reply's level, scope, and
// frames are separate inputs; the pump supplies the limits from its ingress.
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
) -> Result<Decoded<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    // The reply's supplied runs group into nodes one level under the
    // scope's parent: the scope's own children height.
    let children_height = scope.parent().height() - 1;
    let LeafEdge { sender, receiver } = edge;
    let read = ReadReply::read::<B, _>(version_bytes, &ledger, level, scope, frames, sender, codec);
    let assemble = assemble_supplies::<B>(backend, children_height, receiver);
    // The assembler returns `Ok` only after the reader's closing `None`, so
    // the reader never waits on an assembler that has finished. An assembler
    // that fails stops draining the edge, and `try_join` then drops the
    // reader rather than leave it waiting on a full edge.
    let (read, assembled) = try_join(read, assemble).await?;
    Ok(read.into_decoded(assembled))
}

/// Fold one reply's leaves from the edge into complete nodes at the scope's
/// children height, stopping at the reply's closing `None`.
async fn assemble_supplies<B>(
    backend: B,
    height: usize,
    leaves: &mut Receiver<DecodedLeaf<B::Node<Z>, B::Error>>,
) -> Result<Vec<(ErasedPrefix, B::Erased)>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut assembled = assembly(backend, height, leaves);
    let mut nodes = Vec::new();
    while let Some(item) = assembled.next().await {
        nodes.push(item.map_err(DecodeError::Backend)?);
    }
    Ok(nodes)
}

/// Stream one reply's decoded leaves through the backend's node assembler.
///
/// The edge accounts for decoded leaves awaiting the backend. Any further
/// buffering used to construct a node belongs to that backend; `Local`, for
/// example, keeps one complete same-prefix run as replica storage.
///
/// The stream ends at the reply's closing `None`, and is fused, so the
/// assembler cannot read past it into the next reply's leaves.
fn assembly<'a, B>(
    backend: B,
    height: usize,
    leaves: &'a mut Receiver<DecodedLeaf<B::Node<Z>, B::Error>>,
) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send + 'a>>
where
    B: Backend<Node<Z>: Leaf>,
{
    // A closed edge yields `None` too, but the edge's owner holds both
    // halves, so only the reader's closing `None` ends a reply.
    let reply = stream::poll_fn(move |cx| leaves.poll_next_unpin(cx).map(Option::flatten)).fuse();
    #[cfg(test)]
    let reply = reply.inspect(|_| fan_probe::on_recv());
    let reply: BoxNodeStream<'a, B, Z> = Box::pin(reply);
    ops::assemble(backend, height, reply)
}

/// Why a send on a [`LeafEdge`] cannot fail: the edge owns its receiver,
/// and the reader sends only while the edge is borrowed for its reply.
const EDGE_OPEN: &str = "the edge holds its receiver while the reader sends on it";

/// A validated reply skeleton and the scopes its queries created.
struct ReadReply {
    /// Reactions with supplied nodes represented by their prefixes.
    skeleton: Vec<Skeleton>,
    /// Lower scopes created by positional queries.
    questions: Vec<Scope>,
    /// Ordering and scope state for supplied leaves.
    supplies: SupplyRuns,
}

impl ReadReply {
    /// Read and validate one reply while sending its supplied leaves to
    /// assembly, then close the reply on the edge with `None`.
    async fn read<B, F>(
        version_bytes: u64,
        ledger: &SupplyLedger,
        level: ReplyLevel,
        scope: &Scope,
        frames: &mut F,
        leaves: &mut Sender<DecodedLeaf<B::Node<Z>, B::Error>>,
        codec: PayloadCodec,
    ) -> Result<Self, DecodeError<B::Error>>
    where
        B: Backend<Node<Z>: Leaf>,
        F: Stream<Item = Frame> + Unpin,
    {
        let mut positions = scope.positions();
        let mut read = Self {
            skeleton: Vec::new(),
            questions: Vec::new(),
            supplies: SupplyRuns::new(version_bytes),
        };
        loop {
            let Some(frame) = frames.next().await else {
                return Err(DecodeError::TruncatedReply);
            };
            let (reaction, flow) = match frame {
                Frame::Reaction(reaction, flow) => (reaction, flow),
                Frame::End(End::Reply) if read.skeleton.is_empty() => break,
                Frame::End(End::Stream) => return Err(DecodeError::UnexpectedStreamEnd),
                Frame::End(_) => return Err(DecodeError::BareEndAfterReaction),
            };

            match reaction {
                WireReaction::Match => {
                    read.supplies.interrupt();
                    // Reject the overrun at its frame rather than retaining an
                    // unbounded reply skeleton for later walk validation.
                    positions.next().ok_or(ScopeError::UnpositionedMatch)?;
                    read.skeleton.push(Skeleton::Match);
                }
                WireReaction::Query(listing) => {
                    read.supplies.interrupt();
                    read.questions.push(level.derive(&mut positions, &listing)?);
                    read.skeleton.push(Skeleton::Query(listing));
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
                    for record in records.records(codec) {
                        let (version, message) = record.map_err(DecodeError::Record)?;
                        let (leaf_prefix, run) = read
                            .supplies
                            .observe::<B::Error>(scope.parent(), &version)?;
                        if let Some((radix, prefix)) = run {
                            read.skeleton.push(Skeleton::Supply { radix, prefix });
                        }
                        // Charge the peer's declared set length before the
                        // payload enters backend custody.
                        ledger
                            .charge(1)
                            .map_err(|declared| DecodeError::OverdrawnSupply { declared })?;
                        let leaf = <B::Node<Z> as Leaf>::leaf(version, message)
                            .await
                            .map_err(DecodeError::Backend)?;
                        #[cfg(test)]
                        fan_probe::on_send();
                        leaves
                            .send(Some(Ok((leaf_prefix, leaf))))
                            .await
                            .expect(EDGE_OPEN);
                    }
                }
            }

            if flow == Flow::End {
                break;
            }
        }
        leaves.send(None).await.expect(EDGE_OPEN);
        Ok(read)
    }

    /// Replace supply placeholders with assembled nodes and finish the reply.
    fn into_decoded<E>(self, nodes: Vec<(ErasedPrefix, E)>) -> Decoded<E> {
        let Self {
            skeleton,
            questions,
            ..
        } = self;
        let mut nodes = nodes.into_iter();
        let reactions = skeleton
            .into_iter()
            .map(|part| match part {
                Skeleton::Match => ProtocolReaction::Match,
                Skeleton::Query(listing) => ProtocolReaction::Query(listing),
                Skeleton::Supply { radix, prefix } => {
                    let (actual, node) = nodes
                        .next()
                        .expect("each supplied run assembles to exactly one node");
                    assert_eq!(
                        actual, prefix,
                        "assembly preserves the version-derived supplied prefix",
                    );
                    ProtocolReaction::Supply(radix, node)
                }
            })
            .collect();
        assert!(
            nodes.next().is_none(),
            "assembly yields exactly one node per supplied run",
        );
        Decoded {
            reply: Reply { reactions },
            questions,
        }
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
        // it; one arriving over the declaration voids the premise the
        // window solve priced this session with, and fails the session
        // before the record is admitted.
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

/// A reply with supplied nodes represented by their expected prefixes.
enum Skeleton {
    /// A positional match.
    Match,
    /// A positional query with its child listing.
    Query(Vec<(u8, Hash)>),
    /// A supplied node awaiting the assembler's output at `prefix`.
    Supply { radix: u8, prefix: ErasedPrefix },
}

/// Test-gated occupancy probe for the reader/assembler fan channels.
///
/// Counts the decoded leaf records resident between the reader's send
/// ([`ReadReply::read`]) and the assembler's pull, and the peak of that
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
