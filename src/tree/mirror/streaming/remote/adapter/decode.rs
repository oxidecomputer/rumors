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
        convert::Convert,
        message::{Reaction as ProtocolReaction, Reply},
        window::FAN,
    },
    typed::{
        Hash, Path, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    error::{DecodeError, ScopeError},
    scope::Scope,
};

/// One reconstructed reply and any questions it asks next.
pub struct Decoded<B, T, H, Q>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    pub reply: Reply<B, T, H>,
    pub questions: Q,
}

/// Replay the initiator's distinguished opening question from the root-fan
/// listing its greeting carried.
///
/// No wire frame exists for the opening: the greeting decode already
/// validated the listing's canonical order, so synthesizing the one-query
/// reply and its root scope is infallible. An empty listing replays an empty
/// opening `Query` — the empty-tree initiator's "send everything".
pub fn opening_reply<B, T>(listing: Vec<(u8, Hash)>) -> (Reply<B, T, UnderRoot>, Scope<UnderRoot>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let scope = Scope::opening(&listing);
    (
        Reply {
            replies: vec![ProtocolReaction::Query(listing)],
        },
        scope,
    )
}

/// Incrementally decode the initiator's opening-supply reply into whole
/// height-`G` nodes with their root radices, in ascending radix order.
///
/// The wire shape is one supplies-only reply — empty when deletion pruning
/// left nothing to ship — whose leaf records group into height-`G`
/// subtrees by their content-derived paths under `parent`, followed by the
/// stream end. Unlike [`decode_reply`], which materializes one whole reply
/// before yielding it, this stream yields each assembled node as soon as
/// its group completes: the consumer pairs supplies with the responder's
/// root-level requests one radix at a time, so a later group's bulk never
/// gates an earlier group's absorption.
pub fn early_supplies<B, T, G, F>(
    backend: B,
    parent: Prefix<S<G>>,
    frames: F,
) -> impl Stream<Item = Result<(u8, B::Node<G>), DecodeError<B::Error>>> + Send
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    G: Convert,
    S<G>: Height,
    F: Stream<Item = Frame<T>> + Unpin + Send + 'static,
{
    try_stream! {
        // The same reader/assembler split as `decode`, driven jointly so
        // completed groups surface while later frames are still arriving.
        let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
        let leaves: BoxNodeStream<'static, B, T, Z> = Box::pin(ReceiverStream::new(rx));
        let mut assembled = pin!(backend.clone().assemble::<G>(leaves));
        let mut read = pin!(read_early::<B, T, G, _>(parent, frames, tx));
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
async fn read_early<B, T, G, F>(
    parent: Prefix<S<G>>,
    mut frames: F,
    leaves: mpsc::Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
) -> Result<(), DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    G: Height,
    S<G>: Height,
    F: Stream<Item = Frame<T>> + Unpin,
{
    let mut supplies = SupplyRuns::<G>::new();
    let mut any = false;
    loop {
        let Some(frame) = frames.next().await else {
            return Err(DecodeError::TruncatedReply);
        };
        let flow = match frame {
            Frame::Reaction(WireReaction::Supply(records), flow) => {
                any = true;
                for record in records.records() {
                    let (version, message) = record.map_err(DecodeError::Record)?;
                    let (leaf_prefix, _) =
                        supplies.observe::<B::Error, T>(parent, &version, &message)?;
                    let leaf = <B::Node<Z> as Leaf<T>>::leaf(version, message)
                        .await
                        .map_err(DecodeError::Backend)?;
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
pub async fn decode_reply<B, T, H, F>(
    backend: B,
    scope: Scope<S<H>>,
    frames: &mut F,
) -> Result<Decoded<B, T, S<H>, Vec<Scope<H>>>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    H: Height,
    S<H>: Convert,
    S<S<H>>: Height,
    F: Stream<Item = Frame<T>> + Unpin,
{
    decode(backend, scope, frames, |scope, listing| {
        let (_, prefix) = scope.next().ok_or(ScopeError::UnpositionedQuery)?;
        Ok(Scope::new(prefix, listing))
    })
    .await
}

/// Decode one leaf-height reply, where only an empty request for the leaf is valid.
pub async fn decode_leaf_reply<B, T, F>(
    backend: B,
    scope: Scope<Z>,
    frames: &mut F,
) -> Result<Decoded<B, T, Z, Vec<Scope<Z>>>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    F: Stream<Item = Frame<T>> + Unpin,
{
    decode(backend, scope, frames, |scope, listing| {
        if !listing.is_empty() {
            return Err(ScopeError::NonemptyLeafQuery);
        }
        let (_, prefix) = scope.next().ok_or(ScopeError::UnpositionedQuery)?;
        Ok(Scope::leaf(prefix))
    })
    .await
}

async fn decode<B, T, H, F, Q, N>(
    backend: B,
    scope: Scope<H>,
    frames: &mut F,
    question: Q,
) -> Result<Decoded<B, T, H, Vec<N>>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    H: Convert,
    S<H>: Height,
    F: Stream<Item = Frame<T>> + Unpin,
    Q: FnMut(&mut Scope<H>, &[(u8, Hash)]) -> Result<N, ScopeError>,
{
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
    let read = read_reply::<B, T, H, _, _, _>(scope, frames, question, tx);
    let assemble = assemble_supplies::<B, T, H>(backend, rx);
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
    let reply = reify(skeleton, assembled?);
    Ok(Decoded { reply, questions })
}

/// Read and validate exactly one reply while streaming its leaves to assembly.
async fn read_reply<B, T, H, F, Q, N>(
    mut scope: Scope<H>,
    frames: &mut F,
    mut question: Q,
    leaves: mpsc::Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
) -> Result<Option<ReadReply<H, N>>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    F: Stream<Item = Frame<T>> + Unpin,
    Q: FnMut(&mut Scope<H>, &[(u8, Hash)]) -> Result<N, ScopeError>,
{
    let mut read = ReadReply::new();
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
                for record in records.records() {
                    let (version, message) = record.map_err(DecodeError::Record)?;
                    let (leaf_prefix, run) =
                        read.supplies
                            .observe::<B::Error, T>(scope.parent(), &version, &message)?;
                    if let Some((radix, prefix)) = run {
                        read.skeleton.push(Skeleton::Supply { radix, prefix });
                    }
                    let leaf = <B::Node<Z> as Leaf<T>>::leaf(version, message)
                        .await
                        .map_err(DecodeError::Backend)?;
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

/// Fold the reply's one-slot leaf stream into complete height-`H` nodes.
async fn assemble_supplies<B, T, H>(
    backend: B,
    leaves: mpsc::Receiver<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
) -> Result<Vec<(Prefix<H>, B::Node<H>)>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
{
    let leaves: BoxNodeStream<'_, B, T, Z> = Box::pin(ReceiverStream::new(leaves));
    let mut assembled = pin!(backend.assemble::<H>(leaves));
    let mut nodes = Vec::new();
    while let Some(item) = assembled.next().await {
        nodes.push(item.map_err(DecodeError::Backend)?);
    }
    Ok(nodes)
}

/// Replace supplied-prefix placeholders with the nodes assembled for them.
fn reify<B, T, H>(skeleton: Vec<Skeleton<H>>, nodes: Vec<(Prefix<H>, B::Node<H>)>) -> Reply<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
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
                    "assembly preserves the content-derived supplied prefix",
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

struct ReadReply<H: Height, N> {
    skeleton: Vec<Skeleton<H>>,
    questions: Vec<N>,
    supplies: SupplyRuns<H>,
}

impl<H: Height, N> ReadReply<H, N> {
    fn new() -> Self {
        Self {
            skeleton: Vec::new(),
            questions: Vec::new(),
            supplies: SupplyRuns::new(),
        }
    }
}

struct SupplyRuns<H: Height> {
    previous_leaf: Option<Prefix<Z>>,
    current: Option<Prefix<H>>,
    previous_radix: Option<u8>,
}

impl<H: Height> SupplyRuns<H> {
    fn new() -> Self {
        Self {
            previous_leaf: None,
            current: None,
            previous_radix: None,
        }
    }

    fn interrupt(&mut self) {
        self.current = None;
    }

    /// Validate one supplied leaf and identify the start of a new run.
    fn observe<E, T>(
        &mut self,
        expected_parent: Prefix<S<H>>,
        version: &crate::Version,
        message: &crate::message::Message<T>,
    ) -> Result<(Prefix<Z>, Option<(u8, Prefix<H>)>), DecodeError<E>>
    where
        T: Send + Sync + 'static,
        S<H>: Height,
    {
        let path = Path::for_leaf(version, message.as_slice());
        let leaf_prefix = Prefix::<Z>::containing(&path);
        let node_prefix = Prefix::<H>::containing(&path);
        let (parent, radix) = node_prefix.pop();
        if parent != expected_parent {
            return Err(DecodeError::LeafOutsideScope {
                expected: expected_parent.as_bytes().to_vec(),
                actual: path.into(),
            });
        }
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

enum Skeleton<H: Height> {
    Match,
    Query(Vec<(u8, Hash)>),
    Supply { radix: u8, prefix: Prefix<H> },
}
