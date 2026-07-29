//! Conformance suites for the pieces a deployment implements itself.
//!
//! Each caller-implementable boundary gets one submodule carrying its
//! validation suite. [`link`] checks that a caller-built
//! [`Link`](crate::link::Link) transport delivers the stream independence,
//! flow control, half-close, and cancellation tolerance the [link
//! module](crate::link) requires of every implementation. [`kv`] checks
//! that a caller-built [`Kv`](crate::store::Kv) store delivers the
//! transaction atomicity, isolation, and cursor semantics the [storage
//! contract](crate::store::kv) requires. Both are available from a
//! dev-dependency with the `conformance` cargo feature enabled.
//!
//! A further suite validates a storage backend's session-memory pricing;
//! that boundary is crate-internal, so its suite runs as this crate's own
//! test gate rather than as a public entry point.

pub mod kv;
pub mod link;

// Compiled as this crate's own gate: the storage-backend boundary is
// crate-internal; see the module docs' visibility section.
#[cfg(test)]
pub(crate) mod backend;
