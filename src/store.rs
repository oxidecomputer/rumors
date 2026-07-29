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

pub mod kv;
mod memory;

pub use kv::{Kv, ReadTxn, Table, WriteTxn};
pub use memory::{Memory, MemoryError};

// Consumed today only by their own test surfaces; the persistent tree
// backend that drives them from the library proper is the next unit of
// this campaign, and these expectations retire with it.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) mod refcount;
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) mod schema;
