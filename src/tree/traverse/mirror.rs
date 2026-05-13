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
