use imbl::{OrdMap, OrdSet};
use itertools::{EitherOrBoth, Itertools};

use crate::{
    Version,
    tree::{
        traverse::unknown::Unknown,
        typed::{
            Node, Prefix,
            height::{Height, Pred, Root, S, Z},
        },
    },
};

pub trait CounterpartySync<P, T, H>: Sized
where
    H: Height,
    P: Clone + Ord + AsRef<[u8]>,
{
    type Error;
    type Next;

    fn root(self, m: Start<H>) -> Result<(Self::Next, Exchange<P, T, H::Pred>), Self::Error>
    where
        Self::Next: CounterpartySync<P, T, <H::Pred as Pred>::Pred>,
        H: Pred,
        H::Pred: Pred,
        S<<H as Pred>::Pred>: Height;

    fn middle(
        self,
        m: Exchange<P, T, H>,
    ) -> Result<(Self::Next, Exchange<P, T, H::Pred>), Self::Error>
    where
        Self::Next: CounterpartySync<P, T, <H::Pred as Pred>::Pred>,
        H: Pred,
        H::Pred: Pred,
        S<<H as Pred>::Pred>: Height,
        S<H>: Height;

    fn one(self, m: Exchange<P, T, H>) -> Result<Complete<P, T, H::Pred>, Self::Error>
    where
        H: Pred,
        S<H>: Height;

    fn zero(self, m: Complete<P, T, H>) -> Result<(), Self::Error>;
}

pub struct Start<H>
where
    H: Height,
{
    uncertain: OrdMap<Prefix<H>, blake3::Hash>,
}

pub struct Exchange<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]>,
    H: Height,
    S<H>: Height,
{
    requested: OrdSet<Prefix<S<H>>>,
    providing: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    uncertain: OrdMap<Prefix<H>, blake3::Hash>,
}

pub struct Complete<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]>,
    H: Height,
{
    providing: OrdMap<Prefix<H>, Node<P, T, H>>,
}

/// Two-way reconcile this tree against a counterparty's, returning the updated root.
pub async fn mirror_sync<C, P, T>(
    known_there: &Version<P>,
    here: Option<Node<P, T, Root>>,
    counterparty: C,
) -> Result<Option<Node<P, T, Root>>, C::Error>
where
    C: CounterpartySync<P, T, Root>,
    C::Next: CounterpartySync<P, T, <<Root as Pred>::Pred as Pred>::Pred>,
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    let (next, exchange) = counterparty.root(Start {
        uncertain: OrdMap::from_iter(here.as_ref().map(|n| (Prefix::new(), n.hash()))),
    })?;

    todo!()
}

pub trait Mirror: Height {
    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        counterparty: C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T, Self>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone;
}

impl<H> Mirror for S<S<H>>
where
    S<S<H>>: Height,
    H: Unknown + Mirror,
{
    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        counterparty: C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T, Self>,
        P: Clone + Ord + AsRef<[u8]>,
        S<Self>: Height,
        T: Clone,
    {
        todo!()
    }
}

impl Mirror for S<Z> {
    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        counterparty: C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T, Self>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        todo!()
    }
}

impl Mirror for Z {
    fn mirror_sync<C, P, T>(
        known_there: &Version<P>,
        here: OrdMap<Prefix<Self>, Node<P, T, Self>>,
        counterparty: C,
    ) -> Result<OrdMap<Prefix<Self>, Node<P, T, Self>>, C::Error>
    where
        C: CounterpartySync<P, T, Self>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        todo!()
    }
}
