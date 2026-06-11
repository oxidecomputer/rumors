//! Reference oracle — the paper's trees as plain recursive enums.
//!
//! `Party` and `Version` *are* the trees; every operation is a method, so there
//! is no second representation to keep in sync. Deliberately simple,
//! suboptimal, and recursive: its only job is to be obviously correct, so it
//! can serve as differential ground truth. It mirrors the target's **semantic**
//! surface (construction, operations, ordering, operators) and omits the two
//! purely *representational* concerns that carry no semantics: the byte codec
//! (`encode`/`decode`) and the batch optimization (a batch only ever equals its
//! value-level ops). Bounded-depth use only — the deep-tree stack-safety test
//! runs against the impl, never the oracle.
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
