//! Id operations on the packed form. Ids mutate by re-emission, so
//! `split`/`sum`/`diff`/`is_disjoint`/`compare` run directly on the packed id
//! bit stream.
//!
//! Each node is a 2-bit presence tag (see [`idbits`](crate::idbits)): a `0` is
//! the *absence* of a child, never a node. Every operation is `O(n + m)` in its
//! inputs, with no re-scan to find a right child, and none recurses — a deep
//! operand costs bits, not stack frames or grown segments. Two walk shapes
//! serve them, both on the event side's single-use-cursor discipline:
//!
//! - **Node lockstep** (`sum`, `covers`, `is_disjoint`): the consuming
//!   cursors carry the traversal and the per-ancestor control state is two
//!   or three bits on a bit stack. A child is a single-use `&mut`
//!   [`IdReader`](crate::idbits::IdReader): reading a node advances it in
//!   place, so finishing one present child leaves the cursor at the next,
//!   and an *absent* child (a pruned `0`) is stood in by a synthetic
//!   [`Empty`](crate::idbits::IdNode::Empty) node, so the `(Empty, …)` arms
//!   fire for it exactly as for a stored `0`. Where one side prunes early
//!   (a leaf dominates the other's whole subtree), the dominated subtree is
//!   skipped *once*, at the prune point, to resync the cursors — each node
//!   is skipped at most once, so the total stays `O(n)`.
//!
//! - **The boolean-skyline sweep** (`diff`): an id is read as a boolean
//!   skyline — a dyadic tiling of the unit interval into owned and unowned
//!   plateaus — and the operation is a pointwise fold over the two
//!   tilings' overlay, on the event sweep's boundary bookkeeping (`diff`'s
//!   module doc maps the correspondence).
//!
//! (`split` fits neither: it walks its unary spine by bit *position* in a loop
//! and splices the input on the branch. `sum_split`, the fused sum-then-split
//! behind [`Clock::sync`](crate::Clock::sync), walks the *union's* spine as a
//! two-cursor lockstep and delegates the branch children to `sum` or a verbatim
//! splice — its method doc carries the argument that the two structures
//! coincide.)
//!
//! Emptiness/fullness are `O(1)` leaf checks (see [`idbits`](crate::idbits)),
//! valid because every `Party` — and every subtree of one — is in canonical
//! normal form. Output is built by [`build::IdBuilder`] (`sum`), the
//! leaf-driven [`build::IdSkylineBuilder`] (`diff`), or by direct bit-splice
//! (`split`); see `split`'s `build_split` for why it does not use the builder.

mod build;
mod compare;
mod diff;
mod index;
mod split;
mod sum;
mod sum_split;

pub(crate) use index::IdIndex;
