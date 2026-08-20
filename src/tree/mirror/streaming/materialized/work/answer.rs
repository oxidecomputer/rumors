use itertools::{EitherOrBoth, Itertools};

use crate::{
    Version,
    tree::{
        mirror::streaming::{
            Backend, ErasedNode, Leaf,
            erased::{Reaction, ops},
            materialized::{
                Query, Resolve, Violation,
                unknown::{known, unknown},
            },
            stats::Recorder,
        },
        typed::{ErasedPrefix, Hash, Prefix, height::Z},
    },
};

/// Answer one nonempty internal query by merge-joining both child listings.
///
/// `prefix` names the queried scope; `ours` are our children of it, one
/// level below.
///
/// This merge-join is the chokepoint where
/// [`disputed_scopes`](crate::SessionStats::disputed_scopes) is counted: it
/// runs exactly once per scope this side resolves, and the scope was a
/// genuine dispute exactly when both listings were non-empty (both replicas
/// held the subtree) and some child failed to match. An all-match join is a
/// confirmation, not a dispute, and a one-sided join is a request being
/// served.
#[allow(clippy::type_complexity)]
pub(super) async fn internal<B>(
    backend: &B,
    their_version: &Version,
    prefix: ErasedPrefix,
    ours: Vec<(u8, B::Erased)>,
    theirs: Vec<(u8, Hash)>,
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
    B: Backend<Node<Z>: Leaf> + Sync,
{
    let mut reactions = Vec::new();
    let mut asked = Vec::new();
    let mut resolved = Vec::new();
    let jointly_held = !ours.is_empty() && !theirs.is_empty();
    let mut differed = false;

    for pair in ours
        .into_iter()
        .merge_join_by(theirs, |(ours, _), (theirs, _)| ours.cmp(theirs))
    {
        match pair {
            EitherOrBoth::Both((radix, node), (_, hash)) if node.hash() == hash => {
                reactions.push(Reaction::Match);
                resolved.push((radix, Resolve::Ready(Some(node))));
            }
            EitherOrBoth::Both((radix, node), _) => {
                differed = true;
                let prefix = prefix.push(radix);
                let ours = ops::children_of(backend, prefix, node).await?;
                reactions.push(Reaction::Query(
                    ours.iter()
                        .map(|(radix, child)| (*radix, child.hash()))
                        .collect(),
                ));
                asked.push(Query { prefix, ours });
                resolved.push((radix, Resolve::Pending));
            }
            EitherOrBoth::Left((radix, node)) => {
                differed = true;
                let survivor =
                    unknown(backend, their_version, prefix.push(radix), node, stats).await?;
                if let Some(survivor) = &survivor {
                    reactions.push(Reaction::Supply(radix, survivor.clone()));
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right((radix, _)) => {
                differed = true;
                reactions.push(Reaction::Query(Vec::new()));
                asked.push(Query {
                    prefix: prefix.push(radix),
                    ours: Vec::new(),
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
/// The leaf-parent twin of [`internal`]'s dispute chokepoint: a matching
/// radix here always agrees (paths are version-derived, so equal path
/// means equal leaf), so the scope was disputed exactly when both listings
/// were non-empty and some leaf sat on one side alone. Each exclusive local
/// leaf the causal filter drops is one deletion honored
/// ([`messages_shed`](crate::SessionStats::messages_shed)).
#[allow(clippy::type_complexity)]
pub(super) fn leaf_parent<E: ErasedNode + Clone>(
    their_version: &Version,
    prefix: ErasedPrefix,
    ours: Vec<(u8, E)>,
    theirs: Vec<(u8, Hash)>,
    stats: &Recorder,
) -> (Vec<Reaction<E>>, Vec<Prefix<Z>>, Vec<(u8, Resolve<E>)>) {
    let mut reactions = Vec::new();
    let mut asked = Vec::new();
    let mut resolved = Vec::new();
    let jointly_held = !ours.is_empty() && !theirs.is_empty();
    let mut differed = false;

    for pair in ours
        .into_iter()
        .merge_join_by(theirs, |(ours, _), (theirs, _)| ours.cmp(theirs))
    {
        match pair {
            EitherOrBoth::Both((radix, leaf), _) => {
                reactions.push(Reaction::Match);
                resolved.push((radix, Resolve::Ready(Some(leaf))));
            }
            EitherOrBoth::Left((radix, leaf)) => {
                differed = true;
                let survivor = Some(leaf).filter(|leaf| !known(leaf, their_version));
                if let Some(leaf) = &survivor {
                    reactions.push(Reaction::Supply(radix, leaf.clone()));
                } else {
                    stats.shed(1);
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right((radix, _)) => {
                differed = true;
                reactions.push(Reaction::Query(Vec::new()));
                // A leaf-parent scope's child prefix is a full 32-byte
                // path: the length witness makes the leaf-height re-tag
                // exact.
                asked.push(prefix.push(radix).assume::<Z>());
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
    listing: Vec<(u8, Hash)>,
    stats: &Recorder,
) -> Result<(Vec<Reaction<E>>, Option<E>), Violation> {
    if !listing.is_empty() {
        return Err(Violation::UnexpectedQuery);
    }
    let node = Some(node).filter(|leaf| !known(leaf, their_version));
    if node.is_none() {
        stats.shed(1);
    }
    let reactions = node
        .clone()
        .into_iter()
        .map(|leaf| Reaction::Supply(radix, leaf))
        .collect();
    Ok((reactions, node))
}
