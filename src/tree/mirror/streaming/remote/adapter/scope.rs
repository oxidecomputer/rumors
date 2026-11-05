use crate::tree::mirror::streaming::ledger::ReplyBound;
use crate::tree::mirror::streaming::message::{Grandchildren, Listed};
use std::slice;

use crate::tree::{
    mirror::streaming::erased::Reaction,
    typed::{
        ErasedPrefix, Prefix,
        height::{Height, Z},
    },
};

use super::error::ScopeError;

/// Derive the scopes a chunk of a reply's queries create, in wire order.
///
/// `answered` is the record of the question the reply answers, and `taken`
/// counts the record's positions the reply's earlier chunks took, which
/// this chunk's continue from; it returns counting this chunk's too.
/// Matches and queries take a position; supplies name their own radix and
/// take none. At [`ReplyLevel::Branch`] a query opens a scope one level
/// down; at [`ReplyLevel::Leaf`] every query must be empty and requests its
/// leaf. The decoder validated the same positions when the reply arrived,
/// so a failure here means the reply or the record changed in between.
pub fn derive_questions<E>(
    level: ReplyLevel,
    answered: &Scope,
    taken: &mut usize,
    reactions: &[Reaction<E>],
) -> Result<Vec<Scope>, ScopeError> {
    let mut positions = answered.positions_from(*taken);
    let mut questions = Vec::new();
    for reaction in reactions {
        match reaction {
            Reaction::Match => {
                positions.next().ok_or(ScopeError::UnpositionedMatch)?;
                *taken += 1;
            }
            Reaction::Query(listing) => {
                questions.push(level.derive(&mut positions, listing)?);
                *taken += 1;
            }
            Reaction::Supply(_, _) => {}
        }
    }
    Ok(questions)
}

/// How the queries of a reply pair with positions, and what they ask.
///
/// The level is a function of the reply's typed height
/// ([`ReplyLevel::at`]), so a caller that decodes a reply and derives its
/// questions at one typed height uses one level for both.
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

    /// Validate one query at the next position and return that position:
    /// its prefix and the count its lister gave it, building nothing.
    ///
    /// The decoder calls this at each query's frame; [`Self::derive`] adds
    /// only the construction of the scope the query creates.
    pub(super) fn check(
        self,
        positions: &mut Positions<'_>,
        listing: &[Listed],
    ) -> Result<(ErasedPrefix, Grandchildren), ScopeError> {
        if matches!(self, Self::Leaf) && !listing.is_empty() {
            return Err(ScopeError::NonemptyLeafQuery);
        }
        positions.next().ok_or(ScopeError::UnpositionedQuery)
    }

    /// Validate one query at the next position and record the question it
    /// asks.
    ///
    /// The question's reply is bounded by the count its position's lister
    /// gave the child it asks about: the replier's own count, since the
    /// replier listed the child in the question this reply answers.
    pub(super) fn derive(
        self,
        positions: &mut Positions<'_>,
        listing: &[Listed],
    ) -> Result<Scope, ScopeError> {
        let (prefix, bound) = self.check(positions, listing)?;
        Ok(match self {
            Self::Branch => Scope::new(prefix, listing, bound),
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
/// created it, each with the count its lister gave it. A scope is an
/// immutable record: a reply replays against it through a [`Positions`]
/// cursor, so validating a reply and later deriving its questions read the
/// same record without copying it.
///
/// `bound` is the replier's grandchild count of `parent`, which bounds the
/// reply the record awaits ([`ReplyBound`]); a peer's question records the
/// asker's count of the node it asks about, where the asker is the side
/// that listed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    parent: ErasedPrefix,
    children: Vec<(u8, Grandchildren)>,
    bound: Grandchildren,
}

impl Scope {
    /// Record the question represented by `listing` at `parent`, whose
    /// replier's grandchild count of `parent` is `bound`.
    pub fn new(parent: ErasedPrefix, listing: &[Listed], bound: Grandchildren) -> Self {
        Self {
            parent,
            children: listing
                .iter()
                .map(|listed| (listed.radix, listed.grandchildren))
                .collect(),
            bound,
        }
    }

    /// Record the question represented by `listing` at `parent`, bounded
    /// by the largest grandchild count, for tests of everything but the
    /// bound.
    #[cfg(test)]
    pub fn unbounded(parent: ErasedPrefix, listing: &[Listed]) -> Self {
        Self::new(parent, listing, Grandchildren::MAX)
    }

    /// Record the initiator's opening question about the root's children,
    /// bounded by the largest grandchild count, for tests of everything
    /// but the bound.
    #[cfg(test)]
    pub fn opening_unbounded(listing: &[Listed]) -> Self {
        Self::opening(listing, Grandchildren::MAX)
    }

    /// The bound on the reply this record awaits.
    pub fn reply_bound(&self) -> ReplyBound {
        ReplyBound::of(self.parent.height(), self.children.len(), self.bound)
    }

    /// The parent prefix against which keyed supplies are validated.
    pub fn parent(&self) -> ErasedPrefix {
        self.parent
    }

    /// Begin replaying a reply's positional reactions against this
    /// question, from its first listed child.
    pub fn positions(&self) -> Positions<'_> {
        self.positions_from(0)
    }

    /// Resume replaying a reply's positional reactions against this
    /// question after the first `taken` positions, which earlier reactions
    /// took; past the last position, the cursor is exhausted.
    pub fn positions_from(&self, taken: usize) -> Positions<'_> {
        Positions {
            parent: self.parent,
            children: self.children.get(taken..).unwrap_or_default().iter(),
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
            children: vec![(radix, Grandchildren::ONE)],
            bound: Grandchildren::ONE,
        }
    }

    /// Record the initiator's opening question about the root's children,
    /// whose replier's grandchild count of its root is `bound`.
    pub fn opening(listing: &[Listed], bound: Grandchildren) -> Self {
        Self::new(Prefix::new().erase(), listing, bound)
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
    /// The child radices not yet consumed, each with its lister's count.
    children: slice::Iter<'a, (u8, Grandchildren)>,
}

/// Resolve positional reactions to child prefixes in listing order.
impl Iterator for Positions<'_> {
    type Item = (ErasedPrefix, Grandchildren);

    /// Consume the next position and return its child prefix, with the
    /// grandchild count its lister gave it.
    fn next(&mut self) -> Option<(ErasedPrefix, Grandchildren)> {
        let (radix, grandchildren) = *self.children.next()?;
        Some((self.parent.push(radix), grandchildren))
    }
}
