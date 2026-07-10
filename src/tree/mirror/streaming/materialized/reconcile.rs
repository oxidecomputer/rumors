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
//!   ([`message::Exchange`]) and in the backend's node vocabulary the
//!   counterparty reads, which [`outgoing`] forwards into the channel it
//!   reads it from;
//! - the **next stage's frontier** (`down`) — disputed subtrees exploded one
//!   level, sent through a [`FAN`]-bounded channel;
//! - the **reconciled level at the frontier height** (`keep`) — nodes the
//!   counterparty matched by silence, request-answer survivors, and absorbed
//!   `providing`;
//! - the **reconciled level one below** (`level`) — `Matched` and pruned
//!   `Provide` verdicts from [`classify`].
//!
//! The two reconciled-level channels feed the stage's upward reassembly (see
//! [`super`]); their [`FAN`] bound is what pins the whole session's
//! memory to a single parent's fan regardless of diff size.
//!
//! # Channel discipline
//!
//! Every channel send backpressures at [`FAN`] entries. A closed
//! channel means the consuming half of the session was dropped; the walk then
//! ends its output stream rather than erroring, and the counterparty observes
//! an ordinary end-of-stream.

use std::pin::pin;

use async_stream::try_stream;
use futures::SinkExt;
use futures::future::BoxFuture;
use futures::stream::{self, Stream, StreamExt};
use itertools::EitherOrBoth;
use tokio::join;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::Version;
use crate::tree::mirror::streaming::FAN;
use crate::tree::mirror::streaming::protocol::Exchange;
use crate::tree::typed::{
    Hash, Prefix,
    height::{self, Height, S, UnderRoot, UnderUnderRoot, Z},
};

use super::super::backend::{
    Backend, BoxNodeStream, Leaf, Node, NodeStream, OptionNodeStream, one,
};
use super::super::message;
use super::super::protocol::{Requests, Responses};
use super::dispute::{Routed, classify};
use super::merge::{merge, merge_disjoint};
use super::unknown::{Unknown, known, unknown};

/// Open a stage's outgoing wire: push the future that forwards `messages`
/// into a fresh [`FAN`]-bounded channel onto `work`, and return the
/// receiving half.
///
/// The receiving half is what the stage returns as its outgoing [`Responses`]:
/// the counterparty reads a plain channel while the forwarding future advances
/// the walk behind it. Forwarding ends when the wire ends or the counterparty
/// drops the receiver (session teardown), and never fails: errors come with the
/// forwarded items.
pub(super) fn outgoing<M, E, X>(
    work: &mut Vec<BoxFuture<'static, Result<(), X>>>,
    messages: impl Responses<M, E>,
) -> Receiver<Result<M, E>>
where
    M: Send + 'static,
    E: Send + 'static,
    X: Send + 'static,
{
    let (tx, rx) = mpsc::channel(FAN);
    work.push(Box::pin(async move {
        let mut messages = pin!(messages);
        while let Some(item) = messages.next().await {
            let _ = tx.send(item).await;
        }
        Ok(())
    }));
    rx
}

/// Drain a marked prefix-keyed node stream into a channel.
///
/// The dual of [`outgoing`] for the reassembly side: where `outgoing`
/// forwards a walk's *messages* into the channel the counterparty reads,
/// this forwards its reconciled *nodes* — and their watermarks — into the
/// channel the level above folds. A closed channel means the consumer is
/// gone and the session is being torn down; the drain ends quietly and the
/// driver observes the teardown elsewhere.
pub(super) async fn forward<B, T, H>(
    nodes: impl OptionNodeStream<B, T, H>,
    tx: Sender<(Prefix<H>, Option<B::Node<H>>)>,
) -> Result<(), B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let mut nodes = pin!(nodes);
    while let Some(item) = nodes.next().await {
        if tx.send(item?).await.is_err() {
            break;
        }
    }
    Ok(())
}

/// The initiator's opening walk: explode the root one level and list every
/// child on the wire while feeding the subtrees to `down` as the first
/// frontier.
///
/// Nothing precedes this on the wire, and the listing is unconditional: the
/// two roots' hashes always differ here, because they can only match when the
/// versions match — and that already short-circuited the session. So there is
/// no root hash worth exchanging first, and nothing to wait for.
pub(super) fn initiate<B, T>(
    backend: B,
    root: Option<B::Node<height::Root>>,
    down: Sender<(Prefix<UnderRoot>, Option<B::Node<UnderRoot>>)>,
) -> impl Responses<message::Opening, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    try_stream! {
        if let Some(node) = root {
            let mut children = pin!(backend.clone().children(Prefix::new(), node));
            let mut listing = Vec::new();
            while let Some(item) = children.next().await {
                let (prefix, child) = item?;
                let (_, radix) = prefix.pop();
                listing.push((radix, child.hash()));
                if down.send((prefix, Some(child))).await.is_err() {
                    return;
                }
            }
            yield message::Opening::Uncertain(listing);
        }
    }
}

/// The responder's opening walk: the one asymmetric-root round.
///
/// The initiator listed its root's children unconditionally, so silence
/// about a child means the initiator *lacks* it (everywhere below the root,
/// silence means the hash matched). Feeding the whole opening level into one
/// [`route`] realizes exactly that: children only we hold come out as
/// `Provide` (deletion-pruned), children only the initiator holds as
/// `Request`, and an empty side degenerates to all-`Provide` or
/// all-`Request` with no special casing.
pub(super) fn respond<B, T>(
    backend: B,
    their_version: Version,
    root: Option<B::Node<height::Root>>,
    requests: impl Requests<message::Opening>,
    down: Sender<(Prefix<UnderUnderRoot>, Option<B::Node<UnderUnderRoot>>)>,
    level: Sender<(Prefix<UnderRoot>, Option<B::Node<UnderRoot>>)>,
) -> impl Responses<message::Exchange<B, T, UnderRoot>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    try_stream! {
        // The initiator's opening listing: at most one message, and an empty
        // initiator sends none. The parent is statically the root.
        let mut theirs = Vec::new();
        for await item in requests {
            let message::Opening::Uncertain(children) = item;
            theirs = children
                .into_iter()
                .map(|(radix, hash)| (Prefix::new().push(radix), hash))
                .collect();
        }

        match root {
            Some(node) => {
                let ours = backend.clone().children(Prefix::new(), node);
                let theirs = stream::iter(theirs.into_iter().map(Ok));
                for await item in route(backend, their_version, ours, theirs, down, level) {
                    yield item?;
                }
            }
            None => {
                // We are empty: request everything the initiator listed.
                for (_, _hash) in theirs {
                    yield message::Exchange::Requested;
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
/// Shared by [`walk`] and [`close_walk`]. A closed channel ends the
/// stream early: callers observe the teardown by checking the sender they
/// kept.
fn provide<B, T, H>(
    backend: B,
    their_version: Version,
    prefix: Prefix<S<H>>,
    node: B::Node<S<H>>,
    kept: Sender<(Prefix<S<H>>, Option<B::Node<S<H>>>)>,
) -> impl NodeStream<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
{
    try_stream! {
        let mut pruned = unknown(&backend, &their_version, one(prefix, node));
        while let Some(item) = pruned.next().await {
            let (prefix, node) = item?;
            if kept.send((prefix, Some(node.clone()))).await.is_err() {
                return;
            }
            let mut children = pin!(backend.clone().children(prefix, node));
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
/// ([`open_responder`](super::super::protocol::OpenResponder)). A closed
/// channel ends the stream early: callers observe the teardown on their own
/// next send, or by checking the senders they kept.
pub(super) fn route<B, T, H>(
    backend: B,
    their_version: Version,
    ours: impl NodeStream<B, T, S<H>> + 'static,
    theirs: impl Stream<Item = Result<(Prefix<S<H>>, Hash), B::Error>> + Send + 'static,
    down: Sender<(Prefix<H>, Option<B::Node<H>>)>,
    level: Sender<(Prefix<S<H>>, Option<B::Node<S<H>>>)>,
) -> impl Responses<message::Exchange<B, T, S<H>>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height + Unknown,
{
    try_stream! {
        for await verdict in classify(&backend, &their_version, ours, theirs) {
            match verdict? {
                // We have it, they lack it (already pruned by classify): send
                // it and keep it.
                Routed::Provide(prefix, node) => {
                    yield message::Exchange::Providing(prefix, node.clone());
                    if level.send((prefix, Some(node))).await.is_err() {
                        return;
                    }
                }
                // Hashes agree: keep ours, indicate that it matched.
                Routed::Matched(prefix, node) => {
                    yield message::Exchange::Matched;
                    if level.send((prefix, Some(node))).await.is_err() {
                        return;
                    }
                }
                // They have it, we lack it: ask for it.
                Routed::Request(_prefix) => {
                    yield message::Exchange::Requested;
                }
                // Hashes differ: descend. The children's hashes go out as one
                // `uncertain` batch; the children themselves become the next
                // stage's frontier.
                Routed::Dispute(prefix, node) => {
                    let mut children = pin!(backend.clone().children(prefix, node));

                    let mut listing = Vec::new();
                    let mut downward = Vec::new();
                    while let Some(item) = children.next().await {
                        let (child_prefix, child) = item?;
                        let (_, radix) = child_prefix.pop();
                        listing.push((radix, child.hash()));
                        downward.push((child_prefix, child));
                    }

                    yield message::Exchange::Uncertain(listing.clone());
                    for (child_prefix, child) in downward {
                        if down.send((child_prefix, Some(child))).await.is_err() {
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// Run one descent stage's reconciliation: our frontier at `S<S<H>>` against
/// the incoming wire keyed at the same height, producing the outgoing wire
/// keyed one level below.
///
/// The returned stream is the outgoing wire and the walk's engine: pulling it
/// advances the merge-join, which routes every prefix's verdict as described
/// in the [module docs](self). `down` receives disputed subtrees exploded to
/// height `H` (the next stage's frontier); `keep` receives reconciled nodes at
/// the frontier height; `level` receives reconciled children one level below.
///
/// [`protocol::Exchange`](super::super::protocol::Exchange) stages return the
/// output as-is; [`close_responder`](super::super::protocol::CloseResponder)
/// filters it down to [`message::Closing`], dropping `Uncertain` (vacuous at
/// leaf height, exactly as the alternating `Closing` does).
pub(super) fn walk<B, T, H>(
    backend: B,
    their_version: Version,
    frontier: BoxNodeStream<'static, B, T, S<S<H>>>,
    messages: impl Requests<message::Exchange<B, T, S<S<H>>>>,
    down: Sender<(Prefix<H>, Option<B::Node<H>>)>,
    keep: Sender<(Prefix<S<S<H>>>, Option<B::Node<S<S<H>>>>)>,
    level: Sender<(Prefix<S<H>>, Option<B::Node<S<H>>>)>,
) -> impl Responses<message::Exchange<B, T, S<H>>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
{
    try_stream! {
        let mut frontier = pin!(frontier);
        let mut messages = pin!(messages);

        while let Some(message) = messages.next().await {
            use message::Exchange::*;
            match message {
                Providing(prefix, node) => {
                    if keep.send((prefix, Some(node))).await.is_err() {
                        return;
                    }
                },
                Matched => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, node))) => if keep.send((prefix, Some(node))).await.is_err() {
                            return;
                        },
                    }
                },
                Requested => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, node))) => {
                            for await item in provide(
                                backend.clone(),
                                their_version.clone(),
                                prefix,
                                node,
                                keep.clone(),
                            ) {
                                let (prefix, child) = item?;
                                yield message::Exchange::Providing(prefix, child);
                            }
                            // `provide` swallows channel closure to end its own stream;
                            // for the walk it means session teardown.
                            if keep.is_closed() {
                                return;
                            }
                        },
                    }
                },
                Uncertain(children) => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, node))) => {
                            let ours = backend.clone().children(prefix, node);
                            let theirs = stream::iter(
                                children
                                    .into_iter()
                                    .map(move |(radix, hash)| Ok((prefix.push(radix), hash))),
                            );
                            for await item in route(
                                backend.clone(),
                                their_version.clone(),
                                ours,
                                theirs,
                                down.clone(),
                                level.clone(),
                            ) {
                                yield item?;
                            }
                            // `route` swallows channel closure to end its own stream;
                            // for the walk it means the session is tearing down.
                            if down.is_closed() || level.is_closed() {
                                return;
                            }
                        }
                    }
                },
            }
        }
    }
}

/// The initiator's closing walk: our frontier at `S<Z>` against the
/// responder's leaf-parent verdicts, producing the leaf-height
/// [`message::Closing`] words.
///
/// This is [`walk`] with the classify arm degenerated to leaf height. An
/// incoming `uncertain` lists the counterparty's leaves under a parent both
/// sides hold, and leaves never dispute — two leaves at one path are the
/// same leaf — so every cell of the matrix resolves in place: ours-only
/// leaves are pruned against their version and provided, theirs-only leaves
/// requested, shared leaves matched. Kept leaves flow to `down`, the
/// terminal's frontier, where the counterparty's answers join them for the
/// upward reassembly of each disputed parent; undisputed parents flow whole
/// to `keep`.
pub(super) fn close_walk<B, T>(
    backend: B,
    their_version: Version,
    frontier: BoxNodeStream<'static, B, T, S<Z>>,
    messages: impl Requests<message::Exchange<B, T, S<Z>>>,
    down: Sender<(Prefix<Z>, Option<B::Node<Z>>)>,
    keep: Sender<(Prefix<S<Z>>, Option<B::Node<S<Z>>>)>,
) -> impl Responses<message::Closing<B, T>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    try_stream! {
        let mut frontier = pin!(frontier);
        let mut messages = pin!(messages);

        while let Some(message) = messages.next().await {
            use message::Exchange::*;
            match message {
                Providing(prefix, node) => {
                    if keep.send((prefix, Some(node))).await.is_err() {
                        return;
                    }
                },
                Matched => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, node))) => if keep.send((prefix, Some(node))).await.is_err() {
                            return;
                        },
                    }
                },
                Requested => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, node))) => {
                            for await item in provide(
                                backend.clone(),
                                their_version.clone(),
                                prefix,
                                node,
                                keep.clone(),
                            ) {
                                let (prefix, child) = item?;
                                yield message::Closing::Providing(prefix, child);
                            }
                            // `provide` swallows channel closure to end its own stream;
                            // for the walk it means session teardown.
                            if keep.is_closed() {
                                return;
                            }
                        },
                    }
                },
                Uncertain(children) => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, node))) => {
                            let ours = backend.clone().children(prefix, node);
                            let theirs = stream::iter(
                                children
                                    .into_iter()
                                    .map(move |(radix, hash)| Ok::<_, B::Error>((prefix.push(radix), hash))),
                            );
                            for await cell in merge(ours, theirs) {
                                match cell? {
                                    // Ours alone: they deleted it if it is at
                                    // or before their version; otherwise
                                    // provide it and keep it.
                                    (leaf_prefix, EitherOrBoth::Left(leaf)) => {
                                        if !known(&leaf, &their_version) {
                                            yield message::Closing::Providing(
                                                leaf_prefix,
                                                leaf.clone(),
                                            );
                                            if down.send((leaf_prefix, Some(leaf))).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    // Both: two leaves at one path are the same
                                    // leaf. Say so — the word is positional,
                                    // pairing with the next leaf they listed —
                                    // and keep ours.
                                    (leaf_prefix, EitherOrBoth::Both(leaf, _hash)) => {
                                        yield message::Closing::Matched;
                                        if down.send((leaf_prefix, Some(leaf))).await.is_err() {
                                            return;
                                        }
                                    }
                                    // Theirs alone: ask for it. Their answer
                                    // prunes against our version, so a leaf we
                                    // deleted drops there instead of coming
                                    // back.
                                    (_leaf_prefix, EitherOrBoth::Right(_hash)) => {
                                        yield message::Closing::Requested;
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

/// The responder's terminal walk: pair the initiator's closing words with
/// our kept disputed leaves, answering its `Requested` leaves with the final
/// [`message::Complete`].
///
/// The frontier holds exactly the leaves we listed under disputed parents,
/// in listing order, and the initiator speaks one positional word per listed
/// leaf: `Matched` keeps ours, `Requested` answers it pruned against the
/// initiator's version — at or before it means the initiator deleted the
/// leaf, and it drops here instead of shipping. `Providing` rides keyed
/// between them, absorbing a leaf only the initiator held without consuming
/// a frontier leaf.
pub(super) fn respond_leaves<B, T>(
    their_version: Version,
    frontier: BoxNodeStream<'static, B, T, Z>,
    messages: impl Requests<message::Closing<B, T>>,
    leaves: Sender<(Prefix<Z>, Option<B::Node<Z>>)>,
) -> impl Responses<message::Complete<B, T>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    try_stream! {
        let mut frontier = pin!(frontier);
        let mut messages = pin!(messages);

        while let Some(message) = messages.next().await {
            use message::Closing::*;
            match message {
                Providing(prefix, leaf) => {
                    if leaves.send((prefix, Some(leaf))).await.is_err() {
                        return;
                    }
                },
                Matched => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, leaf))) => if leaves.send((prefix, Some(leaf))).await.is_err() {
                            return;
                        },
                    }
                },
                Requested => {
                    match frontier.next().await.transpose() {
                        Err(e) => yield Err(e)?,
                        Ok(None) => return,
                        Ok(Some((prefix, leaf))) => {
                            if !known(&leaf, &their_version) {
                                yield message::Complete::Providing(prefix, leaf.clone());
                                if leaves.send((prefix, Some(leaf))).await.is_err() {
                                    return;
                                }
                            }
                        },
                    }
                },
            }
        }
    }
}

/// The initiator's terminal absorb: merge our kept closing leaves with the
/// responder's final `providing` into the reconciled leaf level.
///
/// The two sets are disjoint: our frontier holds the leaves [`close_walk`]
/// kept — shared ones and our own survivors — while the responder provides
/// only the leaves we `Requested`, which we lack by construction. See
/// [`complete_initiator`](super::super::protocol::CompleteInitiator), which
/// drives this concurrently with the session's accumulated work.
pub(super) async fn absorb_leaves<B, T>(
    frontier: BoxNodeStream<'static, B, T, Z>,
    messages: impl Requests<message::Complete<B, T>>,
    leaves: Sender<(Prefix<Z>, Option<B::Node<Z>>)>,
) -> Result<(), B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let kept = frontier.map(|item| item.map(|(prefix, leaf)| (prefix, Some(leaf))));
    let providing = messages.map(|message| {
        let message::Complete::Providing(prefix, node) = message;
        Ok((prefix, Some(node)))
    });
    // Both inputs ascend by prefix and the reassembly above requires the
    // union in ascending order, so this is a merge, not a concatenation.
    forward::<B, T, Z>(merge_disjoint(kept, providing), leaves).await
}
