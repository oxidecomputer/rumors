use itertools::{EitherOrBoth, Itertools};

use crate::{
    Version,
    tree::{
        mirror::streaming::{
            Backend, Leaf, Node,
            materialized::{
                Query, Resolve, Violation, children_of,
                unknown::{Unknown, known, unknown},
            },
            message::Reaction,
        },
        typed::{
            Hash, Prefix,
            height::{Height, S, Z},
        },
    },
};

/// Answer one nonempty internal query by merge-joining both child listings.
pub(super) async fn internal<B, T, H>(
    backend: &B,
    their_version: &Version,
    prefix: Prefix<S<S<H>>>,
    ours: Vec<(u8, B::Node<S<H>>)>,
    theirs: Vec<(u8, Hash)>,
) -> Result<
    (
        Vec<Reaction<B, T, S<H>>>,
        Vec<Query<B, T, H>>,
        Vec<(u8, Resolve<B, T, S<H>>)>,
    ),
    B::Error,
>
where
    B: Backend<T, Node<Z>: Leaf<T>> + Sync,
    T: Send + Sync + 'static,
    H: Unknown,
    S<H>: Unknown,
    S<S<H>>: Height,
{
    let mut reactions = Vec::new();
    let mut asked = Vec::new();
    let mut resolved = Vec::new();

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
                let prefix = prefix.push(radix);
                let ours = children_of(backend, prefix, node).await?;
                reactions.push(Reaction::Query(
                    ours.iter()
                        .map(|(radix, child)| (*radix, child.hash()))
                        .collect(),
                ));
                asked.push(Query { prefix, ours });
                resolved.push((radix, Resolve::Pending));
            }
            EitherOrBoth::Left((radix, node)) => {
                let survivor = unknown(backend, their_version, prefix.push(radix), node).await?;
                if let Some(survivor) = &survivor {
                    reactions.push(Reaction::Supply(radix, survivor.clone()));
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right((radix, _)) => {
                reactions.push(Reaction::Query(Vec::new()));
                asked.push(Query {
                    prefix: prefix.push(radix),
                    ours: Vec::new(),
                });
                resolved.push((radix, Resolve::Pending));
            }
        }
    }

    Ok((reactions, asked, resolved))
}

/// Answer one leaf-parent query by merge-joining both leaf listings.
pub(super) fn leaf_parent<B, T>(
    their_version: &Version,
    prefix: Prefix<S<Z>>,
    ours: Vec<(u8, B::Node<Z>)>,
    theirs: Vec<(u8, Hash)>,
) -> (
    Vec<Reaction<B, T, Z>>,
    Vec<Prefix<Z>>,
    Vec<(u8, Resolve<B, T, Z>)>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    let mut reactions = Vec::new();
    let mut asked = Vec::new();
    let mut resolved = Vec::new();

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
                let survivor = Some(leaf).filter(|leaf| !known(leaf, their_version));
                if let Some(leaf) = &survivor {
                    reactions.push(Reaction::Supply(radix, leaf.clone()));
                }
                resolved.push((radix, Resolve::Ready(survivor)));
            }
            EitherOrBoth::Right((radix, _)) => {
                reactions.push(Reaction::Query(Vec::new()));
                asked.push(prefix.push(radix));
                resolved.push((radix, Resolve::Pending));
            }
        }
    }

    (reactions, asked, resolved)
}

/// Answer one terminal leaf query.
pub(super) fn leaf<B, T>(
    their_version: &Version,
    radix: u8,
    node: B::Node<Z>,
    listing: Vec<(u8, Hash)>,
) -> Result<(Vec<Reaction<B, T, Z>>, Option<B::Node<Z>>), Violation>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    if !listing.is_empty() {
        return Err(Violation::UnexpectedQuery);
    }
    let node = Some(node).filter(|leaf| !known(leaf, their_version));
    let reactions = node
        .clone()
        .into_iter()
        .map(|leaf| Reaction::Supply(radix, leaf))
        .collect();
    Ok((reactions, node))
}
