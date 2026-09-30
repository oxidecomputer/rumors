//! Test support, independent oracles, and cross-cutting semantic checks.
//!
//! These tests compare the binary implementation with independent models,
//! exercise families of inputs, and check that the public surface is covered.
//! The test-only validation index explains the purpose and limits of each
//! instrument.

mod auto_traits;
pub(crate) mod instrument;

#[cfg(any(test, feature = "laws"))]
pub mod laws;

#[cfg(any(test, feature = "meter"))]
pub mod meter;

#[cfg(any(test, feature = "oracle"))]
pub mod oracles;

#[cfg(any(test, feature = "meter"))]
pub mod surface;

#[cfg(test)]
pub(crate) mod bridge;
#[cfg(test)]
pub(crate) mod generators;
#[cfg(test)]
pub(crate) mod grow_brute_force;
#[cfg(test)]
pub(crate) mod optrace;
#[cfg(test)]
pub(crate) mod rng;
#[cfg(test)]
pub(crate) mod shape_rows;

#[cfg(test)]
mod algebraic_laws;
#[cfg(test)]
mod asymptotics;
#[cfg(test)]
pub(crate) mod compactness;
#[cfg(test)]
pub(crate) mod diff_ops;
#[cfg(test)]
pub(crate) mod exhaustive;
#[cfg(test)]
mod fuelscape_islands;
#[cfg(test)]
mod snapshots;
#[cfg(test)]
pub(crate) mod surface_coverage;
#[cfg(test)]
pub mod validation_index;
