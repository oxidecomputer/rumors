use crate::tree::mirror::streaming::message::Listed;
#[cfg(test)]
use std::pin::Pin;
use std::{future::Future, mem, pin::pin};

use async_stream::try_stream;
use futures::{Stream, StreamExt};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        erased::{Reaction as ProtocolReaction, Reply, ops},
        holding::{Holding, held_by},
    },
    typed::{ErasedPrefix, Path, Prefix, height::Z},
};

#[cfg(test)]
use super::super::codec::Frame;
use super::{
    super::codec::{Flow, LeafRun, Reaction as WireReaction, ReplyFrame, RunBudget},
    error::{EncodeError, OpeningError},
    scope::{Replay, ReplyLevel, Scope},
};

/// A wire frame and the lower question it makes publishable once written.
pub struct Encoded {
    /// The reply frame ready to write.
    frame: ReplyFrame,
    /// The scope made publishable by successfully writing `frame`.
    question: Option<Scope>,
}

impl Encoded {
    /// Write this frame and release its question only after a successful write.
    pub async fn write_with<E, W, F>(self, write: W) -> Result<Option<Scope>, E>
    where
        W: FnOnce(ReplyFrame) -> F,
        F: Future<Output = Result<(), E>>,
    {
        let Self { frame, question } = self;
        write(frame).await?;
        Ok(question)
    }

    #[cfg(test)]
    pub fn into_parts(self) -> (Frame, Option<Scope>) {
        (self.frame.into(), self.question)
    }
}

/// A fallible stream of the wire frames of one whole protocol reply, boxed
/// as the test encoders return it.
#[cfg(test)]
pub type Frames<'a, E> = Pin<Box<dyn Stream<Item = Result<Encoded, EncodeError<E>>> + Send + 'a>>;

/// Validate the initiator's distinguished opening reply and return its
/// question's listing.
///
/// The opening writes no frame of its own: its one query's content already
/// crossed inside the greeting's root-fan listing. Its encoding therefore
/// reduces to checking the canonical shape, exactly one query, and
/// returning the listing whose scope will interpret the responder's
/// top-level reply.
pub fn opening_listing<E>(
    reactions: impl IntoIterator<Item = ProtocolReaction<E>>,
) -> Result<Vec<Listed>, OpeningError> {
    let mut reactions = reactions.into_iter();
    let Some(first) = reactions.next() else {
        return Err(OpeningError::Empty);
    };
    let ProtocolReaction::Query(listing) = first else {
        return Err(OpeningError::NotQuery);
    };
    if reactions.next().is_some() {
        return Err(OpeningError::Trailing);
    }
    Ok(listing)
}

/// One outgoing reply's encoding, carried across its chunks: the record of
/// the question it answers, how far its reactions have replayed against
/// that record, and whether any of its frames has been written.
///
/// Every frame but the reply's last is flagged [`Flow::Continue`], and a
/// chunk's frames are all released by the time its stream ends, so a chunk
/// that does not end its reply reaches the peer whole before the next
/// chunk exists. The reply's last reaction carries [`Flow::End`], which
/// requires the sender to put that reaction in the reply's last chunk; an
/// empty reply is one empty last chunk, encoded as a bare end.
pub struct ReplyEncoder<B> {
    /// The store whose supplied nodes are flattened into leaf runs.
    backend: B,
    /// The byte budget of each supplied run's frame.
    budget: RunBudget,
    /// The record of the question the reply answers.
    scope: Scope,
    /// How the reply's queries pair with positions.
    level: ReplyLevel,
    /// How far the reply's reactions have replayed against the record:
    /// the positions they took, and the later positions its match covers.
    replay: Replay,
    /// Whether the reply has yielded any frame.
    written: bool,
}

impl<B> ReplyEncoder<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Begin encoding a non-leaf reply to the question `scope` records.
    pub fn branch(backend: B, budget: RunBudget, scope: Scope) -> Self {
        Self::new(backend, budget, scope, ReplyLevel::Branch)
    }

    /// Begin encoding a leaf-height reply to the question `scope` records,
    /// where only an empty request for the leaf is a valid query.
    pub fn leaf(backend: B, budget: RunBudget, scope: Scope) -> Self {
        Self::new(backend, budget, scope, ReplyLevel::Leaf)
    }

    /// Begin encoding a reply at `level`.
    fn new(backend: B, budget: RunBudget, scope: Scope, level: ReplyLevel) -> Self {
        Self {
            backend,
            budget,
            scope,
            level,
            replay: Replay::default(),
            written: false,
        }
    }

    /// Encode the reply's next `chunk`, deriving the lower questions it
    /// asks.
    ///
    /// The stream borrows the encoder until it ends. It is returned unboxed
    /// for the caller to pin where it polls it, so encoding a chunk
    /// allocates nothing for the stream itself.
    ///
    /// # Panics
    ///
    /// The stream panics on an empty last chunk after frames of the same
    /// reply: those frames left flagged to continue, so nothing can end the
    /// reply, and the sender broke the rule that its last chunk holds its
    /// last reaction.
    pub fn chunk(
        &mut self,
        chunk: Reply<B::Erased>,
    ) -> impl Stream<Item = Result<Encoded, EncodeError<B::Error>>> + Send + '_ {
        let Self {
            backend,
            budget,
            scope,
            level,
            replay,
            written,
        } = self;
        let (backend, budget, level) = (backend.clone(), *budget, *level);
        try_stream! {
            let mut positions = scope.replay(*replay);
            let mut pending = None;
            // The chunk's charge returns once every frame is out, when the
            // stream ends; its reactions are consumed as they are encoded.
            let Reply {
                reactions,
                charge: _charge,
                last,
                ..
            } = chunk;
            for reaction in reactions {
                let (wire, question) = match reaction {
                    ProtocolReaction::Match(later) => {
                        positions.matched(&later)?;
                        (WireReaction::Match(later), None)
                    }
                    ProtocolReaction::Query(listing) => {
                        let question = held_by(Holding::QuestionRecords, || {
                            level.derive(&mut positions, &listing)
                        })?;
                        (WireReaction::Query(listing), Some(question))
                    }
                    ProtocolReaction::Supply(radix, node) => {
                        let expected = scope.supplied(radix);
                        let mut leaves = pin!(ops::leaves(backend.clone(), expected, node));
                        let mut previous = None;
                        // One run accumulates this reaction's leaves; it flushes
                        // when the next record would push its wire frame past
                        // the budget and always at the end of the enumeration,
                        // so a run never spans reactions.
                        let mut run = LeafRun::new();
                        while let Some(item) = leaves.next().await {
                            let (prefix, leaf) = item.map_err(EncodeError::Backend)?;
                            validate_leaf(expected, previous, prefix);
                            previous = Some(prefix);

                            // The leaf is consumed by serialization alone: the
                            // run copies its version and message bytes straight
                            // out of the borrowed node, so no Version clone (ITC
                            // allocations) and no Arc bump is paid per leaf. The
                            // bounds span rides a local so its borrowed join
                            // endpoint (the leaf's version) outlives both reads
                            // below.
                            let bounds = leaf.span();
                            let version = bounds.hi();
                            let message = leaf.message();
                            if !run.is_empty()
                                && !budget.admits(
                                    run.encoded_len(),
                                    LeafRun::record_len(version, message),
                                )
                            {
                                let full = mem::take(&mut run);
                                if let Some((ready, question)) =
                                    pending.replace((WireReaction::Supply(full), None))
                                {
                                    *written = true;
                                    yield Encoded {
                                        frame: ReplyFrame::reaction(ready, Flow::Continue),
                                        question,
                                    };
                                }
                            }
                            run.push(version, message).map_err(EncodeError::Record)?;
                        }
                        assert!(!run.is_empty(), "a backend node contains at least one leaf");
                        (WireReaction::Supply(run), None)
                    }
                };
                if let Some((previous, question)) = pending.replace((wire, question)) {
                    *written = true;
                    yield Encoded {
                        frame: ReplyFrame::reaction(previous, Flow::Continue),
                        question,
                    }
                }
            }
            *replay = positions.replayed();

            match pending {
                Some((reaction, question)) => {
                    *written = true;
                    let flow = if last { Flow::End } else { Flow::Continue };
                    yield Encoded {
                        frame: ReplyFrame::reaction(reaction, flow),
                        question,
                    }
                }
                None if last => {
                    assert!(
                        !*written,
                        "a reply's last chunk holds its last reaction, so frames left \
                         flagged to continue are never followed by an empty last chunk",
                    );
                    yield Encoded {
                        frame: ReplyFrame::reply_end(),
                        question: None,
                    }
                }
                None => {}
            }
        }
    }
}

/// Encode one whole non-leaf `reply`, given as one last chunk, deriving the
/// lower questions it asks.
#[cfg(test)]
pub fn encode_reply<B>(
    backend: B,
    budget: RunBudget,
    scope: Scope,
    reply: Reply<B::Erased>,
) -> Frames<'static, B::Error>
where
    B: Backend<Node<Z>: Leaf>,
{
    whole(ReplyEncoder::branch(backend, budget, scope), reply)
}

/// Encode one whole leaf-height `reply`, given as one last chunk, where
/// only an empty request for the leaf is valid.
#[cfg(test)]
pub fn encode_leaf_reply<B>(
    backend: B,
    budget: RunBudget,
    scope: Scope,
    reply: Reply<B::Erased>,
) -> Frames<'static, B::Error>
where
    B: Backend<Node<Z>: Leaf>,
{
    whole(ReplyEncoder::leaf(backend, budget, scope), reply)
}

/// Encode `reply` as the one chunk of `encoder`'s reply.
#[cfg(test)]
fn whole<B>(mut encoder: ReplyEncoder<B>, reply: Reply<B::Erased>) -> Frames<'static, B::Error>
where
    B: Backend<Node<Z>: Leaf>,
{
    Box::pin(try_stream! {
        let mut frames = pin!(encoder.chunk(reply));
        while let Some(frame) = frames.next().await {
            yield frame?;
        }
    })
}

/// Enforce the backend's containment and ordering contract for one leaf.
fn validate_leaf(expected: ErasedPrefix, previous: Option<Prefix<Z>>, current: Prefix<Z>) {
    let path = Path::from(current);
    assert_eq!(
        &<[u8; 32]>::from(path)[..expected.as_bytes().len()],
        expected.as_bytes(),
        "a backend enumerates leaves beneath the requested node prefix",
    );
    if let Some(previous) = previous {
        assert!(
            previous < current,
            "a backend enumerates leaves in strict path order",
        );
    }
}
