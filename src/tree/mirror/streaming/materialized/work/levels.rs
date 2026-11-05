//! Phase-specific materialized reconciliation walks.
//!
//! Every walk body runs on the erased vocabulary — one instantiation per
//! backend — behind a thin typed method that erases the stage's request
//! stream on the way in and re-tags its responses on the way out
//! ([`Work::respond`]). The generic methods carry the type-level height
//! the schedule proves; the walk bodies carry it as the runtime witness
//! every scope's prefix length *is*.

use std::pin::pin;

use async_stream::try_stream;
use before::Version;
use futures::{Stream, StreamExt, future::BoxFuture};

use super::{
    Work, answer,
    assembly::assemble,
    queues::*,
    resolver::{Dispute, Resolver},
};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Root,
        channel::{Receiver, Sender},
        erased::{self, Reaction, Reply},
        materialized::{
            Error, Query, Resolution, SupplyLedger, Violation, fan_listing,
            unknown::unknown_providing, violation,
        },
        protocol::{BoxRequests, BoxResponses, Requests},
        tasks::next_or_cancelled,
    },
    typed::{
        Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// Publish one disputed scope in its progress-critical order.
///
/// The yield must remain inside this macro so `async_stream` can lower it.
/// Keeping the wire action, resolution, and dependent queries together also
/// prevents their required publication order from drifting between walks.
/// With the wire action, test builds record how many questions the reply
/// carries and the typed height of the children they list, for the trace's
/// in-flight count.
macro_rules! yield_resolve_query {
    (
        $progress:expr, $scope:expr;
        $yielded:expr;
        $resolutions:expr => $resolution:expr;
        $queries:expr => $next_queries:expr, at height $height:expr;
    ) => {{
        let scope = $scope;
        let next_queries = $next_queries;
        $progress.wire(scope);
        $progress.asked(scope, $height, next_queries.len());
        $yielded;
        let resolution = $resolution;
        $progress.resolution(&resolution);
        if $resolutions.send(resolution).await.is_err() {
            return;
        }
        for query in next_queries {
            $progress.dependent(&query);
            if $queries.send(query).await.is_err() {
                return;
            }
        }
    }};
    (
        $progress:expr, $scope:expr;
        $yielded:expr;
        $ready:expr;
    ) => {{
        let scope = $scope;
        $progress.wire(scope);
        $yielded;
        $progress.ready(scope);
        $ready;
    }};
}

impl<B> Work<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Process the initiator level.
    ///
    /// `fan` is the greeting-time root fan: the opening question is derived
    /// from it by [`fan_listing`] — the same derivation the greeting itself
    /// used — so it is structurally identical to the listing the greeting
    /// already carried, which is what lets the remote proxy satisfy this
    /// stage from the greeting instead of a wire frame.
    ///
    /// The opening reply is that question alone. Root children only this
    /// side holds travel as ordinary supplies, answering the responder's
    /// empty queries for them one level down.
    pub fn initiator_level(
        &mut self,
        ceiling: Version,
        fan: Vec<(u8, B::Erased)>,
    ) -> (
        BoxResponses<B, UnderRoot, Error<B::Error>>,
        Receiver<Query<B::Erased>>,
        Sender<Option<B::Erased>>,
        BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
    ) {
        let (mut queries, queries_rx) = initiator_root_query::<B>();
        let (returns, mut returns_rx) = initiator_root_return::<B>();
        let progress = self.progress;

        let responses = try_stream! {
            let root_scope = Prefix::new().erase();
            progress.wire(root_scope);
            progress.asked(root_scope, UnderRoot::HEIGHT, 1);
            yield Reply {
                reactions: vec![Reaction::Query(fan_listing(&fan))],
            };
            let query = Query {
                prefix: root_scope,
                ours: fan,
            };
            progress.initial_query(&query);
            if queries.send(query).await.is_err() {
                return;
            }
        };

        let finish = Box::pin(async move {
            let root = next_or_cancelled(returns_rx.recv()).await;
            Ok(Root {
                ceiling,
                // The root return is this walk's one fixed-height re-tag:
                // the initiator's single return channel carries exactly the
                // reconciled root.
                root: root.map(B::assume::<height::Root>),
            })
        });

        (
            self.respond::<UnderRoot>(responses),
            queries_rx,
            returns,
            finish,
        )
    }

    /// Process the responder level.
    ///
    /// `fan` is the greeting-time root fan: the greeting already carried
    /// the root's children, so this stage starts from that listing rather
    /// than exploding the root itself.
    ///
    /// The opening reply is the initiator's root question and nothing else.
    pub fn responder_level(
        &mut self,
        their_version: Version,
        ceiling: Version,
        fan: Vec<(u8, B::Erased)>,
        requests: impl Requests<B, UnderRoot>,
    ) -> (
        BoxResponses<B, UnderRoot, Error<B::Error>>,
        Receiver<Query<B::Erased>>,
        Sender<Option<B::Erased>>,
        BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
    ) {
        let requests = requests.erase();
        let backend = self.backend();
        let stats = self.stats.clone();
        let (mut asked, asked_rx) =
            responder_child_queries::<B>(self.window.capacity(UnderUnderRoot::HEIGHT), &stats);
        let (mut resolution, resolution_rx) = responder_root_resolution::<B>();
        let assembling = backend.clone();
        let progress = self.progress;

        let responses = try_stream! {
            let mut requests = requests;
            let root_scope = Prefix::new().erase();
            let Some(Reply { reactions }) = requests.next().await else {
                return violation(Violation::UnansweredQuery)?;
            };
            // The opening reply is not paired positionally against the
            // held fan, so the resolver cannot classify it; its arms are
            // its own, and the first offending reaction names the fault.
            // It carries exactly one query. A supply at a radix this side
            // holds is already known; any other supply is out of place.
            let mut theirs = None;
            for reaction in reactions {
                match reaction {
                    Reaction::Query(listing) => {
                        if theirs.is_some() {
                            return violation(Violation::UnexpectedQuery)?;
                        }
                        theirs = Some(listing);
                    }
                    Reaction::Match => return violation(Violation::UnexpectedMatch)?,
                    Reaction::Supply(radix, _) => {
                        if fan.iter().any(|(held, _)| *held == radix) {
                            return violation(Violation::UnexpectedSupply)?;
                        }
                        return violation(Violation::InvalidSupply)?;
                    }
                }
            }
            let Some(theirs) = theirs else {
                return violation(Violation::UnfinishedReply)?;
            };
            let ours = fan;
            let (reactions, next_queries, resolved) =
                answer::internal(&backend, &their_version, root_scope, ours, theirs, &stats)
                    .await?;
            yield_resolve_query!(
                progress, root_scope;
                yield Reply { reactions };
                resolution => Resolution {
                    prefix: root_scope,
                    resolved,
                };
                asked => next_queries, at height UnderUnderRoot::HEIGHT;
            );

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        };

        let (returns, returns_rx) = responder_root_returns::<B>();
        let assembled = assemble(assembling, resolution_rx, returns_rx);
        let finish = Box::pin(async move {
            let mut assembled = pin!(assembled);
            let root = next_or_cancelled(assembled.next()).await;
            Ok(Root {
                ceiling,
                // The responder's root assembles at the top of the return
                // chain: the same one fixed-height re-tag as the
                // initiator's.
                root: root?.map(B::assume::<height::Root>),
            })
        });

        (
            self.respond::<UnderRoot>(responses),
            asked_rx,
            returns,
            finish,
        )
    }

    /// Walk an internal level, where disputes recur into another internal
    /// level.
    ///
    /// `H` is the height the walk's dependent queries descend to; the
    /// walk's own scopes sit two levels above it, exactly as the schedule's
    /// [`Reply`](crate::tree::mirror::streaming::protocol::Reply)
    /// transition demands.
    pub fn internal_level<H>(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: impl Requests<B, S<S<H>>>,
        queries: Receiver<Query<B::Erased>>,
    ) -> (
        BoxResponses<B, S<H>, Error<B::Error>>,
        Receiver<Query<B::Erased>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
    )
    where
        H: Height,
        S<H>: Height,
        S<S<H>>: Height,
    {
        let requests = requests.erase();
        let (responses, asked_rx, upper_rx, lower_rx) =
            self.internal_walk(their_version, ledger, requests, queries, H::HEIGHT);
        (
            self.respond::<S<H>>(responses),
            asked_rx,
            upper_rx,
            lower_rx,
        )
    }

    /// The internal walk's body, shared by every height:
    /// [`internal_level`](Self::internal_level) with the descent height as
    /// the runtime datum it already is everywhere below the types.
    fn internal_walk(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: BoxRequests<B::Erased>,
        mut queries: Receiver<Query<B::Erased>>,
        asked_height: usize,
    ) -> (
        impl Stream<Item = Result<Reply<B::Erased>, Error<B::Error>>> + Send + 'static + use<B>,
        Receiver<Query<B::Erased>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
    ) {
        let backend = self.backend();
        let stats = self.stats.clone();
        let (mut asked, asked_rx) =
            internal_child_queries::<B>(asked_height, self.window.capacity(asked_height), &stats);
        let (mut upper, upper_rx) = internal_parent_resolutions::<B>(
            asked_height + 2,
            self.window.capacity(asked_height + 2),
            &stats,
        );
        let (mut lower, lower_rx) = internal_child_resolutions::<B>(
            asked_height + 1,
            self.window.capacity(asked_height + 1),
            &stats,
        );
        let progress = self.progress;

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let Some(Reply { reactions }) = requests.next().await else {
                    return violation(Violation::UnansweredQuery)?;
                };
                progress.take(asked_height + 2, &query);

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                for reaction in reactions {
                    let Some(Dispute {
                        child_prefix,
                        radix,
                        node,
                        listing,
                    }) = resolver.react(reaction)?
                    else {
                        continue;
                    };

                    if listing.is_empty() {
                        let (node, children) =
                            unknown_providing(&backend, &their_version, child_prefix, node, &stats)
                                .await?;
                        let reactions = children
                            .into_iter()
                            .map(|(radix, child)| Reaction::Supply(radix, child))
                            .collect();
                        yield_resolve_query!(
                            progress, child_prefix;
                            yield Reply { reactions };
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    let children = erased::ops::children_of(&backend, child_prefix, node).await?;
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
                        progress, child_prefix;
                        yield Reply { reactions };
                        lower => Resolution {
                            prefix: child_prefix,
                            resolved,
                        };
                        asked => next_queries, at height asked_height;
                    );
                    resolver.pending(radix);
                }

                // Launch every `Pending` slot's work before publishing its
                // enclosing parent resolution.
                let resolution = resolver.finish()?;
                progress.parent_resolution(&resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        };

        (responses, asked_rx, upper_rx, lower_rx)
    }

    /// Walk leaf parents, where disputes compare version-addressed leaves.
    pub fn leaf_parent_level(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: impl Requests<B, S<Z>>,
        queries: Receiver<Query<B::Erased>>,
    ) -> (
        BoxResponses<B, Z, Error<B::Error>>,
        Receiver<Prefix<Z>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
    ) {
        let requests = requests.erase();
        let (responses, asked_rx, upper_rx, lower_rx) =
            self.leaf_parent_walk(their_version, ledger, requests, queries);
        (self.respond::<Z>(responses), asked_rx, upper_rx, lower_rx)
    }

    /// The leaf-parent walk's body ([`leaf_parent_level`](Self::leaf_parent_level)).
    fn leaf_parent_walk(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: BoxRequests<B::Erased>,
        mut queries: Receiver<Query<B::Erased>>,
    ) -> (
        impl Stream<Item = Result<Reply<B::Erased>, Error<B::Error>>> + Send + 'static + use<B>,
        Receiver<Prefix<Z>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
    ) {
        let backend = self.backend();
        let stats = self.stats.clone();
        let (mut asked, asked_rx) = leaf_requests(self.window.capacity(Z::HEIGHT), &stats);
        let (mut upper, upper_rx) =
            leaf_parent_resolutions::<B>(self.window.capacity(<S<Z>>::HEIGHT), &stats);
        let (mut lower, lower_rx) =
            leaf_child_resolutions::<B>(self.window.capacity(Z::HEIGHT), &stats);
        let progress = self.progress;

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let Some(Reply { reactions }) = requests.next().await else {
                    return violation(Violation::UnansweredQuery)?;
                };
                progress.take(<S<Z>>::HEIGHT, &query);

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                for reaction in reactions {
                    let Some(Dispute {
                        child_prefix,
                        radix,
                        node,
                        listing,
                    }) = resolver.react(reaction)?
                    else {
                        continue;
                    };

                    if listing.is_empty() {
                        let (node, leaves) =
                            unknown_providing(&backend, &their_version, child_prefix, node, &stats)
                                .await?;
                        let reactions = leaves
                            .into_iter()
                            .map(|(radix, leaf)| Reaction::Supply(radix, leaf))
                            .collect();
                        yield_resolve_query!(
                            progress, child_prefix;
                            yield Reply { reactions };
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    let leaves = erased::ops::children_of(&backend, child_prefix, node).await?;
                    let (reactions, next_queries, resolved) =
                        answer::leaf_parent(&their_version, child_prefix, leaves, listing, &stats);
                    yield_resolve_query!(
                        progress, child_prefix;
                        yield Reply { reactions };
                        lower => Resolution {
                            prefix: child_prefix,
                            resolved,
                        };
                        asked => next_queries, at height Z::HEIGHT;
                    );
                    resolver.pending(radix);
                }

                // Launch every `Pending` slot's work before publishing its
                // enclosing parent resolution.
                let resolution = resolver.finish()?;
                progress.parent_resolution(&resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        };

        (responses, asked_rx, upper_rx, lower_rx)
    }

    /// Walk leaves, where every query is a terminal request.
    pub fn leaf_level(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: impl Requests<B, Z>,
        queries: Receiver<Query<B::Erased>>,
    ) -> (
        BoxResponses<B, Z, Error<B::Error>>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
    ) {
        let requests = requests.erase();
        let (responses, upper_rx) = self.leaf_walk(their_version, ledger, requests, queries);
        (self.respond::<Z>(responses), upper_rx)
    }

    /// The leaf walk's body ([`leaf_level`](Self::leaf_level)).
    fn leaf_walk(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: BoxRequests<B::Erased>,
        mut queries: Receiver<Query<B::Erased>>,
    ) -> (
        impl Stream<Item = Result<Reply<B::Erased>, Error<B::Error>>> + Send + 'static + use<B>,
        OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
    ) {
        let (mut upper, upper_rx) = terminal_leaf_resolutions::<B>();
        let stats = self.stats.clone();
        let progress = self.progress;

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let Some(Reply { reactions }) = requests.next().await else {
                    return violation(Violation::UnansweredQuery)?;
                };
                progress.take(Z::HEIGHT, &query);

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                for reaction in reactions {
                    let Some(Dispute {
                        child_prefix,
                        radix,
                        node,
                        listing,
                    }) = resolver.react(reaction)?
                    else {
                        continue;
                    };

                    let (reactions, node) =
                        answer::leaf(&their_version, radix, node, listing, &stats)
                            .map_err(Error::Violation)?;
                    yield_resolve_query!(
                        progress, child_prefix;
                        yield Reply { reactions };
                        resolver.ready(radix, node);
                    );
                }

                let resolution = resolver.finish()?;
                progress.parent_resolution(&resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        };

        (responses, upper_rx)
    }
}
