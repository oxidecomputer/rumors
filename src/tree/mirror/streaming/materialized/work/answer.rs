//! Build replies and local resolutions for one materialized query.

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
            message::Listed,
            stats::Recorder,
        },
        typed::{ErasedPrefix, height::Z},
    },
};

/// Answer one nonempty internal query by merge-joining both child listings.
///
/// `prefix` names the queried scope; `ours` are our children of it, one
/// level below.
///
/// For the session-budget meter, the reply's reactions are work for the
/// outgoing replies, the questions and the children they list for the
/// questions, and the resolution's slots for whatever place the caller
/// polls this from ([`holding`](crate::tree::mirror::streaming::holding)).
///
/// This merge-join is where
/// [`disputed_scopes`](crate::SessionStats::disputed_scopes) is counted: it
/// runs exactly once per scope this side resolves, and the scope was a
/// dispute exactly when both replicas held the subtree and some child failed
/// to match. An all-match join is a confirmation; a one-sided join serves a
/// request.
pub(super) async fn internal<B>(
    backend: &B,
    ledger: &Ledger,
    their_version: &Version,
    prefix: ErasedPrefix,
    ours: Vec<(u8, B::Erased)>,
    theirs: Vec<Listed>,
    stats: &Recorder,
) -> Result<
    (
        Vec<Reaction<B::Erased>>,
        Vec<Query<B::Erased>>,
        Vec<(u8, Resolve<B::Erased>)>,
    ),
    B::Error,
>
where
    B: Backend<Node<Z>: Leaf>,
{
    let mut reactions = Vec::new();
    let mut asked = Vec::new();
    let mut resolved = Vec::new();
    let jointly_held = !ours.is_empty() && !theirs.is_empty();
    let mut differed = false;

    for pair in ours
        .into_iter()
        .merge_join_by(theirs, |(ours, _), theirs| ours.cmp(&theirs.radix))
    {
        match pair {
            EitherOrBoth::Both((radix, node), theirs) if node.hash() == theirs.hash => {
                held_by(Holding::OutgoingReplies, || reactions.push(Reaction::Match));
                resolved.push((radix, Resolve::Ready(Some(node))));
            }
            EitherOrBoth::Both((radix, node), theirs) => {
                differed = true;
                let prefix = prefix.push(radix);
                let copies = node.explosion_bytes();
                let ours =
                    holding(Holding::Questions, ops::children_of(backend, prefix, node)).await?;
                held_by(Holding::OutgoingReplies, || {
                    reactions.push(Reaction::Query(fan_listing(&ours)));
                });
                let bound = ReplyBound::of(prefix.height(), ours.len(), theirs.grandchildren);
                held_by(Holding::Questions, || {
                    asked.push(charges::query(ledger, prefix, ours, bound, copies));
                });
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
                    held_by(Holding::OutgoingReplies, || {
                        reactions.push(Reaction::Supply(radix, survivor.clone()));
                    });
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right(Listed {
                radix,
                grandchildren,
                ..
            }) => {
                differed = true;
                held_by(Holding::OutgoingReplies, || {
                    reactions.push(Reaction::Query(Vec::new()));
                });
                let prefix = prefix.push(radix);
                let bound = ReplyBound::of(prefix.height(), 0, grandchildren);
                held_by(Holding::Questions, || {
                    asked.push(charges::query(ledger, prefix, Vec::new(), bound, 0));
                });
                resolved.push((radix, Resolve::Pending));
            }
        }
    }

    if jointly_held && differed {
        stats.disputed_scope();
    }

    Ok((reactions, asked, resolved))
}

/// Answer one leaf-parent query by merge-joining both leaf listings.
///
/// The leaf-parent counterpart of [`internal`]: a matching radix here always
/// agrees (paths are version-derived, so equal path means equal leaf), so the
/// scope was disputed exactly when both listings were non-empty and some leaf
/// sat on one side alone. Each exclusive local leaf the causal filter drops is
/// one deletion honored
/// ([`messages_shed`](crate::SessionStats::messages_shed)).
pub(super) fn leaf_parent<E: ErasedNode + Clone>(
    ledger: &Ledger,
    their_version: &Version,
    prefix: ErasedPrefix,
    ours: Vec<(u8, E)>,
    theirs: Vec<Listed>,
    stats: &Recorder,
) -> (Vec<Reaction<E>>, Vec<LeafRequest>, Vec<(u8, Resolve<E>)>) {
    let mut reactions = Vec::new();
    let mut asked = Vec::new();
    let mut resolved = Vec::new();
    let jointly_held = !ours.is_empty() && !theirs.is_empty();
    let mut differed = false;

    for pair in ours
        .into_iter()
        .merge_join_by(theirs, |(ours, _), theirs| ours.cmp(&theirs.radix))
    {
        match pair {
            EitherOrBoth::Both((radix, leaf), _) => {
                held_by(Holding::OutgoingReplies, || reactions.push(Reaction::Match));
                resolved.push((radix, Resolve::Ready(Some(leaf))));
            }
            EitherOrBoth::Left((radix, leaf)) => {
                differed = true;
                let survivor = Some(leaf).filter(|leaf| !known(leaf, their_version));
                if let Some(leaf) = &survivor {
                    held_by(Holding::OutgoingReplies, || {
                        reactions.push(Reaction::Supply(radix, leaf.clone()));
                    });
                } else {
                    stats.shed(1);
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right(Listed { radix, .. }) => {
                differed = true;
                held_by(Holding::OutgoingReplies, || {
                    reactions.push(Reaction::Query(Vec::new()));
                });
                // A leaf-parent scope's child prefix is a full 32-byte
                // path: the length witness makes the leaf-height re-tag
                // exact.
                held_by(Holding::Questions, || {
                    let request = prefix.push(radix).assume::<Z>();
                    asked.push((
                        request,
                        charges::leaf_request::<E>(ledger, queued::<LeafRequest>()),
                    ));
                });
                resolved.push((radix, Resolve::Pending));
            }
        }
    }

    if jointly_held && differed {
        stats.disputed_scope();
    }

    (reactions, asked, resolved)
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
