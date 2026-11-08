//! Reproducible network seeds for every test that creates a network.
//!
//! This module re-exports the seed half of [`rumors::testing::property`], the
//! one copy of the scheme, which this crate's suites and `rumors`'s own unit
//! tests share. Its module documentation says which kind of seed a test takes:
//! a [`network_seed`] input for a claim that holds for every secret, or a fixed
//! [`NetworkSeed::new`] constant for a check of exact numbers.

pub use rumors::testing::property::{NetworkSeed, distinct_network_seeds, network_seed};
