use std::iter::Peekable;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        materialized::{Error, Query, Resolution, Resolve, Violation, violation},
        message::Reaction,
    },
    typed::{
        Hash, Prefix,
        height::{Height, S, Z},
    },
};

pub struct Resolver<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>
where
    S<H>: Height,
{
    prefix: Prefix<S<H>>,
    fan: Peekable<std::vec::IntoIter<(u8, B::Node<H>)>>,
    resolved: Vec<(u8, Resolve<B, T, H>)>,
}

impl<B, T, H> Resolver<B, T, H>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    pub fn new(Query { prefix, ours }: Query<B, T, H>) -> Self {
        Self {
            prefix,
            fan: ours.into_iter().peekable(),
            resolved: Vec::new(),
        }
    }

    pub fn react(
        &mut self,
        reaction: Reaction<B, T, H>,
    ) -> Result<Option<(Prefix<S<H>>, u8, B::Node<H>, Vec<(u8, Hash)>)>, Error<B::Error>> {
        match reaction {
            Reaction::Match => {
                let Some((radix, node)) = self.fan.next() else {
                    return violation(Violation::UnexpectedMatch);
                };
                self.resolved.push((radix, Resolve::Ready(Some(node))));
            }
            Reaction::Supply(radix, node) => {
                if self.resolved.last().is_some_and(|(last, _)| radix <= *last) {
                    return violation(Violation::InvalidSupply);
                }
                match self.fan.peek() {
                    Some((next, _)) if radix == *next => {
                        return violation(Violation::UnexpectedSupply);
                    }
                    Some((next, _)) if radix > *next => {
                        return violation(Violation::InvalidSupply);
                    }
                    _ => self.resolved.push((radix, Resolve::Ready(Some(node)))),
                }
            }
            Reaction::Query(listing) => {
                let Some((radix, node)) = self.fan.next() else {
                    return violation(Violation::UnexpectedQuery);
                };
                return Ok(Some((self.prefix, radix, node, listing)));
            }
        }

        Ok(None)
    }

    pub fn ready(&mut self, radix: u8, node: Option<B::Node<H>>) {
        self.resolved.push((radix, Resolve::Ready(node)));
    }

    pub fn pending(&mut self, radix: u8) {
        self.resolved.push((radix, Resolve::Pending));
    }

    pub fn finish(mut self) -> Result<Resolution<B, T, H>, Error<B::Error>> {
        if self.fan.next().is_some() {
            violation(Violation::UnfinishedReply)
        } else {
            Ok(Resolution {
                prefix: self.prefix,
                resolved: self.resolved,
            })
        }
    }
}
