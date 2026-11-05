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
//! - **replies** are the incoming stream: exactly one reply per query, in
//!   chunks the stage acts on as they arrive, the last marked;
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
//! indifferent — but query-first frees the stage to take the next query as
//! soon as the reply arrives.
//!
//! The first progress-critical ordering invariant is **wire before internal
//! publication**. The walk yields every outgoing query or reply before
//! enqueuing or recording its in-process twin. Backpressure on internal state
//! therefore cannot withhold the wire action that lets the counterparty
//! advance.
//!
//! An answer leaves in chunks, each ending at one question, and each
//! question is built and sent down only after the chunk carrying it, so the
//! peer holds every question this side waits on.
//!
//! The second is **resolution before dependent work**. For every disputed
//! child, the walk publishes the [`Resolution`] containing its
//! [`Resolve::Pending`] slots, right after the answer's first chunk, before
//! it sends the child queries whose returns fill those slots. The responder
//! does the same at the root. Before a parent resolution is published, all
//! of the descendant work needed to fulfill it has already been launched.
//! Thus a blocked query sender has made its resolution available, while a
//! blocked resolution sender is behind an older resolution whose dependent
//! work is already in flight.
//!
//! This makes one slot *sufficient* for every query and resolution channel:
//! the liveness floor. Those channels are unbounded, since the ledger already
//! charges what they hold. A blocked response pump has likewise already
//! published the response which releases it; the initiator's root query and
//! return and the responder's root resolution each occur exactly once; leaf
//! resolutions contain no `Pending` slots and can be assembled immediately.
//!
//! What can hold the walk is admission: before it builds an answer's own
//! items, and before each question, a stage waits until the endpoint's ledger
//! admits the work against its budget. Each depth always has a floor slot
//! for one unit of each kind, which frees once deeper work completes, so
//! every admission is granted; the ledger's
//! [`admission`](super::ledger) module gives the induction.
//!
//! This argument assumes each edge is *independent*: a full edge stalls only
//! its own producer, never delivery on another edge. Within one process that
//! holds by construction because every edge has its own channel. Over a wire,
//! every level's replies share one connection, so the premise holds only
//! because the receiving side reads every frame as it arrives and never waits
//! on the walk to do so; the remote proxy's `pipe` module lists every wait on
//! that receive path and why each ends without the walk. A reader that
//! instead waited for each frame's stage to accept it could create a wait
//! cycle between stages even when each level works in isolation.
//!
//! [`Work::assemble`]'s inter-level return queue is the one exception. A reply
//! can launch a full fan of child scopes before their parent resolution is
//! available to the assembler. That queue therefore holds one full fan;
//! [`Work::assemble`] points to the capacity argument at the queue
//! constructor.
//!
//! # Memory model
//!
//! At most one backend query per prefix: whoever explodes a node carries
//! the fan to every consumer that needs it (queries carry their children;
//! pruning returns the survivors it built), so [`Backend::children`] — which
//! may be a database read — is never repeated. An answer explores each
//! disputed child only when the chunk asking about it is reached, so its
//! built items are one question's at a time. Everything the walk builds is
//! charged on the endpoint's ledger out of the grant that admitted it, and
//! every question carries the room set aside for the peer's reply; on the
//! wire, a decoded reply waits for its stage chunk by chunk, within that
//! room. The ledger holds at most the endpoint's budget plus one grant per
//! floor slot ([`ledger`](super::ledger)).

use std::marker::PhantomData;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::counter::widen;
use crate::message::PayloadDepthLimit;
use crate::tree::{
    mirror::streaming::{
        Backend, ErasedNode, Leaf, Node as _, Root,
        erased::{self, Reaction, Reply},
        ledger::{Charge, DEFAULT_SYNC_MEMORY_BUDGET, Ledger, ReplyBound},
        message::{Grandchildren, Greeting, Listed},
        protocol::{self, BoxResponses, Requests},
        stats::Recorder,
    },
    typed::{
        ErasedPrefix, Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};
use before::Version;
use futures::{Stream, StreamExt, future::BoxFuture};

mod error;
pub(super) mod progress;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(super) mod transcript;
pub(super) mod unknown;
mod work;
use super::channel::{Receiver, Sender};
pub(crate) use work::floor_allowance;
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use work::scratch;
use work::{LeafRequest, Resolver, Work};

pub use error::{Error, Violation};

/// Construct a typed protocol-violation result.
fn violation<T, E>(violation: Violation) -> Result<T, Error<E>> {
    Err(Error::Violation(violation))
}

/// The budget a session's ledger admits work against when the session may
/// hold `budget` in all.
///
/// That is `budget` less the floor allowance of a walk over `B`'s nodes
/// ([`floor_allowance`]), or nothing beyond the floor slots if `budget` is
/// no more than that allowance.
fn admission_budget<B: Backend<Node<Z>: Leaf>>(budget: usize) -> usize {
    budget.saturating_sub(floor_allowance::<B::Erased, B::Error>())
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
    /// The bound on the peer's reply, from the grandchild count the peer
    /// gave the node.
    pub(crate) bound: ReplyBound,
    /// What the question holds: its queue slot and its children. It moves
    /// into the parent resolution the reply becomes, which keeps those
    /// children, and returns at its assembly.
    pub(crate) charge: Charge,
    /// The node copies listing its children made, which the parent
    /// resolution keeps as well.
    pub(crate) copies: Charge,
    /// The room set aside for what the peer's reply becomes ([`ledger`]).
    ///
    /// [`ledger`]: super::ledger
    pub(crate) reservation: Reserved,
}

/// The room a question sets aside for the peer's reply, in one charge per
/// place the reply's items take, so each place's charges cover exactly what
/// that place holds.
#[derive(Default)]
pub(crate) struct Reserved {
    /// The reply's chunks while they park and their stage reads them.
    #[expect(dead_code, reason = "held for its bytes, which return when it drops")]
    pub(crate) parked: Charge,
    /// The records the take derives from the reply's questions.
    #[expect(dead_code, reason = "held for its bytes, which return when it drops")]
    pub(crate) records: Charge,
    /// The parent resolution's slots, which the resolution keeps until its
    /// assembly.
    pub(crate) slots: Charge,
}

/// One scope's resolution: its children in radix order, each resolved
/// locally or pending on the stages beneath.
pub struct Resolution<E> {
    /// The prefix at which the resolved node will sit.
    pub(crate) prefix: ErasedPrefix,
    /// The possibly-resolved children of the node.
    pub(crate) resolved: Vec<(u8, Resolve<E>)>,
    /// What the resolution holds: its queue slot and its slots.
    pub(crate) charge: Charge,
    /// What its slots keep until assembly beyond their handles: the
    /// children that exploding the node made.
    pub(crate) kept: Charge,
    /// The node copies that exploding the node made, which its slots keep
    /// until assembly.
    pub(crate) copies: Charge,
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
    /// This side's supply-run byte target, carried by the greeting; the
    /// session runs at the minimum of the two ends' targets.
    target_message_size: u64,
    /// The session's stats recorder; the walk writes through it, and the
    /// session driver snapshots it after completion.
    stats: Recorder,
    /// The endpoint's ledger, which admits and charges the walk's work
    /// against the endpoint's budget.
    ledger: Ledger,
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
    /// The bytes exploding the root to build the fan copied.
    copies: usize,
}

/// A stage that has exchanged greetings and can proceed with reconciliation.
///
/// Like [`Connecting`], retains the greeting-time root fan for the descent.
pub struct Connected<B: Backend<Node<Z>: Leaf>> {
    /// The remote tree properties consumed when the descent opens.
    peer: PeerSummary,
    /// The root fan, erased at greeting time ([`Connecting`]).
    fan: Vec<(u8, B::Erased)>,
    /// The bytes exploding the root to build the fan copied.
    copies: usize,
}

/// The remote tree properties retained from its greeting.
struct PeerSummary {
    /// The peer's causal version, used to bound supplies and join ceilings.
    version: Version,
    /// The peer's declared message count, which bounds what it can supply.
    messages: u64,
    /// The peer's grandchild count of its root, which bounds its reply to
    /// the opening question.
    grandchildren: Grandchildren,
}

impl PeerSummary {
    /// Retain the greeting fields used after the driver elects roles.
    fn from_greeting(greeting: Greeting) -> Self {
        Self {
            version: greeting.version,
            messages: greeting.set_len,
            // An empty tree counts none; one reaction more costs a
            // conforming reply nothing.
            grandchildren: usize::try_from(greeting.grandchildren)
                .ok()
                .and_then(Grandchildren::new)
                .unwrap_or(Grandchildren::ONE),
        }
    }
}

/// Role-independent state consumed when either descent opening is built.
struct Opening<B: Backend<Node<Z>: Leaf>> {
    /// The peer's causal version, retained through the descent.
    peer_version: Version,
    /// The session-total allowance for supplies received from the peer.
    ledger: SupplyLedger,
    /// The joined ceiling assigned to reconstructed nodes.
    ceiling: Version,
    /// The local root fan computed for the greeting.
    fan: Vec<(u8, B::Erased)>,
    /// The bytes exploding the root to build the fan copied.
    copies: usize,
    /// The peer's grandchild count of its root.
    peer_grandchildren: Grandchildren,
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
    queries: Receiver<LeafRequest>,
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
            target_message_size,
            stats: Recorder::default(),
            ledger: Ledger::new(admission_budget::<B>(DEFAULT_SYNC_MEMORY_BUDGET)),
        }
    }

    /// Select the most this session may hold, `budget`, as
    /// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) sets
    /// it.
    ///
    /// Its ledger admits work against `budget` less the floor allowance
    /// ([`floor_allowance`]), so that what it holds outside its floor slots
    /// and what they hold come to at most `budget` together.
    ///
    /// A budget at or below the floor allowance admits work through the
    /// floor slots alone, as a budget of zero does ([`budget`](Self::budget)),
    /// and holds at most the floor allowance.
    pub fn sync_memory_budget(self, budget: usize) -> Self {
        self.budget(admission_budget::<B>(budget))
    }

    /// Select the budget this session's ledger admits its work against
    /// ([`ledger`](super::ledger)): the most it holds outside its floor
    /// slots, which hold the floor allowance at most beside it.
    pub fn budget(mut self, budget: usize) -> Self {
        self.ledger = Ledger::new(budget);
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
///
/// Returns the fan with the bytes the explosion copied
/// ([`ErasedNode::explosion_bytes`]).
pub(crate) async fn greeting_fan<B: Backend<Node<Z>: Leaf>>(
    backend: &B,
    root: Option<B::Node<height::Root>>,
) -> Result<(Vec<(u8, B::Erased)>, usize), B::Error> {
    match root {
        Some(node) => {
            let node = B::erase(node);
            let copies = node.explosion_bytes();
            let fan = erased::ops::children_of(backend, Prefix::new().erase(), node).await?;
            Ok((fan, copies))
        }
        None => Ok((Vec::new(), 0)),
    }
}

/// Derive the wire listing for a materialized fan.
///
/// In particular, the greeting and the initiator's opening question
/// ([`Work::initiator_level`]) are paired positionally by the remote proxy.
/// Deriving every listing here keeps those representations identical.
pub(crate) fn fan_listing<E: ErasedNode>(fan: &[(u8, E)]) -> Vec<Listed> {
    fan.iter()
        .map(|(radix, node)| Listed {
            radix: *radix,
            hash: node.hash(),
            grandchildren: Grandchildren::of(node),
        })
        .collect()
}

impl<B: Backend<Node<Z>: Leaf>> protocol::Connect<B> for Handshaking<B, Start> {
    type Next = Handshaking<B, Connecting<B>>;

    /// Build and send this participant's greeting.
    async fn connect(self) -> Result<(Greeting, Self::Next), Self::Error> {
        let (fan, copies) = greeting_fan(&self.backend, self.root.root.clone())
            .await
            .map_err(Error::Backend)?;
        let greeting = Greeting {
            version: self.root.ceiling.clone(),
            // The greeting's sizes come from the root's own aggregates,
            // so they cannot drift from the tree they describe.
            set_len: self.root.len(),
            max_version_bytes: self.root.max_version_bytes(),
            grandchildren: self
                .root
                .root
                .as_ref()
                .map_or(0, |root| widen(root.grandchildren())),
            // The walk is not the wire: on a wire session the proxy
            // stamps its codec's configured limit over this field at
            // send, so an in-process participant carries the default.
            payload_depth_limit: PayloadDepthLimit::default().get(),
            target_message_size: self.target_message_size,
            listing: fan_listing(&fan),
        };
        let next = Handshaking {
            backend: self.backend,
            state: Connecting { fan, copies },
            root: self.root,
            target_message_size: self.target_message_size,
            stats: self.stats,
            ledger: self.ledger,
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
                copies: self.state.copies,
            },
            root: self.root,
            target_message_size: self.target_message_size,
            stats: self.stats,
            ledger: self.ledger,
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
    /// Gather the state either descent opening consumes.
    fn open(self) -> Opening<B> {
        let Connected {
            peer, fan, copies, ..
        } = self.state;
        let ceiling = self.root.ceiling | &peer.version;
        let ledger = SupplyLedger::new(peer.messages);
        let work = Work::new(self.backend, self.stats, self.ledger);
        Opening {
            peer_version: peer.version,
            ledger,
            ceiling,
            fan,
            copies,
            peer_grandchildren: peer.grandchildren,
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
            ledger,
            ceiling,
            fan,
            copies,
            peer_grandchildren,
            mut work,
        } = self.open();
        let (responses, queries, returns, finish) =
            work.initiator_level(ceiling, fan, copies, peer_grandchildren);

        (
            responses,
            Descending {
                their_version: peer_version,
                ledger,
                queries,
                returns,
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
            ledger,
            ceiling,
            fan,
            copies,
            peer_grandchildren: _,
            mut work,
        } = self.open();
        let (responses, queries, returns, finish) =
            work.responder_level(peer_version.clone(), ceiling, fan, copies, requests);

        (
            responses,
            Descending {
                their_version: peer_version,
                ledger,
                queries,
                returns,
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

/// The initiator's terminal loop: pair each pending leaf request with the
/// chunks of its final [`Reply`] and pass its provision up, prefix-less,
/// like every return.
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
    mut queries: Receiver<LeafRequest>,
    mut returns: Sender<Option<B::Erased>>,
    stats: Recorder,
) -> Result<(), Error<B::Error>>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut requests = pin!(requests);
    while let Some((prefix, request, reservation)) = queries.recv().await {
        // The request's queue slot is free once dequeued.
        drop(request);
        let Some(mut chunk) = requests.next().await else {
            return violation(Violation::UnansweredQuery);
        };

        // The request names one leaf: its scope is the parent, and the
        // radix it asks for is the path's last byte.
        let (scope, expected) = prefix.pop();
        let request = Query {
            prefix: scope.erase(),
            ours: Vec::new(),
            bound: ReplyBound::LEAF,
            charge: Charge::default(),
            copies: Charge::default(),
            reservation,
        };
        let mut resolver = Resolver::<B>::new(request, &their_version, &ledger, &stats);
        loop {
            let Reply {
                reactions,
                charge: received,
                last,
                record,
                copies,
            } = chunk;
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
            // The chunk is used up, so its charges return now, before the
            // loop waits for the next one: the peer's admission may wait on
            // them.
            drop((received, record, copies));
            if last {
                break;
            }
            let Some(next) = requests.next().await else {
                return violation(Violation::UnansweredQuery);
            };
            chunk = next;
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
