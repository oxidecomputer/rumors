//! Reference oracle: the paper's recursive trees.
//!
//! Public under the `oracle` feature so the benchmark suite can time it against
//! the optimized implementation.
//!
//! `Party` and `Version` *are* the trees; every operation is a method, so there
//! is no second representation to keep in sync. Deliberately simple,
//! suboptimal, and recursive: its only job is to be obviously correct, so it
//! can serve as differential ground truth. It mirrors the target's **semantic**
//! surface (construction, operations, ordering, operators) and omits the one
//! purely *representational* concern that carries no semantics: the byte codec
//! (`encode`/`decode`).
//!
//! # Operating envelope
//!
//! Small-scope, bounded-depth inputs only. Every traversal here — the ops, the
//! derived `Drop` of the boxed trees, `Clone` — recurses on native stack
//! frames, and the boxed representation crawls a long spine pointer by pointer;
//! handed a degenerate spine thousands of levels deep, the oracle overflows the
//! stack that the impl's guarded, iterative walks are built to survive. That is
//! deliberate, and it is the *harnesses* that bound their inputs — the oracle
//! is never hardened. Transcription fidelity outranks robustness: the paper's
//! definitions are recursions over trees, and every guard, explicit stack, or
//! depth check layered onto them would put distance between the reference and
//! the definition it exists to transcribe. Each oracle-facing suite therefore
//! carries its own input bound (generator recursion caps, enumeration depth
//! constants, op-trace length caps, family scale caps), and depth-stress
//! coverage — the 100k-deep spines — runs against the impl alone, asserted with
//! closed-form witnesses in place of oracle output. Counts bound the same way:
//! `ticks(n)` here iterates `n` literal ticks (`O(n · tree)`), so oracle-facing
//! differentials cap `n` small, and wide-count coverage is impl-side (the
//! composition law and closed-form witnesses).
//!
//! All three types derive `Clone`: a reference oracle needs cheap snapshots of
//! "before" states for the property checks, and linearity (`!Clone` on
//! `Party`/`Clock`) is a *type-level* guarantee checked against `before` by
//! compile-fail tests — not a runtime semantic the differential harness
//! exercises.

#![allow(missing_docs)] // A test/bench reference, not real public API, even when the
                        // `oracle` feature re-exports it (the crate warns on missing docs).

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct OverlapError;

mod clock;
mod party;
mod version;

pub use clock::Clock;
pub use party::Party;
pub use version::Version;
