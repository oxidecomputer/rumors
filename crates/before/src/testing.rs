//! Shared test support and cross-cutting semantic checks.
//!
//! These tests compare the binary implementation with independent models,
//! exercise families of inputs, and check that the public surface is covered.
//! [`validation_index`] explains the purpose and limits of each instrument.

// The scaffolding.
pub(crate) mod bridge;
pub(crate) mod generators;
pub(crate) mod grow_brute_force;
pub(crate) mod optrace;
pub(crate) mod rng;
pub(crate) mod shape_rows;

// The suites.
mod algebraic_laws;
mod asymptotics;
pub(crate) mod compactness;
pub(crate) mod diff_ops;
pub(crate) mod exhaustive;
mod fuelscape_islands;
pub(crate) mod semantic_oracle;
mod snapshots;
pub(crate) mod surface_coverage;
pub mod validation_index;
