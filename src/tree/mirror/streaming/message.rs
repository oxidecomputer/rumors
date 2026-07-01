use crate::Version;
use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, Root, S, UnderRoot, Z},
};

use super::{Backend, Leaf};

// The initial handshake message:

pub enum Intent {
    Remain,
    Retire,
}

pub struct Handshake {
    pub version: Version,
}

// The three kinds of messages that can be streamed:

pub struct Providing<B: Backend<T>, T, H: Height>
where
    B::Node<Z>: Leaf<T>,
{
    pub prefix: Prefix<H>,
    pub node: B::Node<H>,
}

pub struct Requested<H: Height> {
    pub prefix: Prefix<H>,
}

pub struct Uncertain<H: Height> {
    pub prefix: Prefix<H>,
    pub hash: Hash,
}

// The five kinds of stream messages:

pub enum Initiate {
    Uncertain(Uncertain<Root>),
}

pub enum Opening {
    Uncertain(Uncertain<UnderRoot>),
}

pub enum Exchange<B: Backend<T>, T, H>
where
    B::Node<Z>: Leaf<T>,
    S<H>: Height,
    H: Height,
{
    Providing(Providing<B, T, S<H>>),
    Requested(Requested<S<H>>),
    Uncertain(Uncertain<H>),
}

pub enum Closing<B: Backend<T>, T>
where
    B::Node<Z>: Leaf<T>,
{
    Providing(Providing<B, T, S<Z>>),
    Requested(Requested<S<Z>>),
}

pub enum Complete<B: Backend<T>, T>
where
    B::Node<Z>: Leaf<T>,
{
    Providing(Providing<B, T, Z>),
}
