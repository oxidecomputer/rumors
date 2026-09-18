use std::pin::{Pin, pin};
use std::task::Poll;

use async_stream::try_stream;
use futures::{FutureExt, Stream, StreamExt};

use crate::{
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

/// Create the bounded edge from wire decoding to leaf assembly.
fn leaf_channel<B>(
    capacity: usize,
) -> (
    Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
    Receiver<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::DecodedLeaves, Z::HEIGHT),
        capacity,
    )
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

/// Incrementally decode the initiator's opening-supply reply into whole
/// nodes one level under `parent`, with their root radices, in ascending
/// radix order.
///
/// The wire shape is one supplies-only reply — empty when deletion pruning
/// left nothing to ship — whose leaf records group into subtrees one level
/// under `parent` by their version-derived paths, followed by the stream
/// end. Unlike [`decode_reply`], which materializes one whole reply before
/// yielding it, this stream yields each assembled node as soon as its
/// group completes: the consumer pairs supplies with the responder's
/// root-level requests one radix at a time, so a later group's bulk never
/// gates an earlier group's absorption.
///
/// `version_bytes` is the peer's greeting-declared `max_version_bytes`:
/// a supplied version encoding over it is a
/// [`DecodeError::OversizedVersion`] session violation. `ledger` is the
/// session's declared-`set_len` allowance, charged per record before the
/// payload takes custody ([`DecodeError::OverdrawnSupply`]).
pub fn early_supplies<B, F>(
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    parent: ErasedPrefix,
    frames: F,
    codec: PayloadCodec,
) -> impl Stream<Item = Result<(u8, B::Erased), DecodeError<B::Error>>> + Send
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin + Send + 'static,
{
    try_stream! {
        // The same reader/assembler split as `decode`, driven jointly so
        // completed groups surface while later frames are still arriving.
        let (tx, rx) = leaf_channel::<B>(FAN);
        let mut assembled = assembly(backend.clone(), parent.height() - 1, rx);
        let mut read = pin!(read_early::<B, _>(
            version_bytes,
            &ledger,
            parent,
            frames,
            tx,
            codec,
        ));
        let mut read_result: Option<Result<(), DecodeError<B::Error>>> = None;
        loop {
            let step = futures::future::poll_fn(|cx| {
                if read_result.is_none()
                    && let Poll::Ready(result) = read.poll_unpin(cx)
                {
                    read_result = Some(result);
                }
                // A reader error poisons everything after it: the group in
                // assembly may be a truncation, so nothing more is yielded.
                if matches!(read_result, Some(Err(_))) {
                    return Poll::Ready(None);
                }
                assembled.as_mut().poll_next(cx)
            })
            .await;
            match step {
                Some(node) => {
                    let (prefix, node) = node.map_err(DecodeError::Backend)?;
                    let (actual, radix) = prefix.pop();
                    debug_assert_eq!(actual, parent, "the reader validated every leaf's scope");
                    yield (radix, node);
                }
                None => break,
            }
        }
        match read_result {
            Some(result) => result?,
            None => read.await?,
        }
    }
}

/// Read the opening-supply reply's frames — supplies only, one reply,
/// nothing after it — streaming its leaves to assembly.
async fn read_early<B, F>(
    version_bytes: u64,
    ledger: &SupplyLedger,
    parent: ErasedPrefix,
    mut frames: F,
    leaves: Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
    codec: PayloadCodec,
) -> Result<(), DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    let mut supplies = SupplyRuns::new(version_bytes);
    let mut any = false;
    loop {
        let Some(frame) = frames.next().await else {
            return Err(DecodeError::TruncatedReply);
        };
        let flow = match frame {
            Frame::Reaction(WireReaction::Supply(records), flow) => {
                any = true;
                for record in records.records(codec) {
                    let (version, message) = record.map_err(DecodeError::Record)?;
                    let (leaf_prefix, _) = supplies.observe::<B::Error>(parent, &version)?;
                    // Charge the peer's declared set length before the payload
                    // enters backend custody.
                    ledger
                        .charge(1)
                        .map_err(|declared| DecodeError::OverdrawnSupply { declared })?;
                    let leaf = <B::Node<Z> as Leaf>::leaf(version, message)
                        .await
                        .map_err(DecodeError::Backend)?;
                    #[cfg(test)]
                    fan_probe::on_send();
                    if leaves.send(Ok((leaf_prefix, leaf))).await.is_err() {
                        return Ok(());
                    }
                }
                flow
            }
            // The opening reply has no positional question, so it admits no
            // positional reaction.
            Frame::Reaction(WireReaction::Match, _) => {
                return Err(ScopeError::UnpositionedMatch.into());
            }
            Frame::Reaction(WireReaction::Query(_), _) => {
                return Err(ScopeError::UnpositionedQuery.into());
            }
            Frame::End(End::Reply) if !any => Flow::End,
            Frame::End(End::Reply) => return Err(DecodeError::BareEndAfterReaction),
            Frame::End(End::Stream) => return Err(DecodeError::UnexpectedStreamEnd),
        };
        if flow == Flow::End {
            break;
        }
    }
    // Do not let assembly see leaf EOF until the stream end is consumed.
    // Dropping `leaves` at reply end flushes the final radix group, and the
    // outer stream may then suspend while publishing that child through its
    // one-slot response queue. While suspended, it no longer polls `frames`. On
    // a small-buffered (at minimum, one-byte) link the peer can be blocked
    // flushing `End(Stream)`, which needs precisely that poll to make room. If
    // the response consumer needs later peer progress before it can drain its
    // slot, the peer waits for the receiver, the receiver waits for queue
    // space, and the queue waits for the peer. Keeping the sender alive with
    // this read delays the final group until `End(Stream)` has been consumed,
    // so downstream backpressure cannot strand unread lifecycle bytes.
    if frames.next().await.is_some() {
        return Err(DecodeError::ExtraOpeningReply);
    }
    Ok(())
}

/// Decode one non-leaf reply and derive the lower questions it asks.
pub async fn decode_reply<B, F>(
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    scope: Scope,
    frames: &mut F,
    codec: PayloadCodec,
) -> Result<Decoded<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    decode::<FAN, _, _>(
        backend,
        version_bytes,
        ledger,
        scope,
        frames,
        ReplyLevel::Branch,
        codec,
    )
    .await
}

/// Decode one leaf-height reply, where only an empty request for the leaf is valid.
pub async fn decode_leaf_reply<B, F>(
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    scope: Scope,
    frames: &mut F,
    codec: PayloadCodec,
) -> Result<Decoded<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    decode::<FAN, _, _>(
        backend,
        version_bytes,
        ledger,
        scope,
        frames,
        ReplyLevel::Leaf,
        codec,
    )
    .await
}

/// Decode one non-leaf reply through a one-slot leaf channel.
///
/// Test-only because production fixes the capacity at [`FAN`] for bounded
/// read-ahead. The smallest capacity Tokio permits establishes whether progress
/// depends on that performance choice.
#[cfg(test)]
pub(super) async fn decode_reply_one_slot<B, F>(
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    scope: Scope,
    frames: &mut F,
    codec: PayloadCodec,
) -> Result<Decoded<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    decode::<1, _, _>(
        backend,
        version_bytes,
        ledger,
        scope,
        frames,
        ReplyLevel::Branch,
        codec,
    )
    .await
}

/// Decode one reply while its supplied leaves are assembled concurrently.
async fn decode<const LEAF_CAPACITY: usize, B, F>(
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    scope: Scope,
    frames: &mut F,
    level: ReplyLevel,
    codec: PayloadCodec,
) -> Result<Decoded<B::Erased>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    // The reply's supplied runs group into nodes one level under the
    // scope's parent: the scope's own children height.
    let children_height = scope.parent().height() - 1;
    // A full fan amortizes reader/assembler wakeups and lets parsing run ahead
    // when assembly is slower. Progress needs only one slot: `join` polls both
    // sides, and a blocked send wakes the assembler that drains it. The
    // production fan is therefore a throughput choice whose maximum residency
    // the window charges, not a liveness requirement.
    let (tx, rx) = leaf_channel::<B>(LEAF_CAPACITY);
    let read = ReadReply::read::<B, _>(version_bytes, &ledger, scope, frames, level, tx, codec);
    let assemble = assemble_supplies::<B>(backend, children_height, rx);
    let (read, assembled) = futures::future::join(read, assemble).await;
    let Some(read) = read? else {
        // `read` owns the channel's only sender until it has consumed the
        // complete reply. `assemble_supplies` consumes until that sender is
        // dropped, so it cannot return `Ok` early and close the receiver. A
        // failed send therefore means that assembly stopped on a backend
        // error. Propagate that error: treating the closed receiver as normal
        // completion would discard the rejected leaf and could accept a reply
        // whose decoded tree omits data that the peer supplied.
        assembled?;
        unreachable!("the assembler accepts leaves until it returns an error")
    };
    Ok(read.into_decoded(assembled?))
}

/// Fold the reply's one-slot leaf stream into complete nodes at the
/// scope's children height.
async fn assemble_supplies<B>(
    backend: B,
    height: usize,
    leaves: Receiver<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
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

/// Stream decoded leaves through the backend's node assembler.
///
/// The channel accounts for decoded leaves awaiting the backend. Any further
/// buffering used to construct a node belongs to that backend; `Local`, for
/// example, keeps one complete same-prefix run as replica storage.
fn assembly<B>(
    backend: B,
    height: usize,
    leaves: Receiver<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send>>
where
    B: Backend<Node<Z>: Leaf>,
{
    #[cfg(test)]
    let leaves = leaves.inspect(|_| fan_probe::on_recv());
    let leaves: BoxNodeStream<'static, B, Z> = Box::pin(leaves);
    ops::assemble(backend, height, leaves)
}

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
    /// Read and validate one reply while sending supplied leaves to assembly.
    ///
    /// Returns `None` when assembly drops the leaf receiver before the reply
    /// is complete. [`decode`] keeps the sender alive for this entire future,
    /// so a conforming assembler can close the channel early only by failing.
    /// The joined assembler future then carries the backend error that caused
    /// the rejected leaf.
    async fn read<B, F>(
        version_bytes: u64,
        ledger: &SupplyLedger,
        mut scope: Scope,
        frames: &mut F,
        level: ReplyLevel,
        leaves: Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
        codec: PayloadCodec,
    ) -> Result<Option<Self>, DecodeError<B::Error>>
    where
        B: Backend<Node<Z>: Leaf>,
        F: Stream<Item = Frame> + Unpin,
    {
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
                    scope.next().ok_or(ScopeError::UnpositionedMatch)?;
                    read.skeleton.push(Skeleton::Match);
                }
                WireReaction::Query(listing) => {
                    read.supplies.interrupt();
                    read.questions.push(level.derive(&mut scope, &listing)?);
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
                        if leaves.send(Ok((leaf_prefix, leaf))).await.is_err() {
                            return Ok(None);
                        }
                    }
                }
            }

            if flow == Flow::End {
                break;
            }
        }
        Ok(Some(read))
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
        if actual as u64 > self.version_bytes {
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
/// ([`ReadReply::read`] serves both decode paths) and the assembler's pull, and
/// the peak of that count. The adapter tests drive [`decode`] and
/// [`early_supplies`] on a current-thread runtime and each channel is FIFO with
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
