//! The asymmetry matrix, as a prefix-ordered stream transducer.
//!
//! [`classify`] is the streaming heart of one mirror round: it merge-joins our
//! frontier's children against the counterparty's `uncertain` hashes and emits,
//! per prefix in ascending order, a single [`Routed`] verdict naming which cell
//! of the asymmetry matrix that prefix fell into. It is the streaming analog of
//! the `merge_join_by` inside the alternating
//! [`partition_uncertain`](crate::tree::mirror::alternating) — the same four
//! cells, one [`Routed`] variant each — but factored so the *routing* (which
//! verdict feeds the wire, which feeds our upward reassembly) is the caller's
//! job, not the transducer's.
//!
//! # The four cells
//!
//! |                | counterparty has it                            | counterparty lacks it |
//! |----------------|------------------------------------------------|-----------------------|
//! | **we have it** | hashes agree: [`Matched`]; differ: [`Dispute`] | [`Provide`]           |
//! | **we lack it** | [`Request`]                                    | (impossible)          |
//!
//! The fourth cell (neither party has it) never reaches the merge-join: a
//! prefix absent from both inputs is never mentioned.
//!
//! [`Matched`]: Routed::Matched
//! [`Dispute`]: Routed::Dispute
//! [`Provide`]: Routed::Provide
//! [`Request`]: Routed::Request

use async_stream::try_stream;
use futures::stream::{Stream, StreamExt};
use itertools::EitherOrBoth;

use crate::Version;
use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, Z},
};

use super::backend::{Backend, Leaf, Node, NodeStream, one};
use super::merge::merge;
use super::unknown::{Unknown, unknown};

/// The asymmetry-matrix verdict for one prefix, at the height its children are
/// compared.
///
/// One variant per reachable cell (see the [module docs](self)). The caller
/// routes each to its channel: [`Provide`](Self::Provide) feeds both the
/// outgoing `providing` and our own reassembly; [`Matched`](Self::Matched)
/// feeds only reassembly; [`Request`](Self::Request) feeds the outgoing
/// `requested`; [`Dispute`](Self::Dispute) descends one level finer.
pub(super) enum Routed<B, T, C>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Height,
{
    /// We have it, the counterparty lacks it. Carries the subtree already
    /// pruned against their version (deletions honored), to `provide` to them
    /// and keep locally.
    Provide(Prefix<C>, B::Node<C>),
    /// Both have it and the hashes agree: keep our copy, say nothing on the
    /// wire.
    Matched(Prefix<C>, B::Node<C>),
    /// The counterparty has it, we lack it: request it.
    Request(Prefix<C>),
    /// Both have it but the hashes differ. Carries our copy so the caller can
    /// explode it one level finer for the next round's comparison.
    Dispute(Prefix<C>, B::Node<C>),
}

/// Merge-join our children `ours` against the counterparty's `uncertain` hashes
/// `theirs`, both ascending by prefix at height `C`, into one ascending stream
/// of [`Routed`] verdicts.
///
/// The counterparty's deletions are honored inside the [`Provide`](Routed::Provide)
/// arm: a we-have-they-lack subtree is first pruned by [`unknown`] against
/// `their_version`, so a subtree they have forgotten vanishes rather than being
/// re-provided. A subtree that prunes away entirely yields no verdict.
pub(super) fn classify<'a, B, T, C>(
    backend: &'a B,
    their_version: &'a Version,
    ours: impl NodeStream<B, T, C> + 'a,
    theirs: impl Stream<Item = Result<(Prefix<C>, Hash), B::Error>> + Send + 'a,
) -> impl Stream<Item = Result<Routed<B, T, C>, B::Error>> + Send + 'a
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync + 'a,
    T: Send + Sync + 'a,
    C: Unknown,
{
    try_stream! {
        for await cell in merge(ours, theirs) {
            match cell? {
                // We have it, they lack it: honor their deletions, then provide
                // the surviving subtree (if any) and keep it.
                EitherOrBoth::Left((prefix, node)) => {
                    let mut pruned = unknown(backend, their_version, one(prefix, node));
                    while let Some(survived) = pruned.next().await {
                        let (prefix, node) = survived?;
                        yield Routed::Provide(prefix, node);
                    }
                }
                // We lack it, they have it: request it.
                EitherOrBoth::Right((prefix, _hash)) => yield Routed::Request(prefix),
                // We both have it: agree on hash means keep it; disagree means
                // recurse one level finer.
                EitherOrBoth::Both((prefix, node), (_, hash)) => {
                    if node.hash() == hash {
                        yield Routed::Matched(prefix, node);
                    } else {
                        yield Routed::Dispute(prefix, node);
                    }
                }
            }
        }
    }
}
