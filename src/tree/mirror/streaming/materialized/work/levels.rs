//! Phase-specific materialized reconciliation walks.
//!
//! Every walk body runs on the erased vocabulary — instantiated per backend,
//! never per height — behind a thin typed method that erases the stage's
//! request stream on the way in and re-tags its responses on the way out
//! ([`Work::respond`]). The generic methods carry the type-level height the
//! schedule proves; the walk bodies carry it as the runtime witness every
//! scope's prefix length *is*.

use std::pin::pin;

use async_stream::try_stream;
use before::Version;
use futures::{FutureExt, Stream, StreamExt, future::BoxFuture};

use super::{
    LeafRequest, Work,
    answer::{self, Answer, Built, Held, Part, Question},
    assembly::assemble,
    charges,
    queues::*,
    resolver::{Dispute, Resolver},
};
use crate::network::NetworkKeys;
use crate::tree::{
    mirror::streaming::{
        Backend, ErasedNode, FAN, Leaf, Root,
        channel::{Receiver, Sender},
        erased::{self, Reaction, Reply},
        holding::Holding,
        ledger::{Charge, Ledger, ReplyBound, Unit},
        materialized::{
            Error, Query, Resolution, SupplyLedger, Violation, fan_listing, progress::Scoped,
            unknown::unknown_providing, violation,
        },
        message::{Carried, Grandchildren, Listed},
        protocol::{BoxRequests, BoxResponses, Requests},
        stats::Recorder,
        tasks::next_or_cancelled,
    },
    typed::{
        ErasedPrefix, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

/// Publish one chunk of an answer in its progress-critical order.
///
/// The caller walks the answer's [`Part`]s, admitting each part's question and
/// building it from its planned form only when the part is reached, so a
/// chunk's built items are its own question's, and passes the built question
/// here with the grant the chunk is built out of, the question's own or, for
/// the chunk without one, the answer's, and with the answer's resolution while
/// it is still unpublished. The chunk's wire action comes first; then, after
/// the first chunk alone, the resolution, whose pending slots the questions'
/// returns fill; then the chunk's question. A blocked question sender has
/// therefore handed on every earlier chunk whole and made the resolution
/// available, and the reply's last chunk holds its last reaction.
///
/// The caller writes the yield, `chunk => yield chunk`, inside its own
/// `async_stream` body, where the stream can lower it, and builds the question
/// there too, so a build failure's `?` is lowered as well.
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
                Holding::OutgoingReplies.create(|| leading.push(reaction));
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

/// Publish one whole-reply scope, which asks nothing, in its progress-critical
/// order: its wire action, then its local resolution.
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

/// The depth of the scope `prefix` names, the root's 0 through a leaf's 32: the
/// depth admission counts a unit concerning that scope at.
fn depth(prefix: ErasedPrefix) -> usize {
    prefix.as_bytes().len()
}

/// The grant of the initiator's opening question, admitted as the roles are
/// elected, with the room its reply parks in already taken out of it.
///
/// The opening question writes no frame of its own: its listing crossed inside
/// the greeting, so the peer may have answered it already, and its reply's
/// frames wait in the receive path, which never waits on the walk, from the
/// moment the session's pipe starts reading. The room that pays for them must
/// be charged by then, whenever the root stage runs. So the initiator admits
/// the question as it opens its descent ([`admit`](Self::admit)), before the
/// session's tasks run, once the peer's grandchild count of its root, which
/// bounds the reply, is known; the root stage then builds the question out of
/// the grant ([`Work::initiator_level`]). The responder asks no opening
/// question and admits none.
pub(crate) struct OpeningAdvance {
    /// What is left of the opening question's grant.
    grant: Charge,
    /// The room the opening's reply parks in.
    room: Charge,
}

impl OpeningAdvance {
    /// Admit, out of `ledger`, the grant of an opening question listing
    /// `listed` of this side's root children, whose reply the peer's count
    /// `responder` of its root's grandchildren bounds, and take its reply's
    /// room.
    ///
    /// # Panics
    ///
    /// Panics if the grant is not admitted at once: a programmer error, since
    /// the opening is the first and only question about the root, so it finds
    /// its kind and depth holding nothing, and admission grants such a unit at
    /// once whatever the budget or the queue.
    pub(crate) fn admit<E, Err>(ledger: &Ledger, listed: usize, responder: Grandchildren) -> Self {
        let root = Prefix::new().erase();
        let bytes =
            charges::question_bound::<E, Err>(root.height(), responder, Held::default(), listed);
        let mut grant = ledger
            .admit(Unit::Question, depth(root), bytes)
            .now_or_never()
            .expect("the opening is the first question about the root, so its kind and depth hold nothing and it is exempt");
        let bound = ReplyBound::of(root.height(), listed, responder);
        let room = charges::room::<E>(&mut grant, bound, listed);
        Self { grant, room }
    }
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

/// A level whose walk answers each disputed child with an answer that asks
/// about the child's own children one level down.
///
/// An internal level's questions are queries about internal scopes; the
/// leaf-parent level's questions request single leaves. Both levels run one
/// walk ([`Work::answering_walk`]). An implementation supplies the two steps in
/// which they differ: classifying a disputed child's children into an answer,
/// and admitting and building each question that answer plans.
///
/// Each step may wait only on the storage backend and, for `ask`, on the
/// ledger's admission of its question. Those are the only waits the walk makes
/// outside its own edges, and the deadlock argument in the materialized
/// module's docs, with the admission induction it cites, covers exactly them: a
/// step that waited on another stage, or on the peer, would wait outside both.
trait AnsweringLevel<B: Backend<Node<Z>: Leaf>>: 'static {
    /// A question as an answer plans it, before it is built.
    type Planned: Send + 'static;
    /// What a built question sends down to the next level's walk.
    type Asked: Scoped + Send + 'static;

    /// Classify the children of the disputed scope `prefix`, this side's `ours`
    /// against the peer's listing `theirs`, charging the plan out of the
    /// answer's `grant`.
    // The arguments are the answer's inputs, one per concern, as for
    // `answer::internal`; bundling them into a struct would only rename the
    // arity.
    #[allow(clippy::too_many_arguments)]
    fn answer(
        backend: &B,
        keys: Option<&NetworkKeys>,
        their_version: &Version,
        stats: &Recorder,
        prefix: ErasedPrefix,
        ours: Vec<(u8, B::Erased)>,
        theirs: Vec<Listed>,
        grant: &mut Charge,
    ) -> impl Future<Output = Result<Answer<B::Erased, Self::Planned>, Error<B::Error>>> + Send;

    /// Admit, out of `account`, the question `planned` that an answer about
    /// `prefix` asks in a chunk behind the `leading` reactions, and build it
    /// out of its grant.
    ///
    /// Returns the question built, and what is left of the grant, out of which
    /// the chunk ending at the question is built.
    fn ask(
        backend: &B,
        keys: Option<&NetworkKeys>,
        account: &Ledger,
        prefix: ErasedPrefix,
        leading: Held,
        planned: Self::Planned,
    ) -> impl Future<Output = Result<(Built<B::Erased, Self::Asked>, Charge), Error<B::Error>>> + Send;
}

/// An internal level: each question its answers plan is a dispute or a request
/// about an internal child, admitted under the invitation share.
struct Internal;

impl<B: Backend<Node<Z>: Leaf>> AnsweringLevel<B> for Internal {
    type Planned = Question<B::Erased>;
    type Asked = Query<B::Erased>;

    async fn answer(
        backend: &B,
        keys: Option<&NetworkKeys>,
        their_version: &Version,
        stats: &Recorder,
        prefix: ErasedPrefix,
        ours: Vec<(u8, B::Erased)>,
        theirs: Vec<Listed>,
        grant: &mut Charge,
    ) -> Result<Answer<B::Erased, Self::Planned>, Error<B::Error>> {
        answer::internal(
            backend,
            their_version,
            prefix,
            ours,
            theirs,
            keys,
            stats,
            grant,
        )
        .await
    }

    async fn ask(
        backend: &B,
        keys: Option<&NetworkKeys>,
        account: &Ledger,
        prefix: ErasedPrefix,
        leading: Held,
        planned: Self::Planned,
    ) -> Result<(Built<B::Erased, Self::Asked>, Charge), Error<B::Error>> {
        let bytes = planned.bound::<B::Error>(prefix, leading);
        let need = planned.need::<B::Error>(prefix);
        let mut grant = account.invite(depth(prefix) + 1, bytes, need).await;
        let built = answer::question(backend, keys, &mut grant, prefix, planned).await?;
        Ok((built, grant))
    }
}

/// The leaf-parent level: each question its answers plan requests, by its
/// radix, a leaf only the peer holds.
struct LeafParent;

impl<B: Backend<Node<Z>: Leaf>> AnsweringLevel<B> for LeafParent {
    type Planned = u8;
    type Asked = LeafRequest;

    async fn answer(
        _backend: &B,
        _keys: Option<&NetworkKeys>,
        their_version: &Version,
        stats: &Recorder,
        _prefix: ErasedPrefix,
        ours: Vec<(u8, B::Erased)>,
        theirs: Vec<Listed>,
        grant: &mut Charge,
    ) -> Result<Answer<B::Erased, Self::Planned>, Error<B::Error>> {
        Ok(answer::leaf_parent(
            their_version,
            ours,
            theirs,
            stats,
            grant,
        ))
    }

    async fn ask(
        _backend: &B,
        _keys: Option<&NetworkKeys>,
        account: &Ledger,
        prefix: ErasedPrefix,
        leading: Held,
        radix: u8,
    ) -> Result<(Built<B::Erased, Self::Asked>, Charge), Error<B::Error>> {
        // A leaf request's reply supplies one leaf, which processing turns
        // into nothing more: it would claim no need, so the invitation share
        // would hold back nothing the budget admits, and the request is
        // admitted by the budget alone.
        let bytes = charges::leaf_question_bound::<B::Erased, B::Error>(leading);
        let mut grant = account
            .admit(Unit::Question, depth(prefix) + 1, bytes)
            .await;
        let built = answer::leaf_question(&mut grant, prefix, radix);
        Ok((built, grant))
    }
}

/// The edges an answering walk publishes on.
struct Edges<Q, E> {
    /// The questions its answers ask, to the next level's walk.
    asked: Sender<Q>,
    /// Each query's resolution, its scope's, once every child's work is
    /// launched.
    upper: Sender<Resolution<E>>,
    /// Each disputed child's resolution, ahead of its questions.
    lower: Sender<Resolution<E>>,
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
    /// already carried, which is what lets the remote proxy satisfy this stage
    /// from the greeting instead of a wire frame.
    ///
    /// The opening reply is that question alone. Root children only this side
    /// holds travel as ordinary supplies, answering the responder's empty
    /// queries for them one level down.
    ///
    /// `root` is the root `fan` was listed from, which the question carries to
    /// the root's assembly, and `advance` the question's grant and its reply's
    /// room, admitted as the roles were elected.
    pub fn initiator_level(
        &mut self,
        ceiling: Version,
        fan: Vec<(u8, B::Erased)>,
        root: Option<B::Erased>,
        responder: Grandchildren,
        advance: OpeningAdvance,
    ) -> (
        BoxResponses<B, UnderRoot, Error<B::Error>>,
        Receiver<Query<B::Erased>>,
        Sender<Option<B::Erased>>,
        BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
    ) {
        let (mut queries, queries_rx) = initiator_root_query::<B>();
        let (returns, mut returns_rx) = initiator_root_return::<B>();
        let progress = self.progress;
        let keys = self.keys();

        let responses = try_stream! {
            let root_scope = Prefix::new().erase();
            // The opening question is the one question no answer asks; it
            // asks about the root. Its grant and its reply's room were
            // admitted as the roles were elected ([`OpeningAdvance`]).
            let OpeningAdvance { mut grant, room } = advance;
            progress.wire(root_scope);
            let bound = ReplyBound::of(root_scope.height(), fan.len(), responder);
            let listing = Holding::OutgoingReplies.create(|| {
                fan_listing(root_scope, &fan, keys.as_deref()).map(|listing| vec![Reaction::Query(listing)])
            })
            .map_err(Error::Count)?;
            let record = charges::record(&mut grant, root_scope.height(), bound, fan.len());
            let query = charges::query(&mut grant, root_scope, root, fan, bound);
            yield Holding::OutgoingReplies.create(|| {
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
    /// `fan` is the greeting-time root fan: the greeting already carried the
    /// root's children, so this stage starts from that listing rather than
    /// exploding the root itself.
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
        let keys = self.keys();

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
                keys.as_deref(),
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
                        // The root's answer is an internal one, whose
                        // questions are internal questions about the root's
                        // children.
                        let (built, grant) = <Internal as AnsweringLevel<B>>::ask(
                            &backend,
                            keys.as_deref(),
                            &account,
                            root_scope,
                            charges::held(&leading),
                            question,
                        )
                        .await?;
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
        let assembled =
            Holding::Assembly.creating_stream(assemble(assembling, resolution_rx, returns_rx));
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
    /// `H` is the height the walk's dependent queries descend to; the walk's
    /// own scopes sit two levels above it, exactly as the schedule's
    /// [`Reply`](crate::tree::mirror::streaming::protocol::Reply) transition
    /// demands.
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
        let (asked, asked_rx) = internal_child_queries::<B>(H::HEIGHT);
        let (upper, upper_rx) = internal_parent_resolutions::<B>(H::HEIGHT + 2);
        let (lower, lower_rx) = internal_child_resolutions::<B>(H::HEIGHT + 1);
        let edges = Edges {
            asked,
            upper,
            lower,
        };
        let responses =
            self.answering_walk::<Internal>(their_version, ledger, requests, queries, edges);
        (
            self.respond::<S<H>>(responses),
            asked_rx,
            upper_rx,
            lower_rx,
        )
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
        let (asked, asked_rx) = leaf_requests();
        let (upper, upper_rx) = leaf_parent_resolutions::<B>();
        let (lower, lower_rx) = leaf_child_resolutions::<B>();
        let edges = Edges {
            asked,
            upper,
            lower,
        };
        let responses =
            self.answering_walk::<LeafParent>(their_version, ledger, requests, queries, edges);
        (self.respond::<Z>(responses), asked_rx, upper_rx, lower_rx)
    }

    /// The walk of the answering level `L`, shared by every height
    /// ([`internal_level`](Self::internal_level),
    /// [`leaf_parent_level`](Self::leaf_parent_level)).
    ///
    /// For each query, the walk reads its reply chunk by chunk, answers each
    /// disputed child with a reply of its own, and publishes the query's
    /// resolution once every child's work is launched. The levels differ only
    /// in the steps [`AnsweringLevel`] supplies, so the orderings their
    /// progress rests on are written once, here, for both.
    ///
    /// The walk is the one producer of its level's answers and of their
    /// questions: it admits each answer, then each of the answer's questions,
    /// awaiting each admission in turn, so it asks for an answer's questions
    /// before its next answer, as admission's liveness argument requires.
    /// Processing a level's replies concurrently would add producers at the
    /// same kinds and depths, which the ledger admits first come, first served
    /// against the same bounds and budget, so the concurrency would add
    /// waiters, not room.
    fn answering_walk<L: AnsweringLevel<B>>(
        &mut self,
        their_version: Version,
        ledger: SupplyLedger,
        requests: BoxRequests<B::Erased>,
        mut queries: Receiver<Query<B::Erased>>,
        edges: Edges<L::Asked, B::Erased>,
    ) -> impl Stream<Item = Result<Reply<B::Erased>, Error<B::Error>>> + Send + 'static + use<B, L>
    {
        let Edges {
            mut asked,
            mut upper,
            mut lower,
        } = edges;
        let backend = self.backend();
        let stats = self.stats.clone();
        let progress = self.progress;
        let account = self.ledger();
        let keys = self.keys();

        try_stream! {
            let mut requests = requests;
            while let Some(query) = queries.recv().await {
                let mut chunk = next_chunk(&mut requests).await?;
                // The reply's need, recomputed from the bound its query
                // carries as its question computed it when admitted: the
                // question withdraws its claim, if it claimed, and the pace
                // weighs the need, once the reply is finished.
                let claimed = query.charge.claimed();
                let (reply_depth, need) = (
                    depth(query.prefix),
                    charges::processing_need::<B::Erased, B::Error>(query.bound),
                );

                let mut resolver = Resolver::<B>::new(query, &ledger, &stats);
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
                        let (node, children) = Holding::OutgoingReplies.creating(unknown_providing(&backend, &their_version, child_prefix, node, &stats))
                        .await?;
                        let reactions = Holding::OutgoingReplies.create(|| {
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
                    let (mut chunks, resolved) = L::answer(
                        &backend,
                        keys.as_deref(),
                        &their_version,
                        &stats,
                        child_prefix,
                        children,
                        listing,
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
                            Some(planned) => {
                                let (built, grant) = L::ask(
                                    &backend,
                                    keys.as_deref(),
                                    &account,
                                    child_prefix,
                                    charges::held(&leading),
                                    planned,
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
                account.processed(reply_depth, need, claimed);
                progress.parent_resolution(&resolution);
                if upper.send(resolution).await.is_err() {
                    return;
                }
            }

            if requests.next().await.is_some() {
                return violation(Violation::UnaskedReply)?;
            }
        }
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

                let mut resolver = Resolver::<B>::new(query, &ledger, &stats);
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
