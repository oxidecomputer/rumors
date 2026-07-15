use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, S, UnderRoot, Z},
};

/// The local knowledge needed to interpret one future prefix-free reply.
///
/// `parent` names the scope whose height-`H` children the reply discusses;
/// `children` preserves the positional radices from the `Query` which created
/// it. Supplies remain self-keying and therefore do not advance `next`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope<H: Height>
where
    S<H>: Height,
{
    parent: Prefix<S<H>>,
    children: Vec<u8>,
    next: usize,
}

impl<H: Height> Scope<H>
where
    S<H>: Height,
{
    /// Record the question represented by `listing` at `parent`.
    pub fn new(parent: Prefix<S<H>>, listing: &[(u8, Hash)]) -> Self {
        Self {
            parent,
            children: listing.iter().map(|(radix, _)| *radix).collect(),
            next: 0,
        }
    }

    /// The parent prefix against which keyed supplies are validated.
    pub fn parent(&self) -> Prefix<S<H>> {
        self.parent
    }

    /// Resolve the next positional reaction to its child radix and prefix.
    pub fn next(&mut self) -> Option<(u8, Prefix<H>)> {
        let radix = *self.children.get(self.next)?;
        self.next += 1;
        Some((radix, self.parent.push(radix)))
    }

    /// Resolve a keyed supply to its claimed child prefix.
    pub fn supplied(&self, radix: u8) -> Prefix<H> {
        self.parent.push(radix)
    }
}

impl Scope<Z> {
    /// Retain the one leaf position requested by a terminal empty query.
    pub fn leaf(prefix: Prefix<Z>) -> Self {
        let (parent, radix) = prefix.pop();
        Self {
            parent,
            children: vec![radix],
            next: 0,
        }
    }
}

impl Scope<UnderRoot> {
    /// Record the initiator's opening question about the root's children.
    pub fn opening(listing: &[(u8, Hash)]) -> Self {
        Self::new(Prefix::new(), listing)
    }
}
