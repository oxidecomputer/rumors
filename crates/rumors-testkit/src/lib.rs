//! Shared infrastructure for Rumors tests and benchmarks.
//!
//! Keeping this support in one crate lets Cargo compile it once instead of
//! rebuilding it inside every integration-test and benchmark binary.

/// Fixtures and drivers shared by integration tests.
pub mod common;

/// Models and fixtures shared by benchmarks and performance tests.
pub mod bench;
