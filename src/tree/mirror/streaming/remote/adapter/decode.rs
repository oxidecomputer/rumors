use std::{convert::Infallible, pin::pin};

use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        backend::BoxNodeStream,
        convert::Convert,
        message::{Reaction as ProtocolReaction, Reply},
    },
    typed::{
        Hash, Path, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    error::{DecodeError, OpeningError, ScopeError},
    scope::Scope,
};

/// One reconstructed reply, its boundary, and any questions it asks next.
pub struct Decoded<B, T, H, Q>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    pub reply: Reply<B, T, H>,
    pub end: End,
    pub questions: Q,
}

/// Decode the initiator's distinguished opening question.
pub fn decode_opening<B, T>(
    frame: Frame<T>,
) -> Result<(Reply<B, T, UnderRoot>, Scope<UnderRoot>), OpeningError>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let Frame::Reaction(WireReaction::Query(listing), Flow::End(End::Stream)) = frame else {
        return Err(OpeningError::InvalidFrame);
    };
    let scope = Scope::opening(&listing);
    Ok((
        Reply {
            replies: vec![ProtocolReaction::Query(listing)],
        },
        scope,
    ))
}

/// Decode one non-leaf reply and derive the lower questions it asks.
pub async fn decode_reply<B, T, H, F>(
    backend: B,
    scope: Scope<S<H>>,
    frames: &mut F,
) -> Result<Decoded<B, T, S<H>, Vec<Scope<H>>>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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

/// Decode one leaf-height reply, where a further query is impossible.
pub async fn decode_leaf_reply<B, T, F>(
    backend: B,
    scope: Scope<Z>,
    frames: &mut F,
) -> Result<Decoded<B, T, Z, ()>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    F: Stream<Item = Frame<T>> + Unpin,
{
    let decoded: Decoded<B, T, Z, Vec<Infallible>> =
        decode(backend, scope, frames, |_scope, _listing| {
            Err(ScopeError::LeafQuery)
        })
        .await?;
    debug_assert!(decoded.questions.is_empty());
    Ok(Decoded {
        reply: decoded.reply,
        end: decoded.end,
        questions: (),
    })
}

async fn decode<B, T, H, F, Q, N>(
    backend: B,
    scope: Scope<H>,
    frames: &mut F,
    question: Q,
) -> Result<Decoded<B, T, H, Vec<N>>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
    S<H>: Height,
    F: Stream<Item = Frame<T>> + Unpin,
    Q: FnMut(&mut Scope<H>, &[(u8, Hash)]) -> Result<N, ScopeError>,
{
    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(1);
    let read = read_reply::<B, T, H, _, _, _>(scope, frames, question, tx);
    let assemble = assemble_supplies::<B, T, H>(backend, rx);
    let (read, assembled) = futures::future::join(read, assemble).await;
    let Some((
        ReadReply {
            skeleton,
            questions,
            ..
        },
        end,
    )) = read?
    else {
        assembled?;
        unreachable!("the assembler accepts leaves until it returns an error")
    };
    let reply = reify(skeleton, assembled?);
    Ok(Decoded {
        reply,
        end,
        questions,
    })
}

/// Read and validate exactly one reply while streaming its leaves to assembly.
async fn read_reply<B, T, H, F, Q, N>(
    mut scope: Scope<H>,
    frames: &mut F,
    mut question: Q,
    leaves: mpsc::Sender<Result<(Prefix<Z>, B::Node<Z>), B::Error>>,
) -> Result<Option<(ReadReply<H, N>, End)>, DecodeError<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    F: Stream<Item = Frame<T>> + Unpin,
    Q: FnMut(&mut Scope<H>, &[(u8, Hash)]) -> Result<N, ScopeError>,
{
    let mut read = ReadReply::new();
    let end = loop {
        let Some(frame) = frames.next().await else {
            return Err(DecodeError::TruncatedReply);
        };
        let (reaction, flow) = match frame {
            Frame::Reaction(reaction, flow) => (reaction, flow),
            Frame::End(end) if read.skeleton.is_empty() => {
                break end;
            }
            Frame::End(_) => return Err(DecodeError::BareEndAfterReaction),
        };

        match reaction {
            WireReaction::Match => {
                read.supplies.interrupt();
                let _ = scope.next();
                read.skeleton.push(Skeleton::Match);
            }
            WireReaction::Query(listing) => {
                read.supplies.interrupt();
                read.questions.push(question(&mut scope, &listing)?);
                read.skeleton.push(Skeleton::Query(listing));
            }
            WireReaction::Supply(version, message) => {
                let (leaf_prefix, run) =
                    read.supplies
                        .observe::<B::Error, T>(scope.parent(), &version, &message)?;
                if let Some((radix, prefix)) = run {
                    read.skeleton.push(Skeleton::Supply { radix, prefix });
                }
                let leaf = <B::Node<Z> as Leaf<T>>::leaf(version, message);
                if leaves.send(Ok((leaf_prefix, leaf))).await.is_err() {
                    return Ok(None);
                }
            }
        }

        if let Flow::End(end) = flow {
            break end;
        }
    };
    Ok(Some((read, end)))
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
    let mut assembled = pin!(H::assemble(backend, leaves));
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
