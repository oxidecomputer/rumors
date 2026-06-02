//! Iterative id operations on the packed form. Ids have no working form, so
//! `split`/`sum`/`is_disjoint`/`compare` run directly on the `enc_id` bit
//! stream.
//!
//! `enc_id(Leaf v) = 0, v` (2 bits); `enc_id(Node l r) = 1, enc_id(l),
//! enc_id(r)`. Every traversal is iterative (explicit stack) and `O(n + m)` in
//! its inputs — no re-scan to find a right child. Two techniques achieve that:
//!
//! - **Threading.** A right child's position is *discovered* when the walk
//!   finishes the left subtree, not recomputed by skipping it. The DFS returns
//!   where each subtree ended; the sibling resumes there.
//!
//! - **Bounded lazy-skip.** Where one side prunes early (a leaf dominates the
//!   other's whole subtree), the dominated subtree is skipped *once*, at the
//!   prune point, to resync the cursors. Each node is skipped at most once, so
//!   the total stays `O(n)`.
//!
//! Emptiness/fullness are `O(1)` leaf checks (see [`idbits`](crate::idbits)),
//! valid because every `Party` — and every subtree of one — is in canonical
//! normal form.

mod build;
mod compare;
mod split;
mod sum;
