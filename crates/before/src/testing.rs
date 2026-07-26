//! Test-only harness: the differential-testing scaffolding and cross-cutting
//! suites.
//!
//! It holds the oracle⇄impl bridge, input generators, the brute-force
//! grow-optimality reference, the step-scaling helpers, the function-space
//! (semantic) oracle, and the cross-cutting suites (exhaustive small-scope,
//! algebraic laws, documentation snapshots). Compiled only under `cfg(test)`;
//! never part of the shipped crate.
//!
//! The per-production-module unit tests live in their own `*/tests.rs`
//! siblings; this module holds the shared scaffolding and the suites that span
//! more than one module.

pub(crate) mod bridge;
pub(crate) mod complexity;
pub(crate) mod fold_oracle;
pub(crate) mod generators;
pub(crate) mod grow_brute_force;
pub(crate) mod metrics;
pub(crate) mod optrace;
pub(crate) mod rng;

mod algebraic_laws;
pub(crate) mod compactness;
pub(crate) mod exhaustive;
pub(crate) mod semantic_oracle;
mod snapshots;
