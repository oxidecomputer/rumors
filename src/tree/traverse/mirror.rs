//! Bidirectional alternating mirror-sync between two replicas of the typed tree.
//!
//! Two replicas reconcile their trees while honoring deletions: leaves one side
//! has and the other has merely *forgotten* (their version is `<=` the other's
//! version vector) vanish; leaves never seen are transmitted. The protocol
//! recurses down the *disjoint frontier* of the two trees one level per message,
//! alternating sender each message, so it costs `O(log n)` round-trips and never
//! re-sends a hash the other side can already infer.

use std::convert::Infallible;

use imbl::{OrdMap, OrdSet};
use itertools::{EitherOrBoth, Itertools};

use crate::{
    Version,
    tree::{
        traverse::unknown::Unknown,
        typed::{
            Levels, Node, Prefix,
            height::{Height, Root, S, Z},
        },
    },
};

fn start<P, T, H: Mirror>(node: Node<P, T, H>) -> (H::Next, H::Request)
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    todo!()
}

pub trait Mirror: Height {
    type Request;
    type Response;
    type Next;

    fn reply<P, T>(
        other_version: &Version<P>,
        levels: impl Levels<P, T, Height = Self>,
        request: Self::Request,
    ) -> (Self::Next, Self::Response)
    where
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone;
}
