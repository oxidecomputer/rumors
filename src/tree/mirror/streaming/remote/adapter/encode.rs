use std::{future::Future, mem, pin::Pin, pin::pin};

use async_stream::try_stream;
use futures::{Stream, StreamExt};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        erased::{Reaction as ProtocolReaction, Reply, ops},
    },
    typed::{ErasedPrefix, Hash, Path, Prefix, height::Z},
};

use super::{
    super::codec::{End, Flow, Frame, LeafRun, Reaction as WireReaction, RunBudget},
    error::{EncodeError, OpeningError, ScopeError},
    scope::Scope,
};

/// A wire frame and the lower question it makes publishable once written.
pub struct Encoded<Q> {
    frame: Frame,
    question: Option<Q>,
}

impl<Q> Encoded<Q> {
    /// Write this frame and release its question only after a successful write.
    pub async fn write_with<E, W, F>(self, write: W) -> Result<Option<Q>, E>
    where
        W: FnOnce(Frame) -> F,
        F: Future<Output = Result<(), E>>,
    {
        let Self { frame, question } = self;
        write(frame).await?;
        Ok(question)
    }

    #[cfg(test)]
    pub fn into_parts(self) -> (Frame, Option<Q>) {
        (self.frame, self.question)
    }
}

/// A fallible stream containing the wire frames of one protocol reply.
pub type Frames<E, Q> = Pin<Box<dyn Stream<Item = Result<Encoded<Q>, EncodeError<E>>> + Send>>;

/// Validate the initiator's distinguished opening reply and split it into
/// its question's listing and its early whole-subtree supplies.
///
/// The opening *question* writes no frame of its own — its content already
/// crossed inside the greeting's root-fan listing — so its encoding reduces
/// to checking the canonical shape (one leading query, then only supplies)
/// and returning the listing whose scope will interpret the responder's
/// top-level reply. The trailing supplies are the initiator's exclusive
/// root children; they alone occupy wire frames, as the opening-supply
/// reply on the initiator's first stream.
#[allow(clippy::type_complexity)]
pub fn opening_parts<E>(
    reply: Reply<E>,
) -> Result<(Vec<(u8, Hash)>, Vec<ProtocolReaction<E>>), OpeningError> {
    let mut reactions = reply.replies.into_iter();
    let Some(first) = reactions.next() else {
        return Err(OpeningError::Empty);
    };
    let ProtocolReaction::Query(listing) = first else {
        return Err(OpeningError::NotQuery);
    };
    let supplies: Vec<_> = reactions.collect();
    if let Some(index) = supplies
        .iter()
        .position(|reaction| !matches!(reaction, ProtocolReaction::Supply(_, _)))
    {
        // Positions are reported in whole-reply terms; the query is 0.
        return Err(OpeningError::NotSupply { index: index + 1 });
    }
    Ok((listing, supplies))
}

/// Encode one non-leaf reply and derive the lower questions it asks.
pub fn encode_reply<B>(
    backend: B,
    budget: RunBudget,
    scope: Scope,
    reply: Reply<B::Erased>,
) -> Frames<B::Error, Scope>
where
    B: Backend<Node<Z>: Leaf>,
{
    render(
        backend,
        budget,
        scope,
        reply,
        |scope, reaction| match reaction {
            ProtocolReaction::Match => {
                // Symmetric with decode: a match past the question's fan
                // is unrepresentable on the wire.
                scope.next().ok_or(ScopeError::UnpositionedMatch)?;
                Ok(None)
            }
            ProtocolReaction::Query(listing) => {
                let (_, prefix) = scope.next().ok_or(ScopeError::UnpositionedQuery)?;
                Ok(Some(Scope::new(prefix, listing)))
            }
            ProtocolReaction::Supply(_, _) => Ok(None),
        },
    )
}

/// Encode one leaf-height reply, where only an empty request for the leaf is valid.
pub fn encode_leaf_reply<B>(
    backend: B,
    budget: RunBudget,
    scope: Scope,
    reply: Reply<B::Erased>,
) -> Frames<B::Error, Scope>
where
    B: Backend<Node<Z>: Leaf>,
{
    render(
        backend,
        budget,
        scope,
        reply,
        |scope, reaction| match reaction {
            ProtocolReaction::Match => {
                // Symmetric with decode: a match past the question's fan
                // is unrepresentable on the wire.
                scope.next().ok_or(ScopeError::UnpositionedMatch)?;
                Ok(None)
            }
            ProtocolReaction::Query(listing) if !listing.is_empty() => {
                Err(ScopeError::NonemptyLeafQuery)
            }
            ProtocolReaction::Query(_) => {
                let (_, prefix) = scope.next().ok_or(ScopeError::UnpositionedQuery)?;
                Ok(Some(Scope::leaf(prefix)))
            }
            ProtocolReaction::Supply(_, _) => Ok(None),
        },
    )
}

fn render<B, D>(
    backend: B,
    budget: RunBudget,
    mut scope: Scope,
    reply: Reply<B::Erased>,
    mut derive: D,
) -> Frames<B::Error, Scope>
where
    B: Backend<Node<Z>: Leaf>,
    D: FnMut(&mut Scope, &ProtocolReaction<B::Erased>) -> Result<Option<Scope>, ScopeError>
        + Send
        + 'static,
{
    Box::pin(try_stream! {
        let mut pending = None;
        for reaction in reply.replies {
            let question = derive(&mut scope, &reaction)?;
            match reaction {
                ProtocolReaction::Match => {
                    if let Some((previous, question)) =
                        pending.replace((WireReaction::Match, question))
                    {
                        yield Encoded {
                            frame: Frame::Reaction(previous, Flow::Continue),
                            question,
                        };
                    }
                }
                ProtocolReaction::Query(listing) => {
                    if let Some((previous, question)) =
                        pending.replace((WireReaction::Query(listing), question))
                    {
                        yield Encoded {
                            frame: Frame::Reaction(previous, Flow::Continue),
                            question,
                        };
                    }
                }
                ProtocolReaction::Supply(radix, node) => {
                    debug_assert!(question.is_none());
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
                            && !budget
                                .admits(run.encoded_len(), LeafRun::record_len(version, message))
                        {
                            let full = mem::take(&mut run);
                            if let Some((ready, question)) =
                                pending.replace((WireReaction::Supply(full), None))
                            {
                                yield Encoded {
                                    frame: Frame::Reaction(ready, Flow::Continue),
                                    question,
                                };
                            }
                        }
                        run.push(version, message).map_err(EncodeError::Record)?;
                    }
                    assert!(!run.is_empty(), "a backend node contains at least one leaf");
                    if let Some((ready, question)) =
                        pending.replace((WireReaction::Supply(run), None))
                    {
                        yield Encoded {
                            frame: Frame::Reaction(ready, Flow::Continue),
                            question,
                        };
                    }
                }
            }
        }

        match pending {
            Some((reaction, question)) => yield Encoded {
                frame: Frame::Reaction(reaction, Flow::End),
                question,
            },
            None => yield Encoded {
                frame: Frame::End(End::Reply),
                question: None,
            },
        }
    })
}

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
