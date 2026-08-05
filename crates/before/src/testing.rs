//! Test-only harness: the differential-testing scaffolding and cross-cutting
//! suites.
//!
//! The scaffolding: the oracle⇄impl bridge ([`bridge`]), the proptest input
//! strategies ([`generators`]), the deterministic RNG ([`rng`]), the
//! op-trace history generator ([`optrace`]), and the brute-force
//! grow-optimality reference ([`grow_brute_force`]).
//!
//! The cross-cutting suites: exhaustive small-scope enumeration
//! ([`exhaustive`]), the function-space semantic oracle
//! ([`semantic_oracle`]), the algebraic-law harness ([`algebraic_laws`] —
//! a thin binding; the named law predicates are [`crate::laws`]'s, shared
//! with the fuzz targets), representation compactness ([`compactness`]),
//! documentation snapshots ([`snapshots`]), the documented-asymptotics
//! liveness pins ([`asymptotics`]), and the public-surface coverage suite
//! ([`surface_coverage`] — the committed prod↔tree↔fs differential-leg
//! roster; "which suite covers operation X" starts there).
//!
//! Part of the same architecture, outside this module: the recursive
//! reference implementation the legs compare against ([`crate::oracle`]),
//! the law predicates ([`crate::laws`]), the per-production-module unit
//! tests in their `*/tests.rs` siblings, and the out-of-process suites in
//! `tests/` (the resource-envelope pins, the board smoke test, the
//! bench-judge membership pins, the fuzz seed corpus). Compiled only under
//! `cfg(test)`; never part of the shipped crate. The map of how every
//! validation instrument — in this module and out of it — fits together,
//! written for a maintainer orienting cold, is [`validation_index`].

// The scaffolding.
pub(crate) mod bridge;
pub(crate) mod generators;
pub(crate) mod grow_brute_force;
pub(crate) mod optrace;
pub(crate) mod rng;

// The suites.
mod algebraic_laws;
mod asymptotics;
pub(crate) mod compactness;
pub(crate) mod exhaustive;
pub(crate) mod semantic_oracle;
mod snapshots;
pub(crate) mod surface_coverage;
pub mod validation_index;
