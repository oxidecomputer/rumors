//! Lower bounds that establish whether the board's heap counter is live.
//!
//! A floor is the least allocation an operation must expose to the counting
//! allocator. It follows from the operation's contract and operands, not from
//! incidental details of the current implementation. A reading below its
//! floor means that the counter no longer observes the allocation it is meant
//! to constrain.
//!
//! Heap floors cover results that must allocate at least their own size. When
//! an operation can validly avoid allocating, the cell records why the counter
//! is not applicable instead of asserting a zero floor. Fuel needs no per-cell
//! floor, since no implementation can bypass it
//! ([`FUEL_FLOOR`](super::currency::FUEL_FLOOR)).

use super::currency::{Floors, Liveness};

/// Heap floor: the result materializes at least its required bytes.
const WHY_HEAP_MATERIALIZES: &str =
    "materializes a result at least as large as its required output bytes";
/// Heap NA: the forked child's version hand-over shares the refcounted
/// stored buffer.
///
/// The floor is trued to the `bytes::Bytes`-backed at-rest form: a
/// version clone is a refcount bump, so no byte copy is semantically
/// forced, so no positive floor is justified.
pub(super) const NA_HEAP_FORK_SHARES: &str = "the forked child's version hand-over is a \
     refcount bump on the shared stored buffer, never a byte copy, and no other allocation \
     is semantically forced";
/// Heap floor (deterministic-liveness): a forked party materializes its
/// child half's party bits.
pub(super) const WHY_HEAP_FORK_HALF: &str =
    "deterministic-liveness: the forked child materializes its \
     own party bits today (fork builds both halves, not an in-place edit); a shared-buffer \
     representation would lower this floor deliberately";
/// Heap NA: allocation is not semantically forced.
pub(super) const NA_HEAP_IN_PLACE: &str =
    "may compute in place or return word-scale results: allocation \
     is not semantically forced (the process allocator itself cannot be re-routed around)";
/// Heap NA: a cloned query may share both its bound list and version buffers.
pub(super) const NA_HEAP_QUERY_CLONE: &str =
    "the representation may share the bound list and every version buffer: \
     no allocation is semantically forced";
/// Heap NA on rejection rows: no result is materialized.
pub(super) const NA_HEAP_REJECTION: &str =
    "a rejecting or empty outcome materializes no result, and \
     buffering the fed stream is not semantically forced: allocation stays the \
     implementation's choice";

/// A materialization heap floor over `output_bytes`.
pub(super) fn heap_materializes(output_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: output_bytes as u64,
        why: WHY_HEAP_MATERIALIZES,
    }
}

/// Shorthand for a not-applicable declaration.
pub(super) fn na(reason: &'static str) -> Liveness {
    Liveness::NotApplicable { reason }
}

/// The floors of the rows that compute in place or return word-scale results:
/// no allocation is forced.
pub(super) fn in_place() -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
    }
}

/// The floors of the rejection rows: a rejecting outcome materializes no
/// result, so no allocation is forced.
pub(super) fn rejection() -> Floors {
    Floors {
        heap: na(NA_HEAP_REJECTION),
    }
}
