//! The storage boundary for canonical party trees.
//!
//! Party algorithms work with nodes, paths, and complete subtrees rather than
//! bit positions. Readers decode those objects without materializing a tree;
//! writers emit them while applying the canonical collapses. The
//! underlying bit stream is confined to this boundary.

mod reader;
mod regions;
pub(crate) mod validate;
pub(crate) mod writer;

pub(crate) use reader::{
    BranchChoice, ForkPoint, PartyBranch, PartyNode, PartyPath, PartyReader, PartySnapshot,
    PartySubtree,
};
pub(crate) use regions::PartyRegionReader;
