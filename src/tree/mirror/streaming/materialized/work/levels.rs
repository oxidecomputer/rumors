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
    answer::{self, Held, Part},
    assembly::assemble,
    charges,
    queues::*,
    resolver::{Dispute, Resolver},
};
use crate::tree::{
    mirror::streaming::{
        Backend, ErasedNode, FAN, Leaf, Root,
        channel::{Receiver, Sender},
        erased::{self, Reaction, Reply},
        holding::{Holding, held_by, holding, holding_stream},
        ledger::{ReplyBound, Unit},
        materialized::{
            Error, Query, Resolution, SupplyLedger, Violation, fan_listing,
            unknown::unknown_providing, violation,
        },
        message::{Carried, Grandchildren},
        protocol::{BoxRequests, BoxResponses, Requests},
        tasks::next_or_cancelled,
    },
    typed::{
        ErasedPrefix, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

/// Publish one chunk of an answer in its progress-critical order.
///
/// The caller walks the answer's [`Part`]s, admitting each part's question
/// and building it from its planned form only when the part is reached, so
/// a chunk's built items are its own question's, and passes the built
/// question here with the grant the chunk is built out of, the question's
/// own or, for the chunk without one, the answer's, and with the answer's
/// resolution while it is still unpublished.
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
        $progress:expr, $grant:ident, $scope:expr;
        $leading:expr, $question:expr, $last:expr;
        $chunk:ident => $yielded:expr;
        $resolutions:expr => $resolution:expr;
        $queries:expr;
    ) => {{
        let scope = $scope;
        let mut leading = $leading;
        let (question, carried) = match $question {
            Some((reaction, question, carried)) => {
                held_by(Holding::OutgoingReplies, || leading.push(reaction));
                (Some(question), carried)
            }
            None => (None, Carried::Nothing),
        };
        let resolution = $resolution;
        if resolution.is_some() {
            $progress.wire(scope);
        } else {
            $progress.chunk(scope);
        }
        let $chunk = charges::chunk::<B>(&mut $grant, leading, $last, carried);
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

/// The depth of the scope `prefix` names, the root's 0 through a leaf's 32:
/// the depth admission slots a unit concerning that scope by.
fn depth(prefix: ErasedPrefix) -> usize {
    prefix.as_bytes().len()
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
    ///
    /// `root` is the root `fan` was listed from, which the question carries
    /// to the root's assembly.
    pub fn initiator_level(
        &mut self,
        ceiling: Version,
        fan: Vec<(u8, B::Erased)>,
        root: Option<B::Erased>,
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
            // The opening question is the one question no answer asks; it
            // asks about the root. As its depth's only question it takes its
            // floor slot, so the invitation share, which only limits
            // admissions against the budget, has nothing to decide for it.
            let bytes = charges::question_bound::<B::Erased, B::Error>(
                root_scope.height(),
                responder,
                Held::default(),
                fan.len(),
            );
            let mut grant = account.admit(Unit::Question, 0, bytes).await;
            progress.wire(root_scope);
            let bound = ReplyBound::of(root_scope.height(), fan.len(), responder);
            let listing = held_by(Holding::OutgoingReplies, || {
                fan_listing(root_scope, &fan).map(|listing| vec![Reaction::Query(listing)])
            })
            .map_err(Error::Count)?;
            let record = charges::record(&mut grant, fan.len());
            let room = charges::room::<B::Erased>(&mut grant, bound, fan.len());
            let query = charges::query(&mut grant, root_scope, root, fan, bound);
            yield held_by(Holding::OutgoingReplies, || {
                charges::chunk::<B>(&mut grant, listing, true, Carried::Question { record, room })
            });
            drop(grant);
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
                // reconciled root, whose coordinate is the empty prefix.
                root: root.map(|root| B::assume(&Prefix::new(), root)),
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
    ///
    /// `root` is the root `fan` was listed from, which the root's resolution
    /// carries to its assembly.
    pub fn responder_level(
        &mut self,
        their_version: Version,
        ceiling: Version,
        fan: Vec<(u8, B::Erased)>,
        root: Option<B::Erased>,
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
        let (mut asked, asked_rx) = responder_child_queries::<B>();
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
            let Reply { reactions, charge: received, last, carried } = next_chunk(&mut requests).await?;
            for reaction in reactions {
                match reaction {
                    Reaction::Query(listing) => {
                        if theirs.is_some() {
                            return violation(Violation::UnexpectedQuery)?;
                        }
                        theirs = Some(listing);
                    }
                    Reaction::Match(_) => return violation(Violation::UnexpectedMatch)?,
                    Reaction::Supply(radix, _) => {
                        if fan.iter().any(|(held, _)| *held == radix) {
                            return violation(Violation::UnexpectedSupply)?;
                        }
                        return violation(Violation::InvalidSupply)?;
                    }
                }
            }
            // The chunk is used up, so its charges return now, before the
            // stage waits for the next one: the peer's admission may wait on
            // them.
            drop((received, carried));
            if last {
                break;
            }
            }
            let Some(theirs) = theirs else {
                return violation(Violation::UnfinishedReply)?;
            };
            let ours = fan;
            let held = ours.iter().map(|(radix, _)| *radix).collect();
            let children = charges::scope_children(held, &theirs);
            let bytes =
                charges::answer_bound::<B::Erased, B::Error>(children.either, children.both);
            let mut answer_grant = account.admit(Unit::Answer, 0, bytes).await;
            let (mut chunks, resolved) = answer::internal(
                &backend,
                &their_version,
                root_scope,
                ours,
                theirs,
                &stats,
                &mut answer_grant,
            )
            .await?
            .into_parts();
            let mut unpublished =
                Some(charges::resolution(&mut answer_grant, root_scope, root, resolved));
            // The grant now needs only the reply's last chunk, if that asks
            // nothing; the rest returns before any question waits for
            // admission, which the returned bytes may admit.
            answer_grant.keep(charges::unasked_chunk_bound::<B::Erased, B::Error>(
                chunks.unasked_last(),
            ));
            while let Some(Part { leading, ask, last }) = chunks.next_part() {
                let (question, mut grant) = match ask {
                    Some(question) => {
                        let bytes = question
                            .bound::<B::Error>(root_scope, charges::held(&leading));
                        let need = question.need::<B::Error>(root_scope);
                        let mut grant = account.invite(1, bytes, need).await;
                        let built =
                            answer::question(&backend, &mut grant, root_scope, question).await?;
                        (Some(built), grant)
                    }
                    None => (None, std::mem::take(&mut answer_grant)),
                };
                yield_chunk!(
                    progress, grant, root_scope;
                    leading, question, last;
                    chunk => yield chunk;
                    resolution => unpublished.take();
                    asked;
                );
            }
            // What the answer's grant has left returns now: everything the
            // answer builds is built.
            drop(answer_grant);

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
                root: root?.map(|root| B::assume(&Prefix::new(), root)),
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
        let (mut asked, asked_rx) = internal_child_queries::<B>(asked_height);
        let (mut upper, upper_rx) = internal_parent_resolutions::<B>(asked_height + 2);
        let (mut lower, lower_rx) = internal_child_resolutions::<B>(asked_height + 1);
        let progress = self.progress;
        let account = self.ledger();

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let mut chunk = next_chunk(&mut requests).await?;
                // The reply's need, recomputed from the bound its query
                // carries as its question computed it when admitted: the
                // question withdraws its claim, if it took no floor slot,
                // and the pace weighs the need, once the reply is finished.
                let floor = query.charge.is_floor();
                let (reply_depth, need) = (
                    depth(query.prefix),
                    charges::processing_need::<B::Erased, B::Error>(query.bound),
                );

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                loop {
                let Reply { reactions, charge: received, last, carried } = chunk;
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

                    let depth = depth(child_prefix);
                    if listing.is_empty() {
                        // The supplies are this side's children of the node
                        // that survive the deletion filter, at most all of
                        // them.
                        let bytes = charges::supply_bound::<B::Erased, B::Error>(
                            node.child_count(&child_prefix).min(FAN),
                        );
                        let mut grant = account.admit(Unit::Answer, depth, bytes).await;
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
                        let reply = charges::reply::<B>(&mut grant, reactions);
                        drop(grant);
                        yield_ready!(
                            progress, child_prefix;
                            yield reply;
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    // The node is kept for its scope's assembly.
                    let original = node.clone();
                    let held = node.child_radices(&child_prefix);
                    let children = charges::scope_children(held, &listing);
                    let bytes = charges::answer_bound::<B::Erased, B::Error>(
                        children.either,
                        children.both,
                    );
                    let mut answer_grant = account.admit(Unit::Answer, depth, bytes).await;
                    let children = erased::ops::children_of(&backend, child_prefix, node).await?;
                    let (mut chunks, resolved) = answer::internal(
                        &backend,
                        &their_version,
                        child_prefix,
                        children,
                        listing,
                        &stats,
                        &mut answer_grant,
                    )
                    .await?
                    .into_parts();
                    let mut unpublished =
                        Some(charges::resolution(
                            &mut answer_grant,
                            child_prefix,
                            Some(original),
                            resolved,
                        ));
                    // The grant now needs only the reply's last chunk, if
                    // that asks nothing; the rest returns before any
                    // question waits for admission, which the returned bytes
                    // may admit.
                    answer_grant.keep(charges::unasked_chunk_bound::<B::Erased, B::Error>(
                        chunks.unasked_last(),
                    ));
                    while let Some(Part { leading, ask, last }) = chunks.next_part() {
                        let (question, mut grant) = match ask {
                            Some(question) => {
                                let bytes = question
                                    .bound::<B::Error>(child_prefix, charges::held(&leading));
                                let need = question.need::<B::Error>(child_prefix);
                                let mut grant =
                                    account.invite(depth + 1, bytes, need).await;
                                let built = answer::question(
                                    &backend,
                                    &mut grant,
                                    child_prefix,
                                    question,
                                )
                                .await?;
                                (Some(built), grant)
                            }
                            None => (None, std::mem::take(&mut answer_grant)),
                        };
                        yield_chunk!(
                            progress, grant, child_prefix;
                            leading, question, last;
                            chunk => yield chunk;
                            lower => unpublished.take();
                            asked;
                        );
                    }
                    drop(answer_grant);
                    resolver.pending(radix);
                }

                // The chunk is used up, so its charges return now, before
                // the stage waits for the next one: the peer's admission may
                // wait on them.
                drop((received, carried));
                if last {
                    break;
                }
                chunk = next_chunk(&mut requests).await?;
                }
                // Launch every `Pending` slot's work before publishing its
                // enclosing parent resolution.
                let resolution = resolver.finish()?;
                account.processed(reply_depth, need, floor);
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
        let (mut asked, asked_rx) = leaf_requests();
        let (mut upper, upper_rx) = leaf_parent_resolutions::<B>();
        let (mut lower, lower_rx) = leaf_child_resolutions::<B>();
        let progress = self.progress;
        let account = self.ledger();

        let responses = try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let mut chunk = next_chunk(&mut requests).await?;
                // The reply's need, recomputed from the bound its query
                // carries as its question computed it when admitted: the
                // question withdraws its claim, if it took no floor slot,
                // and the pace weighs the need, once the reply is finished.
                let floor = query.charge.is_floor();
                let (reply_depth, need) = (
                    depth(query.prefix),
                    charges::processing_need::<B::Erased, B::Error>(query.bound),
                );

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                loop {
                let Reply { reactions, charge: received, last, carried } = chunk;
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

                    let depth = depth(child_prefix);
                    if listing.is_empty() {
                        // The supplies are this side's children of the node
                        // that survive the deletion filter, at most all of
                        // them.
                        let bytes = charges::supply_bound::<B::Erased, B::Error>(
                            node.child_count(&child_prefix).min(FAN),
                        );
                        let mut grant = account.admit(Unit::Answer, depth, bytes).await;
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
                        let reply = charges::reply::<B>(&mut grant, reactions);
                        drop(grant);
                        yield_ready!(
                            progress, child_prefix;
                            yield reply;
                            resolver.ready(radix, node);
                        );
                        continue;
                    }

                    // The node is kept for its scope's assembly.
                    let original = node.clone();
                    let held = node.child_radices(&child_prefix);
                    let children = charges::scope_children(held, &listing);
                    let bytes = charges::answer_bound::<B::Erased, B::Error>(
                        children.either,
                        children.both,
                    );
                    let mut answer_grant = account.admit(Unit::Answer, depth, bytes).await;
                    let leaves = erased::ops::children_of(&backend, child_prefix, node).await?;
                    let (mut chunks, resolved) = answer::leaf_parent(
                        &their_version,
                        leaves,
                        listing,
                        &stats,
                        &mut answer_grant,
                    )
                    .into_parts();
                    let mut unpublished =
                        Some(charges::resolution(
                            &mut answer_grant,
                            child_prefix,
                            Some(original),
                            resolved,
                        ));
                    // The grant now needs only the reply's last chunk, if
                    // that asks nothing; the rest returns before any
                    // question waits for admission, which the returned bytes
                    // may admit.
                    answer_grant.keep(charges::unasked_chunk_bound::<B::Erased, B::Error>(
                        chunks.unasked_last(),
                    ));
                    while let Some(Part { leading, ask, last }) = chunks.next_part() {
                        let (question, mut grant) = match ask {
                            Some(radix) => {
                                // A leaf request's reply supplies one leaf,
                                // which processing turns into nothing more:
                                // it would claim no need, so the invitation
                                // share would hold back nothing the budget
                                // admits, and the request is admitted by
                                // the budget alone.
                                let bytes = charges::leaf_question_bound::<B::Erased, B::Error>(
                                    charges::held(&leading),
                                );
                                let mut grant =
                                    account.admit(Unit::Question, depth + 1, bytes).await;
                                let built = answer::leaf_question(&mut grant, child_prefix, radix);
                                (Some(built), grant)
                            }
                            None => (None, std::mem::take(&mut answer_grant)),
                        };
                        yield_chunk!(
                            progress, grant, child_prefix;
                            leading, question, last;
                            chunk => yield chunk;
                            lower => unpublished.take();
                            asked;
                        );
                    }
                    drop(answer_grant);
                    resolver.pending(radix);
                }

                // The chunk is used up, so its charges return now, before
                // the stage waits for the next one: the peer's admission may
                // wait on them.
                drop((received, carried));
                if last {
                    break;
                }
                chunk = next_chunk(&mut requests).await?;
                }
                // Launch every `Pending` slot's work before publishing its
                // enclosing parent resolution.
                let resolution = resolver.finish()?;
                account.processed(reply_depth, need, floor);
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
                // A reply about a leaf's parent lists nothing, so it is
                // predicted to need nothing processed, and the invitation
                // share's pace has nothing to learn from it.

                let mut resolver = Resolver::<B>::new(query, &their_version, &ledger, &stats);
                loop {
                let Reply { reactions, charge: received, last, carried } = chunk;
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

                    let bytes = charges::supply_bound::<B::Erased, B::Error>(1);
                    let mut grant = account.admit(Unit::Answer, depth(child_prefix), bytes).await;
                    let (reactions, node) =
                        answer::leaf(&their_version, radix, node, listing, &stats)
                            .map_err(Error::Violation)?;
                    yield_ready!(
                        progress, child_prefix;
                        yield charges::reply::<B>(&mut grant, reactions);
                        resolver.ready(radix, node);
                    );
                }

                // The chunk is used up, so its charges return now, before
                // the stage waits for the next one: the peer's admission may
                // wait on them.
                drop((received, carried));
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
