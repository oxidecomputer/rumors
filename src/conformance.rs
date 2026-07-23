//! Conformance suites for the pieces a deployment implements itself.
//!
//! Each caller-implementable boundary gets one submodule carrying its
//! validation suite. [`link`] checks that a caller-built
//! [`Link`](crate::link::Link) transport delivers the stream independence,
//! flow control, half-close, and cancellation tolerance the [link
//! module](crate::link) requires of every implementation. Available from a
//! dev-dependency with the `conformance` cargo feature enabled.
//!
//! A second suite validates a storage backend's session-memory pricing;
//! it runs as this crate's own gate today and becomes caller-visible with
//! the storage-backend boundary itself.

pub mod link;

// Compiled as this crate's own gate until the storage-backend boundary
// ships; see the module docs' visibility section.
#[cfg(test)]
pub(crate) mod backend;
