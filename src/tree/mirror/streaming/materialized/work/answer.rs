//! Classify one materialized query's answer, and build its questions one at
//! a time as the reply's chunks reach them.

use itertools::{EitherOrBoth, Itertools};

use super::{LeafRequest, charges};

use crate::{
    Version,
    tree::{
        mirror::streaming::{
            Backend, ErasedNode, Leaf,
            erased::{Reaction, ops},
            holding::{Holding, held_by, holding},
            ledger::{Ledger, ReplyBound, queued},
            materialized::{
                Query, Resolve, Violation, fan_listing,
                unknown::{known, unknown},
            },
            message::{Grandchildren, Listed},
            stats::Recorder,
        },
        typed::{ErasedPrefix, height::Z},
    },
};

/// One child of an answer's scope, as classification leaves it: a reaction
/// already complete, or a question whose listing is built only when the
/// question's turn in the reply comes.
pub(super) enum Planned<E, Q> {
    /// A match or a supply.
    Ready(Reaction<E>),
    /// A question, built when it is asked.
    Ask(Q),
}

/// A question an internal answer asks about one child.
pub(super) enum Question<E> {
    /// A child both sides hold with different hashes: this side's children
    /// of it, explored when it is asked, list the question.
    Dispute {
        /// The child's radix.
        radix: u8,
        /// This side's child node.
        node: E,
        /// The peer's grandchild count of the child.
        theirs: Grandchildren,
    },
    /// A child only the peer holds, requested with an empty listing.
    Request {
        /// The child's radix.
        radix: u8,
        /// The peer's grandchild count of the child.
        theirs: Grandchildren,
    },
}

/// An answer classified: its planned reactions in radix order, and the
/// resolution of its scope's children, whose questions' slots are
/// [`Resolve::Pending`].
pub(super) struct Answer<E, Q> {
    /// The reactions, questions unbuilt.
    planned: Vec<Planned<E, Q>>,
    /// The scope's child resolutions.
    pub(super) resolved: Vec<(u8, Resolve<E>)>,
}

impl<E, Q> Answer<E, Q> {
    /// Split the answer into its chunks and its child resolutions.
    pub(super) fn into_parts(self) -> (Chunks<E, Q>, Vec<(u8, Resolve<E>)>) {
        (
            Chunks {
                planned: self.planned.into_iter(),
                done: false,
            },
            self.resolved,
        )
    }
}

/// An answer's reactions in chunks: each chunk ends at a question, and the
/// last chunk holds the reactions after the last question, or ends at that
/// question when none follow.
///
/// A sender can then pause before any question until admission allows it,
/// having sent every earlier question whole, and the reply's last chunk
/// always holds its last reaction, as the encoder requires.
pub(super) struct Chunks<E, Q> {
    /// The planned reactions not yet handed out.
    planned: std::vec::IntoIter<Planned<E, Q>>,
    /// Whether the last chunk has been handed out.
    done: bool,
}

/// One chunk of an answer, its question still to build.
pub(super) struct Part<E, Q> {
    /// The complete reactions ahead of the chunk's question.
    pub(super) leading: Vec<Reaction<E>>,
    /// The question the chunk ends at, if any.
    pub(super) ask: Option<Q>,
    /// Whether the chunk ends the reply.
    pub(super) last: bool,
}

impl<E, Q> Chunks<E, Q> {
    /// The next chunk, or `None` once the last chunk has been handed out.
    ///
    /// An answer with no reactions is one empty last chunk.
    pub(super) fn next_part(&mut self) -> Option<Part<E, Q>> {
        if self.done {
            return None;
        }
        let mut leading = Vec::new();
        for planned in self.planned.by_ref() {
            match planned {
                Planned::Ready(reaction) => {
                    held_by(Holding::OutgoingReplies, || leading.push(reaction));
                }
                Planned::Ask(ask) => {
                    self.done = self.planned.len() == 0;
                    return Some(Part {
                        leading,
                        ask: Some(ask),
                        last: self.done,
                    });
                }
            }
        }
        self.done = true;
        Some(Part {
            leading,
            ask: None,
            last: true,
        })
    }
}

/// Classify one nonempty internal query's children by merge-joining both
/// child listings, exploring no disputed child.
///
/// `prefix` names the queried scope; `ours` are our children of it, one
/// level below. A child only this side holds resolves here, through the
/// deletion filter, to the supply that survives it.
///
/// This merge-join is where
/// [`disputed_scopes`](crate::SessionStats::disputed_scopes) is counted: it
/// runs exactly once per scope this side resolves, and the scope was a
/// dispute exactly when both replicas held the subtree and some child failed
/// to match. An all-match join is a confirmation; a one-sided join serves a
/// request.
pub(super) async fn internal<B>(
    backend: &B,
    their_version: &Version,
    prefix: ErasedPrefix,
    ours: Vec<(u8, B::Erased)>,
    theirs: Vec<Listed>,
    stats: &Recorder,
) -> Result<Answer<B::Erased, Question<B::Erased>>, B::Error>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut planned = Vec::new();
    let mut resolved = Vec::new();
    let jointly_held = !ours.is_empty() && !theirs.is_empty();
    let mut differed = false;

    for pair in ours
        .into_iter()
        .merge_join_by(theirs, |(ours, _), theirs| ours.cmp(&theirs.radix))
    {
        match pair {
            EitherOrBoth::Both((radix, node), theirs) if node.hash() == theirs.hash => {
                planned.push(Planned::Ready(Reaction::Match));
                resolved.push((radix, Resolve::Ready(Some(node))));
            }
            EitherOrBoth::Both((radix, node), theirs) => {
                differed = true;
                planned.push(Planned::Ask(Question::Dispute {
                    radix,
                    node,
                    theirs: theirs.grandchildren,
                }));
                resolved.push((radix, Resolve::Pending));
            }
            EitherOrBoth::Left((radix, node)) => {
                differed = true;
                let child = prefix.push(radix);
                let survivor = holding(
                    Holding::OutgoingReplies,
                    unknown(backend, their_version, child, node, stats),
                )
                .await?;
                if let Some(survivor) = &survivor {
                    planned.push(Planned::Ready(Reaction::Supply(radix, survivor.clone())));
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right(Listed {
                radix,
                grandchildren,
                ..
            }) => {
                differed = true;
                planned.push(Planned::Ask(Question::Request {
                    radix,
                    theirs: grandchildren,
                }));
                resolved.push((radix, Resolve::Pending));
            }
        }
    }

    if jointly_held && differed {
        stats.disputed_scope();
    }

    Ok(Answer { planned, resolved })
}

/// Build the question an internal answer asks about one child of `prefix`:
/// its reaction and the query it sends down, charged to `ledger`.
///
/// A disputed child is explored here, when its question is asked, so a
/// chunk holds only its own question's children.
///
/// For the session-budget meter, the reaction is work for the outgoing
/// replies, and the explored children and the query for the questions
/// ([`holding`](crate::tree::mirror::streaming::holding)).
pub(super) async fn question<B>(
    backend: &B,
    ledger: &Ledger,
    prefix: ErasedPrefix,
    question: Question<B::Erased>,
) -> Result<(Reaction<B::Erased>, Query<B::Erased>), B::Error>
where
    B: Backend<Node<Z>: Leaf>,
{
    let (child, ours, theirs, copies) = match question {
        Question::Dispute {
            radix,
            node,
            theirs,
        } => {
            let child = prefix.push(radix);
            let copies = node.explosion_bytes();
            let ours = holding(Holding::Questions, ops::children_of(backend, child, node)).await?;
            (child, ours, theirs, copies)
        }
        Question::Request { radix, theirs } => (prefix.push(radix), Vec::new(), theirs, 0),
    };
    let reaction = held_by(Holding::OutgoingReplies, || {
        Reaction::Query(fan_listing(&ours))
    });
    let bound = ReplyBound::of(child.height(), ours.len(), theirs);
    let query = held_by(Holding::Questions, || {
        charges::query(ledger, child, ours, bound, copies)
    });
    Ok((reaction, query))
}

/// Classify one leaf-parent query's leaves by merge-joining both leaf
/// listings.
///
/// The leaf-parent counterpart of [`internal`]: a matching radix here always
/// agrees (paths are version-derived, so equal path means equal leaf), so the
/// scope was disputed exactly when both listings were non-empty and some leaf
/// sat on one side alone. Each exclusive local leaf the causal filter drops is
/// one deletion honored
/// ([`messages_shed`](crate::SessionStats::messages_shed)). Every question
/// is a request for a leaf only the peer holds, planned by its radix.
pub(super) fn leaf_parent<E: ErasedNode + Clone>(
    their_version: &Version,
    ours: Vec<(u8, E)>,
    theirs: Vec<Listed>,
    stats: &Recorder,
) -> Answer<E, u8> {
    let mut planned = Vec::new();
    let mut resolved = Vec::new();
    let jointly_held = !ours.is_empty() && !theirs.is_empty();
    let mut differed = false;

    for pair in ours
        .into_iter()
        .merge_join_by(theirs, |(ours, _), theirs| ours.cmp(&theirs.radix))
    {
        match pair {
            EitherOrBoth::Both((radix, leaf), _) => {
                planned.push(Planned::Ready(Reaction::Match));
                resolved.push((radix, Resolve::Ready(Some(leaf))));
            }
            EitherOrBoth::Left((radix, leaf)) => {
                differed = true;
                let survivor = Some(leaf).filter(|leaf| !known(leaf, their_version));
                if let Some(leaf) = &survivor {
                    planned.push(Planned::Ready(Reaction::Supply(radix, leaf.clone())));
                } else {
                    stats.shed(1);
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right(Listed { radix, .. }) => {
                differed = true;
                planned.push(Planned::Ask(radix));
                resolved.push((radix, Resolve::Pending));
            }
        }
    }

    if jointly_held && differed {
        stats.disputed_scope();
    }

    Answer { planned, resolved }
}

/// Build the request a leaf-parent answer makes for the leaf at `radix`
/// under `prefix`: its empty query reaction and the leaf request it sends
/// down, charged to `ledger`.
pub(super) fn leaf_question<E>(
    ledger: &Ledger,
    prefix: ErasedPrefix,
    radix: u8,
) -> (Reaction<E>, LeafRequest) {
    let reaction = held_by(Holding::OutgoingReplies, || Reaction::Query(Vec::new()));
    // A leaf-parent scope's child prefix is a full 32-byte path: the length
    // witness makes the leaf-height re-tag exact.
    let request = held_by(Holding::Questions, || {
        (
            prefix.push(radix).assume::<Z>(),
            charges::leaf_request::<E>(ledger, queued::<LeafRequest>()),
        )
    });
    (reaction, request)
}

/// Answer one terminal leaf query.
///
/// A terminal leaf question is always a *request* (leaves cannot be
/// disputed: equal version-derived path means equal leaf, and a non-empty
/// listing here is a protocol violation), so no dispute is counted. A
/// requested leaf the causal filter drops is one deletion honored
/// ([`messages_shed`](crate::SessionStats::messages_shed)).
pub(super) fn leaf<E: ErasedNode + Clone>(
    their_version: &Version,
    radix: u8,
    node: E,
    listing: Vec<Listed>,
    stats: &Recorder,
) -> Result<(Vec<Reaction<E>>, Option<E>), Violation> {
    if !listing.is_empty() {
        return Err(Violation::UnexpectedQuery);
    }
    let node = Some(node).filter(|leaf| !known(leaf, their_version));
    if node.is_none() {
        stats.shed(1);
    }
    let reactions = held_by(Holding::OutgoingReplies, || {
        node.clone()
            .into_iter()
            .map(|leaf| Reaction::Supply(radix, leaf))
            .collect()
    });
    Ok((reactions, node))
}
