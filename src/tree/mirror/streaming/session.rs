//! The streaming mirror session: the protocol's stage schedule, implemented
//! once for every [`Backend`].
//!
//! A stage is one node in the protocol's descending schedule. Unlike the
//! alternating backend — which materializes a `Levels` zipper and pushes two
//! levels per round — the streaming stages thread lazy boxed node streams:
//! our frontier subtrees flow downward to be disassembled, and reconciled
//! nodes flow upward to be reassembled, a hylomorphism in strict prefix
//! order.
//!
//! Every dataflow edge is a [`FAN`]-bounded channel and every worker is a
//! [`Pump`]. Each descent stage contributes two: its walk (the frontier
//! against the incoming wire, fanning out to the wire/down/keep/level
//! channels) and its ascent (folding the reconciled levels back toward the
//! root). The accumulated set is driven concurrently at the session's two
//! terminals — [`complete_initiator`](protocol::CompleteInitiator) on the
//! initiator, the drive future returned by
//! [`complete_responder`](protocol::CompleteResponder) on the responder — and
//! each side's reconciled [`Root`] is delivered through the oneshot handed
//! out by [`Handshaking::start`].

use async_stream::try_stream;
use futures::SinkExt;
use futures::channel::{mpsc, oneshot};
use futures::future::{self, BoxFuture};
use futures::stream::{self, StreamExt};
use std::pin::{Pin, pin};

use crate::{
    Network, Version,
    tree::typed::{
        Prefix,
        height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
    },
};

use super::backend::{Backend, Leaf, Node, NodeStream, Root};
use super::dispute::{Routed, classify};
use super::merge::merge_disjoint;
use super::message;
use super::protocol::{self, Messages};
use super::unknown::Unknown;

mod reconcile;
use reconcile::Out;

/// The bound on every internal channel: one node's child fan (the radix).
///
/// The walk's producers and consumers advance in lockstep per parent — the
/// merge-join holds one item of lookahead per input, and a parent contributes
/// at most one fan of children before the walk must pull its inputs again —
/// so a single fan of slack absorbs the maximum skew between the wire, the
/// descending frontier, and the upward reassembly. This bound is what makes
/// reconciliation fixed-memory regardless of diff size.
const FAN: usize = 256;

/// A boxed, prefix-ordered stream of a backend's nodes at height `H`.
///
/// Every stage boundary boxes: an `impl Stream` threaded through the 32-deep
/// descent would nest each stage's stream type inside the next and balloon
/// the compiler's types past any bound (the same reason
/// [`unknown`](super::unknown) boxes per height).
type BoxNodeStream<B, T, H> = Pin<Box<dyn NodeStream<B, T, H>>>;

/// A prefix-keyed node at height `H`: the item of every level-carrying
/// channel between pumps.
type Level<B, T, H> = (Prefix<H>, <B as Backend<T>>::Node<H>);

/// One independently scheduled worker of a session.
///
/// Every dataflow edge of the session is a [`FAN`]-bounded channel, and every
/// node of the dataflow graph is a pump: a boxed future that pulls its input
/// channels and pushes its output channels until they close. Stages
/// accumulate pumps as they descend (see [`Descending`]); the terminal stages
/// drive the whole set with `join_all`, concurrently with the counterparty's
/// set. A pump's only suspension points are channel operations, and the
/// channel graph is acyclic — wire and `down` edges flow toward later stages,
/// reconciled levels flow up a separate spine into [`deliver`] — so driving
/// every pump unconditionally means a pump parked on a full or empty channel
/// always has its counterpart still scheduled: the session cannot deadlock.
///
/// A pump resolves to `Err` only on a backend failure (`E` is the backend's
/// error); protocol errors ride the wire streams instead, as their items'
/// failure arm.
type Pump<E> = BoxFuture<'static, Result<(), E>>;

/// Box a stage's outgoing wire stream as the [`Pump`] that forwards it into
/// the stage's wire channel.
///
/// The receiving half is what the stage returns as its outgoing [`Messages`]:
/// the counterparty reads a plain channel while this pump — not the
/// counterparty's demand — advances the walk behind it. Ends when the wire
/// ends or the counterparty drops the receiver (session teardown). Never
/// fails: errors ride the forwarded items.
fn pump<M, E, X>(wire: impl Messages<M, E> + 'static, tx: mpsc::Sender<Result<M, E>>) -> Pump<X>
where
    M: Send + 'static,
    E: Send + 'static,
    X: Send + 'static,
{
    Box::pin(async move {
        let _ = wire.map(Ok).forward(tx).await;
        Ok(())
    })
}

/// The [`Pump`] that reassembles one stage's slice of the reconciled tree.
///
/// A stage with its frontier at `S<S<H>>` sees reconciled nodes at three
/// heights: kept frontier nodes (`keep`), classify verdicts one level below
/// (`level`), and — once the stages beneath have resolved them — reconciled
/// disputes two levels below, arriving through `below`. The three sets are
/// prefix-disjoint because they route through mutually exclusive verdicts, so
/// the stage's reconciled frontier level is two folds and two disjoint
/// merges; it flows out through `up`, becoming the previous stage's `below`.
fn ascend<B, T, H>(
    backend: B,
    keep: mpsc::Receiver<Level<B, T, S<S<H>>>>,
    level: mpsc::Receiver<Level<B, T, S<H>>>,
    below: mpsc::Receiver<Level<B, T, H>>,
    mut up: mpsc::Sender<Level<B, T, S<S<H>>>>,
) -> Pump<B::Error>
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
            |(prefix, _)| *prefix,
        )),
        |(prefix, _)| *prefix,
    );
    Box::pin(async move {
        let mut reconciled = pin!(reconciled);
        while let Some(item) = reconciled.next().await {
            // A closed channel means the consumer of this level is gone and
            // the session is being torn down.
            if up.send(item?).await.is_err() {
                break;
            }
        }
        Ok(())
    })
}

/// The topmost [`Pump`]: drain the reconciled root level into this side's
/// reconciled [`Root`] and deliver it through the session's recovery slot.
///
/// The root level holds at most one node — the reassembled root, or nothing
/// when the reconciled tree is empty. The ceiling is the join of both
/// parties' versions, which is what deletion honoring compares against on the
/// next reconciliation. A dropped receiver means the caller no longer wants
/// the result; a backend error means there is no trustworthy result, and the
/// recovery slot drops unresolved.
fn deliver<B, T>(
    top: impl NodeStream<B, T, height::Root> + 'static,
    ceiling: Version,
    recover: oneshot::Sender<Root<B, T>>,
) -> Pump<B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    Box::pin(async move {
        let mut top = pin!(top);
        let mut root = None;
        while let Some(item) = top.next().await {
            let (_prefix, node) = item?;
            debug_assert!(
                root.is_none(),
                "upward reassembly produced more than one root node",
            );
            root = Some(node);
        }
        let _ = recover.send(Root { ceiling, root });
        Ok(())
    })
}

/// The version state of a stage that has been opened but has not yet sent its
/// handshake.
///
/// Carries what the [`connect`](protocol::Connect::connect) /
/// [`accept`](protocol::Accept::accept) step folds into its outgoing
/// [`message::Handshake`]: our universe [`Network`], our latest [`Version`]
/// (the tree's ceiling), and our [`Intent`](message::Intent).
pub struct Start {
    network: Network,
    intent: message::Intent,
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
pub struct Handshaking<B, V, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    backend: B,
    versions: V,
    root: Root<B, T>,
    recover: oneshot::Sender<Root<B, T>>,
}

impl<B, T> Handshaking<B, Start, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    /// Open a mirror session over `root`, advertising `network` and `intent`.
    ///
    /// The handshake version is the tree's ceiling. The returned receiver
    /// resolves with this side's reconciled [`Root`] once its session
    /// completes; it is dropped unresolved if the session never reaches
    /// reconciliation (in particular, when the peers' versions are equal and
    /// there is nothing to reconcile) or if the backend fails mid-session.
    pub fn start(
        backend: B,
        network: Network,
        intent: message::Intent,
        root: Root<B, T>,
    ) -> (Self, oneshot::Receiver<Root<B, T>>) {
        let (recover, recovered) = oneshot::channel();
        (
            Self {
                backend,
                versions: Start {
                    network,
                    intent,
                    our_version: root.ceiling.clone(),
                },
                root,
                recover,
            },
            recovered,
        )
    }
}

impl<B, V, T> protocol::Stage for Handshaking<B, V, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    type Height = height::Root;
    type Node = B::Node<height::Root>;
    type Error = B::Error;
}

impl<B, T> protocol::Connect<B, T> for Handshaking<B, Start, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, Connecting, T>;

    async fn connect<E>(self) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<B::Error> + Send + 'static,
    {
        let Start {
            network,
            intent,
            our_version,
        } = self.versions;

        let handshake = message::Handshake {
            network,
            intent,
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connecting { our_version },
            root: self.root,
            recover: self.recover,
        };
        Ok((handshake, next))
    }
}

impl<B, T> protocol::CompleteConnect<B, T> for Handshaking<B, Connecting, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, Connected, T>;

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
            recover: self.recover,
        })
    }
}

impl<B, T> protocol::Accept<B, T> for Handshaking<B, Start, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, Connected, T>;

    async fn accept<E>(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<B::Error> + Send + 'static,
    {
        let Start {
            network,
            intent,
            our_version,
        } = self.versions;

        let handshake = message::Handshake {
            network,
            intent,
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version,
                their_version: request.version,
            },
            root: self.root,
            recover: self.recover,
        };
        Ok((handshake, next))
    }
}

impl<B, T> protocol::Initiator<B, T> for Handshaking<B, Connected, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Self;

    fn initiator<E>(self) -> (impl Messages<message::Initiate, E> + 'static, Self::Next)
    where
        E: From<B::Error> + Send + 'static,
    {
        // A single uncertain root hash, or nothing at all when the tree is
        // empty: the responder explodes its own root unconditionally either
        // way, so an empty `Initiate` carries exactly as much information as
        // the empty tree's constant hash would. Pure data — the one wire
        // stream that needs no pump behind it.
        let initiate: Option<Result<message::Initiate, E>> = self.root.root.as_ref().map(|node| {
            Ok(message::Initiate::Uncertain(message::Uncertain {
                prefix: Prefix::new(),
                hash: node.hash(),
            }))
        });
        (stream::iter(initiate), self)
    }
}

impl<B, T> protocol::Responder<B, T> for Handshaking<B, Connected, T>
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
            recover,
        } = self;
        let (wire_tx, wire_rx) = mpsc::channel(FAN);
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        // Always explode our root one level and enumerate the resulting
        // children, regardless of the initiator's root hash: an empty
        // `Opening` is the unambiguous "responder has no children" signal
        // that drives the initiator's all-`Provide` opening classify when we
        // are empty, and a matched root costs one fan of hashes pushed
        // through the steady-state pipeline. A single termination path on the
        // wire beats a special case.
        let explode = backend.clone();
        let wire = try_stream! {
            let mut down = down_tx;
            for await item in requests {
                // The initiate's content is not used (we explode
                // unconditionally), but its errors are ours to propagate.
                item?;
            }
            if let Some(node) = root.root {
                let one = stream::once(async move { Ok((Prefix::new(), node)) });
                let mut children = pin!(explode.children(one));
                while let Some(item) = children.next().await {
                    let (prefix, child) = item?;
                    yield message::Opening::Uncertain(message::Uncertain {
                        prefix,
                        hash: child.hash(),
                    });
                    if down.send((prefix, child)).await.is_err() {
                        return;
                    }
                }
            }
        };

        // The responder's reconciled root-child level arrives on `up` whole:
        // one fold reassembles the root for delivery.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = backend.clone().parents(up_rx.map(Ok::<_, B::Error>));
        let pumps = vec![pump(wire, wire_tx), deliver(top, ceiling, recover)];

        let next = Descending {
            backend,
            their_version: versions.their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: up_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

impl<B, T> protocol::OpenInitiator<B, T> for Handshaking<B, Connected, T>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn open_initiator<E>(
        self,
        requests: impl Messages<message::Opening, E> + 'static,
    ) -> (
        impl Messages<message::Exchange<B, T, UnderUnderRoot>, E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Handshaking {
            backend,
            versions,
            root,
            recover,
        } = self;
        let their_version = versions.their_version.clone();
        let (wire_tx, wire_rx) = mpsc::channel(FAN);
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        // The opening round is the one asymmetric-root round: the responder
        // listed its root's children unconditionally, so silence about a
        // child means the responder *lacks* it (everywhere below Root,
        // silence means the hash matched). Feeding the whole opening level
        // into one classify realizes exactly that: children only we hold come
        // out as `Provide` (deletion-pruned), children only the responder
        // holds as `Request`, and an empty side degenerates to all-`Provide`
        // or all-`Request` with no special casing.
        let open = backend.clone();
        let wire = try_stream! {
            let mut down = down_tx;
            let mut level = level_tx;

            // Buffer the responder's opening level: at most one root fan.
            let mut theirs = Vec::new();
            for await item in requests {
                let message::Opening::Uncertain(message::Uncertain { prefix, hash }) = item?;
                theirs.push(Ok((prefix, hash)));
            }

            match root.root {
                Some(node) => {
                    let one = stream::once(async move { Ok((Prefix::new(), node)) });
                    let ours = open.clone().children(one);
                    let verdicts = classify(&open, &their_version, ours, stream::iter(theirs));
                    for await verdict in verdicts {
                        match verdict? {
                            Routed::Provide(prefix, node) => {
                                if level.send((prefix, node.clone())).await.is_err() {
                                    return;
                                }
                                yield message::Exchange::Providing(message::Providing {
                                    prefix,
                                    node,
                                });
                            }
                            Routed::Matched(prefix, node) => {
                                if level.send((prefix, node)).await.is_err() {
                                    return;
                                }
                            }
                            Routed::Request(prefix) => {
                                yield message::Exchange::Requested(message::Requested { prefix });
                            }
                            Routed::Dispute(prefix, node) => {
                                let one = stream::once(async move { Ok((prefix, node)) });
                                let mut children = pin!(open.clone().children(one));
                                while let Some(item) = children.next().await {
                                    let (child_prefix, child) = item?;
                                    yield message::Exchange::Uncertain(message::Uncertain {
                                        prefix: child_prefix,
                                        hash: child.hash(),
                                    });
                                    if down.send((child_prefix, child)).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    // We are empty: request everything the responder listed.
                    for item in theirs {
                        let (prefix, _hash) = item?;
                        yield message::Exchange::Requested(message::Requested { prefix });
                    }
                }
            }
        };

        // The initiator's root-child level is this round's classify verdicts
        // (`level`) joined with the resolved disputes climbing out of the
        // descent (`up`); two folds reassemble the root for delivery.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = backend.clone().parents(merge_disjoint(
            level_rx.map(Ok::<_, B::Error>),
            backend.clone().parents(up_rx.map(Ok)),
            |(prefix, _)| *prefix,
        ));
        let pumps = vec![pump(wire, wire_tx), deliver(top, ceiling, recover)];

        let next = Descending {
            backend,
            their_version: versions.their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: up_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

/// A mirror stage inside the descent: its frontier at height `H` flows
/// downward as a stream while the levels above reassemble concurrently.
///
/// `frontier` holds this side's disputed subtrees at `H`, fed by the previous
/// stage's classify through a [`FAN`]-bounded channel; `up` is where this
/// stage's [`ascend`] pump sends the reconciled level at `H`, feeding the
/// previous stage's ascent in turn; `pumps` accumulates every pump the
/// session has created so far. Each
/// [`exchange`](protocol::Exchange::exchange) consumes the stage and produces
/// its successor two heights finer, until the terminal stages drive the
/// accumulated pumps to completion.
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
    pumps: Vec<Pump<B::Error>>,
}

impl<B, T, H> protocol::Stage for Descending<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    type Height = H;
    type Node = B::Node<H>;
    type Error = B::Error;
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
    // blanket impls; carrying it here keeps this impl from having to
    // case-analyze `H`.
    Descending<B, T, S<H>>: protocol::AfterExchange<B, T, S<H>>,
{
    type Next = Descending<B, T, S<H>>;

    fn exchange<E>(
        self,
        requests: impl Messages<message::Exchange<B, T, S<S<H>>>, E> + 'static,
    ) -> (
        impl Messages<message::Exchange<B, T, S<H>>, E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend,
            their_version,
            frontier,
            up,
            mut pumps,
        } = self;
        let (wire_tx, wire_rx) = mpsc::channel(FAN);
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (keep_tx, keep_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (below_tx, below_rx) = mpsc::channel(FAN);

        let wire = reconcile::walk(
            backend.clone(),
            their_version.clone(),
            frontier,
            requests,
            down_tx,
            keep_tx,
            level_tx,
        )
        .map(|item| {
            item.map(|out| match out {
                Out::Providing(prefix, node) => {
                    message::Exchange::Providing(message::Providing { prefix, node })
                }
                Out::Requested(prefix) => {
                    message::Exchange::Requested(message::Requested { prefix })
                }
                Out::Uncertain(prefix, hash) => {
                    message::Exchange::Uncertain(message::Uncertain { prefix, hash })
                }
            })
        });
        pumps.push(pump(wire, wire_tx));
        pumps.push(ascend(backend.clone(), keep_rx, level_rx, below_rx, up));

        let next = Descending {
            backend,
            their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: below_tx,
            pumps,
        };
        (wire_rx, next)
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
        requests: impl Messages<message::Exchange<B, T, S<Z>>, E> + 'static,
    ) -> (
        impl Messages<message::Closing<B, T>, E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend,
            their_version,
            frontier,
            up,
            mut pumps,
        } = self;
        let (wire_tx, wire_rx) = mpsc::channel(FAN);
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (keep_tx, keep_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (below_tx, below_rx) = mpsc::channel(FAN);

        let wire = reconcile::walk(
            backend.clone(),
            their_version.clone(),
            frontier,
            requests,
            down_tx,
            keep_tx,
            level_tx,
        )
        .filter_map(|item| {
            future::ready(match item {
                Ok(Out::Providing(prefix, node)) => {
                    Some(Ok(message::Closing::Providing(message::Providing {
                        prefix,
                        node,
                    })))
                }
                Ok(Out::Requested(prefix)) => {
                    Some(Ok(message::Closing::Requested(message::Requested {
                        prefix,
                    })))
                }
                // Vacuous at leaf height (see [`message::Closing`]): the
                // disputed leaves still descend into the terminal frontier and
                // stay ours, exactly as the alternating oracle keeps its
                // exploded bottom level.
                Ok(Out::Uncertain(..)) => None,
                Err(error) => Some(Err(error)),
            })
        });
        pumps.push(pump(wire, wire_tx));
        pumps.push(ascend(backend.clone(), keep_rx, level_rx, below_rx, up));

        let next = Descending {
            backend,
            their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, B::Error>)),
            up: below_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

impl<B, T> protocol::CompleteResponder<B, T> for Descending<B, T, S<Z>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    fn complete_responder<E>(
        self,
        requests: impl Messages<message::Closing<B, T>, E> + 'static,
    ) -> (
        impl Messages<message::Complete<B, T>, E> + 'static,
        impl Future<Output = Result<(), E>> + Send,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend,
            their_version,
            frontier,
            up,
            mut pumps,
        } = self;
        let (wire_tx, wire_rx) = mpsc::channel(FAN);

        // The terminal walk reconciles everything at the frontier height, so
        // its level output *is* this stage's contribution to the reassembly:
        // no ascent of its own, just the level sent straight up.
        let wire = reconcile::complete_walk(backend, their_version, frontier, requests, up);
        pumps.push(pump(wire, wire_tx));

        // The responder's terminal drive: every pump of this side's session,
        // polled to completion concurrently with the initiator's terminal
        // (the driver joins the two). Protocol errors ride the wire streams;
        // what a pump itself reports is a backend failure.
        let drive = async move {
            for pumped in future::join_all(pumps).await {
                pumped?;
            }
            Ok(())
        };
        (wire_rx, drive)
    }
}

impl<B, T> protocol::CompleteInitiator<B, T> for Descending<B, T, Z>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    async fn complete_initiator<E>(
        self,
        requests: impl Messages<message::Complete<B, T>, E> + 'static,
    ) -> Result<(), E>
    where
        E: From<B::Error> + Send + 'static,
    {
        let Descending {
            backend: _,
            their_version: _,
            frontier,
            up,
            pumps,
        } = self;

        // The initiator's terminal: absorb the final `providing` into the
        // reconciled leaf level (sent straight up — the terminal stage has no
        // ascent of its own) while driving every pump of this side's session
        // to completion.
        let absorb = reconcile::absorb_leaves(frontier, requests, up);
        let (absorbed, pumped) = future::join(absorb, future::join_all(pumps)).await;
        absorbed?;
        for pumped in pumped {
            pumped?;
        }
        Ok(())
    }
}
