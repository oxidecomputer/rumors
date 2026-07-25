//! Phase-specific materialized reconciliation walks.

use std::collections::BTreeMap;
use std::pin::pin;

use async_stream::try_stream;
use before::Version;
use futures::future::BoxFuture;
use tokio::sync::oneshot;
use tokio_stream::StreamExt;

use super::{Work, answer, assembly::assemble, queues::*, resolver::Resolver};
#[cfg(test)]
use crate::tree::mirror::streaming::materialized::progress;
use crate::tree::{
    mirror::contained,
    mirror::streaming::{
        Backend, Leaf, Node, Root,
        materialized::{
            Error, OkReceiverStream, Query, Resolution, Resolve, Violation,
            channel::{Receiver, Sender},
            children_of, fan_listing,
            unknown::{Unknown, unknown, unknown_providing},
            violation,
        },
        message::{self, Reaction, Reply},
        protocol::{BoxResponses, Requests},
        tasks::next_or_cancelled,
    },
    typed::{
        Hash, Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

impl<B, T> Work<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Process the initiator level.
    ///
    /// `fan` is the greeting-time root fan: the opening question is derived
    /// from it by [`fan_listing`] — the same derivation the greeting itself
    /// used — so it is structurally identical to the listing the greeting
    /// already carried, which is what lets the remote proxy satisfy this
    /// stage from the greeting instead of a wire frame.
    ///
    /// The opening reply also carries the *early supplies*: the Left-arm-only
    /// merge of the two root listings, shipping each initiator-exclusive
    /// root child whole — pruned against `their_version` exactly like any
    /// solicited supply — one hop before the responder could ask for it.
    /// The per-radix survivors (including those pruned to nothing) are
    /// handed to the next level through the returned channel, so the root
    /// resolution answers the responder's now-vestigial empty queries from
    /// local state instead of re-walking the subtrees.
    pub fn initiator_level(
        &mut self,
        their_version: Version,
        ceiling: Version,
        fan: Vec<(u8, B::Node<UnderRoot>)>,
        their_listing: Vec<(u8, Hash)>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Query<B, T, UnderRoot>>,
        Sender<Option<B::Node<height::Root>>>,
        oneshot::Receiver<Vec<(u8, Option<B::Node<UnderRoot>>)>>,
        BoxFuture<'static, Result<Root<B, T>, Error<B::Error>>>,
    )
    where
        B: Sync,
    {
        let (queries, queries_rx) = initiator_root_query();
        let (returns, mut returns_rx) = initiator_root_return::<B, T>();
        let (early_tx, early_rx) = oneshot::channel();
        let backend = self.backend();
        let stats = self.stats.clone();
        #[cfg(test)]
        let trace_id = self.trace_id;

        let responses = try_stream! {
            // The Left-arm-only merge over (fan, their listing): exclusive
            // root children, pruned, in radix order. Asking no question, it
            // adds no question-owner anywhere: every scope keeps exactly one.
            let mut exclusive = Vec::new();
            {
                let mut theirs = their_listing.iter().map(|(radix, _)| *radix).peekable();
                for (radix, node) in &fan {
                    while theirs.next_if(|theirs| theirs < radix).is_some() {}
                    if theirs.peek() != Some(radix) {
                        exclusive.push((*radix, node.clone()));
                    }
                }
            }
            let mut supplies = Vec::new();
            let mut early = Vec::new();
            for (radix, node) in exclusive {
                let survivor =
                    unknown(&backend, &their_version, Prefix::new().push(radix), node, &stats)
                        .await?;
                if let Some(survivor) = &survivor {
                    supplies.push(message::Reaction::Supply(radix, survivor.clone()));
                }
                early.push((radix, survivor));
            }
            // Filled before the opening yields, so the level consuming it
            // never waits: its first query cannot arrive earlier.
            let _ = early_tx.send(early);
            #[cfg(test)]
            progress::wire(trace_id, Prefix::new());
            yield Reply {
                replies: std::iter::once(message::Reaction::Query(fan_listing(&fan)))
                    .chain(supplies)
                    .collect(),
            };
            let query = Query {
                prefix: Prefix::new(),
                ours: fan,
            };
            #[cfg(test)]
            progress::initial_query(trace_id, &query);
            if queries.send(query).await.is_err() {
                return;
            }
        };

        let finish = Box::pin(async move {
            let root = next_or_cancelled(returns_rx.recv()).await;
            Ok(Root { ceiling, root })
        });

        (
            self.respond(responses),
            queries_rx,
            returns,
            early_rx,
            finish,
        )
    }

    /// Process the responder level.
    ///
    /// `fan` is the greeting-time root fan: the greeting already carried
    /// the root's children, so this stage starts from that listing rather
    /// than exploding the root itself.
    ///
    /// The opening request may trail *early supplies* behind its query: the
    /// initiator's exclusive root children, shipped whole without waiting to
    /// be asked. The merge-join below still emits its Right-arm empty
    /// queries for them — the reply/question pairing on every stream is
    /// untouched — and the supplied nodes are exploded into their children
    /// here (this stage's one reply is the only thing a failure can strand)
    /// and handed to the next level through the returned channel, where
    /// they resolve those queries' now-empty answers without touching the
    /// backend mid-loop.
    #[allow(clippy::type_complexity)]
    pub fn responder_level(
        &mut self,
        their_version: Version,
        ceiling: Version,
        fan: Vec<(u8, B::Node<UnderRoot>)>,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (
        BoxResponses<B, T, UnderRoot, Error<B::Error>>,
        Receiver<Query<B, T, UnderUnderRoot>>,
        Sender<Option<B::Node<UnderRoot>>>,
        oneshot::Receiver<Vec<(u8, Vec<(u8, B::Node<UnderUnderRoot>)>)>>,
        BoxFuture<'static, Result<Root<B, T>, Error<B::Error>>>,
    )
    where
        B: Sync,
    {
        let backend = self.backend();
        let stats = self.stats.clone();
        let (asked, asked_rx) =
            responder_child_queries(self.window.capacity(UnderUnderRoot::HEIGHT));
        let (resolution, resolution_rx) = responder_root_resolution();
        let (early_tx, early_rx) = oneshot::channel();
        let assembling = backend.clone();
        #[cfg(test)]
        let trace_id = self.trace_id;

        let responses = try_stream! {
            let mut requests = pin!(requests);
            let Some(Reply { replies }) = requests.next().await else {
                return violation(Violation::UnansweredQuery)?;
            };
            let mut reactions = replies.into_iter();
            let Some(message::Reaction::Query(theirs)) = reactions.next() else {
                return violation(Violation::UnexpectedQuery)?;
            };
            let mut early = Vec::new();
            for reaction in reactions {
                let message::Reaction::Supply(radix, node) = reaction else {
                    return violation(Violation::UnexpectedQuery)?;
                };
                // Early supplies are absorbed here, ahead of the descent's
                // resolver, so they pass the same containment check every
                // other supply does.
                if !contained(node.ceiling(), &their_version) {
                    return violation(Violation::UncontainedSupply)?;
                }
                let children =
                    children_of(&backend, Prefix::new().push(radix), node).await?;
                early.push((radix, children));
            }
            // Filled before this reply yields, so the level consuming it
            // never waits: its first query cannot arrive earlier.
            let _ = early_tx.send(early);
            let ours = fan;
            let (reactions, next_queries, resolved) =
                answer::internal(&backend, &their_version, Prefix::new(), ours, theirs, &stats)
                    .await?;
            yield_resolve_query!(
                trace_id, Prefix::new();
                yield Reply { replies: reactions };
                resolution => Resolution {
                    prefix: Prefix::new(),
                    resolved,
                };
                asked => next_queries;
            );
        };

        let (returns, returns_rx) = responder_root_returns::<B, T>();
        let assembled = assemble(assembling, resolution_rx, returns_rx);
        let finish = Box::pin(async move {
            let mut assembled = pin!(assembled);
            let root = next_or_cancelled(assembled.next()).await;
            Ok(Root {
                ceiling,
                root: root?,
            })
        });

        (self.respond(responses), asked_rx, returns, early_rx, finish)
    }

    /// Walk an internal level, where disputes recur into another internal level.
    ///
    /// The two `early_*` channels are the opening exchange's hand-off into
    /// the one instance that resolves root scopes; every deeper instance
    /// receives `None`:
    ///
    /// - an initiator's `early_survivors` answers the responder's
    ///   root-level empty queries with empty replies (their content shipped
    ///   at the opening) while resolving the radices from the retained
    ///   survivors;
    /// - a responder's `early_supplies` resolves its own root-level
    ///   requests from the pre-exploded children the opening carried when
    ///   the initiator's matching replies arrive empty.
    ///
    /// Both hand-offs resolve without backend calls: a mid-loop failure
    /// here would strand the counterparty's reply pump on a full slot,
    /// ahead of the error's own publication.
    #[allow(clippy::type_complexity)]
    pub fn internal_level<H>(
        &mut self,
        their_version: Version,
        early_survivors: Option<oneshot::Receiver<Vec<(u8, Option<B::Node<S<S<H>>>>)>>>,
        early_supplies: Option<oneshot::Receiver<Vec<(u8, Vec<(u8, B::Node<S<S<H>>>)>)>>>,
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
        let stats = self.stats.clone();
        let (asked, asked_rx) = internal_child_queries(self.window.capacity(H::HEIGHT));
        let (upper, upper_rx) =
            internal_parent_resolutions(self.window.capacity(<S<S<H>>>::HEIGHT));
        let (lower, lower_rx) = internal_child_resolutions(self.window.capacity(<S<H>>::HEIGHT));
        #[cfg(test)]
        let trace_id = self.trace_id;

        let responses = try_stream! {
            let mut requests = pin!(requests);
            let mut early_survivors = early_survivors;
            let mut survivors: Option<BTreeMap<u8, Option<B::Node<S<S<H>>>>>> = None;
            let mut early_supplies = early_supplies;
            let mut supplied: Option<BTreeMap<u8, Vec<(u8, B::Node<S<S<H>>>)>>> = None;
            while let Some(query) = queries.recv().await {
                let Some(Reply { replies }) = requests.next().await else {
                    return violation(Violation::UnansweredQuery)?;
                };

                // A root-level request whose reply arrived empty resolves
                // from the opening's early supplies: the content crossed at
                // the opening, so only the pairing reply travels here. A
                // miss falls through: an empty reply to a request with no
                // early supply means the whole subtree pruned away.
                if replies.is_empty()
                    && query.ours.is_empty()
                    && (early_supplies.is_some() || supplied.is_some())
                    && let Some(&radix) = query.prefix.as_bytes().last()
                {
                    if supplied.is_none()
                        && let Some(early) = early_supplies.take()
                    {
                        supplied = Some(early.await.unwrap_or_default().into_iter().collect());
                    }
                    if let Some(children) = supplied.as_mut().and_then(|nodes| nodes.remove(&radix))
                    {
                        // An early supply claimed here is content this
                        // replica just learned, exactly like a solicited
                        // supply absorbed by the resolver: credit its
                        // exact live-leaf count.
                        stats.gained(
                            children
                                .iter()
                                .map(|(_, child)| child.len() as u64)
                                .sum(),
                        );
                        let resolution = Resolution {
                            prefix: query.prefix,
                            resolved: children
                                .into_iter()
                                .map(|(radix, child)| (radix, Resolve::Ready(Some(child))))
                                .collect(),
                        };
                        #[cfg(test)]
                        progress::parent_resolution(trace_id, &resolution);
                        if upper.send(resolution).await.is_err() {
                            return;
                        }
                        continue;
                    }
                }

                let mut resolver = Resolver::new(query, &their_version, stats.clone());
                for reaction in replies {
                    let Some((prefix, radix, node, listing)) = resolver.react(reaction)? else {
                        continue;
                    };
                    let child_prefix = prefix.push(radix);

                    if listing.is_empty() {
                        // A root-level empty query whose radix the opening
                        // already supplied is answered by an empty reply:
                        // pairing intact, content relocated. The retained
                        // survivor resolves the radix locally, pruned by
                        // the same filter the opening supply used.
                        if early_survivors.is_some() || survivors.is_some() {
                            if survivors.is_none()
                                && let Some(early) = early_survivors.take()
                            {
                                survivors =
                                    Some(early.await.unwrap_or_default().into_iter().collect());
                            }
                            if let Some(survivor) =
                                survivors.as_mut().and_then(|nodes| nodes.remove(&radix))
                            {
                                yield_resolve_query!(
                                    trace_id, child_prefix;
                                    yield Reply { replies: Vec::new() };
                                    resolver.ready(radix, survivor);
                                );
                                continue;
                            }
                        }
                        let (node, children) =
                            unknown_providing(&backend, &their_version, child_prefix, node, &stats)
                                .await?;
                        let replies = children
                            .into_iter()
                            .map(|(radix, child)| Reaction::Supply(radix, child))
                            .collect();
                        yield_resolve_query!(
                            trace_id, child_prefix;
                            yield Reply { replies };
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    let children = children_of(&backend, child_prefix, node).await?;
                    let (reactions, next_queries, resolved) = answer::internal(
                        &backend,
                        &their_version,
                        child_prefix,
                        children,
                        listing,
                        &stats,
                    )
                    .await?;
                    yield_resolve_query!(
                        trace_id, child_prefix;
                        yield Reply { replies: reactions };
                        lower => Resolution {
                            prefix: child_prefix,
                            resolved,
                        };
                        asked => next_queries;
                    );
                    resolver.pending(radix);
                }

                // Launch every `Pending` slot's work before publishing its
                // enclosing parent resolution.
                let resolution = resolver.finish()?;
                #[cfg(test)]
                progress::parent_resolution(trace_id, &resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
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
        let stats = self.stats.clone();
        let (asked, asked_rx) = leaf_requests(self.window.capacity(Z::HEIGHT));
        let (upper, upper_rx) = leaf_parent_resolutions(self.window.capacity(<S<Z>>::HEIGHT));
        let (lower, lower_rx) = leaf_child_resolutions(self.window.capacity(Z::HEIGHT));
        #[cfg(test)]
        let trace_id = self.trace_id;

        let responses = try_stream! {
            let mut requests = pin!(requests);
            while let Some(query) = queries.recv().await {
                let Some(Reply { replies }) = requests.next().await else {
                    return violation(Violation::UnansweredQuery)?;
                };

                let mut resolver = Resolver::new(query, &their_version, stats.clone());
                for reaction in replies {
                    let Some((prefix, radix, node, listing)) = resolver.react(reaction)? else {
                        continue;
                    };
                    let child_prefix = prefix.push(radix);

                    if listing.is_empty() {
                        let (node, leaves) =
                            unknown_providing(&backend, &their_version, child_prefix, node, &stats)
                                .await?;
                        let replies = leaves
                            .into_iter()
                            .map(|(radix, leaf)| Reaction::Supply(radix, leaf))
                            .collect();
                        yield_resolve_query!(
                            trace_id, child_prefix;
                            yield Reply { replies };
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    let leaves = children_of(&backend, child_prefix, node).await?;
                    let (replies, next_queries, resolved) =
                        answer::leaf_parent(&their_version, child_prefix, leaves, listing, &stats);
                    yield_resolve_query!(
                        trace_id, child_prefix;
                        yield Reply { replies };
                        lower => Resolution {
                            prefix: child_prefix,
                            resolved,
                        };
                        asked => next_queries;
                    );
                    resolver.pending(radix);
                }

                // Launch every `Pending` slot's work before publishing its
                // enclosing parent resolution.
                let resolution = resolver.finish()?;
                #[cfg(test)]
                progress::parent_resolution(trace_id, &resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
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
        let (upper, upper_rx) = terminal_leaf_resolutions();
        let stats = self.stats.clone();
        #[cfg(test)]
        let trace_id = self.trace_id;

        let responses = try_stream! {
            let mut requests = pin!(requests);
            while let Some(query) = queries.recv().await {
                let Some(Reply { replies }) = requests.next().await else {
                    return violation(Violation::UnansweredQuery)?;
                };

                let mut resolver = Resolver::new(query, &their_version, stats.clone());
                for reaction in replies {
                    let Some((prefix, radix, node, listing)) = resolver.react(reaction)? else {
                        continue;
                    };

                    let (replies, node) =
                        answer::leaf(&their_version, radix, node, listing, &stats)
                            .map_err(Error::Violation)?;
                    yield_resolve_query!(
                        trace_id, prefix.push(radix);
                        yield Reply { replies };
                        resolver.ready(radix, node);
                    );
                }

                let resolution = resolver.finish()?;
                #[cfg(test)]
                progress::parent_resolution(trace_id, &resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        };

        (self.respond(responses), upper_rx)
    }
}
