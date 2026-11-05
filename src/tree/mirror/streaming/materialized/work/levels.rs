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
    LeafRequest, Work,
    answer::{self, Part},
    assembly::assemble,
    charges,
    queues::*,
    resolver::{Dispute, Resolver},
};
use crate::tree::{
    mirror::streaming::{
        Backend, ErasedNode, Leaf, Root,
        channel::{Receiver, Sender},
        erased::{self, Reaction, Reply},
        holding::{Holding, held_by, holding, holding_stream},
        ledger::ReplyBound,
        materialized::{
            Error, Query, Resolution, SupplyLedger, Violation, fan_listing,
            unknown::unknown_providing, violation,
        },
        message::Grandchildren,
        protocol::{BoxRequests, BoxResponses, Requests},
        tasks::next_or_cancelled,
    },
    typed::{
        Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// Publish one chunk of an answer in its progress-critical order.
///
/// The caller walks the answer's [`Part`]s, building each part's question
/// from its planned form only when the part is reached, so a chunk's built
/// items are its own question's, and passes the built `(reaction, item)`
/// pair here with the answer's resolution while it is still unpublished.
/// The chunk's wire action comes first; then, after the first chunk alone,
/// the resolution, whose pending slots the questions' returns fill; then
/// the chunk's question. A blocked question sender has therefore handed on
/// every earlier chunk whole and made the resolution available, and the
/// reply's last chunk holds its last reaction.
///
/// The caller writes the yield, `chunk => yield chunk`, inside its own
/// `async_stream` body, where the stream can lower it, and builds the
/// question there too, so a build failure's `?` is lowered as well.
macro_rules! yield_chunk {
    (
        $progress:expr, $account:expr, $scope:expr;
        $leading:expr, $question:expr, $last:expr;
        $chunk:ident => $yielded:expr;
        $resolutions:expr => $resolution:expr;
        $queries:expr;
    ) => {{
        let scope = $scope;
        let mut leading = $leading;
        let question = $question.map(|(reaction, question)| {
            held_by(Holding::OutgoingReplies, || leading.push(reaction));
            question
        });
        let resolution = $resolution;
        if resolution.is_some() {
            $progress.wire(scope);
        } else {
            $progress.chunk(scope);
        }
        let $chunk = charges::chunk::<B>(&$account, leading, $last);
        $yielded;
        if let Some(resolution) = resolution {
            $progress.resolution(&resolution);
            if $resolutions.send(resolution).await.is_err() {
                return;
            }
        }
        if let Some(question) = question {
            $progress.dependent(&question);
            if $queries.send(question).await.is_err() {
                return;
            }
        }
    }};
}

/// Publish one whole-reply scope, which asks nothing, in its
/// progress-critical order: its wire action, then its local resolution.
///
/// The yield must remain inside this macro so `async_stream` can lower it.
macro_rules! yield_ready {
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

/// The next chunk of the reply a stage is reading, or the violation of a
/// request stream that ends first: the question the reply answers is still
/// outstanding.
async fn next_chunk<E, Err>(requests: &mut BoxRequests<E>) -> Result<Reply<E>, Error<Err>> {
    match requests.next().await {
        Some(chunk) => Ok(chunk),
        None => violation(Violation::UnansweredQuery),
    }
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
        copies: usize,
        responder: Grandchildren,
    ) -> (
        BoxResponses<B, UnderRoot, Error<B::Error>>,
        Receiver<Query<B::Erased>>,
        Sender<Option<B::Erased>>,
        BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
    ) {
        let (mut queries, queries_rx) = initiator_root_query::<B>();
        let (returns, mut returns_rx) = initiator_root_return::<B>();
        let progress = self.progress;
        let account = self.ledger();

        let responses = try_stream! {
            let root_scope = Prefix::new().erase();
            progress.wire(root_scope);
            yield held_by(Holding::OutgoingReplies, || {
                charges::reply::<B>(&account, vec![Reaction::Query(fan_listing(&fan))])
            });
            let bound = ReplyBound::of(root_scope.height(), fan.len(), responder);
            let query = charges::query(&account, root_scope, fan, bound, copies);
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
        copies: usize,
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
        let account = self.ledger();

        let responses = try_stream! {
            let mut requests = requests;
            let root_scope = Prefix::new().erase();
            // The opening reply is not paired positionally against the
            // held fan, so the resolver cannot classify it; its arms are
            // its own, and the first offending reaction names the fault.
            // It carries exactly one query. A supply at a radix this side
            // holds is already known; any other supply is out of place.
            let mut theirs = None;
            loop {
            let Reply { reactions, charge: received, last } = next_chunk(&mut requests).await?;
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
            drop(received);
            if last {
                break;
            }
            }
            let Some(theirs) = theirs else {
                return violation(Violation::UnfinishedReply)?;
            };
            let ours = fan;
            let (mut chunks, resolved) =
                answer::internal(&backend, &their_version, root_scope, ours, theirs, &stats)
                    .await?
                    .into_parts();
            let mut unpublished = Some(charges::resolution(
                &account,
                root_scope,
                resolved,
                account.charge(Holding::ExplodedNodes, copies),
            ));
            while let Some(Part { leading, ask, last }) = chunks.next_part() {
                let question = match ask {
                    Some(question) => {
                        Some(answer::question(&backend, &account, root_scope, question).await?)
                    }
                    None => None,
                };
                yield_chunk!(
                    progress, account, root_scope;
                    leading, question, last;
                    chunk => yield chunk;
                    resolution => unpublished.take();
                    asked;
                );
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        };

        let (returns, returns_rx) = responder_root_returns::<B>();
        let assembled = holding_stream(
            Holding::Assembly,
            assemble(assembling, resolution_rx, returns_rx),
        );
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
        let account = self.ledger();

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let mut chunk = next_chunk(&mut requests).await?;

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                loop {
                let Reply { reactions, charge: received, last } = chunk;
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
                        let copies = node.explosion_bytes();
                        let (node, children) = holding(
                            Holding::OutgoingReplies,
                            unknown_providing(&backend, &their_version, child_prefix, node, &stats),
                        )
                        .await?;
                        let reactions = held_by(Holding::OutgoingReplies, || {
                            children
                                .into_iter()
                                .map(|(radix, child)| Reaction::Supply(radix, child))
                                .collect()
                        });
                        let mut reply = charges::reply::<B>(&account, reactions);
                        reply.charge.absorb(account.charge(Holding::OutgoingReplies, copies));
                        yield_ready!(
                            progress, child_prefix;
                            yield reply;
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    let copies = node.explosion_bytes();
                    let children = erased::ops::children_of(&backend, child_prefix, node).await?;
                    let (mut chunks, resolved) = answer::internal(
                        &backend,
                        &their_version,
                        child_prefix,
                        children,
                        listing,
                        &stats,
                    )
                    .await?
                    .into_parts();
                    let mut unpublished = Some(charges::resolution(
                        &account,
                        child_prefix,
                        resolved,
                        account.charge(Holding::ExplodedNodes, copies),
                    ));
                    while let Some(Part { leading, ask, last }) = chunks.next_part() {
                        let question = match ask {
                            Some(question) => Some(
                                answer::question(&backend, &account, child_prefix, question).await?,
                            ),
                            None => None,
                        };
                        yield_chunk!(
                            progress, account, child_prefix;
                            leading, question, last;
                            chunk => yield chunk;
                            lower => unpublished.take();
                            asked;
                        );
                    }
                    resolver.pending(radix);
                }

                // The chunk's reactions are used up, so its charge returns.
                drop(received);
                if last {
                    break;
                }
                chunk = next_chunk(&mut requests).await?;
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
        Receiver<LeafRequest>,
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
        Receiver<LeafRequest>,
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
        let account = self.ledger();

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let mut chunk = next_chunk(&mut requests).await?;

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                loop {
                let Reply { reactions, charge: received, last } = chunk;
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
                        let copies = node.explosion_bytes();
                        let (node, leaves) = holding(
                            Holding::OutgoingReplies,
                            unknown_providing(&backend, &their_version, child_prefix, node, &stats),
                        )
                        .await?;
                        let reactions = held_by(Holding::OutgoingReplies, || {
                            leaves
                                .into_iter()
                                .map(|(radix, leaf)| Reaction::Supply(radix, leaf))
                                .collect()
                        });
                        let mut reply = charges::reply::<B>(&account, reactions);
                        reply.charge.absorb(account.charge(Holding::OutgoingReplies, copies));
                        yield_ready!(
                            progress, child_prefix;
                            yield reply;
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    let copies = node.explosion_bytes();
                    let leaves = erased::ops::children_of(&backend, child_prefix, node).await?;
                    let (mut chunks, resolved) =
                        answer::leaf_parent(&their_version, leaves, listing, &stats).into_parts();
                    let mut unpublished = Some(charges::resolution(
                        &account,
                        child_prefix,
                        resolved,
                        account.charge(Holding::ExplodedNodes, copies),
                    ));
                    while let Some(Part { leading, ask, last }) = chunks.next_part() {
                        let question = ask.map(|radix| answer::leaf_question(&account, child_prefix, radix));
                        yield_chunk!(
                            progress, account, child_prefix;
                            leading, question, last;
                            chunk => yield chunk;
                            lower => unpublished.take();
                            asked;
                        );
                    }
                    resolver.pending(radix);
                }

                // The chunk's reactions are used up, so its charge returns.
                drop(received);
                if last {
                    break;
                }
                chunk = next_chunk(&mut requests).await?;
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
        let account = self.ledger();

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let mut chunk = next_chunk(&mut requests).await?;

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                loop {
                let Reply { reactions, charge: received, last } = chunk;
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
                    yield_ready!(
                        progress, child_prefix;
                        yield charges::reply::<B>(&account, reactions);
                        resolver.ready(radix, node);
                    );
                }

                // The chunk's reactions are used up, so its charge returns.
                drop(received);
                if last {
                    break;
                }
                chunk = next_chunk(&mut requests).await?;
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
