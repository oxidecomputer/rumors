use imbl::{OrdMap, OrdSet};

use crate::tree::typed::{
    Node, Prefix,
    height::{Height, S, Z},
};

pub struct Start {
    /// The root hash of our tree; we do not know whether it matches the other
    /// party's root hash.
    pub uncertain: blake3::Hash,
}

pub struct Exchange<P: Clone + Ord + AsRef<[u8]>, T, H: Height>
where
    S<H>: Height,
{
    /// Nodes which we know the other party does not have, since they
    /// `requested` them in the previous round, which we are now sending to them
    /// as they requested.
    ///
    /// We have also filtered these nodes in our own tree and in the provided
    /// ones to only those which are *causally non-prior* to the other party's
    /// version, because if the other party lacked these prefixes entirely,
    /// anything which existed under this prefix causally prior to the other
    /// party's version must have been forgotten; dually, anything causally
    /// non-prior to the other party's version must have been added after those
    /// deletions, and they do not yet know about it.
    pub providing: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// Prefixes which we know the other party has, since they told us about the
    /// hash in the previous round's `uncertain`, but which we do not have at
    /// all, so we request them to send to us.
    pub requested: OrdSet<Prefix<S<H>>>,
    /// Hashes about which we are uncertain: we do not know whether the other
    /// party has the corresponding prefix or not, and we do not know whether
    /// the other party's corresponding hash matches.
    pub uncertain: OrdMap<Prefix<H>, blake3::Hash>,
}

pub struct Complete<P: Clone + Ord + AsRef<[u8]>, T> {
    /// The final set of nodes which we know the other party does not have,
    /// based on their having requested these prefixes in the previous round.
    pub providing: OrdMap<Prefix<Z>, Node<P, T, Z>>,
}
