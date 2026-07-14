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
//! This makes one slot sufficient for every query and resolution channel. A
//! blocked response pump has likewise already published the response which
//! releases it; the initiator's root query and return and the responder's root
//! resolution each occur exactly once; leaf resolutions contain no `Pending`
//! slots and can be assembled immediately.
//!
//! [`Work::assemble`]'s inter-level return queue is the one exception. A reply
//! can dispute a full fan of children. While the walk is still examining those
//! reactions and constructing their parent resolution, already-launched lower
//! scopes can all finish, but the parent resolution containing their `Pending`
//! slots cannot be published until the reaction loop ends. The completed fan
//! must therefore fit. Once that resolution is published, active assembly
//! drains the boundary, so capacity need not grow with width or depth.
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
        Backend, Leaf, Root,
        materialized::{unknown::Unknown, work::Work},
        message::{Handshake, Reaction, Reply},
        protocol::{self, BoxResponses, Requests, Responses},
    },
    typed::{
        Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};
use before::Version;
use futures::{StreamExt, future::BoxFuture};

/// Send a channel item or return when its consumer has been dropped.
macro_rules! send_or_return {
    ($sender:expr, $value:expr) => {
        if $sender.send($value).await.is_err() {
            return;
        }
    };
    ($sender:expr, $value:expr => $result:expr) => {
        if $sender.send($value).await.is_err() {
            return $result;
        }
    };
}

/// Construct a protocol-violation result for return or try-stream propagation.
macro_rules! violation {
    ($violation:ident) => {
        core::result::Result::Err(
            crate::tree::mirror::streaming::materialized::Error::Violation(
                crate::tree::mirror::streaming::materialized::Violation::$violation,
            ),
        )
    };
}

/// Await an optional next item, parking forever if its producer is gone.
macro_rules! next_or_pending {
    ($next:expr) => {
        match $next.await {
            Some(item) => item,
            None => std::future::pending().await,
        }
    };
}

pub(super) mod channel;
mod common;
mod error;
pub(super) mod unknown;
mod work;
use channel::{Receiver, Sender};
use common::*;

pub use error::{Error, Violation};

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
}

/// The version state of a stage that has been opened but has not yet sent its
/// handshake.
pub struct Start {
    our_version: Version,
}

/// The version state of a stage that has sent its version but not yet received
/// the peer's.
pub struct Connecting {
    our_version: Version,
}

/// The version state of a stage that has exchanged versions with its peer and
/// can proceed with reconciliation.
pub struct Connected {
    our_version: Version,
    their_version: Version,
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
        }
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, V: Send> protocol::Protocol
    for Handshaking<B, T, V>
{
    type Height = height::Root;
    type Output = Root<B, T>;
    type Error = Error<B::Error>;
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::Connect<B, T>
    for Handshaking<B, T, Start>
{
    type Next = Handshaking<B, T, Connecting>;

    async fn connect(self) -> Result<(Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let handshake = Handshake {
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connecting { our_version },
            root: self.root,
        };
        Ok((handshake, next))
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::CompleteConnect<B, T>
    for Handshaking<B, T, Connecting>
{
    type Next = Handshaking<B, T, Connected>;

    async fn complete_connect(self, their_version: Version) -> Result<Self::Next, Self::Error> {
        Ok(Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version: self.versions.our_version,
                their_version,
            },
            root: self.root,
        })
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> protocol::Accept<B, T>
    for Handshaking<B, T, Start>
{
    type Next = Handshaking<B, T, Connected>;

    async fn accept(self, request: Handshake) -> Result<(Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let handshake = Handshake {
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version,
                their_version: request.version,
            },
            root: self.root,
        };
        Ok((handshake, next))
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>> + Sync, T: Send + Sync + 'static> protocol::Initiator<B, T>
    for Handshaking<B, T, Connected>
{
    type Next = Descending<B, T, UnderRoot>;

    fn initiator(self) -> (impl Responses<B, T, UnderRoot, Self::Error>, Self::Next) {
        let their_version = self.versions.their_version;
        let ceiling = self.versions.our_version | &their_version;

        let mut work = Work::new(self.backend);
        let (responses, queries, returns, finish) = work.initiator_level(ceiling, self.root);

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
    for Handshaking<B, T, Connected>
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn responder(
        self,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (BoxResponses<B, T, UnderRoot, Self::Error>, Self::Next) {
        let their_version = self.versions.their_version;
        let ceiling = self.versions.our_version | &their_version;

        let mut work = Work::new(self.backend);
        let (responses, queries, returns, finish) =
            work.responder_level(their_version.clone(), ceiling, self.root, requests);

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

/// The initiator's terminal loop: pair each final [`Reply`] with the next
/// pending leaf request and pass its provision up, prefix-less, like every
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
    while let Some(Reply { replies }) = requests.next().await {
        let Some(prefix) = queries.recv().await else {
            return violation!(UnaskedReply);
        };

        // The last radix of the prefix is the one we expect should be supplied.
        let (_, expected) = prefix.pop();

        // Only if we received exactly that radix paired with a leaf, do we absorb it.
        let supply = match replies.as_slice() {
            [] => None,
            [Reaction::Supply(radix, leaf)] if *radix == expected => Some(leaf.clone()),
            [Reaction::Supply(_, _)] => return violation!(InvalidSupply),
            _ => return violation!(UnfinishedReply),
        };

        // Then we send that (optional) leaf upwards.
        send_or_return!(returns, supply => Ok(()));
    }

    // If there are more queries, something is wrong: we should have exhausted
    // all our queries in processing all the replies.
    if queries.recv().await.is_some() {
        return violation!(UnansweredQuery);
    }

    Ok(())
}
