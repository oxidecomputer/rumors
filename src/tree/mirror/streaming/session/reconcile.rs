//! The per-stage reconciliation walk: one prefix-ordered pass fusing the
//! incoming wire with the descending frontier.
//!
//! This is the streaming counterpart of the alternating backend's
//! [`partition`](crate::tree::mirror::alternating) internals. Where the
//! alternating `reply` runs three phases over materialized levels (absorb
//! `providing`, answer `requested`, partition `uncertain`), [`walk`] fuses all
//! three into a single merge-join between our frontier subtrees and the
//! [demuxed](demux) incoming message stream, keyed by the frontier-height
//! prefix. Every verdict of the asymmetry matrix routes to one of four
//! destinations:
//!
//! - the **outgoing wire** — the walk's direct output, already in wire form
//!   ([`message::Exchange`]), forwarded by [`outgoing`](super::outgoing) into
//!   the channel the counterparty reads;
//! - the **next stage's frontier** (`down`) — disputed subtrees exploded one
//!   level, sent through a [`FAN`](super::FAN)-bounded channel;
//! - the **reconciled level at the frontier height** (`keep`) — nodes the
//!   counterparty matched by silence, request-answer survivors, and absorbed
//!   `providing`;
//! - the **reconciled level one below** (`level`) — `Matched` and pruned
//!   `Provide` verdicts from [`classify`].
//!
//! The two reconciled-level channels feed the stage's upward reassembly (see
//! [`super`]); their [`FAN`](super::FAN) bound is what pins the whole session's memory to a
//! single parent's fan regardless of diff size.
//!
//! # Channel discipline
//!
//! Every channel send backpressures at [`FAN`](super::FAN) entries. A closed channel means
//! the consuming half of the session was dropped; the walk then ends its output
//! stream rather than erroring, and the counterparty observes an ordinary
//! end-of-stream.

use std::pin::pin;

use async_stream::try_stream;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::stream::{self, Stream, StreamExt};
use itertools::EitherOrBoth;

use crate::Version;
use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, S, Z},
};

use super::super::backend::{Backend, Leaf, Node, NodeStream, one};
use super::super::dispute::{Routed, classify};
use super::super::merge::merge;
use super::super::message;
use super::super::protocol::Messages;
use super::super::unknown::{Unknown, unknown};
use super::{BoxNodeStream, Level};

/// A run of uncertain children under one parent: their prefixes and hashes.
type Hashes<C> = Vec<(Prefix<C>, Hash)>;

/// One demuxed wire reaction, keyed by the frontier-height prefix it answers.
type Reaction<B, T, H> = (Prefix<S<H>>, Incoming<B, T, H>);

/// One incoming wire reaction, grouped under the frontier-height prefix it
/// concerns.
///
/// The wire interleaves kinds in one globally prefix-ascending stream; what the
/// walk's merge-join needs is one item per frontier prefix. `Providing` and
/// `Requested` already sit at the frontier height and key by their own prefix;
/// a run of `Uncertain` children keys by their shared parent, and [`demux`]
/// coalesces each run into one `Uncertain` entry. The kinds are mutually
/// exclusive per prefix: each reacts to a different channel of our previous
/// message (`Requested`/`Uncertain` answer our `uncertain`, `Providing` answers
/// our `requested` or our inferred lack).
pub(super) enum Incoming<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    H: Height,
    S<H>: Height,
{
    /// A subtree we asked for (or provably lacked): absorb it.
    Providing(B::Node<S<H>>),
    /// The counterparty lacks our subtree at this prefix: explode and provide
    /// its children, pruned against their version.
    Requested,
    /// The counterparty disputed our subtree at this prefix: its children's
    /// hashes, for [`classify`] to compare against our own.
    Uncertain(Hashes<H>),
}

/// Group an incoming [`message::Exchange`] stream by frontier-height prefix.
///
/// `Uncertain` items at height `M` coalesce into per-parent runs at `S<M>`
/// (buffering at most one parent's fan); `Providing`/`Requested` items pass
/// through keyed by their own prefix. Requires the input to be globally
/// prefix-ascending — which the canonical wire order guarantees — and produces
/// strictly ascending keys.
pub(super) fn demux<B, T, H, E>(
    messages: impl Messages<message::Exchange<B, T, H>, E>,
) -> impl Stream<Item = Result<Reaction<B, T, H>, E>> + Send
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    E: Send,
{
    try_stream! {
        // The open `Uncertain` run: its parent prefix and the children seen so
        // far. Flushed when the parent changes or a non-uncertain kind
        // arrives (both of which, in ascending order, end the run).
        let mut run: Option<(Prefix<S<H>>, Hashes<H>)> = None;

        for await item in messages {
            match item? {
                message::Exchange::Providing(message::Providing { prefix, node }) => {
                    if let Some((parent, children)) = run.take() {
                        yield (parent, Incoming::Uncertain(children));
                    }
                    yield (prefix, Incoming::Providing(node));
                }
                message::Exchange::Requested(message::Requested { prefix }) => {
                    if let Some((parent, children)) = run.take() {
                        yield (parent, Incoming::Uncertain(children));
                    }
                    yield (prefix, Incoming::Requested);
                }
                message::Exchange::Uncertain(message::Uncertain { prefix, hash }) => {
                    let (parent, _) = prefix.pop();
                    if let Some((open_parent, children)) = &mut run
                        && *open_parent == parent
                    {
                        children.push((prefix, hash));
                    } else if let Some((finished_parent, children)) =
                        run.replace((parent, vec![(prefix, hash)]))
                    {
                        yield (finished_parent, Incoming::Uncertain(children));
                    }
                }
            }
        }

        if let Some((parent, children)) = run.take() {
            yield (parent, Incoming::Uncertain(children));
        }
    }
}

/// Route one disputed parent's [`classify`] verdicts to their destinations.
///
/// `Provide` children go to `level` and the wire; `Matched` to `level` only;
/// `Request` to the wire only; `Dispute`d children explode one level finer —
/// their hashes to the wire as the next `uncertain`, the subtrees themselves
/// to `down` as the next stage's frontier.
///
/// Shared by [`walk`]'s dispute arm and the opening round
/// ([`open_initiator`](super::super::protocol::OpenInitiator)). A closed
/// channel ends the stream early: callers observe the teardown on their own
/// next send, or by checking the senders they kept.
pub(super) fn route<B, T, H, E>(
    backend: B,
    their_version: Version,
    ours: impl NodeStream<B, T, S<H>> + 'static,
    theirs: impl Stream<Item = Result<(Prefix<S<H>>, Hash), B::Error>> + Send + 'static,
    mut down: mpsc::Sender<Level<B, T, H>>,
    mut level: mpsc::Sender<Level<B, T, S<H>>>,
) -> impl Messages<message::Exchange<B, T, H>, E>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height + Unknown,
    E: From<B::Error> + Send,
{
    try_stream! {
        let verdicts = classify(&backend, &their_version, ours, theirs);
        for await verdict in verdicts {
            match verdict? {
                // We have it, they lack it (already pruned by classify):
                // send it and keep it.
                Routed::Provide(prefix, node) => {
                    if level.send((prefix, node.clone())).await.is_err() {
                        return;
                    }
                    yield message::Exchange::Providing(message::Providing { prefix, node });
                }
                // Hashes agree: keep ours, say nothing.
                Routed::Matched(prefix, node) => {
                    if level.send((prefix, node)).await.is_err() {
                        return;
                    }
                }
                // They have it, we lack it: ask for it.
                Routed::Request(prefix) => {
                    yield message::Exchange::Requested(message::Requested { prefix });
                }
                // Hashes differ: descend. The children's hashes go out as the
                // next `uncertain`; the children themselves become the next
                // stage's frontier.
                Routed::Dispute(prefix, node) => {
                    let mut children = pin!(backend.clone().children(one(prefix, node)));
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
}

/// Run one descent stage's reconciliation: our frontier at `S<S<G>>` against
/// the incoming message at `S<G>`, producing the outgoing wire at `S<G>`/`G`.
///
/// The returned stream is the outgoing wire and the walk's engine: pulling it
/// advances the merge-join, which routes every prefix's verdict as described
/// in the [module docs](self). `down` receives disputed subtrees exploded to
/// height `G` (the next stage's frontier); `keep` receives reconciled nodes at
/// the frontier height; `level` receives reconciled children one level below.
///
/// [`protocol::Exchange`](super::super::protocol::Exchange) stages return the
/// output as-is; [`close_initiator`](super::super::protocol::CloseInitiator)
/// filters it down to [`message::Closing`], dropping `Uncertain` (vacuous at
/// leaf height, exactly as the alternating `Closing` does).
pub(super) fn walk<B, T, H, E>(
    backend: B,
    their_version: Version,
    frontier: BoxNodeStream<B, T, S<S<H>>>,
    messages: impl Messages<message::Exchange<B, T, S<H>>, E>,
    down: mpsc::Sender<Level<B, T, H>>,
    mut keep: mpsc::Sender<Level<B, T, S<S<H>>>>,
    level: mpsc::Sender<Level<B, T, S<H>>>,
) -> impl Messages<message::Exchange<B, T, H>, E>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
    E: From<B::Error> + Send,
{
    try_stream! {
        let frontier = frontier.map(|item| item.map_err(E::from));
        let joined = merge(frontier, demux(messages));

        for await cell in joined {
            match cell? {
                // Matched by silence: the counterparty compared our hash and
                // agreed, so neither side says anything further. Keep our copy.
                EitherOrBoth::Left((prefix, node)) => {
                    if keep.send((prefix, node)).await.is_err() {
                        return;
                    }
                }

                // They lack this subtree entirely. Prune it against their
                // version first: parts causally at or before their version
                // that they lack were deleted there, so we forget them too.
                // The surviving node stays ours; its children go on the wire.
                EitherOrBoth::Both((prefix, node), (_, Incoming::Requested)) => {
                    let mut pruned = unknown(&backend, &their_version, one(prefix, node));
                    while let Some(item) = pruned.next().await {
                        let (prefix, node) = item?;
                        if keep.send((prefix, node.clone())).await.is_err() {
                            return;
                        }
                        let mut children = pin!(backend.clone().children(one(prefix, node)));
                        while let Some(item) = children.next().await {
                            let (child_prefix, child) = item?;
                            yield message::Exchange::Providing(message::Providing {
                                prefix: child_prefix,
                                node: child,
                            });
                        }
                    }
                }

                // They dispute this subtree: compare children via the
                // asymmetry matrix, one verdict per child prefix, each routed
                // by the shared `route`.
                EitherOrBoth::Both((prefix, node), (_, Incoming::Uncertain(theirs))) => {
                    let ours = backend.clone().children(one(prefix, node));
                    let theirs = stream::iter(theirs.into_iter().map(Ok));
                    let routed = route::<B, T, H, E>(
                        backend.clone(),
                        their_version.clone(),
                        ours,
                        theirs,
                        down.clone(),
                        level.clone(),
                    );
                    for await item in routed {
                        yield item?;
                    }
                    // `route` swallows channel closure to end its own stream;
                    // for the walk it means the session is tearing down.
                    if down.is_closed() || level.is_closed() {
                        return;
                    }
                }

                // A subtree we asked for (or provably lacked): absorb it into
                // the reconciled frontier level.
                EitherOrBoth::Right((prefix, Incoming::Providing(node))) => {
                    if keep.send((prefix, node)).await.is_err() {
                        return;
                    }
                }

                // The counterparty may only provide against prefixes we lack,
                // and may only request or dispute prefixes we listed; anything
                // else means the peer is misbehaving, or we are.
                EitherOrBoth::Both((prefix, _), (_, Incoming::Providing(_))) => {
                    debug_assert!(
                        false,
                        "counterparty provided prefix {prefix:?} we already hold",
                    );
                }
                EitherOrBoth::Right((prefix, Incoming::Requested | Incoming::Uncertain(_))) => {
                    debug_assert!(
                        false,
                        "counterparty mentioned prefix {prefix:?} we never listed",
                    );
                }
            }
        }
    }
}

/// The responder's terminal walk: our frontier at `S<Z>` against the
/// initiator's [`message::Closing`], producing the final
/// [`message::Complete`] wire items.
///
/// A `Closing` carries no `uncertain` (vacuous at leaf height), so this is
/// [`walk`] minus the classify arm: silence keeps, `requested` explodes into
/// pruned leaf `providing`, and incoming `providing` is absorbed. All
/// reconciled nodes land at the frontier height through `level`.
pub(super) fn complete_walk<B, T, E>(
    backend: B,
    their_version: Version,
    frontier: BoxNodeStream<B, T, S<Z>>,
    messages: impl Messages<message::Closing<B, T>, E>,
    mut level: mpsc::Sender<Level<B, T, S<Z>>>,
) -> impl Stream<Item = Result<message::Complete<B, T>, E>> + Send
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error> + Send,
{
    try_stream! {
        let frontier = frontier.map(|item| item.map_err(E::from));
        // `Some(node)` for an incoming `providing`, `None` for a `requested`;
        // both kinds key by their own prefix at the frontier height.
        let incoming = messages.map(|item| {
            item.map(|message| match message {
                message::Closing::Providing(message::Providing { prefix, node }) => {
                    (prefix, Some(node))
                }
                message::Closing::Requested(message::Requested { prefix }) => (prefix, None),
            })
        });
        let joined = merge(frontier, incoming);

        for await cell in joined {
            match cell? {
                // Matched by silence: keep our copy.
                EitherOrBoth::Left((prefix, node)) => {
                    if level.send((prefix, node)).await.is_err() {
                        return;
                    }
                }
                // They lack it entirely: prune against their version, keep
                // the survivor, and send its leaves as the final providing.
                EitherOrBoth::Both((prefix, node), (_, None)) => {
                    let mut pruned = unknown(&backend, &their_version, one(prefix, node));
                    while let Some(item) = pruned.next().await {
                        let (prefix, node) = item?;
                        if level.send((prefix, node.clone())).await.is_err() {
                            return;
                        }
                        let mut children = pin!(backend.clone().children(one(prefix, node)));
                        while let Some(item) = children.next().await {
                            let (child_prefix, child) = item?;
                            yield message::Complete::Providing(message::Providing {
                                prefix: child_prefix,
                                node: child,
                            });
                        }
                    }
                }
                // A subtree we asked for: absorb it.
                EitherOrBoth::Right((prefix, Some(node))) => {
                    if level.send((prefix, node)).await.is_err() {
                        return;
                    }
                }
                EitherOrBoth::Both((prefix, _), (_, Some(_))) => {
                    debug_assert!(
                        false,
                        "counterparty provided prefix {prefix:?} we already hold",
                    );
                }
                EitherOrBoth::Right((prefix, None)) => {
                    debug_assert!(
                        false,
                        "counterparty requested prefix {prefix:?} we never listed",
                    );
                }
            }
        }
    }
}

/// The initiator's terminal absorb: merge our kept disputed leaves with the
/// responder's final `providing` into the reconciled leaf level.
///
/// The two sets are disjoint: our frontier holds leaves under parents *we*
/// disputed, while the responder provides only leaves under parents we
/// requested (and so lack entirely). See
/// [`complete_initiator`](super::super::protocol::CompleteInitiator), which
/// drives this concurrently with the session's accumulated work.
pub(super) async fn absorb_leaves<B, T, E>(
    frontier: BoxNodeStream<B, T, Z>,
    messages: impl Messages<message::Complete<B, T>, E>,
    mut leaves: mpsc::Sender<Level<B, T, Z>>,
) -> Result<(), E>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error> + Send,
{
    let frontier = frontier.map(|item| item.map_err(E::from));
    let incoming = messages.map(|item| {
        item.map(|message| match message {
            message::Complete::Providing(message::Providing { prefix, node }) => (prefix, node),
        })
    });
    let joined = merge(frontier, incoming);

    let mut joined = pin!(joined);
    while let Some(cell) = joined.next().await {
        let sent = match cell? {
            EitherOrBoth::Left(leaf) | EitherOrBoth::Right(leaf) => leaves.send(leaf).await,
            EitherOrBoth::Both(ours, _) => {
                debug_assert!(
                    false,
                    "counterparty provided leaf {:?} we already hold",
                    ours.0,
                );
                leaves.send(ours).await
            }
        };
        if sent.is_err() {
            // The reassembly was dropped; the session is being torn down.
            break;
        }
    }
    Ok(())
}
