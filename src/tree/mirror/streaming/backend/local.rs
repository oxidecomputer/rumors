use std::convert::Infallible;
use std::pin::{Pin, pin};

use async_stream::try_stream;
use futures::SinkExt;
use futures::channel::{mpsc, oneshot};
use futures::future::{self, BoxFuture};
use futures::stream::{self, StreamExt};

use crate::{
    Network, Version,
    message::Message,
    tree::{
        self,
        typed::{
            self, Prefix,
            height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
            node::Children,
        },
    },
};

use super::super::dispute::{Routed, classify};
use super::super::merge::merge_disjoint;
use super::super::message;
use super::super::protocol::{self, Messages};
use super::{Backend, Leaf, Node, NodeStream};

mod reconcile;
use reconcile::Out;

impl<T, H: Height> Node for typed::Node<T, H> {
    type Height = H;

    fn hash(&self) -> typed::Hash {
        self.hash()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn ceiling(&self) -> &Version {
        self.ceiling()
    }

    fn floor(&self) -> &Version {
        self.floor()
    }
}

impl<T> Leaf<T> for typed::Node<T, Z> {
    fn message(&self) -> &Message<T> {
        self.message()
    }

    fn leaf(version: Version, message: Message<T>) -> Self {
        Self::leaf(version, message)
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Local;

/// The one shared instance of the zero-sized [`Local`] backend.
///
/// Stage machinery borrows the backend for the lifetime of the streams it
/// builds; borrowing a `'static` instance sidesteps self-referential stage
/// structs without cloning anything (there is nothing to clone).
static LOCAL: Local = Local;

/// The bound on every internal channel: one node's child fan (the radix).
///
/// The walk's producers and consumers advance in lockstep per parent — the
/// merge-join holds one item of lookahead per input, and a parent contributes
/// at most one fan of children before the walk must pull its inputs again —
/// so a single fan of slack absorbs the maximum skew between the wire, the
/// descending frontier, and the upward reassembly. This bound is what makes
/// reconciliation fixed-memory regardless of diff size.
const FAN: usize = 256;

/// A boxed, prefix-ordered stream of local nodes at height `H`.
///
/// Every stage boundary boxes: an `impl Stream` threaded through the 32-deep
/// descent would nest each stage's stream type inside the next and balloon
/// the compiler's types past any bound (the same reason
/// [`unknown`](super::super::unknown) boxes per height).
type BoxNodeStream<T, H> = Pin<Box<dyn NodeStream<Local, T, H>>>;

/// A prefix-keyed node at height `H`: the item of every level-carrying
/// channel between pumps.
type Level<T, H> = (Prefix<H>, typed::Node<T, H>);

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
type Pump = BoxFuture<'static, ()>;

impl<T: Send + Sync + 'static> Backend<T> for Local {
    type Node<H: Height> = typed::Node<T, H>;
    type Error = Infallible;

    fn parents<H>(&self, children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        H: Height,
        S<H>: Height,
    {
        // Children of a given parent arrive contiguously, so coalesce each run
        // of equal parent prefixes into one branch node: flush the open parent
        // when the prefix changes, then once more when the input ends. `fuse`
        // keeps the poll after that final flush well-defined.
        stream::unfold(
            // Our state is the pair of the children stream (which we'll pull
            // from) and an optional pair of our current prefix and its
            // children.
            (Box::pin(children.fuse()), None::<(_, Children<_, _>)>),
            |(mut children, mut current)| async move {
                // Loop internally to the single output future, pulling children...
                while let Some(Ok((path, child))) = children.next().await
                    && let (prefix, radix) = path.pop()
                {
                    if let Some((current_prefix, current_children)) = &mut current
                        && *current_prefix == prefix
                    {
                        // If the current prefix matches, append to children:
                        current_children.insert(radix, child);
                    } else if let Some((finished_prefix, finished_children)) =
                        current.replace((prefix, Children::from_iter([(radix, child)])))
                        && let Some(finished_parent) = typed::Node::branch(finished_children)
                    {
                        // Otherwise, pull out a finished prefix and children and
                        // construct the corresponding parent output:
                        let output = (finished_prefix, finished_parent);
                        return Some((Ok(output), (children, current)));
                    }
                }

                // When there are no more children in the input stream, flush any remaining
                // single buffered parent:
                current
                    .take()
                    .and_then(|(current_prefix, current_children)| {
                        typed::Node::branch(current_children)
                            .map(|parent| (Ok((current_prefix, parent)), (children, None)))
                    })
            },
        )
    }

    fn children<H>(&self, parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        T: Send + Sync,
        H: Height,
        S<H>: Height,
    {
        // We box the stream so that traversing down the whole 32-deep tree
        // does not build a gigantic stream type.
        Box::pin(parents.fuse().flat_map(move |Ok((prefix, node))| {
            stream::iter(
                node.into_children()
                    .into_iter()
                    .map(move |(radix, child)| Ok((prefix.push(radix), child))),
            )
        }))
    }
}

// ---------------------------------------------------------------------------
// The streaming mirror stages for the `Local` backend.
//
// A stage is one node in the protocol's descending schedule. Unlike the
// alternating backend — which materializes a `Levels` zipper and pushes two
// levels per round — the streaming stages thread lazy boxed node streams:
// our frontier subtrees flow downward to be disassembled, and reconciled
// nodes flow upward to be reassembled, a hylomorphism in strict prefix order.
//
// Every dataflow edge is a `FAN`-bounded channel and every worker is a
// `Pump`. Each descent stage contributes two: its walk (the frontier against
// the incoming wire, fanning out to the wire/down/keep/level channels) and
// its ascent (folding the reconciled levels back toward the root). The
// accumulated set is driven concurrently at the session's two terminals —
// `complete_initiator` on the initiator, the drive future returned by
// `complete_responder` on the responder — and each side's reconciled
// `tree::Root` is delivered through the oneshot handed out by
// `Handshaking::start`.
// ---------------------------------------------------------------------------

/// Box a stage's outgoing wire stream as the [`Pump`] that forwards it into
/// the stage's wire channel.
///
/// The receiving half is what the stage returns as its outgoing [`Messages`]:
/// the counterparty reads a plain channel while this pump — not the
/// counterparty's demand — advances the walk behind it. Ends when the wire
/// ends or the counterparty drops the receiver (session teardown).
fn pump<M, E>(wire: impl Messages<M, E> + 'static, tx: mpsc::Sender<Result<M, E>>) -> Pump
where
    M: Send + 'static,
    E: Send + 'static,
{
    Box::pin(async move {
        let _ = wire.map(Ok).forward(tx).await;
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
fn ascend<T, H>(
    keep: mpsc::Receiver<Level<T, S<S<H>>>>,
    level: mpsc::Receiver<Level<T, S<H>>>,
    below: mpsc::Receiver<Level<T, H>>,
    up: mpsc::Sender<Level<T, S<S<H>>>>,
) -> Pump
where
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    let reconciled = merge_disjoint(
        keep.map(Ok::<_, Infallible>),
        LOCAL.parents(merge_disjoint(
            level.map(Ok),
            LOCAL.parents(below.map(Ok)),
            |(prefix, _)| *prefix,
        )),
        |(prefix, _)| *prefix,
    );
    Box::pin(async move {
        let _ = reconciled
            .map(|item| item.map_err(|error| match error {}))
            .forward(up)
            .await;
    })
}

/// The topmost [`Pump`]: drain the reconciled root level into this side's
/// reconciled [`tree::Root`] and deliver it through the session's recovery
/// slot.
///
/// The root level holds at most one node — the reassembled root, or nothing
/// when the reconciled tree is empty. The ceiling is the join of both
/// parties' versions, which is what deletion honoring compares against on the
/// next reconciliation. A dropped receiver means the caller no longer wants
/// the result.
fn deliver<T>(
    top: impl NodeStream<Local, T, Root> + 'static,
    ceiling: Version,
    recover: oneshot::Sender<tree::Root<T>>,
) -> Pump
where
    T: Send + Sync + 'static,
{
    Box::pin(async move {
        let mut top = pin!(top);
        let mut root = None;
        while let Some(item) = top.next().await {
            let Ok((_prefix, node)) = item;
            debug_assert!(
                root.is_none(),
                "upward reassembly produced more than one root node",
            );
            root = Some(node);
        }
        let _ = recover.send(tree::Root { ceiling, root });
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

/// A local mirror stage still at [`Root`] height: the handshake phases, before
/// the tree has been disassembled into streams.
///
/// `V` is the version state ([`Start`] → [`Connecting`] → [`Connected`]). The
/// whole tree is held intact as `root` until reconciliation begins at
/// [`initiator`](protocol::Initiator::initiator) /
/// [`responder`](protocol::Responder::responder).
pub struct Handshaking<V, T> {
    versions: V,
    root: tree::Root<T>,
    recover: oneshot::Sender<tree::Root<T>>,
}

impl<T> Handshaking<Start, T> {
    /// Open a local mirror over `root`, advertising `network` and `intent`.
    ///
    /// The handshake version is the tree's ceiling. The returned receiver
    /// resolves with this side's reconciled [`tree::Root`] once its session
    /// completes; it is dropped unresolved if the session never reaches
    /// reconciliation (in particular, when the peers' versions are equal and
    /// there is nothing to reconcile).
    pub fn start(
        network: Network,
        intent: message::Intent,
        root: tree::Root<T>,
    ) -> (Self, oneshot::Receiver<tree::Root<T>>) {
        let (recover, recovered) = oneshot::channel();
        (
            Self {
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

impl<V, T> protocol::Stage for Handshaking<V, T> {
    type Height = Root;
    type Node = typed::Node<T, Root>;
    type Error = Infallible;
}

impl<T> protocol::Connect<Local, T> for Handshaking<Start, T>
where
    T: Send + Sync + 'static,
{
    type Next = Handshaking<Connecting, T>;

    async fn connect<E>(self) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<Infallible> + Send + 'static,
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
            versions: Connecting { our_version },
            root: self.root,
            recover: self.recover,
        };
        Ok((handshake, next))
    }
}

impl<T> protocol::CompleteConnect<Local, T> for Handshaking<Connecting, T>
where
    T: Send + Sync + 'static,
{
    type Next = Handshaking<Connected, T>;

    async fn complete_connect<E>(self, their_version: Version) -> Result<Self::Next, E>
    where
        E: From<Infallible> + Send + 'static,
    {
        Ok(Handshaking {
            versions: Connected {
                our_version: self.versions.our_version,
                their_version,
            },
            root: self.root,
            recover: self.recover,
        })
    }
}

impl<T> protocol::Accept<Local, T> for Handshaking<Start, T>
where
    T: Send + Sync + 'static,
{
    type Next = Handshaking<Connected, T>;

    async fn accept<E>(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<Infallible> + Send + 'static,
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

impl<T> protocol::Initiator<Local, T> for Handshaking<Connected, T>
where
    T: Send + Sync + 'static,
{
    type Next = Self;

    fn initiator<E>(self) -> (impl Messages<message::Initiate, E> + 'static, Self::Next)
    where
        E: From<Infallible> + Send + 'static,
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

impl<T> protocol::Responder<Local, T> for Handshaking<Connected, T>
where
    T: Send + Sync + 'static,
{
    type Next = Descending<T, UnderRoot>;

    fn responder<E>(
        self,
        requests: impl Messages<message::Initiate, E> + 'static,
    ) -> (impl Messages<message::Opening, E> + 'static, Self::Next)
    where
        E: From<Infallible> + Send + 'static,
    {
        let Handshaking {
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
        let wire = try_stream! {
            let mut down = down_tx;
            for await item in requests {
                // The initiate's content is not used (we explode
                // unconditionally), but its errors are ours to propagate.
                item?;
            }
            if let Some(node) = root.root {
                for (radix, child) in node.into_children() {
                    let prefix = Prefix::new().push(radix);
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
        let top = LOCAL.parents(up_rx.map(Ok::<_, Infallible>));
        let pumps = vec![pump(wire, wire_tx), deliver(top, ceiling, recover)];

        let next = Descending {
            their_version: versions.their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, Infallible>)),
            up: up_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

impl<T> protocol::OpenInitiator<Local, T> for Handshaking<Connected, T>
where
    T: Send + Sync + 'static,
{
    type Next = Descending<T, UnderUnderRoot>;

    fn open_initiator<E>(
        self,
        requests: impl Messages<message::Opening, E> + 'static,
    ) -> (
        impl Messages<message::Exchange<Local, T, UnderUnderRoot>, E> + 'static,
        Self::Next,
    )
    where
        E: From<Infallible> + Send + 'static,
    {
        let Handshaking {
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
                    let ours =
                        LOCAL.children(stream::once(async move { Ok((Prefix::new(), node)) }));
                    let verdicts = classify(&LOCAL, &their_version, ours, stream::iter(theirs));
                    for await verdict in verdicts {
                        let Ok(verdict) = verdict;
                        match verdict {
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
                                for (radix, child) in node.into_children() {
                                    let child_prefix = prefix.push(radix);
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
                        let Ok((prefix, _hash)) = item;
                        yield message::Exchange::Requested(message::Requested { prefix });
                    }
                }
            }
        };

        // The initiator's root-child level is this round's classify verdicts
        // (`level`) joined with the resolved disputes climbing out of the
        // descent (`up`); two folds reassemble the root for delivery.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = LOCAL.parents(merge_disjoint(
            level_rx.map(Ok::<_, Infallible>),
            LOCAL.parents(up_rx.map(Ok)),
            |(prefix, _)| *prefix,
        ));
        let pumps = vec![pump(wire, wire_tx), deliver(top, ceiling, recover)];

        let next = Descending {
            their_version: versions.their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, Infallible>)),
            up: up_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

/// A local mirror stage inside the descent: its frontier at height `H` flows
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
pub struct Descending<T: Send + Sync + 'static, H: Height> {
    their_version: Version,
    frontier: BoxNodeStream<T, H>,
    up: mpsc::Sender<Level<T, H>>,
    pumps: Vec<Pump>,
}

impl<T: Send + Sync + 'static, H: Height> protocol::Stage for Descending<T, H> {
    type Height = H;
    type Node = typed::Node<T, H>;
    type Error = Infallible;
}

impl<T, H> protocol::Exchange<Local, T> for Descending<T, S<S<S<H>>>>
where
    T: Send + Sync + 'static,
    H: Height + super::super::unknown::Unknown,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    // Discharged at each concrete height by one of the three `AfterExchange`
    // blanket impls; carrying it here keeps this impl from having to
    // case-analyze `H`.
    Descending<T, S<H>>: protocol::AfterExchange<Local, T, S<H>>,
{
    type Next = Descending<T, S<H>>;

    fn exchange<E>(
        self,
        requests: impl Messages<message::Exchange<Local, T, S<S<H>>>, E> + 'static,
    ) -> (
        impl Messages<message::Exchange<Local, T, S<H>>, E> + 'static,
        Self::Next,
    )
    where
        E: From<Infallible> + Send + 'static,
    {
        let Descending {
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
        pumps.push(ascend(keep_rx, level_rx, below_rx, up));

        let next = Descending {
            their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, Infallible>)),
            up: below_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

impl<T> protocol::CloseInitiator<Local, T> for Descending<T, S<S<Z>>>
where
    T: Send + Sync + 'static,
{
    type Next = Descending<T, Z>;

    fn close_initiator<E>(
        self,
        requests: impl Messages<message::Exchange<Local, T, S<Z>>, E> + 'static,
    ) -> (
        impl Messages<message::Closing<Local, T>, E> + 'static,
        Self::Next,
    )
    where
        E: From<Infallible> + Send + 'static,
    {
        let Descending {
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
        pumps.push(ascend(keep_rx, level_rx, below_rx, up));

        let next = Descending {
            their_version,
            frontier: Box::pin(down_rx.map(Ok::<_, Infallible>)),
            up: below_tx,
            pumps,
        };
        (wire_rx, next)
    }
}

impl<T> protocol::CompleteResponder<Local, T> for Descending<T, S<Z>>
where
    T: Send + Sync + 'static,
{
    fn complete_responder<E>(
        self,
        requests: impl Messages<message::Closing<Local, T>, E> + 'static,
    ) -> (
        impl Messages<message::Complete<Local, T>, E> + 'static,
        impl Future<Output = Result<(), E>> + Send,
    )
    where
        E: From<Infallible> + Send + 'static,
    {
        let Descending {
            their_version,
            frontier,
            up,
            mut pumps,
        } = self;
        let (wire_tx, wire_rx) = mpsc::channel(FAN);

        // The terminal walk reconciles everything at the frontier height, so
        // its level output *is* this stage's contribution to the reassembly:
        // no ascent of its own, just the level sent straight up.
        let wire = reconcile::complete_walk(their_version, frontier, requests, up);
        pumps.push(pump(wire, wire_tx));

        // The responder's terminal drive: every pump of this side's session,
        // polled to completion concurrently with the initiator's terminal
        // (the driver joins the two). Errors ride the wire streams, so the
        // pumps themselves have nothing fallible to report.
        let drive = async move {
            future::join_all(pumps).await;
            Ok(())
        };
        (wire_rx, drive)
    }
}

impl<T> protocol::CompleteInitiator<Local, T> for Descending<T, Z>
where
    T: Send + Sync + 'static,
{
    async fn complete_initiator<E>(
        self,
        requests: impl Messages<message::Complete<Local, T>, E> + 'static,
    ) -> Result<(), E>
    where
        E: From<Infallible> + Send + 'static,
    {
        let Descending {
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
        let (absorbed, _pumped) = future::join(absorb, future::join_all(pumps)).await;
        absorbed
    }
}
