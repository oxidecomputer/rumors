use std::{future::Future, pin::Pin, pin::pin};

use async_stream::try_stream;
use futures::{Stream, StreamExt};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        convert::Convert,
        message::{Reaction as ProtocolReaction, Reply},
    },
    typed::{
        Path, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

use super::{
    super::codec::{End, Flow, Frame, Reaction as WireReaction},
    error::{EncodeError, OpeningError, ScopeError},
    scope::Scope,
};

/// A wire frame and the lower question it makes publishable once written.
pub struct Encoded<T, Q> {
    frame: Frame<T>,
    question: Option<Q>,
}

impl<T, Q> Encoded<T, Q> {
    /// Write this frame and release its question only after a successful write.
    pub async fn write_with<E, W, F>(self, write: W) -> Result<Option<Q>, E>
    where
        W: FnOnce(Frame<T>) -> F,
        F: Future<Output = Result<(), E>>,
    {
        let Self { frame, question } = self;
        write(frame).await?;
        Ok(question)
    }

    #[cfg(test)]
    pub fn into_parts(self) -> (Frame<T>, Option<Q>) {
        (self.frame, self.question)
    }
}

/// A fallible stream containing the wire frames of one protocol reply.
pub type Frames<T, E, Q> =
    Pin<Box<dyn Stream<Item = Result<Encoded<T, Q>, EncodeError<E>>> + Send>>;

/// Encode the initiator's distinguished opening question.
pub fn encode_opening<B, T>(
    reply: Reply<B, T, UnderRoot>,
) -> Result<Encoded<T, Scope<UnderRoot>>, OpeningError>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let count = reply.replies.len();
    if count != 1 {
        return Err(OpeningError::ReactionCount { count });
    }
    let reaction = reply
        .replies
        .into_iter()
        .next()
        .expect("an opening reply checked to contain one reaction");
    let ProtocolReaction::Query(listing) = reaction else {
        return Err(OpeningError::NotQuery);
    };
    let scope = Scope::opening(&listing);
    Ok(Encoded {
        frame: Frame::Reaction(WireReaction::Query(listing), Flow::End(End::Stream)),
        question: Some(scope),
    })
}

/// Encode one non-leaf reply and derive the lower questions it asks.
pub fn encode_reply<B, T, H>(
    backend: B,
    scope: Scope<S<H>>,
    reply: Reply<B, T, S<H>>,
    end: End,
) -> Frames<T, B::Error, Scope<H>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Convert,
    S<S<H>>: Height,
{
    render(
        backend,
        scope,
        reply,
        end,
        |scope, reaction| match reaction {
            ProtocolReaction::Match => {
                let _ = scope.next();
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
pub fn encode_leaf_reply<B, T>(
    backend: B,
    scope: Scope<Z>,
    reply: Reply<B, T, Z>,
    end: End,
) -> Frames<T, B::Error, Scope<Z>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    render(
        backend,
        scope,
        reply,
        end,
        |scope, reaction| match reaction {
            ProtocolReaction::Match => {
                let _ = scope.next();
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

fn render<B, T, H, Q, D>(
    backend: B,
    mut scope: Scope<H>,
    reply: Reply<B, T, H>,
    end: End,
    mut derive: D,
) -> Frames<T, B::Error, Q>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
    S<H>: Height,
    Q: Send + 'static,
    D: FnMut(&mut Scope<H>, &ProtocolReaction<B, T, H>) -> Result<Option<Q>, ScopeError>
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
                    let mut leaves = pin!(backend.clone().leaves(expected, node));
                    let mut previous = None;
                    let mut supplied = false;
                    while let Some(item) = leaves.next().await {
                        let (prefix, leaf) = item.map_err(EncodeError::Backend)?;
                        validate_leaf(expected, previous, prefix);
                        previous = Some(prefix);
                        supplied = true;

                        let wire = WireReaction::Supply(
                            leaf.ceiling().clone(),
                            leaf.message().clone(),
                        );
                        if let Some((previous, question)) = pending.replace((wire, None)) {
                            yield Encoded {
                                frame: Frame::Reaction(previous, Flow::Continue),
                                question,
                            };
                        }
                    }
                    assert!(supplied, "a backend node contains at least one leaf");
                }
            }
        }

        match pending {
            Some((reaction, question)) => yield Encoded {
                frame: Frame::Reaction(reaction, Flow::End(end)),
                question,
            },
            None => yield Encoded {
                frame: Frame::End(end),
                question: None,
            },
        }
    })
}

fn validate_leaf<H: Height>(expected: Prefix<H>, previous: Option<Prefix<Z>>, current: Prefix<Z>) {
    let path = Path::from(current);
    assert_eq!(
        Prefix::<H>::containing(&path),
        expected,
        "a backend enumerates leaves beneath the requested node prefix",
    );
    if let Some(previous) = previous {
        assert!(
            previous < current,
            "a backend enumerates leaves in strict path order",
        );
    }
}
