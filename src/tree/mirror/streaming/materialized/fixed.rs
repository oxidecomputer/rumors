//! The *session allowance*: what a session holds outside its ledger whatever
//! its budget and the replicas' sizes, which its memory budget sets aside
//! beside the floor allowance.
//!
//! It is priced from types and the peer's own message-size target.
//!
//! The ledger charges the walk's items, which grow with the work in flight.
//! Some state grows with neither: the decode fans that carry supplied leaves to
//! assembly, assembly's queues and open spines, the preimages hashing builds,
//! the scratch the walk's stages fill before they charge what they built, and
//! the pipe's fixed state. Each is bounded by capacities and type sizes alone,
//! so the budget sets the sum aside once, the *fixed allowance*
//! ([`fixed_allowance`]). The supply runs on their way through the pipe are
//! bounded by the wire pools, sized at the target ([`wire_pools_bytes`]). The
//! two together are the session allowance ([`session_allowance`]), and the
//! ledger admits work against what the budget leaves beside it and the floor
//! allowance.
//!
//! Every term counts requested bytes, as the ledger's charges do, so the
//! allocator's rounding is part of none of them.

use std::mem::size_of;

use crate::tree::{
    mirror::streaming::{
        Backend, FAN, Leaf,
        holding::Holding,
        remote::{STREAM_COUNT, decode_ahead_bytes, fixed_wire_bytes, wire_pools_bytes},
    },
    typed::{
        ErasedPrefix,
        hash::HASHING_SCRATCH_BYTES,
        height::{Height, Root, Z},
        untyped::SortedLeaves,
    },
};

use super::{Resolution, Resolve, work::scratch};

/// The most bytes one side's assembly holds at once, over `B`'s nodes, whatever
/// the replicas' sizes.
///
/// Assembly holds the queues of fixed capacity around it and the backend's
/// build of each supplied subtree. Counted at their capacities:
///
/// - each assembled height, the root's through the leaves', returns at most a
///   fan of child handles to the height above, and builds at most one parent at
///   a time from at most a fan of children;
/// - the terminal leaf resolutions wait at most a fan deep, each with one slot;
/// - each of the [`STREAM_COUNT`] reply decoders assembles at most one supplied
///   subtree at a time, holding only its open right spine
///   ([`SortedLeaves::MAX_TRANSIENT_BYTES`]) however many leaves it has, and
///   queues at most a fan of assembled supplies for its reader.
///
/// Every queued item is counted at twice its size: the most a vector holds for
/// its length, or a channel node beside its item. The spine is the in-memory
/// backend's build, the one every session the public API runs uses.
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

/// The most bytes one side's stages, encoders, and decoders hold beyond their
/// ledger's charges at once, over `B`'s nodes: one logical stream's scratch for
/// every place ([`scratch`]), for each of the [`STREAM_COUNT`] streams.
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
/// - hashing's preimages ([`HASHING_SCRATCH_BYTES`]): a session runs its tasks
///   inside one future and hashing never yields, so a session holds the
///   preimages of one hashing descent at a time;
/// - the working buffers its stages fill before they charge what they built
///   ([`working_buffers`]);
/// - the pipe's fixed state: its write buffer, its frame reader's scratch, its
///   lanes' unpriced nodes, and its own setup ([`fixed_wire_bytes`]).
///
/// Every term follows from capacities and type sizes, so none depends on the
/// budget, the message-size target, or the replicas' sizes. The spines and the
/// preimages are the in-memory backend's, the one every session the public API
/// runs uses.
///
/// On the in-memory backend on 64-bit targets that is about 7 MiB, most of it
/// the open spines of one supplied subtree per logical stream.
pub(crate) const fn fixed_allowance<B: Backend<Node<Z>: Leaf>>() -> usize {
    decode_ahead_bytes::<B::Node<Z>>()
        + assembly_allowance::<B>()
        + HASHING_SCRATCH_BYTES
        + working_buffers::<B>()
        + fixed_wire_bytes()
}

/// The *session allowance* of a session over `B`'s nodes whose peer's own
/// message-size target is `target`: the most it holds outside its ledger.
///
/// Its memory budget sets this aside beside the floor allowance
/// ([`floor_allowance`](super::floor_allowance)).
///
/// That is the fixed allowance ([`fixed_allowance`]) beside the session's two
/// wire pools at `target` ([`wire_pools_bytes`]), saturating at `usize::MAX` on
/// targets whose `usize` cannot count the largest pools. A record larger than
/// its direction's pool is the one thing at the wire this leaves out: it takes
/// the whole pool, and holds its excess over the pool beside it, at most one
/// such record in each direction at once.
///
/// The tasks a session runs and what the executor allocates to run them are
/// outside every allowance: no constant can name an async future's size, and
/// the task set's and the executor's allocations are theirs to size. So is what
/// the payload type's deserialization allocates and frees while a decoder
/// checks a received record, beyond the value it keeps in the replica: only the
/// payload type sizes it, though a session decodes one record at a time, since
/// decoding never yields.
pub(crate) const fn session_allowance<B: Backend<Node<Z>: Leaf>>(target: usize) -> usize {
    fixed_allowance::<B>().saturating_add(wire_pools_bytes(target))
}

#[cfg(test)]
mod tests;
