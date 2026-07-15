//! Semantic wire frames after signal decoding.

use crate::{Version, message::Message, tree::typed::Hash};

use super::error::QueryOrderError;
use super::signal::{End, Flow, Stream};

/// The body of one complete reaction frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reaction<T> {
    Match,
    Query(Vec<(u8, Hash)>),
    Supply(Version, Message<T>),
}

/// The body and boundary state of one complete wire frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<T> {
    /// A reaction and whether another follows in its reply.
    Reaction(Reaction<T>, Flow),
    /// An empty reply or stream.
    End(End),
}

/// A frame paired with the logical stream named by its signal byte.
pub type WireFrame<T> = (Stream, Frame<T>);

pub(super) fn validate_children(children: &[(u8, Hash)]) -> Result<(), QueryOrderError> {
    for pair in children.windows(2) {
        if pair[0].0 >= pair[1].0 {
            return Err(QueryOrderError {
                previous: pair[0].0,
                radix: pair[1].0,
            });
        }
    }
    Ok(())
}
