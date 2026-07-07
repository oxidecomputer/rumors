//! The subtree leaf codec: one node's leaves as a flat run of
//! `(version, message)` pairs, their paths derived on receipt.
//!
//! A `providing` subtree never crosses the wire whole: it streams as
//! bounded per-leaf items, so the driver can interleave other levels'
//! traffic between them and neither side ever materializes a wire-form
//! subtree. The encoder explodes the node to its leaf stream through the
//! sending party's backend ([`Convert::explode`]); the decoder reassembles
//! the leaves through the receiving party's ([`Convert::assemble`]).
//!
//! # Negative space: no structure travels
//!
//! The run carries no radix tags, no prefixes, no shape of any kind,
//! because the tree is content-addressed: a leaf's whole 32-byte path is
//! [`Path::for_leaf`] of exactly the `(version, message)` pair its item
//! already carries, so the receiver derives every path locally — the same
//! derivation the sender's own insert once performed. Deriving instead of
//! trusting makes placement *self-certifying*: a counterparty cannot put
//! content anywhere but where its hash says it lives, which closes off the
//! silent tree corruption (a permanent split-brain; see [`Path::for_leaf`])
//! that a structure-bearing encoding would have to detect — and could only
//! detect by recomputing these same hashes. It is also the smallest
//! possible encoding: shipping the trie's shape spends tens of structural
//! bytes per leaf (even path-compressed, a random-keyed leaf's ~30-byte
//! private chain must be spelled out) to assert what the leaf's content
//! already proves. The price is three short blake3 invocations per received
//! leaf.
//!
//! # Grammar
//!
//! Heights are static wherever a subtree travels, so a run needs no opening
//! marker — its first leaf item opens it, and its derived path names the
//! subtree ([`decode`] *returns* the prefix rather than being told it). At
//! height `0` a subtree is exactly one leaf item and nothing more; at any
//! greater height it is one or more leaf items, strictly ascending in
//! derived path, all sharing the `32 − h` prefix bytes, terminated by the
//! level vocabulary's `end` item (represented to [`decode`] as its source
//! running dry).

use std::pin::pin;

use async_stream::try_stream;
use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt, future};

use crate::Version;
use crate::message::Message;
use crate::tree::mirror::Error;
use crate::tree::mirror::streaming::FAN;
use crate::tree::typed::{Path, Prefix, height::Z};

use super::super::backend::{Backend, Keyed, Leaf, one};
use super::super::convert::Convert;
use super::Violation;

/// Encode the subtree at `prefix` as its leaf run: each leaf's
/// `(version, message)` pair, in ascending path order.
///
/// The node explodes to leaves through `backend` ([`Convert::explode`]), so
/// this runs in the subtree's size in time and one fan in memory, like every
/// other whole-subtree pass. Errors are the backend's own, raised while
/// exploding. The run's terminator is the level vocabulary's to send: the
/// stream simply ends.
pub(in super::super) fn encode<B, T, H>(
    backend: B,
    prefix: Prefix<H>,
    node: B::Node<H>,
) -> impl Stream<Item = Result<(Version, Message<T>), B::Error>> + Send
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
{
    try_stream! {
        let mut leaves = pin!(H::explode(backend, Box::pin(one(prefix, node))));
        while let Some(item) = leaves.next().await {
            let (_prefix, leaf) = item?;
            yield (leaf.version().clone(), leaf.message().clone());
        }
    }
}

/// An asynchronous source of one subtree's leaf run.
///
/// [`decode`] pulls through this, letting a level adapter lend out its own
/// incoming stream without handing over ownership. `None` is the run's
/// end-of-subtree terminator; a source whose underlying stream dies
/// mid-run reports [`Violation::Truncated`] itself rather than fabricating
/// an end.
pub(in super::super) trait Leaves<T>: Send {
    /// Pull the run's next leaf, or `None` at the run's end.
    fn next(
        &mut self,
    ) -> impl Future<Output = Result<Option<(Version, Message<T>)>, Violation>> + Send;
}

/// Decode one leaf run from `leaves`, reassembling the subtree through
/// `backend` and deriving where it belongs.
///
/// Each leaf's path is re-derived from its content ([`Path::for_leaf`]);
/// the first leaf's path names the subtree, and the derived
/// [`Prefix`] returns alongside the node — the wire never asserts
/// placement, the leaves prove it. Feeding runs concurrently with
/// [`Convert::assemble`] through a [`FAN`]-bounded channel, as
/// [`convert`](super::super::convert) does when crossing backends. Wire
/// faults ([`Violation`]) return in the first position, the reassembling
/// backend's own faults in the second.
///
/// Everything reaching the backend is validated first: strictly ascending
/// derived paths (the [`assemble`](Convert::assemble) contract), every leaf
/// under the first one's prefix, and at least one leaf in the run. At
/// height `0` the run is statically a single leaf: exactly one is pulled
/// and no terminator is expected.
pub(in super::super) async fn decode<B, T, H>(
    backend: &B,
    leaves: &mut impl Leaves<T>,
) -> Result<(Prefix<H>, B::Node<H>), Error<Violation, B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Convert,
{
    let (mut tx, rx) = mpsc::channel::<Result<Keyed<B, T, Z>, B::Error>>(FAN);

    let feed = async move {
        // The subtree's derived prefix and the previous leaf's full path,
        // set by the first leaf.
        let mut derived: Option<(Prefix<H>, [u8; 32])> = None;
        loop {
            let Some((version, message)) = leaves.next().await? else {
                break;
            };
            let path = Path::for_leaf(&version, message.bytes());
            let bytes = <[u8; 32]>::from(path);
            match &mut derived {
                None => derived = Some((Prefix::containing(&path), bytes)),
                Some((prefix, last)) => {
                    if prefix.as_bytes() != &bytes[..32 - H::HEIGHT] {
                        return Err(Violation::Misplaced);
                    }
                    if *last >= bytes {
                        return Err(Violation::LeafOrder);
                    }
                    *last = bytes;
                }
            }
            let leaf = <B::Node<Z> as Leaf<T>>::leaf(version, message);
            if tx.send(Ok((Prefix::from(path), leaf))).await.is_err() {
                // The assembly half stopped pulling: its own failure already
                // ends the decode, so stop feeding it.
                break;
            }
            // Height 0: the run is statically this one leaf.
            if H::HEIGHT == 0 {
                break;
            }
        }
        match derived {
            Some((prefix, _last)) => Ok(prefix),
            None => Err(Violation::EmptySubtree),
        }
    };

    let build = async {
        let mut nodes = pin!(H::assemble(backend.clone(), Box::pin(rx)));
        nodes.next().await
    };

    let (fed, built) = future::join(feed, build).await;
    // A violation truncates the leaf stream, which explains anything odd in
    // the assembly, so it outranks whatever the build half produced.
    let prefix = fed.map_err(Error::Client)?;
    let node = built
        .expect("a nonempty leaf run reassembles to exactly one node")
        .map(|(_prefix, node)| node)
        .map_err(Error::Server)?;
    Ok((prefix, node))
}

#[cfg(test)]
mod tests;
