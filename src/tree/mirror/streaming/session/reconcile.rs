//! The per-stage reconciliation walk: one prefix-ordered pass fusing the
//! incoming wire with the descending frontier.
//!
//! This is the streaming counterpart of the alternating backend's
//! [`partition`](crate::tree::mirror::alternating) internals. Where the
//! alternating `reply` runs three phases over materialized levels (absorb
//! `providing`, answer `requested`, partition `uncertain`), [`walk`] fuses all
//! three into a single merge-join between our frontier subtrees and the
//! incoming message stream — every [`message::Exchange`] kind keys by a
//! frontier-height prefix, so the wire joins directly. Every verdict of the
//! asymmetry matrix routes to one of four destinations:
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
//! [`super`]); their [`FAN`](super::FAN) bound is what pins the whole session's
//! memory to a single parent's fan regardless of diff size.
//!
//! # Channel discipline
//!
//! Every channel send backpressures at [`FAN`](super::FAN) entries. A closed
//! channel means the consuming half of the session was dropped; the walk then
//! ends its output stream rather than erroring, and the counterparty observes
//! an ordinary end-of-stream.

use std::pin::pin;

use async_stream::try_stream;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::stream::{self, Stream, StreamExt};
use itertools::EitherOrBoth;

use crate::Version;
use crate::tree::typed::{
    Hash, Prefix,
    height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
};

use super::super::backend::{Backend, BoxNodeStream, Leaf, Material, Node, NodeStream, one};
use super::super::dispute::{Routed, classify};
use super::super::merge::merge;
use super::super::message;
use super::super::protocol::Messages;
use super::super::unknown::{Unknown, unknown};
use super::Level;

/// The responder's opening walk: explode the root one level and list every
/// child on the wire while feeding the subtrees to `down` as the first
/// frontier.
///
/// The listing is unconditional, regardless of the initiator's root hash:
/// the root hashes always differ here, because they can only match when the
/// versions match — and that already short-circuited the session.
pub(super) fn respond<B, T, E>(
    backend: B,
    root: Option<B::Node<height::Root>>,
    requests: impl Messages<message::Initiate, E>,
    mut down: mpsc::Sender<Level<B, T, UnderRoot>>,
) -> impl Messages<message::Opening, E>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error> + Send + 'static,
{
    try_stream! {
        for await item in requests {
            // The initiate's content is not used (we explode
            // unconditionally), but its errors are ours to propagate.
            item?;
        }
        if let Some(node) = root {
            let mut children = pin!(backend.clone().children(one(Prefix::new(), node)));
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
    }
}

/// The initiator's opening walk: the one asymmetric-root round.
///
/// The responder listed its root's children unconditionally, so silence
/// about a child means the responder *lacks* it (everywhere below the root,
/// silence means the hash matched). Feeding the whole opening level into one
/// [`route`] realizes exactly that: children only we hold come out as
/// `Provide` (deletion-pruned), children only the responder holds as
/// `Request`, and an empty side degenerates to all-`Provide` or
/// all-`Request` with no special casing.
pub(super) fn open<B, T, E>(
    backend: B,
    their_version: Version,
    root: Option<B::Node<height::Root>>,
    requests: impl Messages<message::Opening, E> + 'static,
    down: mpsc::Sender<Level<B, T, UnderUnderRoot>>,
    level: mpsc::Sender<Level<B, T, UnderRoot>>,
) -> impl Messages<message::Exchanged<B, T, UnderRoot>, E>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error> + Send + 'static,
{
    try_stream! {
        // The responder's opening listing: at most one message, and an empty
        // responder sends none. The parent is statically the root.
        let mut theirs = Vec::new();
        for await item in requests {
            let message::Opening::Uncertain(children) = item?;
            theirs = children
                .into_iter()
                .map(|(radix, hash)| (Prefix::new().push(radix), hash))
                .collect();
        }

        match root {
            Some(node) => {
                let ours = backend.clone().children(one(Prefix::new(), node));
                let theirs = stream::iter(theirs.into_iter().map(Ok));
                let routed =
                    route::<B, T, UnderUnderRoot, E>(backend, their_version, ours, theirs, down, level);
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
    }
}

/// Answer a `requested` subtree: honor the counterparty's deletions, keep
/// the survivors, and yield their children for the wire.
///
/// The subtree is pruned against `their_version` first — parts causally at
/// or before their version that they lack were deleted there, so we forget
/// them too. Each surviving node goes to `kept` (it stays ours); its
/// children are yielded for the caller to wrap in its wire message kind.
///
/// Shared by [`walk`] and [`complete_walk`]. A closed channel ends the
/// stream early: callers observe the teardown by checking the sender they
/// kept.
fn provide<B, T, C, E>(
    backend: B,
    their_version: Version,
    prefix: Prefix<S<C>>,
    node: B::Node<S<C>>,
    mut kept: mpsc::Sender<Level<B, T, S<C>>>,
) -> impl Stream<Item = Result<Level<B, T, C>, E>> + Send
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    C: Height + Unknown,
    S<C>: Height,
    E: From<B::Error> + Send + 'static,
{
    try_stream! {
        let mut pruned = unknown(&backend, &their_version, one(prefix, node));
        while let Some(item) = pruned.next().await {
            let (prefix, node) = item?;
            if kept.send((prefix, node.clone())).await.is_err() {
                return;
            }
            let mut children = pin!(backend.clone().children(one(prefix, node)));
            while let Some(item) = children.next().await {
                yield item?;
            }
        }
    }
}

/// Route one disputed parent's [`classify`] verdicts to their destinations.
///
/// `Provide` children go to `level` and the wire; `Matched` to `level` only;
/// `Request` to the wire only; `Dispute`d children explode one level finer —
/// their hashes to the wire as the next `uncertain`, the subtrees themselves to
/// `down` as the next stage's frontier.
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
) -> impl Messages<message::Exchanged<B, T, S<H>>, E>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height + Unknown,
    E: From<B::Error> + Send + 'static,
{
    try_stream! {
        for await verdict in classify(&backend, &their_version, ours, theirs) {
            match verdict? {
                // We have it, they lack it (already pruned by classify): send
                // it and keep it.
                Routed::Provide(prefix, node) => {
                    if level.send((prefix, node.clone())).await.is_err() {
                        return;
                    }
                    yield (prefix, message::Exchange::Providing(node));
                }
                // Hashes agree: keep ours, say nothing.
                Routed::Matched(prefix, node) => {
                    if level.send((prefix, node)).await.is_err() {
                        return;
                    }
                }
                // They have it, we lack it: ask for it.
                Routed::Request(prefix) => {
                    yield (prefix, message::Exchange::Requested);
                }
                // Hashes differ: descend. The children's hashes go out as one
                // `uncertain` batch; the children themselves become the next
                // stage's frontier.
                Routed::Dispute(prefix, node) => {
                    let mut children = pin!(backend.clone().children(one(prefix, node)));
                    let mut listing = Vec::new();
                    while let Some(item) = children.next().await {
                        let (child_prefix, child) = item?;
                        let (_, radix) = child_prefix.pop();
                        listing.push((radix, child.hash()));
                        if down.send((child_prefix, child)).await.is_err() {
                            return;
                        }
                    }
                    yield (prefix, message::Exchange::Uncertain(listing));
                }
            }
        }
    }
}

/// Run one descent stage's reconciliation: our frontier at `S<S<G>>` against
/// the incoming wire keyed at the same height, producing the outgoing wire
/// keyed one level below.
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
    messages: impl Messages<message::Exchanged<B, T, S<S<H>>>, E>,
    down: mpsc::Sender<Level<B, T, H>>,
    mut keep: mpsc::Sender<Level<B, T, S<S<H>>>>,
    level: mpsc::Sender<Level<B, T, S<H>>>,
) -> impl Messages<message::Exchanged<B, T, S<H>>, E>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
    E: From<B::Error> + Send + 'static,
{
    try_stream! {
        for await cell in merge(frontier.map(|item| item.map_err(E::from)), messages) {
            match cell? {
                // Matched by silence: the counterparty compared our hash and
                // agreed, so neither side says anything further. Keep our copy.
                (prefix, EitherOrBoth::Left(node)) => {
                    if keep.send((prefix, node)).await.is_err() {
                        return;
                    }
                }

                // They lack this subtree entirely: keep what survives their
                // deletions, provide its children (see [`provide`]).
                (prefix, EitherOrBoth::Both(node, message::Exchange::Requested)) => {
                    let provided = provide::<B, T, S<H>, E>(
                        backend.clone(),
                        their_version.clone(),
                        prefix,
                        node,
                        keep.clone(),
                    );
                    for await item in provided {
                        let (child_prefix, child) = item?;
                        yield (child_prefix, message::Exchange::Providing(child));
                    }
                    // `provide` swallows channel closure to end its own stream;
                    // for the walk it means session teardown.
                    if keep.is_closed() {
                        return;
                    }
                }

                // They dispute this subtree: compare children via the asymmetry
                // matrix, one verdict per child prefix, each routed by the
                // shared `route`.
                (prefix, EitherOrBoth::Both(node, message::Exchange::Uncertain(children))) => {
                    let ours = backend.clone().children(one(prefix, node));
                    let theirs = stream::iter(
                        children
                            .into_iter()
                            .map(move |(radix, hash)| Ok((prefix.push(radix), hash))),
                    );
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
                (prefix, EitherOrBoth::Right(message::Exchange::Providing(node))) => {
                    if keep.send((prefix, node)).await.is_err() {
                        return;
                    }
                }

                // The counterparty may only provide against prefixes we lack,
                // and may only request or dispute prefixes we listed; anything
                // else means the peer is misbehaving, or we are.
                (prefix, EitherOrBoth::Both(_, message::Exchange::Providing(_))) => {
                    debug_assert!(
                        false,
                        "counterparty provided prefix {prefix:?} we already hold",
                    );
                }
                (prefix, EitherOrBoth::Right(
                    message::Exchange::Requested | message::Exchange::Uncertain(_),
                )) => {
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
    messages: impl Messages<(Prefix<S<Z>>, message::Closing<B, T>), E>,
    mut level: mpsc::Sender<Level<B, T, S<Z>>>,
) -> impl Stream<Item = Result<(Prefix<Z>, message::Complete<B, T>), E>> + Send
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error> + Send + 'static,
{
    try_stream! {
        for await cell in merge(frontier.map(|item| item.map_err(E::from)), messages) {
            match cell? {
                // Matched by silence: keep our copy.
                (prefix, EitherOrBoth::Left(node)) => {
                    if level.send((prefix, node)).await.is_err() {
                        return;
                    }
                }
                // They lack it entirely: keep what survives their deletions,
                // send its leaves as the final providing (see [`provide`]).
                (prefix, EitherOrBoth::Both(node, message::Closing::Requested)) => {
                    let provided = provide::<B, T, Z, E>(
                        backend.clone(),
                        their_version.clone(),
                        prefix,
                        node,
                        level.clone(),
                    );
                    for await item in provided {
                        let (child_prefix, child) = item?;
                        yield (child_prefix, message::Complete::Providing(child));
                    }
                    // `provide` swallows channel closure to end its own
                    // stream; here it means session teardown.
                    if level.is_closed() {
                        return;
                    }
                }
                // A subtree we asked for: absorb it.
                (prefix, EitherOrBoth::Right(message::Closing::Providing(node))) => {
                    if level.send((prefix, node)).await.is_err() {
                        return;
                    }
                }
                (prefix, EitherOrBoth::Both(_, message::Closing::Providing(_))) => {
                    debug_assert!(
                        false,
                        "counterparty provided prefix {prefix:?} we already hold",
                    );
                }
                (prefix, EitherOrBoth::Right(message::Closing::Requested)) => {
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
    messages: impl Messages<(Prefix<Z>, message::Complete<B, T>), E>,
    mut leaves: mpsc::Sender<Level<B, T, Z>>,
) -> Result<(), E>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error> + Send,
{
    let frontier = frontier.map(|item| item.map_err(E::from));
    let incoming = messages
        .map(|item| item.map(|(prefix, message::Complete::Providing(node))| (prefix, node)));
    let joined = merge(frontier, incoming);

    let mut joined = pin!(joined);
    while let Some(cell) = joined.next().await {
        let sent = match cell? {
            (prefix, EitherOrBoth::Left(leaf) | EitherOrBoth::Right(leaf)) => {
                leaves.send((prefix, leaf)).await
            }
            (prefix, EitherOrBoth::Both(ours, _)) => {
                debug_assert!(
                    false,
                    "counterparty provided leaf {:?} we already hold",
                    prefix,
                );
                leaves.send((prefix, ours)).await
            }
        };
        if sent.is_err() {
            // The reassembly was dropped; the session is being torn down.
            break;
        }
    }
    Ok(())
}
