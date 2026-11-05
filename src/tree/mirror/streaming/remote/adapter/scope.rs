use crate::tree::mirror::streaming::byte_set::ByteSet;
use crate::tree::mirror::streaming::ledger::ReplyBound;
#[cfg(test)]
use crate::tree::mirror::streaming::ledger::{buffer, queued};
use crate::tree::mirror::streaming::message::{Grandchildren, LaterMatches, Listed};
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
/// `answered` is the record of the question the reply answers, and
/// `replay` how far the reply's earlier chunks replayed against it, which
/// this chunk continues; it returns covering this chunk too. Matches and
/// queries take a position, passing over the positions the reply's match
/// covers; supplies name their own radix and take none. At
/// [`ReplyLevel::Branch`] a query opens a scope one level down; at
/// [`ReplyLevel::Leaf`] every query must be empty and requests its leaf.
/// The decoder validated the same positions when the reply arrived, so a
/// failure here means the reply or the record changed in between.
pub fn derive_questions<E>(
    level: ReplyLevel,
    answered: &Scope,
    replay: &mut Replay,
    reactions: &[Reaction<E>],
) -> Result<Vec<Scope>, ScopeError> {
    let mut positions = answered.replay(*replay);
    let mut questions = Vec::new();
    for reaction in reactions {
        match reaction {
            Reaction::Match(later) => {
                positions.matched(later)?;
            }
            Reaction::Query(listing) => {
                questions.push(level.derive(&mut positions, listing)?);
            }
            Reaction::Supply(_, _) => {}
        }
    }
    *replay = positions.replayed();
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
    children: Vec<RecordedChild>,
    bound: Grandchildren,
    /// How the reply pairs with `children`.
    pairing: Pairing,
}

/// How the reply a record awaits pairs with the record's children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pairing {
    /// The children are a question's listing: the reply takes a reaction
    /// for each, or its match covers it, so at the reply's end every one
    /// left must be covered.
    Positional,
    /// The one child is the leaf a terminal request asks for: the reply
    /// supplies it by its radix, or omits it when the replier's deletion
    /// filter drops it, and takes no position, so the record offers its
    /// reply none.
    Requested,
}

/// How a record's reply pairs rides in the record's padding, so it raises
/// no record's price, which counts a record at its size.
const _: () = assert!(
    size_of::<Scope>() == size_of::<(ErasedPrefix, Vec<RecordedChild>, Grandchildren)>(),
    "a record's pairing must fit its padding",
);

/// One child a record names: its radix, the count its lister gave it, and,
/// in the record of a question this side asked, this side's child count of
/// it less one ([`Listed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecordedChild {
    /// The child's radix.
    radix: u8,
    /// The lister's grandchild count of the child.
    grandchildren: Grandchildren,
    /// This side's child count of the child, less one, where this side
    /// listed it; zero otherwise.
    children_less_one: u8,
}

/// The child count rides in the padding of a radix and a count, so
/// recording it raises no record's price, which counts each child at this
/// size.
const _: () = assert!(
    size_of::<RecordedChild>() == size_of::<(u8, Grandchildren)>(),
    "a recorded child's child count must fit its padding",
);

impl Scope {
    /// Record the question represented by `listing` at `parent`, whose
    /// replier's grandchild count of `parent` is `bound`.
    pub fn new(parent: ErasedPrefix, listing: &[Listed], bound: Grandchildren) -> Self {
        Self {
            parent,
            children: listing
                .iter()
                .map(|listed| RecordedChild {
                    radix: listed.radix,
                    grandchildren: listed.grandchildren,
                    children_less_one: listed.children_less_one,
                })
                .collect(),
            bound,
            pairing: Pairing::Positional,
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

    /// The bytes this record holds while it waits in a channel, as the
    /// take's record of a question the peer asked: its node in the channel
    /// and its children's buffer, at its capacity.
    #[cfg(test)]
    pub(crate) fn held_bytes(&self) -> usize {
        queued::<Self>() + buffer(&self.children)
    }

    /// The bound on the reply this record awaits: the one a question
    /// listing the record's children reserved for, or, for a leaf request,
    /// one supplied leaf ([`ReplyBound::LEAF`]), the bound the request
    /// reserved for.
    pub fn reply_bound(&self) -> ReplyBound {
        match self.pairing {
            Pairing::Positional => {
                ReplyBound::of(self.parent.height(), self.children.len(), self.bound)
            }
            Pairing::Requested => ReplyBound::LEAF,
        }
    }

    /// The parent prefix against which keyed supplies are validated.
    pub fn parent(&self) -> ErasedPrefix {
        self.parent
    }

    /// Begin replaying a reply's positional reactions against this
    /// question, from its first listed child.
    pub fn positions(&self) -> Positions<'_> {
        self.replay(Replay::default())
    }

    /// Resume replaying a reply's positional reactions against this
    /// question where `replay` left off.
    ///
    /// The cursor starts after the positions earlier reactions took or
    /// their match covered, and passes over the later positions that match
    /// covers. Past the last position, it is exhausted. A leaf request's
    /// record lists no position: its reply supplies the leaf by its radix
    /// or omits it, and asks and matches nothing.
    pub fn replay(&self, replay: Replay) -> Positions<'_> {
        let Replay {
            taken,
            covered,
            matched,
        } = replay;
        let children = match self.pairing {
            Pairing::Positional => self.children.get(taken..).unwrap_or_default(),
            Pairing::Requested => &[],
        };
        Positions {
            parent: self.parent,
            children: children.iter(),
            taken,
            covered,
            covered_ahead: children
                .iter()
                .filter(|child| covered.contains(child.radix))
                .count(),
            matched,
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
            children: vec![RecordedChild {
                radix,
                grandchildren: Grandchildren::ONE,
                children_less_one: 0,
            }],
            bound: Grandchildren::ONE,
            pairing: Pairing::Requested,
        }
    }

    /// Record the initiator's opening question about the root's children,
    /// whose replier's grandchild count of its root is `bound`.
    pub fn opening(listing: &[Listed], bound: Grandchildren) -> Self {
        Self::new(Prefix::new().erase(), listing, bound)
    }
}

/// How far a reply's positional reactions have replayed against the record
/// of the question it answers, carried from one chunk of the reply to the
/// next.
///
/// The [`Default`] replay is a reply's start: nothing taken, nothing
/// covered, no match yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Replay {
    /// The positions passed, taken by a reaction or covered by the match.
    taken: usize,
    /// The radixes the reply's match covers.
    covered: ByteSet,
    /// Whether the reply has carried its one match.
    matched: bool,
}

/// What a match took: its own position, and the child counts of the
/// children it matched.
pub(crate) struct Matched {
    /// The matched child's prefix.
    pub(crate) prefix: ErasedPrefix,
    /// This side's child counts of the matched child and of every child
    /// the match covers, in the record of a question this side asked
    /// ([`Listed`]); one for each of them in any other record.
    pub(crate) children: usize,
}

/// A cursor over one [`Scope`]'s positional children.
///
/// `Match` and `Query` reactions each consume the next position the reply's
/// match does not cover, and the cursor passes over the covered positions
/// before it, as matched; supplies are self-keying and consume none. The
/// cursor yields each position's child prefix.
pub(crate) struct Positions<'a> {
    /// The scope's parent prefix, which each child radix extends.
    parent: ErasedPrefix,
    /// The child radices not yet passed, each with its lister's count.
    children: slice::Iter<'a, RecordedChild>,
    /// The positions passed.
    taken: usize,
    /// The radixes the reply's match covers.
    covered: ByteSet,
    /// The covered positions not yet passed.
    covered_ahead: usize,
    /// Whether the reply has carried its one match.
    matched: bool,
}

/// Resolve positional reactions to child prefixes in listing order.
impl Positions<'_> {
    /// The positions not yet passed that a later reaction must take: every
    /// one the match does not cover.
    pub fn remaining(&self) -> usize {
        self.children.len() - self.covered_ahead
    }

    /// Where the replay stands, for the reply's next chunk to resume from.
    pub fn replayed(&self) -> Replay {
        Replay {
            taken: self.taken,
            covered: self.covered,
            matched: self.matched,
        }
    }

    /// Take the next position for a match, and cover the later positions
    /// it names, `later`.
    ///
    /// # Errors
    ///
    /// Returns [`ScopeError::RepeatedMatch`] if the reply already carried
    /// a match, [`ScopeError::UnpositionedMatch`] if no position is left,
    /// and [`ScopeError::UnlistedCover`] if `later` names a radix that is
    /// not a position after the match's own: one the question did not
    /// list, or one at or before the match.
    pub fn matched(&mut self, later: &LaterMatches) -> Result<Matched, ScopeError> {
        if self.matched {
            return Err(ScopeError::RepeatedMatch);
        }
        let own = self.upcoming_children();
        let (prefix, _) = self.next().ok_or(ScopeError::UnpositionedMatch)?;
        self.matched = true;
        let mut matched = Matched {
            prefix,
            children: own,
        };
        let mut covered = 0;
        let Some(set) = later.get() else {
            return Ok(matched);
        };
        let mut unlisted = *set;
        for child in self.children.as_slice() {
            if unlisted.remove(child.radix) {
                matched.children += usize::from(child.children_less_one) + 1;
                covered += 1;
            }
        }
        if let Some(radix) = unlisted.iter().next() {
            return Err(ScopeError::UnlistedCover { radix });
        }
        self.covered = *set;
        self.covered_ahead = covered;
        Ok(matched)
    }

    /// Pass over every position left, which the reply's end requires the
    /// match to cover. A leaf request's record lists none.
    ///
    /// # Errors
    ///
    /// Returns [`ScopeError::UncoveredEnd`] naming the first position left
    /// that the match does not cover.
    pub fn end(mut self) -> Result<(), ScopeError> {
        self.pass_covered();
        match self.children.next() {
            Some(child) => Err(ScopeError::UncoveredEnd { radix: child.radix }),
            None => Ok(()),
        }
    }

    /// This side's child count of the next uncovered position's child, in
    /// the record of a question this side asked ([`Listed`]); one for any
    /// other record.
    fn upcoming_children(&mut self) -> usize {
        self.pass_covered();
        self.children
            .as_slice()
            .first()
            .map_or(0, |child| usize::from(child.children_less_one) + 1)
    }

    /// Pass over the covered positions ahead of the next uncovered one.
    fn pass_covered(&mut self) {
        while let Some(child) = self.children.as_slice().first()
            && self.covered.contains(child.radix)
        {
            self.children.next();
            self.taken += 1;
            self.covered_ahead -= 1;
        }
    }
}

impl Iterator for Positions<'_> {
    type Item = (ErasedPrefix, Grandchildren);

    /// Pass over the covered positions ahead, consume the next one, and
    /// return its child prefix, with the grandchild count its lister gave
    /// it.
    fn next(&mut self) -> Option<(ErasedPrefix, Grandchildren)> {
        self.pass_covered();
        let RecordedChild {
            radix,
            grandchildren,
            ..
        } = *self.children.next()?;
        self.taken += 1;
        Some((self.parent.push(radix), grandchildren))
    }
}
