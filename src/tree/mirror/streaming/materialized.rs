//! The streaming protocol implemented generically for every materialized
//! backend.
//!
//! Any [`Backend`] can be used here, with no further ceremony.
//!
//! # The session dataflow
//!
//! Two terms carry everything below. A *scope* is the subtree one question
//! names — a prefix and whatever both sides hold under it; a *stage* is one
//! height's pairing loop over such scopes.
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
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::tree::{
    mirror::contained,
    mirror::streaming::{
        Backend, ErasedNode, Leaf, Root,
        erased::{self, Reaction, Reply},
        materialized::work::Work,
        message::Greeting,
        protocol::{self, BoxResponses, Requests},
        remote::DEFAULT_TARGET_MESSAGE_SIZE,
        stats::Recorder,
        window::WindowConfig,
    },
    typed::{
        ErasedPrefix, Hash, Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};
use before::Version;
use futures::{StreamExt, future::BoxFuture};
use tokio::sync::oneshot;

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
mod tests;
#[cfg(test)]
pub(super) mod transcript;
pub(super) mod unknown;
mod work;
use channel::{Receiver, Sender};
use common::*;
// The remote proxy explodes early-supplied whole root children into the
// same per-child shape the walks consume, with the walks' own helper.
pub(crate) use common::children_of;

pub use error::{Error, Violation};

/// Construct a typed protocol-violation result.
fn violation<T, E>(violation: Violation) -> Result<T, Error<E>> {
    Err(Error::Violation(violation))
}

/// The session-total supplied-leaf ledger: the ingestion-side counterpart
/// of the greeting's declared set length, shared by every stage that
/// absorbs supplies.
///
/// An honest replica supplies each leaf at most once (disputed scopes
/// partition the tree) and only leaves its own set holds, so the running
/// total of absorbed live leaves is bounded by the `set_len` its greeting
/// declared — a premise the session's window solve priced. Two
/// instruments enforce it, each holding its own ledger over the same
/// declaration: the walk charges each supply exactly where it checks
/// version containment, so the two declared premises are enforced side by
/// side; the wire decoder charges each supplied record at ingress, before
/// its payload takes backend custody, so the bound holds at every instant
/// of a still-open reply.
#[derive(Clone, Debug)]
pub(crate) struct SupplyLedger {
    /// The sender's greeting-declared set length.
    declared: u64,
    /// Live leaves absorbed so far, across every ingestion site sharing
    /// this ledger.
    absorbed: Arc<AtomicU64>,
}

impl SupplyLedger {
    pub(crate) fn new(declared: u64) -> Self {
        SupplyLedger {
            declared,
            absorbed: Arc::default(),
        }
    }

    /// Charge `leaves` absorbed supplies against the declaration.
    ///
    /// Errors with the declared length at the first leaf past it; each
    /// enforcement point renders the overdraw in its own typed vocabulary
    /// (the walk's [`Violation::OverdrawnSupply`], the wire decoder's
    /// ingress rejection).
    pub(crate) fn charge(&self, leaves: u64) -> Result<(), u64> {
        let prior = self.absorbed.fetch_add(leaves, Ordering::Relaxed);
        match prior.checked_add(leaves) {
            Some(total) if total <= self.declared => Ok(()),
            // A wrapped counter is past any declarable length too.
            _ => Err(self.declared),
        }
    }

    /// Charge `leaves` absorbed supplies, failing the session at the first
    /// leaf past the declaration ([`Violation::OverdrawnSupply`]).
    pub(crate) fn absorb<E>(&self, leaves: u64) -> Result<(), Error<E>> {
        match self.charge(leaves) {
            Ok(()) => Ok(()),
            Err(_) => violation(Violation::OverdrawnSupply),
        }
    }
}

/// A pending query, which we will resolve by a remote reply: the pairing
/// queue between consecutive same-side stages, and the in-process twin of
/// the wire's expected scopes.
///
/// `E` is the backend's erased node representation
/// ([`Backend::Erased`]); the prefix names the queried scope, and its
/// byte length is the scope's height witness (see
/// [`erased`]). A query pairs with the reply at its
/// children's height, one level below the prefix.
///
/// If we issued a request for a node, `ours` is empty and we expect the
/// reply to consist entirely of supplied nodes.
pub struct Query<E> {
    /// The prefix at which the resolved node will sit.
    pub prefix: ErasedPrefix,
    /// Our children of the node (empty if we don't have it at all).
    pub ours: Vec<(u8, E)>,
}

/// One scope's resolution: its children in radix order, each resolved
/// locally or pending on the stages beneath.
pub struct Resolution<E> {
    /// The prefix at which the resolved node will sit.
    pub(crate) prefix: ErasedPrefix,
    /// The possibly-resolved children of the node.
    pub(crate) resolved: Vec<(u8, Resolve<E>)>,
}

/// One child's slot in a [`Resolution`].
pub enum Resolve<E> {
    /// Resolved at the current level: kept, absorbed, or pruned (`None` = gone;
    /// flows into `Backend::parent` as its deletion vocabulary).
    Ready(Option<E>),
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
pub struct Handshaking<B: Backend<Node<Z>: Leaf>, V> {
    backend: B,
    versions: V,
    root: Root<B>,
    /// The session's window choice, resolved against the exchanged set
    /// sizes; see [`window`](super::window).
    window: WindowConfig,
    /// This side's supply-run byte target, carried by the greeting; the
    /// session runs at the minimum of the two ends' targets.
    target_message_size: u64,
    /// The session's stats recorder; the walk and the window solve write
    /// through it, and the session driver snapshots it after completion.
    stats: Recorder,
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
pub struct Connecting<B: Backend<Node<Z>: Leaf>> {
    our_version: Version,
    /// The root fan, already erased: everything downstream of the
    /// greeting — the descent's workers included — speaks the erased
    /// representation.
    fan: Vec<(u8, B::Erased)>,
}

/// The version state of a stage that has exchanged greetings with its peer
/// and can proceed with reconciliation.
///
/// Like [`Connecting`], retains the greeting-time root fan for the descent.
pub struct Connected<B: Backend<Node<Z>: Leaf>> {
    our_version: Version,
    their_version: Version,
    /// The peer's live message count, from its greeting.
    their_len: u64,
    /// The peer's largest live version-bound encoding in bytes, from
    /// its greeting.
    their_version_bytes: u64,
    /// The peer's root-fan listing, from its greeting: what an elected
    /// initiator merges its own fan against to ship its exclusive root
    /// children as the opening's early supplies.
    their_listing: Vec<(u8, Hash)>,
    /// The root fan, erased at greeting time ([`Connecting`]).
    fan: Vec<(u8, B::Erased)>,
}

/// A mirror stage inside the descent, consuming [`Reply<B, H>`](Reply)
/// against a [`Query`] queue at the same height.
pub struct Descending<B: Backend<Node<Z>: Leaf>, H: Height>
where
    S<H>: Height,
{
    /// The version of the counterparty.
    their_version: Version,
    /// The counterparty's declared-set-length ledger, charged by every
    /// absorbed supply ([`SupplyLedger`]).
    ledger: SupplyLedger,
    /// The questions we asked, awaiting their replies in order.
    ///
    /// The payloads are erased ([`Backend::Erased`]); the typestate's
    /// `H` is what pins this queue to the walk stage that consumes it at
    /// the right height, and every payload's prefix carries the runtime
    /// witness.
    queries: Receiver<Query<B::Erased>>,
    /// One resolved scope per query, in query order, to the stage above.
    returns: Sender<Option<B::Erased>>,
    /// An elected initiator's opening hand-off: the early-supplied root
    /// radices' survivors, consumed by the first descending stage to answer
    /// the responder's empty queries about them (`None` below it).
    early_survivors: Option<oneshot::Receiver<Vec<(u8, Option<B::Erased>)>>>,
    /// An elected responder's opening hand-off (`None` below the first
    /// descending stage).
    ///
    /// The root children the initiator supplied early, pre-exploded into
    /// their own children, consumed by the first descending stage to
    /// resolve its own root-level requests.
    #[allow(clippy::type_complexity)]
    early_supplies: Option<oneshot::Receiver<Vec<(u8, Vec<(u8, B::Erased)>)>>>,
    /// The reassembly work accumulated so far; the terminals drive it to
    /// completion.
    work: Work<B>,
    /// Resolves to this side's reconciled root once the top return arrives.
    finish: BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
    /// The stage's height, phantom.
    ///
    /// The payloads above are erased; this tag is what the schedule's
    /// typestates keep proving about them (`PhantomData<fn() -> H>` for
    /// the auto-trait shortcut; see [`typed::Node`](crate::tree::typed::Node)).
    height: std::marker::PhantomData<fn() -> H>,
}

/// The initiator's terminal state: the pending leaf requests, and the
/// accumulated [`Work`] which produces the reconciled root.
///
/// This is not a [`Descending`] stage: its returns are the requested leaves
/// themselves (height `Z`), not an assembled scope one height up, because
/// nothing exists below a leaf to assemble from.
pub struct Completing<B: Backend<Node<Z>: Leaf>> {
    /// The peer's declared greeting version: the containment bound every
    /// supplied leaf is checked against
    /// ([`Violation::UncontainedSupply`]).
    their_version: Version,
    /// The counterparty's declared-set-length ledger, charged by every
    /// absorbed supply ([`SupplyLedger`]).
    ledger: SupplyLedger,
    /// Where each requested leaf will sit, one per request, in order.
    queries: Receiver<Prefix<Z>>,
    /// The requested leaves' resolutions, in request order.
    returns: Sender<Option<B::Erased>>,
    /// The accumulated work to drive the pipeline.
    work: Work<B>,
    /// The future result of the pipeline.
    finish: BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
}

impl<B: Backend<Node<Z>: Leaf>> Handshaking<B, Start> {
    /// Construct the session in its opening phase, at the default window
    /// and message-size target.
    pub fn start(backend: B, root: Root<B>) -> Self {
        Self {
            backend,
            versions: Start {
                our_version: root.ceiling.clone(),
            },
            root,
            window: WindowConfig::default(),
            target_message_size: DEFAULT_TARGET_MESSAGE_SIZE as u64,
            stats: Recorder::default(),
        }
    }

    /// Select this session's window choice; see [`window`](super::window).
    pub fn window(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }

    /// Declare this side's supply-run byte target for the greeting; the
    /// session's encoders on both ends run at the minimum of the two
    /// exchanged targets.
    pub fn target_message_size(mut self, bytes: u64) -> Self {
        self.target_message_size = bytes;
        self
    }

    /// Share the session's stats recorder, so a driver holding its clone
    /// can read the walk's counts after the session completes.
    ///
    /// Without this call the session still records, into a recorder
    /// nobody reads.
    pub fn stats(mut self, stats: Recorder) -> Self {
        self.stats = stats;
        self
    }
}

impl<B: Backend<Node<Z>: Leaf>, V: Send> protocol::Protocol for Handshaking<B, V> {
    type Height = height::Root;
    type Output = Root<B>;
    type Error = Error<B::Error>;
}

/// Explode the root into the fan every greeting derives its listing from.
///
/// Runs unconditionally at greeting time — before versions compare — because
/// the listing must ride the greeting regardless of how the session resolves
/// (see [`Greeting`] for the trade). The fan itself is retained through
/// [`Connecting`]/[`Connected`] so the descent never re-asks the backend for
/// the root's children.
pub(crate) async fn greeting_fan<B: Backend<Node<Z>: Leaf>>(
    backend: &B,
    root: Option<B::Node<height::Root>>,
) -> Result<Vec<(u8, B::Erased)>, B::Error> {
    match root {
        Some(node) => {
            erased::ops::children_of(backend, Prefix::new().erase(), B::erase(node)).await
        }
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
pub(crate) fn fan_listing<E: ErasedNode>(fan: &[(u8, E)]) -> Vec<(u8, Hash)> {
    fan.iter()
        .map(|(radix, node)| (*radix, node.hash()))
        .collect()
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Connect<B> for Handshaking<B, Start> {
    type Next = Handshaking<B, Connecting<B>>;

    async fn connect(self) -> Result<(Greeting, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let fan = greeting_fan(&self.backend, self.root.root.clone())
            .await
            .map_err(Error::Backend)?;
        let greeting = Greeting {
            version: our_version.clone(),
            // The greeting's sizes come from the root's own aggregates,
            // so they cannot drift from the tree they describe.
            set_len: self.root.len(),
            max_version_bytes: self.root.max_version_bytes(),
            target_message_size: self.target_message_size,
            listing: fan_listing(&fan),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connecting { our_version, fan },
            root: self.root,
            window: self.window,
            target_message_size: self.target_message_size,
            stats: self.stats,
        };
        Ok((greeting, next))
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::CompleteConnect<B> for Handshaking<B, Connecting<B>> {
    type Next = Handshaking<B, Connected<B>>;

    async fn complete_connect(self, theirs: Greeting) -> Result<Self::Next, Self::Error> {
        Ok(Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version: self.versions.our_version,
                their_version: theirs.version,
                their_len: theirs.set_len,
                their_version_bytes: theirs.max_version_bytes,
                their_listing: theirs.listing,
                fan: self.versions.fan,
            },
            root: self.root,
            window: self.window,
            target_message_size: self.target_message_size,
            stats: self.stats,
        })
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Accept<B> for Handshaking<B, Start> {
    type Next = Handshaking<B, Connected<B>>;

    async fn accept(self, request: Greeting) -> Result<(Greeting, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let fan = greeting_fan(&self.backend, self.root.root.clone())
            .await
            .map_err(Error::Backend)?;
        let greeting = Greeting {
            version: our_version.clone(),
            // The greeting's sizes come from the root's own aggregates,
            // so they cannot drift from the tree they describe.
            set_len: self.root.len(),
            max_version_bytes: self.root.max_version_bytes(),
            target_message_size: self.target_message_size,
            listing: fan_listing(&fan),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version,
                their_version: request.version,
                their_len: request.set_len,
                their_version_bytes: request.max_version_bytes,
                their_listing: request.listing,
                fan,
            },
            root: self.root,
            window: self.window,
            target_message_size: self.target_message_size,
            stats: self.stats,
        };
        Ok((greeting, next))
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::CompleteEqual<B> for Handshaking<B, Connected<B>> {
    async fn complete_equal(self) -> Result<Root<B>, Self::Error> {
        Ok(self.root)
    }
}

impl<B: Backend<Node<Z>: Leaf> + Sync> protocol::Initiator<B> for Handshaking<B, Connected<B>> {
    type Next = Descending<B, UnderRoot>;

    fn initiator(self) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let Connected {
            our_version,
            their_version,
            their_len,
            their_version_bytes,
            their_listing,
            fan,
        } = self.versions;
        let ceiling = our_version | &their_version;

        let window = self.window.resolve(
            self.root.len(),
            their_len,
            self.root.max_version_bytes(),
            their_version_bytes,
            B::node_bytes,
        );
        self.stats.window_granted(window.widest());
        let ledger = SupplyLedger::new(their_len);
        let mut work = Work::new(self.backend, window, self.stats);
        let (responses, queries, returns, early, finish) =
            work.initiator_level(their_version.clone(), ceiling, fan, their_listing);

        (
            responses,
            Descending {
                their_version,
                ledger,
                queries,
                returns,
                early_survivors: Some(early),
                early_supplies: None,
                work,
                finish,
                height: std::marker::PhantomData,
            },
        )
    }
}

impl<B: Backend<Node<Z>: Leaf> + Sync> protocol::Responder<B> for Handshaking<B, Connected<B>> {
    type Next = Descending<B, UnderUnderRoot>;

    fn responder(
        self,
        requests: impl Requests<B, UnderRoot>,
    ) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let Connected {
            our_version,
            their_version,
            their_len,
            their_version_bytes,
            their_listing: _,
            fan,
        } = self.versions;
        let ceiling = our_version | &their_version;

        let window = self.window.resolve(
            self.root.len(),
            their_len,
            self.root.max_version_bytes(),
            their_version_bytes,
            B::node_bytes,
        );
        self.stats.window_granted(window.widest());
        let ledger = SupplyLedger::new(their_len);
        let mut work = Work::new(self.backend, window, self.stats);
        let (responses, queries, returns, early, finish) = work.responder_level(
            their_version.clone(),
            ledger.clone(),
            ceiling,
            fan,
            requests,
        );

        (
            responses,
            Descending {
                their_version,
                ledger,
                queries,
                returns,
                early_survivors: None,
                early_supplies: Some(early),
                work,
                finish,
                height: std::marker::PhantomData,
            },
        )
    }
}

impl<B: Backend<Node<Z>: Leaf>, H: Height> protocol::Protocol for Descending<B, H>
where
    S<H>: Height,
{
    type Height = H;
    type Output = Root<B>;
    type Error = Error<B::Error>;
}

impl<B, H> protocol::Reply<B> for Descending<B, S<S<H>>>
where
    B: Backend<Node<Z>: Leaf> + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, H>;

    fn reply(
        mut self,
        requests: impl Requests<B, S<S<H>>>,
    ) -> (BoxResponses<B, S<H>, Self::Error>, Self::Next) {
        let (responses, queries, upper, lower) = self.work.internal_level::<H>(
            self.their_version.clone(),
            self.ledger.clone(),
            self.early_survivors.take(),
            self.early_supplies.take(),
            requests,
            self.queries,
        );
        let returns = self.work.assemble(<S<S<H>>>::HEIGHT, self.returns, upper);
        let returns = self.work.assemble(<S<H>>::HEIGHT, returns, lower);

        (
            responses,
            Descending {
                their_version: self.their_version,
                ledger: self.ledger,
                queries,
                returns,
                early_survivors: None,
                early_supplies: None,
                work: self.work,
                finish: self.finish,
                height: std::marker::PhantomData,
            },
        )
    }
}

impl<B> protocol::Reply<B> for Descending<B, S<Z>>
where
    B: Backend<Node<Z>: Leaf> + Sync,
{
    type Next = Completing<B>;

    fn reply(
        mut self,
        requests: impl Requests<B, S<Z>>,
    ) -> (BoxResponses<B, Z, Self::Error>, Self::Next) {
        debug_assert!(
            self.early_survivors.is_none() && self.early_supplies.is_none(),
            "the opening hand-off is consumed by the first descending stage"
        );
        let (responses, queries, upper, lower) = self.work.leaf_parent_level(
            self.their_version.clone(),
            self.ledger.clone(),
            requests,
            self.queries,
        );
        let returns = self.work.assemble(<S<Z>>::HEIGHT, self.returns, upper);
        let returns = self.work.assemble(Z::HEIGHT, returns, lower);

        (
            responses,
            Completing {
                their_version: self.their_version,
                ledger: self.ledger,
                queries,
                returns,
                work: self.work,
                finish: self.finish,
            },
        )
    }
}

impl<B> protocol::CompleteResponder<B> for Descending<B, Z>
where
    B: Backend<Node<Z>: Leaf>,
{
    fn complete_responder(
        mut self,
        requests: impl Requests<B, Z>,
    ) -> (
        BoxResponses<B, Z, Self::Error>,
        impl Future<Output = Result<Root<B>, Self::Error>> + Send,
    ) {
        let (responses, resolutions) =
            self.work
                .leaf_level(self.their_version, self.ledger, requests, self.queries);
        self.work.assemble_leaves(self.returns, resolutions);
        (responses, self.work.execute(self.finish))
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Protocol for Completing<B> {
    type Height = Z;
    type Output = Root<B>;
    type Error = Error<B::Error>;
}

impl<B> protocol::CompleteInitiator<B> for Completing<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<B, Z>,
    ) -> Result<Root<B>, Self::Error> {
        let stats = self.work.stats();
        let mut absorb = pin!(absorb::<B>(
            self.their_version,
            self.ledger,
            requests.map(erased::erase_reply::<B, Z>),
            self.queries,
            self.returns,
            stats,
        ));
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
///
/// Each absorbed leaf is content this replica just learned, credited as
/// [`messages_gained`](crate::SessionStats::messages_gained) exactly like
/// the resolver's supply arm.
async fn absorb<B>(
    their_version: Version,
    ledger: SupplyLedger,
    requests: impl futures::Stream<Item = Reply<B::Erased>> + Send,
    mut queries: Receiver<Prefix<Z>>,
    returns: Sender<Option<B::Erased>>,
    stats: Recorder,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut requests = pin!(requests);
    while let Some(prefix) = queries.recv().await {
        let Some(Reply { replies }) = requests.next().await else {
            return violation(Violation::UnansweredQuery);
        };

        // The last radix of the prefix is the one we expect to be supplied.
        let (_, expected) = prefix.pop();

        // Only if we received exactly that radix paired with a leaf whose
        // version the sender's declared version contains, do we absorb it.
        let supply = match replies.as_slice() {
            [] => None,
            [Reaction::Supply(radix, leaf)] if *radix == expected => {
                if !contained(leaf.span().hi(), &their_version) {
                    return violation(Violation::UncontainedSupply);
                }
                ledger.absorb(leaf.len() as u64)?;
                stats.gained(1);
                Some(leaf.clone())
            }
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
