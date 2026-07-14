use std::pin::pin;

use async_stream::try_stream;
use before::Version;
use futures::{
    Stream,
    future::{self, BoxFuture},
    stream,
};
use tokio::sync::mpsc::{self, Receiver, Sender, channel};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

mod answer;
mod resolver;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node, Root,
        materialized::{
            Error, OkReceiverStream, Query, Resolution, Resolve, children_of, ok_channel,
            unknown::{Unknown, unknown_providing},
            work::resolver::Resolver,
        },
        message::{self, Reaction, Reply},
        protocol::{BoxResponses, Requests, Responses},
    },
    typed::{
        Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// The safe buffer size for channels which must accommodate up to the tree's
/// branching factor without backpressure.
const FAN: usize = 256;

pub struct Work<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    backend: B,
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
}

impl<B, T> Work<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Construct a new work context.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            tasks: Vec::new(),
        }
    }

    /// Clone out the backend so it can be used elsewhere.
    fn backend(&self) -> B {
        self.backend.clone()
    }

    /// Add a new work queue item to actively drive the stream of responses.
    fn respond<H: Height>(
        &mut self,
        messages: impl Responses<B, T, H, Error<B::Error>>,
    ) -> BoxResponses<B, T, H, Error<B::Error>> {
        let (tx, rx) = mpsc::channel(1);
        self.tasks.push(Box::pin(async move {
            let mut messages = pin!(messages);
            while let Some(item) = messages.next().await {
                send_or_return!(tx, item => Ok(()));
            }
            Ok(())
        }));
        Box::pin(ReceiverStream::new(rx))
    }

    /// Forward a stream of ndoes into a sender, returning them upwards.
    fn return_into<H: Height>(
        &mut self,
        returns: Sender<Option<B::Node<H>>>,
        stream: impl Stream<Item = Result<Option<B::Node<H>>, Error<B::Error>>> + Send + 'static,
    ) {
        self.tasks.push(Box::pin(async move {
            let mut stream = pin!(stream);
            while let Some(item) = stream.next().await {
                send_or_return!(returns, item? => Ok(()));
            }
            Ok(())
        }));
    }

    /// Drive every registered task and the final output to completion.
    ///
    /// Either side may reveal the first error while the other is legitimately
    /// parked waiting for work that the failed session will never produce.
    pub async fn execute<O>(
        self,
        finish: BoxFuture<'static, Result<O, Error<B::Error>>>,
    ) -> Result<O, Error<B::Error>> {
        let mut tasks = pin!(future::join_all(self.tasks));
        let mut finish = pin!(finish);
        tokio::select! {
            finished = &mut tasks => {
                for result in finished {
                    result?;
                }
                finish.await
            }
            output = &mut finish => {
                let output = output?;
                for result in tasks.await {
                    result?;
                }
                Ok(output)
            }
        }
    }

    /// Assemble one level upward and return its one-slot lower-level sender.
    pub fn assemble<H>(
        &mut self,
        returns: Sender<Option<B::Node<S<H>>>>,
        resolutions: impl Stream<Item = Result<Resolution<B, T, H>, Error<B::Error>>> + Send + 'static,
    ) -> Sender<Option<B::Node<H>>>
    where
        H: Height,
        S<H>: Height,
    {
        let (level, level_rx) = ok_channel(1);
        self.return_into(
            returns,
            assemble(self.backend.clone(), resolutions, level_rx),
        );
        level
    }

    /// Assemble leaf resolutions upward with no level beneath them.
    pub fn assemble_leaves(
        &mut self,
        returns: Sender<Option<B::Node<S<Z>>>>,
        resolutions: impl Stream<Item = Result<Resolution<B, T, Z>, Error<B::Error>>> + Send + 'static,
    ) {
        self.return_into(
            returns,
            assemble(self.backend.clone(), resolutions, stream::empty()),
        );
    }

    /// Process the initiator level.
    pub fn initiator_level(
        &mut self,
        ceiling: Version,
        root: Root<B, T>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Query<B, T, UnderRoot>>,
        Sender<Option<B::Node<height::Root>>>,
        BoxFuture<'static, Result<Root<B, T>, Error<B::Error>>>,
    ) {
        let (queries, queries_rx) = channel(1);
        let (returns, mut returns_rx) = channel(1);
        let backend = self.backend();

        let responses = try_stream! {
            let fan = match root.root {
                Some(node) => children_of(&backend, Prefix::new(), node).await?,
                None => Vec::new(),
            };
            yield Reply {
                replies: vec![message::Reaction::Query(
                    fan.iter().map(|(radix, node)| (*radix, node.hash())).collect()
                )],
            };
            send_or_return!(queries, Query {
                prefix: Prefix::new(),
                ours: fan,
            });
        };

        let finish = Box::pin(async move {
            let root = next_or_pending!(returns_rx.recv());
            Ok(Root { ceiling, root })
        });

        (self.respond(responses), queries_rx, returns, finish)
    }

    /// Process the responder level.
    pub fn responder_level(
        &mut self,
        their_version: Version,
        ceiling: Version,
        root: Root<B, T>,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Query<B, T, UnderUnderRoot>>,
        Sender<Option<B::Node<UnderRoot>>>,
        BoxFuture<'static, Result<Root<B, T>, Error<B::Error>>>,
    )
    where
        B: Sync,
    {
        let backend = self.backend();
        let (asked, asked_rx) = mpsc::channel(1);
        let (resolution, resolution_rx) = ok_channel(1);
        let assembling = backend.clone();

        let responses = try_stream! {
            let mut requests = pin!(requests);
            let Some(Reply { replies }) = requests.next().await else {
                return violation!(UnansweredQuery)?;
            };
            let [message::Reaction::Query(theirs)] = replies.as_slice() else {
                return violation!(UnexpectedQuery)?;
            };
            let ours = match root.root {
                Some(node) => children_of(&backend, Prefix::new(), node).await?,
                None => Vec::new(),
            };
            let (reactions, next_queries, resolved) = answer::internal(
                &backend,
                &their_version,
                Prefix::new(),
                ours,
                theirs.clone(),
            )
            .await?;
            yield Reply { replies: reactions };
            for query in next_queries {
                send_or_return!(asked, query);
            }
            send_or_return!(resolution, Resolution {
                prefix: Prefix::new(),
                resolved,
            });
        };

        let (returns, returns_rx) = ok_channel(1);
        let assembled = assemble(assembling, resolution_rx, returns_rx);
        let finish = Box::pin(async move {
            let mut assembled = pin!(assembled);
            let root = next_or_pending!(assembled.next());
            Ok(Root {
                ceiling,
                root: root?,
            })
        });

        (self.respond(responses), asked_rx, returns, finish)
    }

    /// Walk an internal level, where disputes recur into another internal level.
    pub fn internal_level<H>(
        &mut self,
        their_version: Version,
        requests: impl Requests<B, T, S<S<H>>>,
        mut queries: Receiver<Query<B, T, S<S<H>>>>,
    ) -> (
        BoxResponses<B, T, S<H>, Error<B::Error>>,
        Receiver<Query<B, T, H>>,
        OkReceiverStream<Resolution<B, T, S<S<H>>>, Error<B::Error>>,
        OkReceiverStream<Resolution<B, T, S<H>>, Error<B::Error>>,
    )
    where
        B: Sync,
        H: Unknown,
        S<H>: Unknown,
        S<S<H>>: Unknown,
        S<S<S<H>>>: Height,
    {
        let backend = self.backend();
        let (asked, asked_rx) = mpsc::channel(FAN);
        let (upper, upper_rx) = ok_channel(FAN);
        let (lower, lower_rx) = ok_channel(FAN);

        let responses = try_stream! {
            let mut requests = pin!(requests);
            while let Some(Reply { replies }) = requests.next().await {
                let Some(query) = queries.recv().await else {
                    return violation!(UnaskedReply)?;
                };

                let mut resolver = Resolver::new(query);
                for reaction in replies {
                    let Some((prefix, radix, node, listing)) = resolver.react(reaction)? else {
                        continue;
                    };

                    let child_prefix = prefix.push(radix);

                    if listing.is_empty() {
                        let (node, children) =
                            unknown_providing(&backend, &their_version, child_prefix, node).await?;
                        let replies = children
                            .into_iter()
                            .map(|(radix, child)| Reaction::Supply(radix, child))
                            .collect();
                        yield Reply { replies };
                        resolver.ready(radix, node);
                        continue;
                    }

                    let children = children_of(&backend, child_prefix, node).await?;
                    let (reactions, next_queries, resolved) = answer::internal(
                        &backend,
                        &their_version,
                        child_prefix,
                        children,
                        listing,
                    )
                    .await?;
                    yield Reply { replies: reactions };
                    for query in next_queries {
                        send_or_return!(asked, query);
                    }
                    send_or_return!(
                        lower,
                        Resolution {
                            prefix: child_prefix,
                            resolved,
                        }
                    );
                    resolver.pending(radix);
                }

                let resolution = resolver.finish()?;
                send_or_return!(upper, resolution);
            }

            if queries.recv().await.is_some() {
                return violation!(UnansweredQuery)?;
            }
        };

        (self.respond(responses), asked_rx, upper_rx, lower_rx)
    }

    /// Walk leaf parents, where disputes compare content-addressed leaves.
    pub fn leaf_parent_level(
        &mut self,
        their_version: Version,
        requests: impl Requests<B, T, S<Z>>,
        mut queries: Receiver<Query<B, T, S<Z>>>,
    ) -> (
        BoxResponses<B, T, Z, Error<B::Error>>,
        Receiver<Prefix<Z>>,
        OkReceiverStream<Resolution<B, T, S<Z>>, Error<B::Error>>,
        OkReceiverStream<Resolution<B, T, Z>, Error<B::Error>>,
    )
    where
        B: Sync,
    {
        let backend = self.backend();
        let (asked, asked_rx) = mpsc::channel(FAN);
        let (upper, upper_rx) = ok_channel(FAN);
        let (lower, lower_rx) = ok_channel(FAN);

        let responses = try_stream! {
            let mut requests = pin!(requests);
            while let Some(Reply { replies }) = requests.next().await {
                let Some(query) = queries.recv().await else {
                    return violation!(UnaskedReply)?;
                };

                let mut resolver = Resolver::new(query);
                for reaction in replies {
                    let Some((prefix, radix, node, listing)) = resolver.react(reaction)? else {
                        continue;
                    };

                    let child_prefix = prefix.push(radix);

                    if listing.is_empty() {
                        let (node, leaves) =
                            unknown_providing(&backend, &their_version, child_prefix, node).await?;
                        let replies = leaves
                            .into_iter()
                            .map(|(radix, leaf)| Reaction::Supply(radix, leaf))
                            .collect();
                        yield Reply { replies };
                        resolver.ready(radix, node);
                        continue;
                    }

                    let leaves = children_of(&backend, child_prefix, node).await?;
                    let (replies, next_queries, resolved) =
                        answer::leaf_parent(&their_version, child_prefix, leaves, listing);
                    yield Reply { replies };
                    for query in next_queries {
                        send_or_return!(asked, query);
                    }
                    send_or_return!(
                        lower,
                        Resolution {
                            prefix: child_prefix,
                            resolved,
                        }
                    );
                    resolver.pending(radix);
                }

                let resolution = resolver.finish()?;
                send_or_return!(upper, resolution);
            }

            if queries.recv().await.is_some() {
                return violation!(UnansweredQuery)?;
            }
        };

        (self.respond(responses), asked_rx, upper_rx, lower_rx)
    }

    /// Walk leaves, where every query is a terminal request.
    pub fn leaf_level(
        &mut self,
        their_version: Version,
        requests: impl Requests<B, T, Z>,
        mut queries: Receiver<Query<B, T, Z>>,
    ) -> (
        BoxResponses<B, T, Z, Error<B::Error>>,
        OkReceiverStream<Resolution<B, T, Z>, Error<B::Error>>,
    ) {
        let (upper, upper_rx) = ok_channel(1);

        let responses = try_stream! {
            let mut requests = pin!(requests);
            while let Some(Reply { replies }) = requests.next().await {
                let Some(query) = queries.recv().await else {
                    return violation!(UnaskedReply)?;
                };

                let mut resolver = Resolver::new(query);
                for reaction in replies {
                    let Some((_, radix, node, listing)) = resolver.react(reaction)? else {
                        continue;
                    };

                    let (replies, node) =
                        answer::leaf(&their_version, radix, node, listing).map_err(Error::Violation)?;
                    yield Reply { replies };
                    resolver.ready(radix, node);
                }

                let resolution = resolver.finish()?;
                send_or_return!(upper, resolution);
            }

            if queries.recv().await.is_some() {
                return violation!(UnansweredQuery)?;
            }
        };

        (self.respond(responses), upper_rx)
    }
}

/// Complete a stream of resolutions into the parent nodes they describe,
/// filling each [`Resolve::Pending`] slot from `level` in order.
///
/// The pairing is purely positional: resolutions arrive in the order their
/// scopes were asked about, and `level` carries exactly one item per
/// `Pending`, in the same order — the internal contract every walk upholds.
/// An empty resolution (the pruned-to-nothing reply to a request) reaches
/// [`Backend::parent`] with an empty group, which resolves the scope to
/// `None`.
fn assemble<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>(
    backend: B,
    resolutions: impl Stream<Item = Result<Resolution<B, T, H>, Error<B::Error>>> + Send,
    level: impl Stream<Item = Result<Option<B::Node<H>>, Error<B::Error>>> + Send,
) -> impl Stream<Item = Result<Option<B::Node<S<H>>>, Error<B::Error>>> + Send
where
    S<H>: Height,
{
    try_stream! {
        let mut level = pin!(level.fuse());
        for await resolved in resolutions {
            let Resolution { prefix, resolved } = resolved?;
            let mut children = Vec::with_capacity(resolved.len());
            for (radix, slot) in resolved {
                children.push((radix, match slot {
                    Resolve::Ready(child) => child,
                    Resolve::Pending => {
                        // A `Pending` is a promise our own stages made: its
                        // level item exists by construction (one per query, in
                        // order). An early end here means a walk upstream
                        // dropped its channels mid-scope and the session is
                        // already aborting through the driver's error slot, so
                        // park and let the abort win rather than panic into it.
                        next_or_pending!(level.next())?
                    }
                }));
            }
            yield backend.clone().parent(prefix, children).await?;
        }
    }
}

#[cfg(test)]
mod tests;
