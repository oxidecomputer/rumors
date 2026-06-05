//! Shared infrastructure for the simulation integration tests.
//!
//! Each per-category test binary (`single_peer`, `pairwise`, `multi_peer`,
//! `redaction`, `partition`, `sanity`, `shadow_validity`) pulls this module
//! in via `mod common;` and reaches its pieces through `crate::common::*`.
//!
//! Not every binary uses every module; suppress unused-code warnings here
//! rather than peppering allows across modules.
#![allow(dead_code, unused_imports)]

pub mod action;
pub mod gossip_snapshot;
pub mod oracle;
pub mod peer;
pub mod schedule;
pub mod sync_wire;
pub mod wire;
