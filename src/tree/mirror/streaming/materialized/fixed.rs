//! The *fixed allowance*: what a session holds outside its ledger whatever
//! its budget and the replicas' sizes, priced from types, which its memory
//! budget sets aside beside the floor allowance.
//!
//! The ledger charges the walk's items, which grow with the work in
//! flight. Some state grows with neither: the decode fans that carry
//! supplied leaves to assembly, assembly's queues and open spines, the
//! preimages hashing builds, and the scratch the walk's stages fill before
//! they charge what they built. Each is bounded by capacities and type
//! sizes alone, so the budget sets the sum aside once
//! ([`fixed_allowance`]), and the ledger admits work against what remains.
//!
//! Every term counts requested bytes, as the ledger's charges do, so the
//! allocator's rounding is part of none of them.

use std::mem::size_of;

use crate::tree::{
    mirror::streaming::{
        Backend, FAN, Leaf,
        holding::Holding,
        remote::{STREAM_COUNT, decode_ahead_bytes},
    },
    typed::{
        ErasedPrefix,
        hash::HASHING_SCRATCH_BYTES,
        height::{Height, Root, Z},
        untyped::SortedLeaves,
    },
};

use super::{Resolution, Resolve, work::scratch};

/// The most bytes one side's assembly holds at once, over `B`'s nodes,
/// whatever the replicas' sizes.
///
/// Assembly holds the queues of fixed capacity around it and the backend's
/// build of each supplied subtree. Counted at their capacities:
///
/// - each assembled height, the root's through the leaves', returns at most
///   a fan of child handles to the height above, and builds at most one
///   parent at a time from at most a fan of children;
/// - the terminal leaf resolutions wait at most a fan deep, each with one
///   slot;
/// - each of the [`STREAM_COUNT`] reply decoders assembles at most one
///   supplied subtree at a time, holding only its open right spine
///   ([`SortedLeaves::MAX_TRANSIENT_BYTES`]) however many leaves it has, and
///   queues at most a fan of assembled supplies for its reader.
///
/// Every queued item is counted at twice its size: the most a vector holds
/// for its length, or a channel node beside its item. The spine is the
/// in-memory backend's build, the one every session the public API runs
/// uses.
pub(crate) const fn assembly_allowance<B: Backend<Node<Z>: Leaf>>() -> usize {
    let heights = Root::HEIGHT + 1;
    let returned = size_of::<Option<B::Erased>>();
    let child = size_of::<(u8, Option<B::Erased>)>();
    let terminal = size_of::<Resolution<B::Erased>>() + size_of::<(u8, Resolve<B::Erased>)>();
    let supplied = size_of::<(ErasedPrefix, B::Erased)>();
    let levels = 2 * (heights * FAN * (returned + child) + FAN * terminal);
    let decoders = STREAM_COUNT * (SortedLeaves::MAX_TRANSIENT_BYTES + 2 * FAN * supplied);
    levels + decoders
}

/// The most bytes one side's stages, encoders, and decoders hold beyond
/// their ledger's charges at once, over `B`'s nodes: one logical stream's
/// scratch for every place ([`scratch`]), for each of the [`STREAM_COUNT`]
/// streams.
pub(crate) const fn working_buffers<B: Backend<Node<Z>: Leaf>>() -> usize {
    let mut per_stream = 0;
    let mut place = 0;
    while place < Holding::ALL.len() {
        per_stream += scratch::<B::Erased, B::Error>(Holding::ALL[place]);
        place += 1;
    }
    STREAM_COUNT * per_stream
}

/// The *fixed allowance* of a session over `B`'s nodes: the most its fixed
/// state holds at once, which its memory budget sets aside beside the floor
/// allowance ([`floor_allowance`](super::floor_allowance)).
///
/// It is the sum of:
///
/// - the decode fans' read-ahead ([`decode_ahead_bytes`]);
/// - assembly's queues and open spines ([`assembly_allowance`]);
/// - hashing's preimages ([`HASHING_SCRATCH_BYTES`]): a session runs its
///   tasks inside one future and hashing never yields, so a session holds
///   the preimages of one hashing descent at a time;
/// - the working buffers its stages fill before they charge what they
///   built ([`working_buffers`]).
///
/// Every term follows from capacities and type sizes, so none depends on
/// the budget, the message-size target, or the replicas' sizes. The
/// spines and the preimages are the in-memory backend's, the one every
/// session the public API runs uses.
///
/// On the in-memory backend on 64-bit targets that is about 7 MiB, most of
/// it the open spines of one supplied subtree per logical stream.
pub(crate) const fn fixed_allowance<B: Backend<Node<Z>: Leaf>>() -> usize {
    decode_ahead_bytes::<B::Node<Z>>()
        + assembly_allowance::<B>()
        + HASHING_SCRATCH_BYTES
        + working_buffers::<B>()
}

#[cfg(test)]
mod tests;
