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

pub trait CounterpartyAsync<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    type Error;

    async fn step<H>(
        &mut self,
        request: Request<P, T, H>,
    ) -> Result<Response<P, T, H>, Self::Error>
    where
        S<H>: Height,
        H: Height;

    async fn complete<H>(
        &mut self,
        unknown_there: OrdMap<Prefix<H>, Node<P, T, H>>,
    ) -> Result<(), Self::Error>
    where
        H: Height;
}

pub trait CounterpartySync<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    type Error;

    fn step<H>(&mut self, request: Request<P, T, H>) -> Result<Response<P, T, H>, Self::Error>
    where
        S<H>: Height,
        H: Height;

    fn complete<H>(
        &mut self,
        unknown_there: OrdMap<Prefix<H>, Node<P, T, H>>,
    ) -> Result<(), Self::Error>
    where
        H: Height;
}

pub struct Request<P: Clone + Ord + AsRef<[u8]>, T, H: Height>
where
    S<H>: Height,
{
    /// Which subtrees at the level above (having just been discussed) are known
    /// by us to be unknown to our counterparty?
    ///
    /// This is a pipelining of messages, because this value comes from the
    /// outcome of the previous round of the protocol.
    ///
    /// Note that "there" is from the perspective of the party who sends the
    /// request.
    pub unknown_there: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// What are the hashes of all the nodes at this level which may or may not
    /// match our counterparty's view?
    ///
    /// Note that "here" is from the perspective of the party who sends the
    /// request.
    pub known_here: OrdMap<Prefix<H>, blake3::Hash>,
}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone, H: Height> Request<P, T, H>
where
    S<H>: Height,
{
    /// Make a new request based on the level here and the unknown subtrees from
    /// the previous iteration, by hashing the level here.
    pub fn new(
        here: &OrdMap<Prefix<H>, Node<P, T, H>>,
        unknown_there: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    ) -> Self {
        Request {
            known_here: here
                .iter()
                .map(|(prefix, node)| (prefix.clone(), node.hash()))
                .collect(),
            unknown_there,
        }
    }
}

pub struct Response<P: Clone + Ord + AsRef<[u8]>, T, H: Height> {
    /// Which subtrees at this level have just been discovered to be unknown to
    /// our counterparty?
    ///
    /// Note that "here" is from the perspective of the party who sent the
    /// request.
    pub unknown_here: OrdMap<Prefix<H>, Node<P, T, H>>,
    /// What are the hashes of all the nodes at this level which may or may not
    /// match our counterparty's view?
    ///
    /// Note that "there" is from the perspective of the party who sends the
    /// request.
    pub known_there: OrdMap<Prefix<H>, blake3::Hash>,
}

/// Two-way reconcile this tree against a counterparty's, returning the updated root.
pub async fn mirror_async<C, P, T>(
    known_there: &Version<P>,
    here: Option<Node<P, T, Root>>,
    counterparty: &mut C,
) -> Result<Option<Node<P, T, Root>>, C::Error>
where
    C: CounterpartyAsync<P, T>,
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

    let rebuilt = Mirror::mirror_async(known_there, OrdMap::new(), children, counterparty).await?;

    // Re-derive the root from its reconciled children (inverse of the explode).
    Ok(Node::branch(
        rebuilt
            .into_iter()
            .map(|(prefix, node)| (prefix.pop().0, node))
            .collect(),
    ))
}

/// Two-way reconcile this tree against a counterparty's, returning the updated root.
pub async fn mirror_sync<C, P, T>(
    known_there: &Version<P>,
    here: Option<Node<P, T, Root>>,
    counterparty: &mut C,
) -> Result<Option<Node<P, T, Root>>, C::Error>
where
    C: CounterpartySync<P, T>,
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

    let rebuilt = Mirror::mirror_sync(known_there, OrdMap::new(), children, counterparty)?;

    // Re-derive the root from its reconciled children (inverse of the explode).
    Ok(Node::branch(
        rebuilt
            .into_iter()
            .map(|(prefix, node)| (prefix.pop().0, node))
            .collect(),
    ))
}

pub trait Mirror: Height {
    async fn mirror_async<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartyAsync<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone;

    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone;
}

impl<H> Mirror for S<H>
where
    S<H>: Height,
    H: Unknown + Mirror,
{
    async fn mirror_async<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartyAsync<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone,
    {
        let response = other.step(Request::new(&here, unknown_there)).await?;
        let mut step = Step::down(known_there, here, response.known_there);

        // Only recur when there are non-disjoint differences to resolve:
        if !step.changed.is_empty() {
            step.changed =
                Mirror::mirror_async(known_there, step.unknown.clone(), step.changed, other)
                    .await?;
        } else {
            // Otherwise complete by sending what we know the other party doesn't know:
            other.complete(step.unknown.clone()).await?;
        }

        Ok(step.up(response.unknown_here))
    }

    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone,
    {
        let response = other.step(Request::new(&here, unknown_there))?;
        let mut step = Step::down(known_there, here, response.known_there);

        // Only recur when there are non-disjoint differences to resolve:
        if !step.changed.is_empty() {
            step.changed =
                Mirror::mirror_sync(known_there, step.unknown.clone(), step.changed, other)?;
        } else {
            // Otherwise complete by sending what we know the other party doesn't know:
            other.complete(step.unknown.clone())?;
        }

        Ok(step.up(response.unknown_here))
    }
}

impl Mirror for Z {
    async fn mirror_async<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartyAsync<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        let response = other.step(Request::new(&here, unknown_there)).await?;
        let complete = Complete::complete(
            known_there,
            here,
            response.known_there,
            response.unknown_here,
        );
        other.complete(complete.unknown).await?;
        Ok(complete.mirrored)
    }

    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        unknown_there: OrdMap<Prefix<S<Self>>, Node<P, T, S<Self>>>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        other: &mut C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        let response = other.step(Request::new(&here, unknown_there))?;
        let complete = Complete::complete(
            known_there,
            here,
            response.known_there,
            response.unknown_here,
        );
        other.complete(complete.unknown)?;
        Ok(complete.mirrored)
    }
}

pub struct Complete<P: Clone + Ord + AsRef<[u8]>, T> {
    /// All of the leaf level, including leaves unknown to the counterparty and
    /// leaves that matched.
    ///
    /// This is where we start when rolling the tree back up again.
    pub mirrored: OrdMap<Prefix<Z>, Node<P, T, Z>>,
    /// The portion of the leaf level which is known to be unknown to the
    /// counterparty.
    pub unknown: OrdMap<Prefix<Z>, Node<P, T, Z>>,
}

impl<P: Clone + Ord + AsRef<[u8]>, T> Complete<P, T> {
    /// The bottom of the recursion. There are no children to recur into, so the
    /// `down`/recurse/`up` cycle collapses: classify the leaves we hold against the
    /// counterparty's, reassemble in one step. Returns the reconciled leaf map and
    /// the leaves we hold that the counterparty doesn't know: the leaf-level
    /// analogue of every recursive level's `unknown` set, which has no next round's
    /// [`Request`] to ride on and so gets shipped via [`Counterparty::complete`].
    fn complete(
        known_there: &Version<P>,
        here: OrdMap<Prefix<Z>, Node<P, T, Z>>,
        there: OrdMap<Prefix<Z>, blake3::Hash>,
        unknown_here: OrdMap<Prefix<Z>, Node<P, T, Z>>,
    ) -> Complete<P, T>
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
                Both((prefix, here), (_, there)) => {
                    debug_assert_eq!(
                        here.hash(),
                        there,
                        "leaf uniqueness: a shared path is the same leaf, hence the same hash",
                    );
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

        // matched / our-unknown / their-unknown leaves are over disjoint prefixes:
        // each prefix is classified once, and a well-behaved counterparty never
        // sends us (in `unknown_here`) a leaf in our own frontier, i.e. one we
        // already hold. An entry lost to the unions below means it did.
        let distinct = mirrored.len() + unknown.len() + unknown_here.len();
        let matched = mirrored.union(unknown.clone()).union(unknown_here);
        debug_assert_eq!(
            matched.len(),
            distinct,
            "matched / our-unknown / their-unknown leaves overlap: a leaf we already hold was re-sent",
        );

        Complete {
            mirrored: matched,
            unknown,
        }
    }
}

pub struct Step<P: Clone + Ord + AsRef<[u8]>, T, H: Height>
where
    S<H>: Height,
{
    /// Nodes which are known to match between the two counterparties.
    pub matched: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// Nodes which are known to be unknown to our counterparty.
    pub unknown: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// For any node at height `S<H>` which was known to differ from our
    /// counterparty, its children, for processing at the next level down.
    pub changed: OrdMap<Prefix<H>, Node<P, T, H>>,
}

impl<P: Clone + Ord + AsRef<[u8]>, T, H: Height> Step<P, T, H>
where
    S<H>: Height,
{
    fn down(
        known_there: &Version<P>,
        here: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
        there: OrdMap<Prefix<S<H>>, blake3::Hash>,
    ) -> Step<P, T, H>
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
            .merge_join_by(there, |(p, _), (q, _)| p.cmp(q))
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

        Step {
            matched,
            unknown,
            changed,
        }
    }

    /// Reassemble one level of the tree from its rebuilt children.
    fn up(
        self,
        unknown_here: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    ) -> OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>
    where
        H: Height,
        S<H>: Height,
        T: Clone,
        P: Clone + Ord + AsRef<[u8]>,
    {
        let Step {
            matched,
            unknown: unknown_there,
            changed,
        } = self;

        // Combine the matched, remote-unknown, and local-unknown fields. They are
        // over disjoint prefixes: `down` classifies each prefix once, and a
        // well-behaved counterparty never sends us (in `unknown_here`) a subtree in
        // our own frontier, i.e. one we already hold. An entry lost to the unions
        // means it did.
        let distinct = matched.len() + unknown_there.len() + unknown_here.len();
        let mut here = matched.union(unknown_there).union(unknown_here);
        debug_assert_eq!(
            here.len(),
            distinct,
            "matched / our-unknown / their-unknown subtrees overlap: a subtree we already hold was re-sent",
        );

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
            debug_assert!(
                !new_children.is_empty(),
                "every reassembly bucket holds at least one rebuilt child",
            );
            // A prefix we descended into (its hashes differed) is neither matched
            // nor among the unknown subtrees on either side, so this `remove` is a
            // no-op; if it ever isn't, the counterparty re-sent a subtree we
            // already hold. (A release build still keeps what was displaced, but
            // prefers the freshly reconciled children on conflict.)
            let displaced = here.remove(&parent_prefix);
            debug_assert!(
                displaced.is_none(),
                "a descended-into prefix also appeared among the matched/unknown subtrees: it was re-sent",
            );
            let old_children = displaced.map(Node::into_children).unwrap_or_default();
            if let Some(node) = Node::branch(new_children.union(old_children)) {
                rebuilt.insert(parent_prefix, node);
            }
        }

        // Untouched parents survive verbatim.
        rebuilt.union(here)
    }
}
