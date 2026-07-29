//! Durable storage: the transactional boundary a deployment implements
//! and the layers the crate builds over it.
//!
//! A peer that must outlive its process (or outgrow its memory) stores its
//! tree in a transactional key-value store instead of the heap. This
//! module is that stack, bottom up:
//!
//! - [`Kv`] is the caller-implementable contract — named byte tables with
//!   atomic, serializable transactions. Implement it for an embedded store
//!   and validate the implementation with the `conformance` feature's kv
//!   suite.
//! - [`Memory`] is the reference implementation: the store the crate's own
//!   persistence tests run against, with the crash-model and
//!   fault-injection instrumentation those tests need.
//! - The record schema and reference-counted custody layers (crate
//!   internal) turn a `Kv` into copy-on-write tree storage: one record per
//!   node, strong counts for the durable structure, a held table pinning
//!   what live process handles reach, deferred reclamation in bounded
//!   transactions, and a recovery sweep that makes any crash equivalent to
//!   dropping every in-process handle at once.
//!
//! End-user documentation for persistent peers lives in the [crate
//! docs](crate); implementor documentation for the storage contract lives
//! in [`kv`].

mod backend;
pub mod kv;
/// The reference store rides the same features that carry its imbl
/// dependency: the crate's own tests (via `test-internals`) and the
/// public conformance surface. Production builds compile no imbl.
#[cfg(any(feature = "conformance", feature = "test-internals"))]
mod memory;

pub use backend::{KvBackend, OpenError};
pub use kv::{Kv, ReadTxn, Table, WriteTxn};
#[cfg(any(feature = "conformance", feature = "test-internals"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "conformance", feature = "test-internals")))
)]
pub use memory::{Memory, MemoryError};

pub(crate) mod refcount;
pub(crate) mod schema;
