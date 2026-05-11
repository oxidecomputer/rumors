use imbl::OrdMap;
use itertools::{EitherOrBoth, Itertools};

use crate::{
    Version,
    tree::{
        traverse::unknown::Unknown,
        typed::{
            Node, Prefix,
            height::{Height, Root, S, Z},
        },
    },
};

/// Reconcile this tree against a counterparty's, returning the updated root.
///
/// `known_there` is the counterparty's version vector, exchanged before this
/// call: it's what tells us which of our leaves the counterparty has merely
/// *forgotten* (so they should vanish here too) versus never seen (so we should
/// transmit them).
///
/// The root itself pairs trivially — the equal-root-hash short-circuit is the
/// caller's business — so the recursion begins one level down, at the root's
/// children, with nothing already discovered to carry in. An empty tree just
/// explodes to an empty frontier and is reconciled by the same descent.
/// (`Mirror` is not, and can not be, implemented for `Root`: `S<Root>` is not a
/// `Height`, which is exactly why we descend one step here.)
pub async fn mirror<C, P, T>(
    known_there: &Version<P>,
    here: Option<Node<P, T, Root>>,
    counterparty: &mut C,
) -> Result<Option<Node<P, T, Root>>, C::Error>
where
    C: Counterparty<P, T>,
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    // Explode the root into the level-below frontier the recursion expects.
    let children = here
        .map(|here| here.into_children())
        .unwrap_or_default()
        .into_iter()
        .map(|(byte, child)| (Prefix::new().push(byte), child))
        .collect();

    let rebuilt = Mirror::mirror(known_there, OrdMap::new(), children, counterparty).await?;

    // Re-derive the root from its reconciled children (inverse of the explode).
    Ok(Node::branch(
        rebuilt
            .into_iter()
            .map(|(prefix, node)| (prefix.pop().0, node))
            .collect(),
    ))
}

pub trait Mirror: Height {
    async fn mirror<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: Counterparty<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone;
}

impl<H> Mirror for S<H>
where
    S<H>: Height,
    H: Unknown + Mirror,
{
    async fn mirror<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: Counterparty<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone,
    {
        let Response {
            unknown_here,
            changed_there,
        } = other
            .step(Request {
                here: here
                    .iter()
                    .map(|(prefix, node)| (prefix.clone(), node.hash()))
                    .collect(),
                unknown_there,
            })
            .await?;

        let mut down = down(known_there, here, changed_there);

        down.changed =
            Mirror::mirror(known_there, down.unknown.clone(), down.changed, other).await?;

        Ok(up(down, unknown_here))
    }
}

impl Mirror for Z {
    async fn mirror<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: Counterparty<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        let Response {
            unknown_here,
            changed_there,
        } = other
            .step(Request {
                here: here
                    .iter()
                    .map(|(prefix, node)| (prefix.clone(), node.hash()))
                    .collect(),
                unknown_there,
            })
            .await?;

        let bottom = bottom(known_there, here, changed_there, unknown_here);
        other.finalize(bottom.unknown).await?;
        Ok(bottom.mirrored)
    }
}

pub struct Request<P: Clone + Ord + AsRef<[u8]>, T, H: Height>
where
    S<H>: Height,
{
    pub unknown_there: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    pub here: OrdMap<Prefix<H>, blake3::Hash>,
}

pub struct Response<P: Clone + Ord + AsRef<[u8]>, T, H: Height> {
    pub unknown_here: OrdMap<Prefix<H>, Node<P, T, H>>,
    pub changed_there: OrdMap<Prefix<H>, blake3::Hash>,
}

pub trait Counterparty<P, T> {
    type Error;

    async fn step<H>(
        &mut self,
        request: Request<P, T, H>,
    ) -> Result<Response<P, T, H>, Self::Error>
    where
        P: Clone + Ord + AsRef<[u8]>,
        S<H>: Height,
        H: Height;

    async fn finalize(
        &mut self,
        unknown_there: OrdMap<Prefix<Z>, Node<P, T, Z>>,
    ) -> Result<(), Self::Error>
    where
        P: Clone + Ord + AsRef<[u8]>;
}

pub struct Bottom<P: Clone + Ord + AsRef<[u8]>, T> {
    pub mirrored: OrdMap<Prefix<Z>, Node<P, T, Z>>,
    pub unknown: OrdMap<Prefix<Z>, Node<P, T, Z>>,
}

/// The bottom of the recursion. There are no children to recur into, so the
/// `down`/recurse/`up` cycle collapses: classify the leaves we hold against the
/// counterparty's, reassemble in one step. Returns the reconciled leaf map and
/// the leaves we hold that the counterparty doesn't know: the leaf-level
/// analogue of every recursive level's `unknown` set, which has no next round's
/// [`Request`] to ride on and so gets shipped via [`Counterparty::finalize`].
fn bottom<P, T>(
    known_there: &Version<P>,
    here: OrdMap<Prefix<Z>, Node<P, T, Z>>,
    there: OrdMap<Prefix<Z>, blake3::Hash>,
    unknown_here: OrdMap<Prefix<Z>, Node<P, T, Z>>,
) -> Bottom<P, T>
where
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    let mut mirrored = OrdMap::new();
    let mut unknown = OrdMap::new();

    for merged in here
        .into_iter()
        .merge_join_by(there, |(p, _), (q, _)| p.cmp(q))
    {
        use EitherOrBoth::*;
        match merged {
            // The counterparty has a leaf here too. By leaf-uniqueness it is
            // byte-for-byte ours (both hashes are the leaf sentinel, hence the
            // equality `down` would test holds trivially); keep it.
            Both((prefix, here), _) => {
                mirrored.insert(prefix, here);
            }
            // We have a leaf the counterparty doesn't. `Unknown::unknown` at Z
            // reduces to the version test: `None` if the counterparty has seen
            // its version (it forgot it ⇒ we forget it, by dropping it here),
            // else `Some` ⇒ keep it and remember to ship it.
            Left((prefix, here)) => {
                if let Some(here) =
                    Unknown::unknown(Some(here), prefix.clone(), known_there, &mut |_, _, _| {})
                {
                    unknown.insert(prefix, here);
                }
            }
            // The counterparty has a leaf we don't; it's already coming back in
            // `unknown_here`, since the counterparty detected this and sent it.
            Right(_) => {
                // Do nothing
            }
        }
    }

    Bottom {
        mirrored: mirrored.union(unknown.clone()).union(unknown_here),
        unknown,
    }
}

pub struct Down<P: Clone + Ord + AsRef<[u8]>, T, H: Height>
where
    S<H>: Height,
{
    pub matched: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    pub unknown: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    pub changed: OrdMap<Prefix<H>, Node<P, T, H>>,
}

fn down<H, P, T>(
    known_there: &Version<P>,
    here: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    other: OrdMap<Prefix<S<H>>, blake3::Hash>,
) -> Down<P, T, H>
where
    H: Unknown,
    S<H>: Height,
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    let mut matched = OrdMap::new();
    let mut changed = OrdMap::new();
    let mut unknown = OrdMap::new();

    for merged in here
        .into_iter()
        .merge_join_by(other, |(p, _), (q, _)| p.cmp(q))
    {
        use EitherOrBoth::*;
        match merged {
            // If the hash is the same for the prefix, keep this node and don't
            // recur, because we're aligned on its contents
            Both((prefix, here), (_, there)) if here.hash() == there => {
                matched.insert(prefix, here);
            }
            // If the hash differs, recur on the children to diff them
            Both((prefix, here), _) => {
                for (byte, child) in here.into_children() {
                    changed.insert(prefix.clone().push(byte), child);
                }
            }
            // If the other is missing something we have, filter this subtree
            // for only things causally non-prior to the other (i.e. things it
            // can't have seen and then deleted), and transmit that set to it
            Left((prefix, here)) => {
                if let Some(here) =
                    Unknown::unknown(Some(here), prefix.clone(), known_there, &mut |_, _, _| {})
                {
                    unknown.insert(prefix, here);
                }
            }
            // If we are missing something that the other has, we should expect
            // that dually the other will discover this fact and transmit to us
            // the leaves we are missing; we do not need to explicitly request
            // it to do so
            Right(_) => {
                // No need to do anything
            }
        }
    }

    Down {
        matched,
        unknown,
        changed,
    }
}

/// Reassemble one level of the tree from its rebuilt children.
fn up<H, P, T>(
    Down {
        matched,
        unknown: unknown_there,
        changed,
    }: Down<P, T, H>,
    unknown_here: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
) -> OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>
where
    H: Height,
    S<H>: Height,
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    // Combine the matched, remote-unknown, and local-unknown fields, which
    // should all be disjoint
    let mut here = matched.union(unknown_there).union(unknown_here);

    // Bucket each rebuilt child under its parent's prefix, recovering the radix
    // it hangs off by popping the deepest byte of its own prefix.
    let mut by_parent: OrdMap<Prefix<S<H>>, OrdMap<u8, Node<P, T, H>>> = OrdMap::new();
    for (child_prefix, child) in changed {
        let (radix, parent_prefix) = child_prefix.pop();
        by_parent
            .entry(parent_prefix)
            .or_default()
            .insert(radix, child);
    }

    // Splice each parent's rebuilt children over its original children (pulling
    // the parent out of `here` as we go, so only untouched parents remain), and
    // re-derive the parent node from the merged child map.
    let mut rebuilt = OrdMap::new();
    for (parent_prefix, new_children) in by_parent {
        let old_children = here
            .remove(&parent_prefix)
            .map(Node::into_children)
            .unwrap_or_default();
        if let Some(node) = Node::branch(new_children.union(old_children)) {
            rebuilt.insert(parent_prefix, node);
        }
    }

    // Untouched parents survive verbatim.
    rebuilt.union(here)
}
