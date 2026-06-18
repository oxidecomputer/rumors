//! Id operations on the packed form. Ids have no working form, so
//! `split`/`sum`/`is_disjoint`/`compare` run directly on the packed id bit
//! stream.
//!
//! Each node is a 2-bit presence tag (see [`idbits`](crate::idbits)): a `0` is
//! the *absence* of a child, never a node. Every traversal recurses on depth —
//! guarded against overflow by [`crate::recurse`] — and is `O(n + m)` in its
//! inputs, with no re-scan to find a right child. The same single-use-cursor
//! discipline as the event side governs them (see the [traversal
//! overview](crate::version::event)); two points are specific to ids:
//!
//! - **Threading via the cursor.** A child is a single-use `&mut`
//!   [`IdReader`](crate::idbits::IdReader): reading a node advances it in place,
//!   so finishing one present child leaves the cursor at the next. An *absent*
//!   child (a pruned `0`) is threaded as a synthetic
//!   [`Empty`](crate::idbits::IdReader::Empty), so the `(Empty, …)` arms fire
//!   for it exactly as for a stored `0`. (`split` is the exception: it walks the
//!   spine by bit *position* and splices the input on the branch.)
//!
//! - **Bounded lazy-skip.** Where one side prunes early (a leaf dominates the
//!   other's whole subtree), the dominated subtree is skipped *once*, at the
//!   prune point, to resync the cursors. Each node is skipped at most once, so
//!   the total stays `O(n)`.
//!
//! Emptiness/fullness are `O(1)` leaf checks (see [`idbits`](crate::idbits)),
//! valid because every `Party` — and every subtree of one — is in canonical
//! normal form. Output is built by [`build::IdBuilder`] (`sum`/`diff`) or by
//! direct bit-splice (`split`); see `split`'s `build_split` for why it does not
//! use the builder.

mod build;
mod compare;
mod diff;
mod split;
mod sum;
