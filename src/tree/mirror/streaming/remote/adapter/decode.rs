use crate::message::PayloadCodec;
use std::pin::pin;
use std::task::Poll;

use async_stream::try_stream;
use futures::{FutureExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        backend::BoxNodeStream,
        erased::{Reaction as ProtocolReaction, Reply, ops},
        materialized::SupplyLedger,
        window::FAN,
    },
    typed::{ErasedPrefix, Hash, Path, Prefix, height::Z},
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    error::{DecodeError, ScopeError},
    scope::Scope,
};

/// One reconstructed reply and any questions it asks next.
pub struct Decoded<E, Q> {
    pub reply: Reply<E>,
    pub questions: Q,
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
            replies: vec![ProtocolReaction::Query(listing)],
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
        let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
        let leaves = ReceiverStream::new(rx);
        #[cfg(test)]
        let leaves = leaves.inspect(|_| fan_probe::on_recv());
        let leaves: BoxNodeStream<'static, B, Z> = Box::pin(leaves);
        let mut assembled = pin!(ops::assemble(
            backend.clone(),
            parent.height() - 1,
            leaves
        ));
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
    leaves: mpsc::Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
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
                    // The set-length half of the greeting's priced
                    // premises, charged per record before the payload
                    // takes backend custody: a peer supplying past its
                    // declaration fails at the offending record, while
                    // the reply is still open.
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
            // The codec's per-stream grammar admits no other reaction here,
            // so positional forms surface as their unpositioned rejections
            // only when frames are constructed in process.
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
) -> Result<Decoded<B::Erased, Vec<Scope>>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    decode(
        backend,
        version_bytes,
        ledger,
        scope,
        frames,
        |scope, listing| {
            let (_, prefix) = scope.next().ok_or(ScopeError::UnpositionedQuery)?;
            Ok(Scope::new(prefix, listing))
        },
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
) -> Result<Decoded<B::Erased, Vec<Scope>>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
{
    decode(
        backend,
        version_bytes,
        ledger,
        scope,
        frames,
        |scope, listing| {
            if !listing.is_empty() {
                return Err(ScopeError::NonemptyLeafQuery);
            }
            let (_, prefix) = scope.next().ok_or(ScopeError::UnpositionedQuery)?;
            Ok(Scope::leaf(prefix))
        },
        codec,
    )
    .await
}

async fn decode<B, F, Q, N>(
    backend: B,
    version_bytes: u64,
    ledger: SupplyLedger,
    scope: Scope,
    frames: &mut F,
    question: Q,
    codec: PayloadCodec,
) -> Result<Decoded<B::Erased, Vec<N>>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
    Q: FnMut(&mut Scope, &[(u8, Hash)]) -> Result<N, ScopeError>,
{
    // The reply's supplied runs group into nodes one level under the
    // scope's parent: the scope's own children height.
    let children_height = scope.parent().height() - 1;
    // One fan of buffered leaves, amortizing the reader/assembler waker
    // round trip over runs of consecutive leaves instead of paying it per
    // leaf. The capacity is load-bearing for liveness: the channel must
    // admit one full fan of records while the assembler holds a parent
    // group open, so no configuration may shrink it. Its residency is
    // charged: each slot holds a backend-priced node — the payload's
    // custody already passed to the backend at `Leaf::leaf` — and the
    // session budget prices all of them, one fan plus the record in the
    // reader's hand per reply stream, at `node_bytes(0, version_bound)`
    // plus the slot itself (the window's supply-decode envelope).
    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
    let read = read_reply::<B, _, _, _>(version_bytes, &ledger, scope, frames, question, tx, codec);
    let assemble = assemble_supplies::<B>(backend, children_height, rx);
    let (read, assembled) = futures::future::join(read, assemble).await;
    let Some(ReadReply {
        skeleton,
        questions,
        ..
    }) = read?
    else {
        assembled?;
        unreachable!("the assembler accepts leaves until it returns an error")
    };
    let reply = reify::<B::Erased>(skeleton, assembled?);
    Ok(Decoded { reply, questions })
}

/// Read and validate exactly one reply while streaming its leaves to assembly.
async fn read_reply<B, F, Q, N>(
    version_bytes: u64,
    ledger: &SupplyLedger,
    mut scope: Scope,
    frames: &mut F,
    mut question: Q,
    leaves: mpsc::Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
    codec: PayloadCodec,
) -> Result<Option<ReadReply<N>>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    F: Stream<Item = Frame> + Unpin,
    Q: FnMut(&mut Scope, &[(u8, Hash)]) -> Result<N, ScopeError>,
{
    let mut read = ReadReply::new(version_bytes);
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
                // Eager, symmetric with the query arm: a match past the
                // question's fan fails at its own frame, so a
                // nonconforming peer cannot grow the skeleton unboundedly
                // before the walk's whole-reply validation would see it.
                scope.next().ok_or(ScopeError::UnpositionedMatch)?;
                read.skeleton.push(Skeleton::Match);
            }
            WireReaction::Query(listing) => {
                read.supplies.interrupt();
                read.questions.push(question(&mut scope, &listing)?);
                read.skeleton.push(Skeleton::Query(listing));
            }
            WireReaction::Supply(records) => {
                // An empty run is unreachable from wire bytes (the codec
                // rejects it as `LeafRunError::Empty`) but constructible in
                // process, and the record loop below would silently drop the
                // reaction with it.
                debug_assert!(
                    !records.is_empty(),
                    "the codec never yields an empty supply run",
                );
                // Records leave the run one at a time and flow straight into
                // assembly: the whole-run bound is its encoded bytes, never a
                // decoded vector of leaves.
                for record in records.records(codec) {
                    let (version, message) = record.map_err(DecodeError::Record)?;
                    let (leaf_prefix, run) = read
                        .supplies
                        .observe::<B::Error>(scope.parent(), &version)?;
                    if let Some((radix, prefix)) = run {
                        read.skeleton.push(Skeleton::Supply { radix, prefix });
                    }
                    // The set-length half of the greeting's priced
                    // premises, charged per record before the payload
                    // takes backend custody: a peer supplying past its
                    // declaration fails at the offending record, while
                    // the reply is still open.
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

/// Fold the reply's one-slot leaf stream into complete nodes at the
/// scope's children height.
async fn assemble_supplies<B>(
    backend: B,
    height: usize,
    leaves: mpsc::Receiver<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
) -> Result<Vec<(ErasedPrefix, B::Erased)>, DecodeError<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let leaves = ReceiverStream::new(leaves);
    #[cfg(test)]
    let leaves = leaves.inspect(|_| fan_probe::on_recv());
    let leaves: BoxNodeStream<'static, B, Z> = Box::pin(leaves);
    let mut assembled = pin!(ops::assemble(backend, height, leaves));
    let mut nodes = Vec::new();
    while let Some(item) = assembled.next().await {
        nodes.push(item.map_err(DecodeError::Backend)?);
    }
    Ok(nodes)
}

/// Replace supplied-prefix placeholders with the nodes assembled for them.
fn reify<E>(skeleton: Vec<Skeleton>, nodes: Vec<(ErasedPrefix, E)>) -> Reply<E> {
    let mut nodes = nodes.into_iter();
    let replies = skeleton
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
    Reply { replies }
}

struct ReadReply<N> {
    skeleton: Vec<Skeleton>,
    questions: Vec<N>,
    supplies: SupplyRuns,
}

impl<N> ReadReply<N> {
    fn new(version_bytes: u64) -> Self {
        Self {
            skeleton: Vec::new(),
            questions: Vec::new(),
            supplies: SupplyRuns::new(version_bytes),
        }
    }
}

struct SupplyRuns {
    /// The peer's greeting-declared `max_version_bytes`, covering every
    /// version its tree materializes and so every version it may supply.
    version_bytes: u64,
    previous_leaf: Option<Prefix<Z>>,
    current: Option<ErasedPrefix>,
    previous_radix: Option<u8>,
}

impl SupplyRuns {
    fn new(version_bytes: u64) -> Self {
        Self {
            version_bytes,
            previous_leaf: None,
            current: None,
            previous_radix: None,
        }
    }

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
                previous: previous
                    .as_bytes()
                    .try_into()
                    .expect("a leaf prefix occupies a full content path"),
                current: path.into(),
            });
        }
        self.previous_leaf = Some(leaf_prefix);

        let run = if self.current != Some(node_prefix) {
            if let Some(previous) = self.previous_radix.filter(|previous| *previous >= radix) {
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

enum Skeleton {
    Match,
    Query(Vec<(u8, Hash)>),
    Supply { radix: u8, prefix: ErasedPrefix },
}

/// Test-gated occupancy probe for the reader/assembler fan channels.
///
/// Counts the decoded leaf records resident between the reader's send
/// ([`read_reply`] and [`read_early`] hook the same counter) and the
/// assembler's pull, and the peak of that count. The adapter tests
/// drive [`decode`] and [`early_supplies`] on a current-thread runtime
/// and each channel is FIFO with one producer and one consumer, so a
/// thread-local counter mirrors the occupancy exactly: incremented
/// before the reader awaits the send (the record in the reader's hand
/// is resident), decremented when the assembler's stream yields the
/// record.
#[cfg(test)]
pub(super) mod fan_probe {
    use std::cell::Cell;

    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    // fallback-TLS lowering (illumos among the gate's targets) and denies
    // initializers that already sit in `const` blocks; the allow keeps
    // `-D warnings` honest on every platform the gate runs.
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
