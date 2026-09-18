//! The streaming protocol implemented generically for every materialized
//! backend.
//!
//! Any [`Backend`] can be used here, with no further ceremony.
//!
//! # The session dataflow
//!
//! A *scope* is the subtree named by one query: its prefix and whatever both
//! sides hold beneath it. A *stage* reconciles scopes at one tree height.
//!
//! Each stage pairs pending [`Query`]s with the peer's replies, in order.
//! [`Work::assemble`] rebuilds the tree from the results. Three bounded streams
//! connect adjacent stages:
//!
//! - **queries** flow down: one queue item per question asked, in question
//!   order (message order, then radix order);
//! - **replies** are the incoming stream: exactly one message per query;
//! - **returns** flow up: exactly one `Option<Node>` per query, in query
//!   order — the reconciled scope, `None` meaning it resolved to nothing
//!   (recursive deletion, the same reading as [`Backend::parent`]'s `None`
//!   return). Returns are prefix-less: the consumer issued the query, so
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
//! This argument assumes each edge is *independent*: a full edge stalls only
//! its own producer, never delivery on another edge. Within one process that
//! holds by construction because every edge has its own channel. Over a wire it
//! is a premise the transport must supply, which is why the [link
//! contract](crate::link) demands independently flow-controlled streams.
//! Multiplexing every stream through one FIFO can create a wait cycle between
//! stages even when each stream works in isolation; the conformance suite
//! exercises this failure mode.
//!
//! [`Work::assemble`]'s inter-level return queue is the one exception. A reply
//! can launch a full fan of child scopes before their parent resolution is
//! available to the assembler. That queue therefore holds one full fan rather
//! than using the session window; [`Work::assemble`] points to the capacity
//! argument at the queue constructor.
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

use std::marker::PhantomData;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::message::PayloadDepthLimit;
use crate::tree::{
    mirror::streaming::{
        Backend, ErasedNode, Leaf, Root,
        erased::{self, Reaction, Reply},
        message::Greeting,
        protocol::{self, BoxResponses, Requests},
        stats::Recorder,
        window::{ReplicaSize, WindowConfig},
    },
    typed::{
        ErasedPrefix, Hash, Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};
use before::Version;
use futures::{Stream, StreamExt, future::BoxFuture};
use tokio::sync::oneshot;

mod error;
pub(super) mod progress;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(super) mod transcript;
pub(super) mod unknown;
mod work;
use super::channel::{Receiver, Sender};
use work::{Resolver, Work};

pub use error::{Error, Violation};

/// Construct a typed protocol-violation result.
fn violation<T, E>(violation: Violation) -> Result<T, Error<E>> {
    Err(Error::Violation(violation))
}

/// Tracks supplied leaves against the peer's declared set length.
///
/// The peer cannot supply more live leaves than the set length in its greeting:
/// disputed scopes partition the tree, and each leaf belongs to at most one
/// supplied subtree. All walk stages share this counter. The wire decoder has
/// a separate counter for the same declaration so it can enforce the bound
/// before handing payloads to the backend.
#[derive(Clone, Debug)]
pub(crate) struct SupplyLedger {
    /// The sender's greeting-declared set length.
    declared: u64,
    /// Live leaves absorbed so far, across every ingestion site sharing
    /// this ledger.
    absorbed: Arc<AtomicU64>,
}

impl SupplyLedger {
    /// Begin counting against the peer's declared live-message total.
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

/// One pending question, resolved by one remote reply.
///
/// Queries form the pairing queue between consecutive same-side stages and
/// mirror the scopes expected on the wire. `E` is the backend's erased node
/// representation ([`Backend::Erased`]); the prefix names the queried scope,
/// and its byte length is the scope's height witness (see [`erased`]). A query
/// pairs with the reply at its children's height, one level below the prefix.
///
/// If we issued a request for a node, `ours` is empty and we expect the
/// reply to consist entirely of supplied nodes.
pub struct Query<E> {
    /// The prefix at which the resolved node will sit.
    pub(crate) prefix: ErasedPrefix,
    /// Our children of the node (empty if we don't have it at all).
    pub(crate) ours: Vec<(u8, E)>,
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

/// A mirror stage still at [`Root`](height::Root) height: the handshake phases,
/// before the tree has been disassembled into streams.
///
/// `State` tracks the greeting exchange ([`Start`] → [`Connecting`] →
/// [`Connected`]). The whole tree remains intact until reconciliation begins at
/// [`initiator`](protocol::Initiator::initiator) /
/// [`responder`](protocol::Responder::responder). The session's outgoing
/// messages carry `backend`'s own node types, which are the ones its
/// counterparty reads.
pub struct Handshaking<B: Backend<Node<Z>: Leaf>, State> {
    /// The store used to inspect and rebuild this participant's tree.
    backend: B,
    /// State retained by the current handshake phase.
    state: State,
    /// The tree consumed by this session.
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

/// A stage that has been opened but has not yet sent its greeting.
pub struct Start;

/// A stage that has sent its greeting but not yet received the peer's.
///
/// Carries the root fan the greeting's listing was derived from, so the
/// descent reuses it instead of asking the backend for the root's children a
/// second time (the memory model's one-query-per-prefix rule).
pub struct Connecting<B: Backend<Node<Z>: Leaf>> {
    /// The root fan, already erased: everything downstream of the
    /// greeting — the descent's workers included — speaks the erased
    /// representation.
    fan: Vec<(u8, B::Erased)>,
}

/// A stage that has exchanged greetings and can proceed with reconciliation.
///
/// Like [`Connecting`], retains the greeting-time root fan for the descent.
pub struct Connected<B: Backend<Node<Z>: Leaf>> {
    /// The remote tree properties consumed when the descent opens.
    peer: PeerSummary,
    /// The root fan, erased at greeting time ([`Connecting`]).
    fan: Vec<(u8, B::Erased)>,
}

/// A one-shot result passed from the opening pump to the first internal stage.
///
/// The opening owns the producer. The first internal stage takes this receiver;
/// deeper stages have no opening result.
pub(crate) enum OpeningHandoff<E> {
    /// Root children the initiator supplied before the responder asked.
    Supplies(oneshot::Receiver<Vec<(u8, Vec<(u8, E)>)>>),
    /// Initiator-owned root children already pruned against the peer's version.
    Survivors(oneshot::Receiver<Vec<(u8, Option<E>)>>),
}

/// The remote tree properties retained from its greeting.
struct PeerSummary {
    /// The peer's causal version, used to bound supplies and join ceilings.
    version: Version,
    /// The peer's declared message count and maximum encoded version size.
    size: ReplicaSize,
    /// The peer's root-fan listing, consumed by the elected initiator.
    listing: Vec<(u8, Hash)>,
}

impl PeerSummary {
    /// Retain the greeting fields used after the driver elects roles.
    fn from_greeting(greeting: Greeting) -> Self {
        Self {
            version: greeting.version,
            size: ReplicaSize::new(greeting.set_len, greeting.max_version_bytes),
            listing: greeting.listing,
        }
    }
}

/// Role-independent state consumed when either descent opening is built.
struct Opening<B: Backend<Node<Z>: Leaf>> {
    /// The peer's causal version, retained through the descent.
    peer_version: Version,
    /// The peer's root-fan listing, used only by an elected initiator.
    peer_listing: Vec<(u8, Hash)>,
    /// The session-total allowance for supplies received from the peer.
    ledger: SupplyLedger,
    /// The joined ceiling assigned to reconstructed nodes.
    ceiling: Version,
    /// The local root fan computed for the greeting.
    fan: Vec<(u8, B::Erased)>,
    /// The work accumulator configured from both greetings.
    work: Work<B>,
}

/// A descent stage that pairs erased [`Reply`] messages with pending [`Query`]s.
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
    /// Opening-only state, consumed by the first descent stage.
    opening: Option<OpeningHandoff<B::Erased>>,
    /// The reassembly work accumulated so far; the terminals drive it to
    /// completion.
    work: Work<B>,
    /// Resolves to this side's reconciled root once the top return arrives.
    finish: BoxFuture<'static, Result<Root<B>, Error<B::Error>>>,
    /// The stage's type-level height after its payloads have been erased.
    ///
    /// The payloads above are erased; this tag is what the schedule's
    /// typestates keep proving about them (`PhantomData<fn() -> H>` for
    /// the auto-trait shortcut; see [`typed::Node`](crate::tree::typed::Node)).
    height: PhantomData<fn() -> H>,
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
    /// Begin a session and advertise `target_message_size` in its greeting.
    pub fn start(backend: B, root: Root<B>, target_message_size: u64) -> Self {
        Self {
            backend,
            state: Start,
            root,
            window: WindowConfig::default(),
            target_message_size,
            stats: Recorder::default(),
        }
    }

    /// Select this session's window choice; see [`window`](super::window).
    pub fn window(mut self, window: WindowConfig) -> Self {
        self.window = window;
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

/// A materialized participant remains at root height through the greeting exchange.
impl<B: Backend<Node<Z>: Leaf>, State: Send> protocol::Phase for Handshaking<B, State> {
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

/// Derive the wire listing for a materialized fan.
///
/// In particular, the greeting and the initiator's opening question
/// ([`Work::initiator_level`]) are paired positionally by the remote proxy.
/// Deriving every listing here keeps those representations identical.
pub(crate) fn fan_listing<E: ErasedNode>(fan: &[(u8, E)]) -> Vec<(u8, Hash)> {
    fan.iter()
        .map(|(radix, node)| (*radix, node.hash()))
        .collect()
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Connect<B> for Handshaking<B, Start> {
    type Next = Handshaking<B, Connecting<B>>;

    /// Build and send this participant's greeting.
    async fn connect(self) -> Result<(Greeting, Self::Next), Self::Error> {
        let fan = greeting_fan(&self.backend, self.root.root.clone())
            .await
            .map_err(Error::Backend)?;
        let greeting = Greeting {
            version: self.root.ceiling.clone(),
            // The greeting's sizes come from the root's own aggregates,
            // so they cannot drift from the tree they describe.
            set_len: self.root.len(),
            max_version_bytes: self.root.max_version_bytes(),
            // The walk is not the wire: on a wire session the proxy
            // stamps its codec's configured limit over this field at
            // send, so an in-process participant carries the default.
            payload_depth_limit: PayloadDepthLimit::default().get(),
            target_message_size: self.target_message_size,
            listing: fan_listing(&fan),
        };
        let next = Handshaking {
            backend: self.backend,
            state: Connecting { fan },
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

    /// Retain the peer's greeting fields needed after role election.
    async fn complete_connect(self, theirs: Greeting) -> Result<Self::Next, Self::Error> {
        Ok(Handshaking {
            backend: self.backend,
            state: Connected {
                peer: PeerSummary::from_greeting(theirs),
                fan: self.state.fan,
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

    /// Send our greeting and consume the peer greeting already received.
    async fn accept(self, request: Greeting) -> Result<(Greeting, Self::Next), Self::Error> {
        // The server sends the same greeting as the client path, then consumes
        // the already-received peer greeting to reach the shared connected
        // state. Composing the two transitions keeps greeting construction in
        // one place.
        let (greeting, connecting) = <Self as protocol::Connect<B>>::connect(self).await?;
        let next = protocol::CompleteConnect::complete_connect(connecting, request).await?;
        Ok((greeting, next))
    }
}

/// Prepare the role-independent work shared by both descent openings.
impl<B: Backend<Node<Z>: Leaf>> Handshaking<B, Connected<B>> {
    fn open(self) -> Opening<B> {
        let Connected { peer, fan } = self.state;
        let local_size = ReplicaSize::new(self.root.len(), self.root.max_version_bytes());
        let ceiling = self.root.ceiling | &peer.version;
        let window = self.window.resolve([local_size, peer.size], B::node_bytes);
        self.stats.window_granted(window.widest());
        let ledger = SupplyLedger::new(peer.size.messages());
        let work = Work::new(self.backend, window, self.stats);
        Opening {
            peer_version: peer.version,
            peer_listing: peer.listing,
            ledger,
            ceiling,
            fan,
            work,
        }
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::CompleteEqual<B> for Handshaking<B, Connected<B>> {
    /// Return the unchanged tree when both greetings describe equal versions.
    async fn complete_equal(self) -> Result<Root<B>, Self::Error> {
        Ok(self.root)
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Initiator<B> for Handshaking<B, Connected<B>> {
    type Next = Descending<B, UnderRoot>;

    /// Open the elected initiator's first descent stage.
    fn initiator(self) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let Opening {
            peer_version,
            peer_listing,
            ledger,
            ceiling,
            fan,
            mut work,
        } = self.open();
        let (responses, queries, returns, early, finish) =
            work.initiator_level(peer_version.clone(), ceiling, fan, peer_listing);

        (
            responses,
            Descending {
                their_version: peer_version,
                ledger,
                queries,
                returns,
                opening: Some(OpeningHandoff::Survivors(early)),
                work,
                finish,
                height: PhantomData,
            },
        )
    }
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Responder<B> for Handshaking<B, Connected<B>> {
    type Next = Descending<B, UnderUnderRoot>;

    /// Open the elected responder's first descent stage.
    fn responder(
        self,
        requests: impl Requests<B, UnderRoot>,
    ) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let Opening {
            peer_version,
            peer_listing: _,
            ledger,
            ceiling,
            fan,
            mut work,
        } = self.open();
        let (responses, queries, returns, early, finish) =
            work.responder_level(peer_version.clone(), ledger.clone(), ceiling, fan, requests);

        (
            responses,
            Descending {
                their_version: peer_version,
                ledger,
                queries,
                returns,
                opening: Some(OpeningHandoff::Supplies(early)),
                work,
                finish,
                height: PhantomData,
            },
        )
    }
}

/// A materialized descent phase processes the height carried by its state.
impl<B: Backend<Node<Z>: Leaf>, H: Height> protocol::Phase for Descending<B, H>
where
    S<H>: Height,
{
    type Height = H;
    type Output = Root<B>;
    type Error = Error<B::Error>;
}

impl<B, H> protocol::Reply<B> for Descending<B, S<S<H>>>
where
    B: Backend<Node<Z>: Leaf>,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, H>;

    /// Advance an internal descent by two height-indexed protocol phases.
    fn reply(
        mut self,
        requests: impl Requests<B, S<S<H>>>,
    ) -> (BoxResponses<B, S<H>, Self::Error>, Self::Next) {
        let (responses, queries, upper, lower) = self.work.internal_level::<H>(
            self.their_version.clone(),
            self.ledger.clone(),
            self.opening.take(),
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
                opening: None,
                work: self.work,
                finish: self.finish,
                height: PhantomData,
            },
        )
    }
}

impl<B> protocol::Reply<B> for Descending<B, S<Z>>
where
    B: Backend<Node<Z>: Leaf>,
{
    type Next = Completing<B>;

    /// Advance from leaf-parent reconciliation to the terminal leaf replies.
    fn reply(
        mut self,
        requests: impl Requests<B, S<Z>>,
    ) -> (BoxResponses<B, Z, Self::Error>, Self::Next) {
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
    /// Drive leaf reconciliation and return its response stream and result.
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

/// A completing materialized participant processes the final leaf-height replies.
impl<B: Backend<Node<Z>: Leaf>> protocol::Phase for Completing<B> {
    type Height = Z;
    type Output = Root<B>;
    type Error = Error<B::Error>;
}

impl<B> protocol::CompleteInitiator<B> for Completing<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Drain the final leaf replies and finish assembling the reconciled root.
    async fn complete_initiator(
        self,
        requests: impl Requests<B, Z>,
    ) -> Result<Root<B>, Self::Error> {
        let stats = self.work.stats();
        let absorb = absorb::<B>(
            self.their_version,
            self.ledger,
            requests.erase(),
            self.queries,
            self.returns,
            stats,
        );
        // Both operations must succeed. Either error cancels the other,
        // which may be waiting on a peer that cannot finish the session.
        // Poll replies first to report an available protocol violation before
        // an assembly error; fixed ordering also makes local tests replayable.
        futures::future::try_join(absorb, self.work.execute(self.finish))
            .await
            .map(|((), root)| root)
    }
}

/// The initiator's terminal loop: pair each pending leaf request with its
/// final [`Reply`] and pass its provision up, prefix-less, like every
/// return.
///
/// The reply is classified by the [`Resolver`] every descending stage
/// uses, over a request that holds nothing. One rule is the terminal
/// leg's own: a leaf request names a single radix where a scope request
/// names a subtree, so a supply at any other radix is out of order
/// ([`Violation::InvalidSupply`]). Each absorbed leaf is content this
/// replica just learned, credited as
/// [`messages_gained`](crate::SessionStats::messages_gained) by the
/// resolver's supply arm.
async fn absorb<B>(
    their_version: Version,
    ledger: SupplyLedger,
    requests: impl Stream<Item = Reply<B::Erased>> + Send,
    mut queries: Receiver<Prefix<Z>>,
    returns: Sender<Option<B::Erased>>,
    stats: Recorder,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut requests = pin!(requests);
    while let Some(prefix) = queries.recv().await {
        let Some(Reply { reactions }) = requests.next().await else {
            return violation(Violation::UnansweredQuery);
        };

        // The request names one leaf: its scope is the parent, and the
        // radix it asks for is the path's last byte.
        let (scope, expected) = prefix.pop();
        let request = Query {
            prefix: scope.erase(),
            ours: Vec::new(),
        };
        let mut resolver = Resolver::<B>::new(request, &their_version, &ledger, &stats);
        for reaction in reactions {
            if let Reaction::Supply(radix, _) = &reaction
                && *radix != expected
            {
                return violation(Violation::InvalidSupply);
            }
            let routed = resolver.react(reaction)?;
            debug_assert!(
                routed.is_none(),
                "a request that holds nothing routes no query"
            );
        }
        let Resolution { mut resolved, .. } = resolver.finish()?;
        let supply = match resolved.pop() {
            None => None,
            Some((_, Resolve::Ready(leaf))) => leaf,
            Some((_, Resolve::Pending)) => {
                unreachable!("a request that holds nothing routes no query, so no slot is pending")
            }
        };
        debug_assert!(
            resolved.is_empty(),
            "one radix is admitted, and the resolver rejects its duplicate"
        );

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
