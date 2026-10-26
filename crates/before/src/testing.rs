//! Test support, independent oracles, and cross-cutting semantic checks.
//!
//! These tests compare the binary implementation with independent models and
//! exercise families of semantic and resource-sensitive inputs.

mod auto_traits;

#[cfg(any(test, feature = "laws"))]
pub mod laws;

#[cfg(any(test, feature = "meter"))]
pub mod meter;

#[cfg(any(test, feature = "oracle"))]
pub mod oracles;

#[cfg(any(test, feature = "meter"))]
pub(crate) mod version;

#[cfg(test)]
pub(crate) mod bridge;
#[cfg(test)]
pub(crate) mod canonical;
#[cfg(test)]
pub(crate) mod generators;
#[cfg(test)]
pub(crate) mod grow_brute_force;
#[cfg(test)]
pub(crate) mod optrace;
#[cfg(test)]
pub(crate) mod property;
#[cfg(test)]
pub(crate) mod rng;
#[cfg(test)]
pub(crate) mod scripted_io;
#[cfg(test)]
pub(crate) mod shape_rows;

#[cfg(test)]
mod algebraic_laws;
#[cfg(test)]
mod asymptotics;
// Needs both codec features, so only the all-features legs run it: the
// gate's `test-all` and the landing check's tests leg, not `just test`.
#[cfg(all(test, feature = "serde", feature = "borsh"))]
mod decoders;
#[cfg(test)]
pub(crate) mod diff_ops;
#[cfg(test)]
pub(crate) mod exhaustive;
#[cfg(test)]
mod fuelscape_islands;
#[cfg(test)]
mod snapshots;
