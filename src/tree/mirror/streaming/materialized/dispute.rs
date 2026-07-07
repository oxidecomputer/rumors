//! The asymmetry matrix, as a prefix-ordered stream transducer.
//!
//! [`classify`] is the streaming heart of one mirror round: it merge-joins our
//! frontier's children against the counterparty's `uncertain` hashes and emits,
//! per prefix in ascending order, a single [`Routed`] verdict naming which cell
//! of the asymmetry matrix that prefix fell into.
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

use super::super::backend::{Backend, Leaf, Material, Node, NodeStream, one};
use super::merge::merge;
use super::unknown::{Unknown, unknown};

/// The asymmetry-matrix verdict for one prefix, at the height its children are
/// compared.
pub(super) enum Routed<B, T, C>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
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
pub(super) fn classify<'a, B, T, C>(
    backend: &'a B,
    their_version: &'a Version,
    ours: impl NodeStream<B, T, C> + 'a,
    theirs: impl Stream<Item = Result<(Prefix<C>, Hash), B::Error>> + Send + 'a,
) -> impl Stream<Item = Result<Routed<B, T, C>, B::Error>> + Send + 'a
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>> + Sync + 'a,
    T: Send + Sync + 'a,
    C: Unknown,
{
    try_stream! {
        for await cell in merge(ours, theirs) {
            match cell? {
                // We have it, they lack it: honor their deletions, then provide
                // the surviving subtree (if any) and keep it.
                (prefix, EitherOrBoth::Left(node)) => {
                    let mut pruned = unknown(backend, their_version, one(prefix, node));
                    while let Some(survived) = pruned.next().await {
                        let (prefix, node) = survived?;
                        yield Routed::Provide(prefix, node);
                    }
                }
                // We lack it, they have it: request it.
                (prefix, EitherOrBoth::Right(_)) => yield Routed::Request(prefix),
                // We both have it: agree on hash means keep it; disagree means
                // recurse one level finer.
                (prefix, EitherOrBoth::Both(node, hash)) => {
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
