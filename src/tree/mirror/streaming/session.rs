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
//! terminals ([`complete_initiator`](super::protocol::CompleteInitiator), and
//! the future returned by
//! [`complete_responder`](super::protocol::CompleteResponder)),
//! each of which resolves to its side's reconciled [`Root`].
//!
//! The stage schedule lives in [`handshake`] (the root-height phases) and
//! [`descend`] (the height-recursive rounds); the per-round walks live in
//! [`reconcile`]. This module holds what they share: the channel bound and
//! the futures plumbing every stage threads through.

use futures::channel::mpsc;
use futures::future::{self, BoxFuture};
use futures::stream::StreamExt;
use futures::{SinkExt, join};
use std::pin::pin;

use crate::{
    Version,
    tree::typed::{
        Prefix,
        height::{self, Height, S, Z},
    },
};

use super::backend::{Backend, Leaf, NodeStream, Root};
use super::merge::merge_disjoint;
use super::protocol::Messages;

mod descend;
mod handshake;
mod reconcile;

pub use handshake::Handshaking;

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
/// teardown), and never fails: errors ride the forwarded items — which is
/// why the work set's error type `X` is unconstrained here.
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
/// heights: kept frontier nodes (`keep`), walk verdicts one level below
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
fn reassemble<B, T>(
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

/// Drive the session's accumulated `work` and its root reassembly to
/// completion together, resolving to the reconciled [`Root`].
///
/// A failed worker explains anything odd about the reassembly — a truncated
/// channel chain can still fold to a plausible-looking root — so worker
/// errors outrank the fold's own result.
async fn settle<B, T, E>(
    work: Vec<BoxFuture<'static, Result<(), B::Error>>>,
    finish: BoxFuture<'static, Result<Root<B, T>, B::Error>>,
) -> Result<Root<B, T>, E>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error>,
{
    let (finished, root) = join!(future::join_all(work), finish);
    for result in finished {
        result?;
    }
    Ok(root?)
}
