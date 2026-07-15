//! Semantic wire frames after signal decoding.

use crate::{
    Version,
    message::Message,
    tree::typed::{Hash, hash::MERKLE_HASH_LEN},
};

use super::error::QueryOrderError;
use super::signal::{End, Flow, Stream};

/// The count byte stores one less than the nonempty query's actual fan.
pub const QUERY_COUNT_BIAS: usize = 1;

/// Largest query fan representable by a count-minus-one byte.
pub const MAX_QUERY_CHILDREN: usize = u8::MAX as usize + QUERY_COUNT_BIAS;

/// Bytes occupied by one query child: its radix followed by its Merkle hash.
pub const QUERY_CHILD_LEN: usize = std::mem::size_of::<u8>() + MERKLE_HASH_LEN;

/// Bytes occupied by the count-minus-one field of a nonempty query.
pub const QUERY_COUNT_LEN: usize = std::mem::size_of::<u8>();

/// Items in the adjacent-child window used to validate strict ordering.
const ADJACENT_CHILD_COUNT: usize = 2;

/// The body of one complete reaction frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaction<T> {
    Match,
    Query(Vec<(u8, Hash)>),
    Supply(Version, Message<T>),
}

/// A protocol reaction frame or a boundary-only frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<T> {
    /// A reaction and whether another follows in its reply.
    Reaction(Reaction<T>, Flow),
    /// An empty reply or a transport-level stream-end control.
    End(End),
}

impl<T> Frame<T> {
    /// Return the reply or stream boundary carried by this frame, if any.
    pub fn end(&self) -> Option<End> {
        match self {
            Frame::Reaction(_, Flow::Continue) => None,
            Frame::Reaction(_, Flow::End) => Some(End::Reply),
            Frame::End(end) => Some(*end),
        }
    }
}

/// A frame paired with the logical stream named by its signal byte.
pub type WireFrame<T> = (Stream, Frame<T>);

pub fn validate_children(children: &[(u8, Hash)]) -> Result<(), QueryOrderError> {
    for pair in children.windows(ADJACENT_CHILD_COUNT) {
        let [previous, current] = pair else {
            unreachable!("an adjacent-child window contains exactly two items")
        };
        if previous.0 >= current.0 {
            return Err(QueryOrderError {
                previous: previous.0,
                radix: current.0,
            });
        }
    }
    Ok(())
}
