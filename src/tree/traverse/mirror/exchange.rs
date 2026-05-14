use std::convert::Infallible;

use crate::{
    Version,
    tree::{
        traverse::unknown::Unknown,
        typed::{
            Levels, Node, Prefix,
            height::{Height, Pred, Root, S, Z},
            levels::{Below, Top},
        },
    },
};

use super::message;

/// The height just under the root, i.e. 31.
type UnderRoot = <Root as Pred>::Pred;

pub struct Exchange<P, L>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    levels: L,
    other_version: Version<P>,
}

/// The initiator's start of the protocol.
pub fn initiator<P, T>(
    node: Option<Node<P, T, Root>>,
    other_version: Version<P>,
) -> (message::Start, Exchange<P, Top<P, T>>)
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    todo!()
}

/// The responder's start of the protocol.
pub fn responder<P, T>(
    node: Option<Node<P, T, Root>>,
    other_version: Version<P>,
    start: message::Start,
) -> (
    message::Exchange<P, T, UnderRoot>,
    Result<Exchange<P, Below<P, T, UnderRoot, Top<P, T>>>, Option<Node<P, T, Root>>>,
)
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    todo!()
}

impl<P, L> Exchange<P, L>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    /// The symmetric middle of the protocol.
    pub fn exchange<T, H>(
        self,
        exchange: message::Exchange<P, T, S<S<H>>>,
    ) -> (
        message::Exchange<P, T, S<H>>,
        Result<Exchange<P, Below<P, T, S<H>, Below<P, T, S<S<H>>, L>>>, Option<Node<P, T, Root>>>,
    )
    where
        L: Levels<P, T, Height = S<S<S<H>>>>,
        S<S<S<H>>>: Height,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height,
        T: Clone,
    {
        todo!()
    }

    /// The initiator's end of the protocol.
    pub fn complete_initiator<T>(
        self,
        exchange: message::Exchange<P, T, S<Z>>,
    ) -> (
        message::Complete<P, T>,
        Result<Infallible, Option<Node<P, T, Root>>>,
    )
    where
        L: Levels<P, T, Height = S<S<Z>>>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        todo!()
    }

    /// The responder's end of the protocol.
    pub fn complete_responder<T>(
        self,
        complete: message::Complete<P, T>,
    ) -> Result<Infallible, Option<Node<P, T, Root>>>
    where
        L: Levels<P, T, Height = S<Z>>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        todo!()
    }
}
