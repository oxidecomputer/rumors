//! The test-only immaterial backend: a wire party's shape without a wire.
//!
//! [`Flat`] represents every subtree as its flat leaf sequence — exactly what
//! a wire format conveys — so it carries no intermediate Merkle hashes or
//! version bounds and implements [`Backend`] at
//! [`Materialized`](Backend::Materialized)` = `[`Immaterial`]. It exists to
//! pin the design boundary the materiality dispatch draws: the conversion
//! machinery (and the drivers above it) must accept it, while the session's
//! `Materialized = `[`Material`](super::Material) walks must reject it at
//! compile time.

use std::convert::Infallible;
use std::marker::PhantomData;

use async_stream::try_stream;

use crate::{
    Version,
    message::Message,
    tree::typed::{
        Path, Prefix,
        height::{Height, S, Z},
    },
};

use super::{Backend, Immaterial, Leaf, Node, NodeStream};

/// One leaf as the wire sees it: its full content-addressed key, its
/// version, and its message.
type Entry<T> = (Prefix<Z>, Version, Message<T>);

/// A run of leaves being coalesced under one open prefix.
type Run<T, H> = (Prefix<H>, Vec<Entry<T>>);

/// A [`Flat`] node at height `H`: the prefix-ordered leaf sequence beneath
/// it, and nothing else.
pub struct FlatNode<T, H> {
    leaves: Vec<Entry<T>>,
    height: PhantomData<fn() -> H>,
}

// Manual because the derive would demand `T: Clone`; `Message` shares its
// payload regardless.
impl<T, H> Clone for FlatNode<T, H> {
    fn clone(&self) -> Self {
        FlatNode {
            leaves: self.leaves.clone(),
            height: PhantomData,
        }
    }
}

impl<T, H: Height> Node<Immaterial> for FlatNode<T, H> {
    fn ceiling(&self) -> &() {
        &()
    }

    fn floor(&self) -> &() {
        &()
    }

    fn hash(&self) {}
}

impl<T> Leaf<T> for FlatNode<T, Z> {
    fn version(&self) -> &Version {
        &self.leaves[0].1
    }

    fn message(&self) -> &Message<T> {
        &self.leaves[0].2
    }

    fn leaf(version: Version, message: Message<T>) -> Self {
        // The same content-addressed derivation as a real insert
        // ([`Path::for_leaf`]), so a crossed leaf lands at the key its
        // material counterpart occupies.
        let prefix = Path::for_leaf(&version, message.bytes()).into();
        FlatNode {
            leaves: vec![(prefix, version, message)],
            height: PhantomData,
        }
    }
}

/// The immaterial in-memory backend: flat leaf sequences, re-chunked by key
/// algebra alone.
#[derive(Default, Clone, Copy, Debug)]
pub struct Flat;

impl<T: Send + Sync + 'static> Backend<T> for Flat {
    type Materialized = Immaterial;
    type Node<H: Height> = FlatNode<T, H>;
    type Error = Infallible;

    fn parents<H>(self, children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        H: Height,
        S<H>: Height,
    {
        // Children of a given parent arrive contiguously, so coalesce each
        // run of equal popped prefixes by concatenating leaf sequences.
        try_stream! {
            let mut current: Option<Run<T, S<H>>> = None;
            for await item in children {
                let (prefix, node) = item?;
                let (parent, _radix) = prefix.pop();
                match &mut current {
                    Some((open, leaves)) if *open == parent => leaves.extend(node.leaves),
                    _ => {
                        if let Some((finished, leaves)) = current.replace((parent, node.leaves)) {
                            yield (finished, FlatNode { leaves, height: PhantomData });
                        }
                    }
                }
            }
            if let Some((finished, leaves)) = current {
                yield (finished, FlatNode { leaves, height: PhantomData });
            }
        }
    }

    fn children<H>(self, parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        H: Height,
        S<H>: Height,
    {
        // A parent's leaves are ordered by full key and share its prefix, so
        // each child is a contiguous run keyed by the byte just below it.
        try_stream! {
            for await item in parents {
                let (prefix, node) = item?;
                let depth = prefix.as_bytes().len();
                let mut current: Option<(u8, Vec<Entry<T>>)> = None;
                for entry in node.leaves {
                    let radix = entry.0.as_bytes()[depth];
                    match &mut current {
                        Some((open, run)) if *open == radix => run.push(entry),
                        _ => {
                            if let Some((finished, leaves)) = current.replace((radix, vec![entry]))
                            {
                                yield (
                                    prefix.push(finished),
                                    FlatNode { leaves, height: PhantomData },
                                );
                            }
                        }
                    }
                }
                if let Some((finished, leaves)) = current {
                    yield (
                        prefix.push(finished),
                        FlatNode { leaves, height: PhantomData },
                    );
                }
            }
        }
    }
}
