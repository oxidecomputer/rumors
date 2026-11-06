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
        ledger::Charge,
    },
    typed::{ErasedPrefix, Path, Prefix, height::Z},
};

#[cfg(test)]
use super::super::codec::Frame;
use super::{
    super::codec::{Flow, LeafRun, Reaction as WireReaction, ReplyFrame, RunBudget},
    super::pipe::{WirePermit, WirePool},
    error::{EncodeError, OpeningError},
    scope::{Replay, ReplyLevel, Scope},
};

/// A wire frame, the lower question it makes publishable once written, and
/// the charge and the permit it carries to the multiplexer.
pub struct Encoded {
    /// The reply frame ready to write.
    frame: ReplyFrame,
    /// The scope made publishable by successfully writing `frame`.
    question: Option<Scope>,
    /// The charge of the chunk whose last frame this is, or no charge for
    /// every earlier frame: the multiplexer drops it once `frame` is
    /// written.
    charge: Charge,
    /// The send pool's permit for a supply run's bytes, which the
    /// multiplexer drops once `frame` is written; empty for every other
    /// frame.
    permit: WirePermit,
}

impl Encoded {
    /// Hand this frame, its charge and its permit to `write`, and release
    /// its question only after a successful write.
    pub async fn write_with<E, W, F>(self, write: W) -> Result<Option<Scope>, E>
    where
        W: FnOnce(ReplyFrame, Charge, WirePermit) -> F,
        F: Future<Output = Result<(), E>>,
    {
        let Self {
            frame,
            question,
            charge,
            permit,
        } = self;
        write(frame, charge, permit).await?;
        Ok(question)
    }

    /// Split off the frame and its question, dropping the charge.
    #[cfg(test)]
    pub fn into_parts(self) -> (Frame, Option<Scope>) {
        (self.frame.into(), self.question)
    }

    /// The bytes of the charge this frame carries.
    #[cfg(test)]
    pub fn charged(&self) -> usize {
        self.charge.bytes()
    }

    /// The bytes of the permit this frame carries.
    #[cfg(test)]
    pub fn permitted(&self) -> usize {
        self.permit.bytes()
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
///
/// Each frame's flow is known as soon as the frame is complete, so the
/// encoder yields every frame then: it never holds a complete frame, a
/// supply run included, besides the one it is yielding. Each run's buffer
/// grows to at most the body the run budget admits, unless one record
/// alone needs more ([`LeafRun::push`]).
///
/// With a send pool ([`pooled`](Self::pooled)), the encoder takes each
/// run's permit when the run's first record arrives, before it copies any
/// of it: a whole run's worth, the body the run budget admits, or the
/// record's own bytes if it alone needs more, beside the run's node in the
/// lane, at most the whole pool ([`WirePool::acquire`]). Once the run
/// closes, it keeps only what the run's buffer holds and returns the rest,
/// and the permit rides the run to the multiplexer. It holds at most one
/// permit at a time: it yields each run, with its permit, before the next
/// record can open another.
pub struct ReplyEncoder<B> {
    /// The store whose supplied nodes are flattened into leaf runs.
    backend: B,
    /// The byte budget of each supplied run's frame.
    budget: RunBudget,
    /// The send pool each run takes its permit from, if any.
    pool: Option<WirePool>,
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
            pool: None,
            scope,
            level,
            replay: Replay::default(),
            written: false,
        }
    }

    /// Take each run's permit from `pool`.
    pub fn pooled(mut self, pool: WirePool) -> Self {
        self.pool = Some(pool);
        self
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
            pool,
            scope,
            level,
            replay,
            written,
        } = self;
        let (backend, budget, level) = (backend.clone(), *budget, *level);
        let pool = pool.clone();
        try_stream! {
            let mut positions = scope.replay(*replay);
            // The chunk's charge rides its last frame to the multiplexer,
            // which drops it once that frame is written; a lane keeps its
            // frames in order, so every earlier frame of the chunk is
            // written by then. The reactions are consumed as they are
            // encoded.
            let Reply {
                reactions,
                mut charge,
                last,
                ..
            } = chunk;
            let mut reactions = reactions.into_iter();
            if reactions.len() == 0 {
                *replay = positions.replayed();
                // A chunk without reactions encodes nothing, unless it is an
                // empty reply's one last chunk, encoded as a bare end.
                if last {
                    assert!(
                        !*written,
                        "a reply's last chunk holds its last reaction, so frames left \
                         flagged to continue are never followed by an empty last chunk",
                    );
                    yield Encoded {
                        frame: ReplyFrame::reply_end(),
                        question: None,
                        charge: mem::take(&mut charge),
                        permit: WirePermit::default(),
                    };
                }
            }
            while let Some(reaction) = reactions.next() {
                // The frame that closes the chunk's last reaction is the
                // chunk's last frame, and the reply's last exactly when the
                // chunk is. So every frame is handed over as soon as it is
                // complete, with its flow already known.
                let closing = reactions.len() == 0;
                let (wire, question, permit) = match reaction {
                    ProtocolReaction::Match(later) => {
                        positions.matched(&later)?;
                        (WireReaction::Match(later), None, WirePermit::default())
                    }
                    ProtocolReaction::Query(listing) => {
                        let question = held_by(Holding::QuestionRecords, || {
                            level.derive(&mut positions, &listing)
                        })?;
                        (WireReaction::Query(listing), Some(question), WirePermit::default())
                    }
                    ProtocolReaction::Supply(radix, node) => {
                        let expected = scope.supplied(radix);
                        let mut leaves = pin!(ops::leaves(backend.clone(), expected, node));
                        let mut previous = None;
                        // One run accumulates this reaction's leaves; it flushes
                        // when the next record would push its wire frame past
                        // the budget and always at the end of the enumeration,
                        // so a run never spans reactions. A run flushed early
                        // is never the reply's last frame, since a record
                        // follows it.
                        let mut run = LeafRun::new();
                        let mut permit = WirePermit::default();
                        while let Some(item) = leaves.next().await {
                            let (prefix, leaf) = item.map_err(EncodeError::Backend)?;
                            validate_leaf(expected, previous, prefix);
                            previous = Some(prefix);

                            // The leaf is consumed by serialization alone: the
                            // run copies its version and message bytes straight
                            // out of the borrowed node, so no Version is cloned
                            // (an ITC allocation). The in-memory backend's walk
                            // allocates nothing per leaf either: it yields a
                            // shared handle to each stored leaf. The bounds span
                            // rides a local so its borrowed join endpoint (the
                            // leaf's version) outlives both reads below.
                            let bounds = leaf.span();
                            let version = bounds.hi();
                            let message = leaf.message();
                            let record = LeafRun::record_len(version, message);
                            if !run.is_empty() && !budget.admits(run.encoded_len(), record) {
                                *written = true;
                                if let Some(pool) = &pool {
                                    pool.keep(&mut permit, run.capacity());
                                }
                                yield Encoded {
                                    frame: ReplyFrame::reaction(
                                        WireReaction::Supply(mem::take(&mut run)),
                                        Flow::Continue,
                                    ),
                                    question: None,
                                    charge: Charge::default(),
                                    permit: mem::take(&mut permit),
                                };
                            }
                            if run.is_empty()
                                && let Some(pool) = &pool
                            {
                                // The record opens a run: a whole run's
                                // worth, or the record's own bytes if it
                                // alone needs more.
                                let need = if budget.covers(record) { budget.body() } else { record };
                                permit = pool.acquire(need).await;
                            }
                            run.push(version, message, budget)
                                .map_err(EncodeError::Record)?;
                        }
                        assert!(!run.is_empty(), "a backend node contains at least one leaf");
                        if let Some(pool) = &pool {
                            pool.keep(&mut permit, run.capacity());
                        }
                        (WireReaction::Supply(run), None, permit)
                    }
                };
                if closing {
                    *replay = positions.replayed();
                }
                *written = true;
                let flow = if closing && last { Flow::End } else { Flow::Continue };
                yield Encoded {
                    frame: ReplyFrame::reaction(wire, flow),
                    question,
                    charge: if closing { mem::take(&mut charge) } else { Charge::default() },
                    permit,
                };
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
