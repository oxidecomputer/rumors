//! The streaming mirror session: the protocol's stage schedule, implemented
//! once for every [`Backend`].
//!
//! A stage is one node in the protocol's descending schedule. Unlike the
//! alternating backend — which materializes a `Levels` zipper and pushes two
//! levels per round — the streaming stages thread lazy boxed node streams: our
//! frontier subtrees flow downward to be disassembled, and reconciled nodes
//! flow upward to be reassembled, a hylomorphism in strict prefix order.
//!
//! Every dataflow edge is a [`FAN`]-bounded channel. Each descent stage
//! contributes two futures which must be driven concurrently: its walk (the
//! frontier against the incoming wire, fanning out to the wire/down/keep/level
//! channels) and its ascent (folding the reconciled levels back toward the
//! root). The accumulated `work` is driven concurrently at the session's two
//! terminals ([`complete_initiator`](protocol::CompleteInitiator), and the
//! future returned by [`complete_responder`](protocol::CompleteResponder)),
//! each of which resolves to its side's reconciled [`Root`].

use async_stream::try_stream;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::future::{self, BoxFuture};
use futures::stream::{self, StreamExt};
use std::pin::pin;

use crate::{
    Version,
    tree::typed::{
        Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

use super::backend::{Backend, BoxNodeStream, Leaf, Node, NodeStream, Root, one};
use super::merge::merge_disjoint;
use super::message;
use super::protocol::{self, Messages};
use super::unknown::Unknown;

mod reconcile;

/// The bound on every internal channel: one node's child fan (the radix).
///
/// The walk's producers and consumers advance in lockstep per parent — the
/// merge-join holds one item of lookahead per input, and a parent contributes
/// at most one fan of children before the walk must pull its inputs again —
/// so a single fan of slack absorbs the maximum skew between the wire, the
/// descending frontier, and the upward reassembly. This bound is what makes
/// reconciliation fixed-memory regardless of diff size.
pub(super) const FAN: usize = 256;

/// A prefix-keyed node at height `H`: the item of every level-carrying
/// channel between a stage's walk and the ascents that fold it back up.
type Level<B, T, H> = (Prefix<H>, <B as Backend<T>>::Node<H>);

/// Open a stage's outgoing wire: push the future that forwards `messages`
/// into a fresh [`FAN`]-bounded channel onto `work`, and return the
/// receiving half.
///
/// The receiving half is what the stage returns as its outgoing [`Messages`]:
/// the counterparty reads a plain channel while the forwarding future — not
/// the counterparty's demand — advances the walk behind it. Forwarding ends
/// when the wire ends or the counterparty drops the receiver (session
/// teardown), and never fails: errors ride the forwarded items.
fn outgoing<M, E, X>(
    work: &mut Vec<BoxFuture<'static, Result<(), X>>>,
    messages: impl Messages<M, E> + 'static,
) -> mpsc::Receiver<Result<M, E>>
where
    M: Send + 'static,
    E: Send + 'static,
    X: Send + 'static,
{
    let (tx, rx) = mpsc::channel(FAN);
    work.push(Box::pin(async move {
        let _ = messages.map(Ok).forward(tx).await;
        Ok(())
    }));
    rx
}

/// Reassemble one stage's slice of the reconciled tree.
///
/// A stage with its frontier at `S<S<H>>` sees reconciled nodes at three
/// heights: kept frontier nodes (`keep`), classify verdicts one level below
/// (`level`), and — once the stages beneath have resolved them — reconciled
/// disputes two levels below, arriving through `below`. The three sets are
/// prefix-disjoint because they route through mutually exclusive verdicts, so
/// the stage's reconciled frontier level is two folds and two disjoint merges;
/// it flows out through `up`, becoming the previous stage's `below`.
fn ascend<B, T, H>(
    backend: B,
    keep: mpsc::Receiver<Level<B, T, S<S<H>>>>,
    level: mpsc::Receiver<Level<B, T, S<H>>>,
    below: mpsc::Receiver<Level<B, T, H>>,
    mut up: mpsc::Sender<Level<B, T, S<S<H>>>>,
) -> BoxFuture<'static, Result<(), B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    let reconciled = merge_disjoint(
        keep.map(Ok::<_, B::Error>),
        backend.clone().parents(merge_disjoint(
            level.map(Ok),
            backend.parents(below.map(Ok)),
        )),
    );
    Box::pin(async move {
        let mut reconciled = pin!(reconciled);
        while let Some(item) = reconciled.next().await {
            // A closed channel means the consumer of this level is gone and the
            // session is being torn down.
            if up.send(item?).await.is_err() {
                break;
            }
        }
        Ok(())
    })
}

/// Drain the reconciled root level into this side's reconciled [`Root`]: the
/// future the session's terminal resolves to.
///
/// The root level holds at most one node: the reassembled root, or nothing
/// when the reconciled tree is empty. A backend error means there is no
/// trustworthy result.
fn finish<B, T>(
    top: impl NodeStream<B, T, height::Root> + 'static,
    ceiling: Version,
) -> BoxFuture<'static, Result<Root<B, T>, B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    Box::pin(async move {
        let mut top = pin!(top);
        let mut root = None;
        while let Some(item) = top.next().await {
            debug_assert!(
                root.is_none(),
                "upward reassembly produced more than one root node",
            );
            let (_prefix, node) = item?;
            root = Some(node);
        }
        Ok(Root { ceiling, root })
    })
}

/// The version state of a stage that has been opened but has not yet sent its
/// handshake.
///
/// Carries what the [`connect`](protocol::Connect::connect) /
/// [`accept`](protocol::Accept::accept) step folds into its outgoing
/// [`message::Handshake`]: our latest [`Version`], the tree's ceiling.
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

/// A mirror stage still at [`Root`](height::Root) height: the handshake
/// phases, before the tree has been disassembled into streams.
///
/// `V` is the version state ([`Start`] → [`Connecting`] → [`Connected`]). The
/// whole tree is held intact as `root` until reconciliation begins at
/// [`initiator`](protocol::Initiator::initiator) /
/// [`responder`](protocol::Responder::responder).
pub struct Handshaking<B, T, V>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    backend: B,
    versions: V,
    root: Root<B, T>,
}

impl<B, T> Handshaking<B, T, Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
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

impl<B, T, V> protocol::Stage for Handshaking<B, T, V>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    type Height = height::Root;
}

impl<B, T> protocol::Connect<B, T> for Handshaking<B, T, Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connecting>;

    async fn connect<E>(self) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<B::Error> + Send + 'static,
    {
        let Start { our_version } = self.versions;

        let handshake = message::Handshake {
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

impl<B, T> protocol::CompleteConnect<B, T> for Handshaking<B, T, Connecting>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connected>;

    async fn complete_connect<E>(self, their_version: Version) -> Result<Self::Next, E>
    where
        E: From<B::Error> + Send + 'static,
    {
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

impl<B, T> protocol::Accept<B, T> for Handshaking<B, T, Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connected>;

    async fn accept<E>(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<B::Error> + Send + 'static,
    {
        let Start { our_version } = self.versions;

        let handshake = message::Handshake {
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

impl<B, T> protocol::Initiator<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Self;

    fn initiator<E>(self) -> (impl Messages<message::Initiate, E> + 'static, Self::Next)
    where
        E: From<B::Error> + Send + 'static,
    {
        let initiate = self
            .root
            .root
            .as_ref()
            .map(|node| Ok(message::Initiate::Uncertain(node.hash())));
        (stream::iter(initiate), self)
    }
}

impl<B, T> protocol::Responder<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderRoot>;

    fn responder<E>(
        self,
        requests: impl Messages<message::Initiate, E> + 'static,
    ) -> (impl Messages<message::Opening, E> + 'static, Self::Next)
    where
        E: From<B::Error> + Send + 'static,
    {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        // Always explode our root one level and enumerate the resulting
        // children, regardless of the initiator's root hash: the root hashes
        // will always differ at this point, because they can only match when
        // the versions are the same (with well-behaved parties), and this
        // already would have short-circuited before we got here.
        let explode = backend.clone();
        let sendable = try_stream! {
            let mut down = down_tx;
            for await item in requests {
                // The initiate's content is not used (we explode
                // unconditionally), but its errors are ours to propagate.
                item?;
            }
            if let Some(node) = root.root {
                let mut children = pin!(explode.children(one(Prefix::new(), node)));
                let mut listing = Vec::new();
                while let Some(item) = children.next().await {
                    let (prefix, child) = item?;
                    let (_, radix) = prefix.pop();
                    listing.push((radix, child.hash()));
                    if down.send((prefix, child)).await.is_err() {
                        return;
                    }
                }
                yield message::Opening::Uncertain(listing);
            }
        };

        // The responder's reconciled root-child level arrives on `up` whole:
        // one fold reassembles the root the terminal resolves to.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = backend.clone().parents(up_rx.map(Ok::<_, B::Error>));
        let finish = finish(top, ceiling);
        let mut work = Vec::new();
        let sending = outgoing(&mut work, sendable);

        let next = Descending {
            backend,
            their_version: versions.their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: up_tx,
            work,
            finish,
        };
        (sending, next)
    }
}

impl<B, T> protocol::OpenInitiator<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn open_initiator<E>(
        self,
        requests: impl Messages<message::Opening, E> + 'static,
    ) -> (
        impl Messages<message::Exchanged<B, T, UnderRoot>, E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;
        let their_version = versions.their_version.clone();
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        // The opening round is the one asymmetric-root round: the responder
        // listed its root's children unconditionally, so silence about a child
        // means the responder *lacks* it (everywhere below Root, silence means
        // the hash matched). Feeding the whole opening level into one classify
        // realizes exactly that: children only we hold come out as `Provide`
        // (deletion-pruned), children only the responder holds as `Request`,
        // and an empty side degenerates to all-`Provide` or all-`Request` with
        // no special casing.
        let open = backend.clone();
        let sendable = try_stream! {
            let (down, level) = (down_tx, level_tx);

            // The responder's opening listing: at most one message, and an
            // empty responder sends none. The parent is statically the root.
            let mut theirs = Vec::new();
            for await item in requests {
                let message::Opening::Uncertain(children) = item?;
                theirs = children
                    .into_iter()
                    .map(|(radix, hash)| (Prefix::new().push(radix), hash))
                    .collect();
            }

            match root.root {
                Some(node) => {
                    let ours = open.clone().children(one(Prefix::new(), node));
                    let theirs = stream::iter(theirs.into_iter().map(Ok));
                    let routed = reconcile::route::<B, T, UnderUnderRoot, E>(
                        open,
                        their_version.clone(),
                        ours,
                        theirs,
                        down,
                        level,
                    );
                    for await item in routed {
                        yield item?;
                    }
                }
                None => {
                    // We are empty: request everything the responder listed.
                    for (prefix, _hash) in theirs {
                        yield (prefix, message::Exchange::Requested);
                    }
                }
            }
        };

        // The initiator's root-child level is this round's classify verdicts
        // (`level`) joined with the resolved disputes climbing out of the
        // descent (`up`); two folds reassemble the root the terminal
        // resolves to.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = backend.clone().parents(merge_disjoint(
            level_rx.map(Ok::<_, B::Error>),
            backend.clone().parents(up_rx.map(Ok)),
        ));
        let finish = finish(top, ceiling);
        let mut work = Vec::new();
        let sending = outgoing(&mut work, sendable);

        let next = Descending {
            backend,
            their_version: versions.their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: up_tx,
            work,
            finish,
        };
        (sending, next)
    }
}

/// A mirror stage inside the descent: its frontier at height `H` flows
/// downward as a stream while the levels above reassemble concurrently.
///
/// `frontier` holds this side's disputed subtrees at `H`, fed by the previous
/// stage's classify through a [`FAN`]-bounded channel; `up` is where this
/// stage's [`ascend`] future sends the reconciled level at `H`, feeding the
/// previous stage's ascent in turn; `work` accumulates every concurrent
/// future the session has created so far, and `finish` is the fold their
/// reassembly converges to: the reconciled [`Root`] the terminal resolves
/// to. Each [`exchange`](protocol::Exchange::exchange) consumes the stage
/// and produces its successor two heights finer, until the terminal stages
/// drive the accumulated work to completion.
pub struct Descending<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    backend: B,
    their_version: Version,
    frontier: BoxNodeStream<B, T, H>,
    up: mpsc::Sender<Level<B, T, H>>,
    work: Vec<BoxFuture<'static, Result<(), B::Error>>>,
    finish: BoxFuture<'static, Result<Root<B, T>, B::Error>>,
}

impl<B, T, H> protocol::Stage for Descending<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    type Height = H;
}

impl<B, T, H> Descending<B, T, S<S<H>>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
{
    /// One step of the descent: start this stage's [`walk`](reconcile::walk)
    /// and [`ascend`] futures and construct the successor two heights finer.
    ///
    /// Shared by [`exchange`](protocol::Exchange::exchange) and
    /// [`close_initiator`](protocol::CloseInitiator::close_initiator).
    #[allow(clippy::type_complexity)]
    fn descend<E>(
        self,
        requests: impl Messages<message::Exchanged<B, T, S<S<H>>>, E> + 'static,
    ) -> (
        impl Messages<message::Exchanged<B, T, S<H>>, E> + 'static,
        Descending<B, T, H>,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend,
            their_version,
            frontier,
            up,
            mut work,
            finish,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (keep_tx, keep_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (below_tx, below_rx) = mpsc::channel(FAN);

        let walk = reconcile::walk(
            backend.clone(),
            their_version.clone(),
            frontier,
            requests,
            down_tx,
            keep_tx,
            level_tx,
        );
        work.push(ascend(backend.clone(), keep_rx, level_rx, below_rx, up));

        let next = Descending {
            backend,
            their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: below_tx,
            work,
            finish,
        };
        (walk, next)
    }
}

impl<B, T, H> protocol::Exchange<B, T> for Descending<B, T, S<S<S<H>>>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    // Discharged at each concrete height by one of the three `AfterExchange`
    // blanket impls; cannot be proven generically by the trait solver.
    Descending<B, T, S<H>>: protocol::AfterExchange<B, T, S<H>>,
{
    type Next = Descending<B, T, S<H>>;

    fn exchange<E>(
        self,
        requests: impl Messages<(Prefix<S<S<S<H>>>>, message::Exchange<B, T, S<S<S<H>>>>), E> + 'static,
    ) -> (
        impl Messages<message::Exchanged<B, T, S<S<H>>>, E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let (walk, mut next) = self.descend(requests);
        let sending = outgoing(&mut next.work, walk);
        (sending, next)
    }
}

impl<B, T> protocol::CloseInitiator<B, T> for Descending<B, T, S<S<Z>>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, Z>;

    fn close_initiator<E>(
        self,
        requests: impl Messages<message::Exchanged<B, T, S<S<Z>>>, E> + 'static,
    ) -> (
        impl Messages<(Prefix<S<Z>>, message::Closing<B, T>), E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let (walk, mut next) = self.descend(requests);
        let closing = walk.filter_map(|item| {
            future::ready(match item {
                Ok((prefix, message::Exchange::Providing(node))) => {
                    Some(Ok((prefix, message::Closing::Providing(node))))
                }
                Ok((prefix, message::Exchange::Requested)) => {
                    Some(Ok((prefix, message::Closing::Requested)))
                }
                // Vacuous at leaf height (see [`message::Closing`]) because
                // it's not possible to be uncertain about a leaf: you either
                // know you have it, or know you don't.
                Ok((_, message::Exchange::Uncertain(..))) => None,
                Err(error) => Some(Err(error)),
            })
        });
        let sending = outgoing(&mut next.work, closing);
        (sending, next)
    }
}

impl<B, T> protocol::CompleteResponder<B, T> for Descending<B, T, S<Z>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    fn complete_responder<E>(
        self,
        requests: impl Messages<(Prefix<S<Z>>, message::Closing<B, T>), E> + 'static,
    ) -> (
        impl Messages<(Prefix<Z>, message::Complete<B, T>), E> + 'static,
        impl Future<Output = Result<Root<B, T>, E>> + Send,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend,
            their_version,
            frontier,
            up,
            mut work,
            finish,
        } = self;

        let complete = reconcile::complete_walk(backend, their_version, frontier, requests, up);
        let sending = outgoing(&mut work, complete);
        (sending, async move {
            let (finished, root) = future::join(future::join_all(work), finish).await;
            // A failed worker explains anything odd about the fold's result,
            // so it outranks the fold.
            for result in finished {
                result?;
            }
            Ok(root?)
        })
    }
}

impl<B, T> protocol::CompleteInitiator<B, T> for Descending<B, T, Z>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    async fn complete_initiator<E>(
        self,
        requests: impl Messages<(Prefix<Z>, message::Complete<B, T>), E> + 'static,
    ) -> Result<Root<B, T>, E>
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend: _,
            their_version: _,
            frontier,
            up,
            work,
            finish,
        } = self;

        let absorb = reconcile::absorb_leaves(frontier, requests, up);
        let (absorbed, finished, root) =
            future::join3(absorb, future::join_all(work), finish).await;
        absorbed?;
        for result in finished {
            result?;
        }
        Ok(root?)
    }
}
