use std::slice;

use crate::tree::typed::{
    ErasedPrefix, Hash, Prefix,
    height::{Height, Z},
};

use super::error::ScopeError;

/// How the queries of a reply pair with positions, and what they ask.
///
/// The level is a function of the reply's typed height
/// ([`ReplyLevel::at`]), so a caller that decodes a reply at a typed height
/// cannot pick the other level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyLevel {
    /// A query opens a scope beneath the current one.
    Branch,
    /// A query must be empty and requests the current leaf.
    Leaf,
}

impl ReplyLevel {
    /// The level of replies at typed height `H`: leaf replies at [`Z`],
    /// branch replies above it.
    pub fn at<H: Height>() -> Self {
        if H::HEIGHT == Z::HEIGHT {
            Self::Leaf
        } else {
            Self::Branch
        }
    }

    /// Validate one query at the next position and record the question it
    /// asks.
    pub(super) fn derive(
        self,
        positions: &mut Positions<'_>,
        listing: &[(u8, Hash)],
    ) -> Result<Scope, ScopeError> {
        if matches!(self, Self::Leaf) && !listing.is_empty() {
            return Err(ScopeError::NonemptyLeafQuery);
        }
        let prefix = positions.next().ok_or(ScopeError::UnpositionedQuery)?;
        Ok(match self {
            Self::Branch => Scope::new(prefix, listing),
            Self::Leaf => Scope::leaf(prefix),
        })
    }
}

/// The local knowledge needed to interpret one future prefix-free reply.
///
/// `parent` names the scope whose children the reply discusses — its byte
/// length is the scope's height witness, exactly one level above the
/// children (see [`erased`](crate::tree::mirror::streaming::erased)) —
/// and `children` preserves the positional radices from the `Query` which
/// created it. A scope is an immutable record: a reply replays against it
/// through a [`Positions`] cursor, so validating a reply and later deriving
/// its questions read the same record without copying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    parent: ErasedPrefix,
    children: Vec<u8>,
}

impl Scope {
    /// Record the question represented by `listing` at `parent`.
    pub fn new(parent: ErasedPrefix, listing: &[(u8, Hash)]) -> Self {
        Self {
            parent,
            children: listing.iter().map(|(radix, _)| *radix).collect(),
        }
    }

    /// The parent prefix against which keyed supplies are validated.
    pub fn parent(&self) -> ErasedPrefix {
        self.parent
    }

    /// Begin replaying a reply's positional reactions against this
    /// question, from its first listed child.
    pub fn positions(&self) -> Positions<'_> {
        Positions {
            parent: self.parent,
            children: self.children.iter(),
        }
    }

    /// Resolve a keyed supply to its claimed child prefix.
    pub fn supplied(&self, radix: u8) -> ErasedPrefix {
        self.parent.push(radix)
    }

    /// Retain the one leaf position requested by a terminal empty query.
    ///
    /// `prefix` is the requested leaf's full path, so the derived scope
    /// sits one level above the leaves.
    pub fn leaf(prefix: ErasedPrefix) -> Self {
        debug_assert_eq!(
            prefix.height(),
            0,
            "a terminal request names a full leaf path",
        );
        let (parent, radix) = prefix.pop();
        Self {
            parent,
            children: vec![radix],
        }
    }

    /// Record the initiator's opening question about the root's children.
    pub fn opening(listing: &[(u8, Hash)]) -> Self {
        Self::new(Prefix::new().erase(), listing)
    }
}

/// A cursor over one [`Scope`]'s positional children.
///
/// `Match` and `Query` reactions each consume the next position; supplies
/// are self-keying and consume none. The cursor yields each position's
/// child prefix.
pub(crate) struct Positions<'a> {
    /// The scope's parent prefix, which each child radix extends.
    parent: ErasedPrefix,
    /// The child radices not yet consumed.
    children: slice::Iter<'a, u8>,
}

/// Resolve positional reactions to child prefixes in listing order.
impl Iterator for Positions<'_> {
    type Item = ErasedPrefix;

    /// Consume the next position and return its child prefix.
    fn next(&mut self) -> Option<ErasedPrefix> {
        let radix = *self.children.next()?;
        Some(self.parent.push(radix))
    }
}
