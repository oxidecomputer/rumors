#![allow(unused)]

use borsh::BorshSerialize;
use futures::Stream;

use crate::{
    Version,
    tree::{
        Message,
        typed::{
            Hash, Prefix,
            height::{Height, S, Z},
        },
    },
};

mod local;

pub trait Node {
    fn ceiling(&self) -> &Version;
    fn floor(&self) -> &Version;
    fn len(&self) -> usize;
    fn hash(&self) -> Hash;
}

pub trait Storage {
    type Node<T, H: Height>: Node;
    type Error<T, H: Height>;

    fn branches<T, H>(children: impl NodeStream<Self, T, H>) -> impl NodeStream<Self, T, S<H>>
    where
        T: Send + Sync,
        H: Height,
        S<H>: Height;

    fn children<T, H>(parents: impl NodeStream<Self, T, S<H>>) -> impl NodeStream<Self, T, H>
    where
        T: Send + Sync,
        H: Height,
        S<H>: Height;

    fn leaves<T>(leaves: impl LeafStream<Self, T>) -> impl NodeStream<Self, T, Z>
    where
        T: BorshSerialize + Send + Sync;
}

pub trait NodeStream<B: Storage + ?Sized, T, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<T, H>), B::Error<T, H>>> + Send
{
}
impl<N, B: Storage, T, H: Height> NodeStream<B, T, H> for N where
    N: Stream<Item = Result<(Prefix<H>, B::Node<T, H>), B::Error<T, H>>> + Send
{
}

pub trait LeafStream<B: Storage + ?Sized, T>:
    Stream<Item = Result<(Version, Message<T>), B::Error<T, Z>>> + Send
{
}
impl<N, B: Storage, T> LeafStream<B, T> for N where
    N: Stream<Item = Result<(Version, Message<T>), B::Error<T, Z>>> + Send
{
}
