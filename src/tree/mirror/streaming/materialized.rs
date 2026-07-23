//! The streaming protocol implemented generically for every materialized
//! backend.
//!
//! Any [`Backend`] can be used here, with no further ceremony.
//!
//! # The session dataflow
//!
//! Each stage runs a loop pairing the counterparty's reply messages, in order,
//! with the stage's queue of pending [`Query`]s — and two [`Work::assemble`]
//! instances recombining what the walk resolves. Three item kinds connect
//! consecutive same-side stages over bounded channels:
//!
//! - **queries** flow down: one queue item per question asked, in question
//!   order (message order, then radix order);
//! - **replies** are the incoming stream: exactly one message per query;
//! - **returns** flow up: exactly one `Option<Node>` per query, in query
//!   order — the reconciled scope, `None` meaning it resolved to nothing
//!   (recursive deletion, the same reading as [`Backend::parent`]'s `None`
//!   return). Returns are prefix-less: the consumer minted the query, so
//!   the key is redundant and the pairing is purely positional.
//!
//! # Why this is deadlock-free
//!
//! Every await in the system is for the k-th item of one specific stream,
//! and every producer produces items 1..k in that order: replies pair with
//! queries, returns pair with queries, and level items arrive in resolution
//! order. Completeness travels *inside* message and item boundaries, never in
//! their absence.
//!
//! Within one pairing loop, the query is dequeued *before* its wire reply is
//! awaited. Either order pairs the same k-th items — the argument above is
//! indifferent — but query-first frees the queue slot one wire round trip
//! earlier, so a K-slot edge admits K truly in-flight scopes rather than
//! K − 1.
//!
//! The first progress-critical ordering invariant is **wire before internal
//! publication**. The walk yields every outgoing query or reply before
//! enqueuing or recording its in-process twin. Backpressure on internal state
//! therefore cannot withhold the wire action that lets the counterparty
//! advance.
//!
//! The second is **resolution before dependent work**. For every disputed
//! child, the walk publishes the [`Resolution`] containing its
//! [`Resolve::Pending`] slots before it sends the child queries whose returns
//! fill those slots. The responder does the same at the root. Before a parent
//! resolution is published, all of the descendant work needed to fulfill it
//! has already been launched. Thus a blocked one-slot query sender has made its
//! resolution available, while a blocked resolution sender is behind an older
//! resolution whose dependent work is already in flight.
//!
//! This makes one slot *sufficient* for every query and resolution channel: the
//! liveness floor. Actual capacities come from the session's
//! [`Window`](super::window) — one slot serializes the descent into a wire
//! round trip per disputed scope, and widening only relaxes the wait graph, so
//! the argument above covers every width. A blocked response pump has
//! likewise already published the response which releases it; the initiator's
//! root query and return and the responder's root resolution each occur exactly
//! once; leaf resolutions contain no `Pending` slots and can be assembled
//! immediately.
//!
//! Every argument above additionally assumes each edge is *independent*: a
//! full edge stalls only its own producer, never delivery on another edge.
//! In process that holds by construction — every edge is its own channel.
//! Over a wire it is a premise the transport must supply, which is why the
//! [link contract](crate::link) demands independently flow-controlled
//! streams: replies then travel edges with exactly the semantics assumed
//! here, and this argument covers remote sessions verbatim. It is not a
//! premise that can be quietly weakened: multiplexing every stream onto
//! one shared FIFO pipe looks sound edge-by-edge yet composes into a
//! cross-stream wait cycle and deadlocks — the conformance suite's mux
//! fixture rebuilds exactly that construction and pins that the probes
//! catch it. Independence is an interface obligation, supplied by the
//! link or not at all.
//!
//! [`Work::assemble`]'s inter-level return queue is the one exception. A reply
//! can dispute a full fan of children. While the walk is still examining those
//! reactions and constructing their parent resolution, already-launched lower
//! scopes can all finish, but the parent resolution containing their `Pending`
//! slots cannot be published until the reaction loop ends. Capacity for one
//! full fan lets every completion enqueue without relying on how many blocked
//! sender futures happen to remain independently runnable. Once the resolution
//! is published, active assembly drains the boundary, so capacity need not grow
//! with width or depth.
//!
//! # Memory model
//!
//! At most one backend query per prefix: whoever explodes a node carries
//! the fan to every consumer that needs it (queries carry their children;
//! pruning returns the survivors it built), so [`Backend::children`] — which
//! may be a database read — is never repeated. The price is that an answer's
//! local batch may hold a fan of queries containing a fan of node handles
//! apiece, at most fan² handles per recursive stage at full fan-out. Bounded
//! query and resolution channels retain only one item; the exceptional fan
//! queue retains completed node handles. On the wire, the memory unit is one
//! reply message.

use std::pin::pin;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node, Root,
        materialized::{unknown::Unknown, work::Work},
        message::{Handshake, Reaction, Reply},
        protocol::{self, BoxResponses, Requests},
        window::WindowConfig,
    },
    typed::{
        Hash, Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};
use before::Version;
use futures::{StreamExt, future::BoxFuture};

/// Publish one disputed scope in its progress-critical order.
///
/// The `yield` expression must be written inside the invocation so
/// `async_stream` can lower it before this macro expands. Keeping all three
/// phases in one expansion prevents a caller from sending dependent work
/// before its resolution or publishing either before the wire reply.
macro_rules! yield_resolve_query {
    (
        $work:expr, $scope:expr;
        $yielded:expr;
        $resolutions:expr => $resolution:expr;
        $queries:expr => $next_queries:expr;
    ) => {{
        let _scope = $scope;
        #[cfg(test)]
        progress::wire($work, _scope);
        $yielded;
        let resolution = $resolution;
        #[cfg(test)]
        progress::resolution($work, &resolution);
        if $resolutions.send(resolution).await.is_err() {
            return;
        }
        for query in $next_queries {
            #[cfg(test)]
            progress::dependent($work, &query);
            if $queries.send(query).await.is_err() {
                return;
            }
        }
    }};
    (
        $work:expr, $scope:expr;
        $yielded:expr;
        $ready:expr;
    ) => {{
        let _scope = $scope;
        #[cfg(test)]
        progress::wire($work, _scope);
        $yielded;
        #[cfg(test)]
        progress::ready($work, _scope);
        $ready;
    }};
}

pub(super) mod channel;
mod common;
mod error;
#[cfg(test)]
pub(super) mod progress;
#[cfg(test)]
pub(super) mod transcript;
pub(super) mod unknown;
mod work;
use channel::{Receiver, Sender};
use common::*;

pub use error::{Error, Violation};

/// Construct a typed protocol-violation result.
fn violation<T, E>(violation: Violation) -> Result<T, Error<E>> {
    Err(Error::Violation(violation))
}

/// A pending query, which we will resolve by a remote reply: the pairing
/// queue between consecutive same-side stages, and the in-process twin of
/// the wire's expected scopes.
///
/// `H` is the children's height; the scope sits at `S<H>`, so
/// `Query<_, _, H>` pairs with [`Reply<_, _, H>`](Reply).
///
/// If we issued a request for a node, `ours` is empty and we expect the
/// reply to consist entirely of supplied nodes.
pub struct Query<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    /// The prefix at which the resolved node will sit.
    pub prefix: Prefix<S<H>>,
    /// Our children of the node (empty if we don't have it at all).
    pub ours: Vec<(u8, B::Node<H>)>,
}

/// One scope's resolution: its children in radix order, each resolved
/// locally or pending on the stages beneath.
pub struct Resolution<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    /// The prefix at which the resolved node will sit.
    prefix: Prefix<S<H>>,
    /// The possibly-resolved children of the node.
    resolved: Vec<(u8, Resolve<B, T, H>)>,
}

pub enum Resolve<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> {
    /// Resolved at the current level: kept, absorbed, or pruned (`None` = gone;
    /// flows into `Backend::parent` as its deletion vocabulary).
    Ready(Option<B::Node<H>>),
    /// Resolved elsewhere: filled by the level stream's next item.
    Pending,
}

// --------------------------------------------------------------------------------
// PROTOCOL IMPLEMENTATION TIME
// --------------------------------------------------------------------------------

/// A mirror stage still at [`Root`](height::Root) height: the handshake phases,
/// before the tree has been disassembled into streams.
///
/// `V` is the version state ([`Start`] → [`Connecting`] → [`Connected`]). The
/// whole tree is held intact as `root` until reconciliation begins at
/// [`initiator`](protocol::Initiator::initiator) /
/// [`responder`](protocol::Responder::responder). The session's outgoing
/// messages carry `backend`'s own node types, which are the ones its
/// counterparty reads.
pub struct Handshaking<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, V> {
    backend: B,
    versions: V,
    root: Root<B, T>,
    /// The session's window choice, resolved against the exchanged set
    /// sizes; see [`window`](super::window).
    window: WindowConfig,
    /// This side's live message count, carried by the greeting.
    local_len: u64,
    /// This side's largest live version encoding in bytes, carried by the
    /// greeting.
    local_version_bytes: u64,
}

/// The version state of a stage that has been opened but has not yet sent its
/// handshake.
pub struct Start {
    our_version: Version,
}

/// The version state of a stage that has sent its greeting but not yet
/// received the peer's.
///
/// Carries the root fan the greeting's listing was derived from, so the
/// descent reuses it instead of asking the backend for the root's children a
/// second time (the memory model's one-query-per-prefix rule).
pub struct Connecting<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> {
    our_version: Version,
    fan: Vec<(u8, B::Node<UnderRoot>)>,
}

/// The version state of a stage that has exchanged greetings with its peer
/// and can proceed with reconciliation.
///
/// Like [`Connecting`], retains the greeting-time root fan for the descent.
pub struct Connected<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> {
    our_version: Version,
    their_version: Version,
    /// The peer's live message count, from its greeting.
    their_len: u64,
    fan: Vec<(u8, B::Node<UnderRoot>)>,
}

/// A mirror stage inside the descent, consuming [`Reply<B, T, H>`](Reply)
/// against a [`Query`] queue at the same height.
pub struct Descending<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    /// The version of the counterparty.
    their_version: Version,
    /// The questions we asked, awaiting their replies in order.
    queries: Receiver<Query<B, T, H>>,
    /// One resolved scope per query, in query order, to the stage above.
    returns: Sender<Option<B::Node<S<H>>>>,
    /// The reassembly work accumulated so far; the terminals drive it to
    /// completion.
    work: Work<B, T>,
    /// Resolves to this side's reconciled root once the top return arrives.
    finish: BoxFuture<'static, Result<Root<B, T>, Error<B::Error>>>,
}

/// The initiator's terminal state: the pending leaf requests, and the
/// accumulated [`Work`] which produces the reconciled root.
///
/// This is not a [`Descending`] stage: its returns are the requested leaves
/// themselves (height `Z`), not an assembled scope one height up, because
/// nothing exists below a leaf to assemble from.
pub struct Completing<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> {
    /// Where each requested leaf will sit, one per request, in order.
    queries: Receiver<Prefix<Z>>,
    /// The requested leaves' resolutions, in request order.
    returns: Sender<Option<B::Node<Z>>>,
    /// The accumulated work to drive the pipeline.
    work: Work<B, T>,
    /// The future result of the pipeline.
    finish: BoxFuture<'static, Result<Root<B, T>, Error<B::Error>>>,
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> Handshaking<B, T, Start> {
    pub fn start(backend: B, root: Root<B, T>) -> Self {
        Self {
            backend,
            versions: Start {
                our_version: root.ceiling.clone(),
            },
            root,
            window: WindowConfig::default(),
            local_len: 0,
            local_version_bytes: 0,
        }
    }

    /// Select this session's window choice; see [`window`](super::window).
    pub fn window(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }

    /// Declare this side's live message count for the greeting; the pair
    /// of exchanged counts sizes a budget-configured window.
    pub fn set_len(mut self, len: u64) -> Self {
        self.local_len = len;
        self
    }

    /// Declare this side's largest live version encoding, in bytes, for
    /// the greeting; the pair of exchanged bounds prices a
    /// budget-configured window's per-node version bytes.
    pub fn max_version_bytes(mut self, bytes: u64) -> Self {
        self.local_version_bytes = bytes;
        self
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, V: Send> protocol::Protocol
    for Handshaking<B, T, V>
{
    type Height = height::Root;
    type Output = Root<B, T>;
    type Error = Error<B::Error>;
}

/// Explode the root into the fan every greeting derives its listing from.
///
/// Runs unconditionally at greeting time — before versions compare — because
/// the listing must ride the greeting regardless of how the session resolves
/// (see [`Handshake`] for the trade). The fan itself is retained through
/// [`Connecting`]/[`Connected`] so the descent never re-asks the backend for
/// the root's children.
pub(crate) async fn greeting_fan<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>(
    backend: &B,
    root: Option<B::Node<height::Root>>,
) -> Result<Vec<(u8, B::Node<UnderRoot>)>, B::Error> {
    match root {
        Some(node) => children_of(backend, Prefix::new(), node).await,
        None => Ok(Vec::new()),
    }
}

/// Derive a `(radix, hash)` listing from a greeting-time fan.
///
/// This is the *single* derivation behind both the greeting's carried
/// listing and the initiator's in-process opening question
/// ([`Work::initiator_level`]). The remote
/// proxy pairs the two positionally, so they must be byte-identical;
/// routing both through this one function makes drift structurally
/// impossible rather than a coincidence of two matching code bodies.
pub(crate) fn fan_listing<N: Node<T>, T: Send + Sync + 'static>(
    fan: &[(u8, N)],
) -> Vec<(u8, Hash)> {
    fan.iter()
        .map(|(radix, node)| (*radix, node.hash()))
        .collect()
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::Connect<B, T>
    for Handshaking<B, T, Start>
{
    type Next = Handshaking<B, T, Connecting<B, T>>;

    async fn connect(self) -> Result<(Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let fan = greeting_fan(&self.backend, self.root.root.clone())
            .await
            .map_err(Error::Backend)?;
        let handshake = Handshake {
            version: our_version.clone(),
            set_len: self.local_len,
            max_version_bytes: self.local_version_bytes,
            listing: fan_listing(&fan),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connecting { our_version, fan },
            root: self.root,
            window: self.window,
            local_len: self.local_len,
            local_version_bytes: self.local_version_bytes,
        };
        Ok((handshake, next))
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::CompleteConnect<B, T>
    for Handshaking<B, T, Connecting<B, T>>
{
    type Next = Handshaking<B, T, Connected<B, T>>;

    async fn complete_connect(self, theirs: Handshake) -> Result<Self::Next, Self::Error> {
        Ok(Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version: self.versions.our_version,
                their_version: theirs.version,
                their_len: theirs.set_len,
                fan: self.versions.fan,
            },
            root: self.root,
            window: self.window,
            local_len: self.local_len,
            local_version_bytes: self.local_version_bytes,
        })
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::Accept<B, T>
    for Handshaking<B, T, Start>
{
    type Next = Handshaking<B, T, Connected<B, T>>;

    async fn accept(self, request: Handshake) -> Result<(Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let fan = greeting_fan(&self.backend, self.root.root.clone())
            .await
            .map_err(Error::Backend)?;
        let handshake = Handshake {
            version: our_version.clone(),
            set_len: self.local_len,
            max_version_bytes: self.local_version_bytes,
            listing: fan_listing(&fan),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version,
                their_version: request.version,
                their_len: request.set_len,
                fan,
            },
            root: self.root,
            window: self.window,
            local_len: self.local_len,
            local_version_bytes: self.local_version_bytes,
        };
        Ok((handshake, next))
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::CompleteEqual<B, T>
    for Handshaking<B, T, Connected<B, T>>
{
    async fn complete_equal(self) -> Result<Root<B, T>, Self::Error> {
        Ok(self.root)
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>> + Sync, T: Send + Sync + 'static> protocol::Initiator<B, T>
    for Handshaking<B, T, Connected<B, T>>
{
    type Next = Descending<B, T, UnderRoot>;

    fn initiator(self) -> (BoxResponses<B, T, UnderRoot, Self::Error>, Self::Next) {
        let Connected {
            our_version,
            their_version,
            their_len,
            fan,
        } = self.versions;
        let ceiling = our_version | &their_version;

        let window = self
            .window
            .resolve(self.local_len, their_len, B::NODE_BYTES);
        let mut work = Work::new(self.backend, window);
        let (responses, queries, returns, finish) = work.initiator_level(ceiling, fan);

        (
            responses,
            Descending {
                their_version,
                queries,
                returns,
                work,
                finish,
            },
        )
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>> + Sync, T: Send + Sync + 'static> protocol::Responder<B, T>
    for Handshaking<B, T, Connected<B, T>>
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn responder(
        self,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (BoxResponses<B, T, UnderRoot, Self::Error>, Self::Next) {
        let Connected {
            our_version,
            their_version,
            their_len,
            fan,
        } = self.versions;
        let ceiling = our_version | &their_version;

        let window = self
            .window
            .resolve(self.local_len, their_len, B::NODE_BYTES);
        let mut work = Work::new(self.backend, window);
        let (responses, queries, returns, finish) =
            work.responder_level(their_version.clone(), ceiling, fan, requests);

        (
            responses,
            Descending {
                their_version,
                queries,
                returns,
                work,
                finish,
            },
        )
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> protocol::Protocol
    for Descending<B, T, H>
where
    S<H>: Height,
{
    type Height = H;
    type Output = Root<B, T>;
    type Error = Error<B::Error>;
}

impl<B, T, H> protocol::Reply<B, T> for Descending<B, T, S<S<H>>>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
    S<H>: Unknown,
    S<S<H>>: Unknown,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, T, H>;

    fn reply(
        mut self,
        requests: impl Requests<B, T, S<S<H>>>,
    ) -> (BoxResponses<B, T, S<H>, Self::Error>, Self::Next) {
        let (responses, queries, upper, lower) =
            self.work
                .internal_level(self.their_version.clone(), requests, self.queries);
        let returns = self.work.assemble(self.returns, upper);
        let returns = self.work.assemble(returns, lower);

        (
            responses,
            Descending {
                their_version: self.their_version,
                queries,
                returns,
                work: self.work,
                finish: self.finish,
            },
        )
    }
}

impl<B, T> protocol::Reply<B, T> for Descending<B, T, S<Z>>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
{
    type Next = Completing<B, T>;

    fn reply(
        mut self,
        requests: impl Requests<B, T, S<Z>>,
    ) -> (BoxResponses<B, T, Z, Self::Error>, Self::Next) {
        let (responses, queries, upper, lower) =
            self.work
                .leaf_parent_level(self.their_version.clone(), requests, self.queries);
        let returns = self.work.assemble(self.returns, upper);
        let returns = self.work.assemble(returns, lower);

        (
            responses,
            Completing {
                queries,
                returns,
                work: self.work,
                finish: self.finish,
            },
        )
    }
}

impl<B, T> protocol::CompleteResponder<B, T> for Descending<B, T, Z>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    fn complete_responder(
        mut self,
        requests: impl Requests<B, T, Z>,
    ) -> (
        BoxResponses<B, T, Z, Self::Error>,
        impl Future<Output = Result<Root<B, T>, Self::Error>> + Send,
    ) {
        let (responses, resolutions) =
            self.work
                .leaf_level(self.their_version, requests, self.queries);
        self.work.assemble_leaves(self.returns, resolutions);
        (responses, self.work.execute(self.finish))
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::Protocol
    for Completing<B, T>
{
    type Height = Z;
    type Output = Root<B, T>;
    type Error = Error<B::Error>;
}

impl<B, T> protocol::CompleteInitiator<B, T> for Completing<B, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<B, T, Z>,
    ) -> Result<Root<B, T>, Self::Error> {
        let mut absorb = pin!(absorb(requests, self.queries, self.returns));
        let mut finish = pin!(self.work.execute(self.finish));

        // Race rather than join: a violation in `absorb` must surface even
        // though the session's remaining work, which includes streams the
        // now-misbehaving counterparty feeds, may never complete.
        tokio::select! {
            absorbed = &mut absorb => {
                absorbed?;
                finish.await
            }
            finished = &mut finish => {
                let root = finished?;
                absorb.await?;
                Ok(root)
            }
        }
    }
}

/// The initiator's terminal loop: pair each pending leaf request with its
/// final [`Reply`] and pass its provision up, prefix-less, like every
/// return.
async fn absorb<B, T>(
    requests: impl Requests<B, T, Z>,
    mut queries: Receiver<Prefix<Z>>,
    returns: Sender<Option<B::Node<Z>>>,
) -> Result<(), Error<B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let mut requests = pin!(requests);
    while let Some(prefix) = queries.recv().await {
        let Some(Reply { replies }) = requests.next().await else {
            return violation(Violation::UnansweredQuery);
        };

        // The last radix of the prefix is the one we expect should be supplied.
        let (_, expected) = prefix.pop();

        // Only if we received exactly that radix paired with a leaf, do we absorb it.
        let supply = match replies.as_slice() {
            [] => None,
            [Reaction::Supply(radix, leaf)] if *radix == expected => Some(leaf.clone()),
            [Reaction::Supply(_, _)] => return violation(Violation::InvalidSupply),
            _ => return violation(Violation::UnfinishedReply),
        };

        // Then we send that (optional) leaf upwards.
        if returns.send(supply).await.is_err() {
            return Ok(());
        }
    }

    // If there are more replies, something is wrong: every reply should have
    // been claimed by one of the now-exhausted queries.
    if requests.next().await.is_some() {
        return violation(Violation::UnaskedReply);
    }

    Ok(())
}
