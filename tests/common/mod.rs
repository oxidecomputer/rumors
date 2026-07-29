//! Shared infrastructure for the simulation integration tests.
//!
//! Each per-category test binary (`single_peer`, `pairwise`, `multi_peer`,
//! `redaction`, `partition`, `disruption`, `sanity`, `shadow_validity`)
//! pulls this module in via `mod common;` and reaches its pieces through
//! `crate::common::*`.
//!
//! How the pieces compose:
//!
//! - [`action`] generates single-peer `Insert`/`Redact` sequences that
//!   suites apply to one peer at a time, building each side's local
//!   history by hand.
//! - [`schedule`] generates arbitrary multi-peer interleavings of peer
//!   events (inserts, redactions, gossip sessions) and executes them
//!   deterministically, one gossip session at a time
//!   ([`schedule::executor`]), against real peers ([`peer`]) over
//!   in-memory links, with [`oracle`] computing the expected converged
//!   set for comparison.
//! - [`sim`] runs whole plans concurrently instead: overlapping sessions
//!   on a multi-thread runtime, genuinely nondeterministic.
//! - [`fault`] injects transport adversity; [`flaky`] injects
//!   bookmark-storage adversity.
//! - [`wire`] and [`tcp`] carry the same sessions over in-memory links
//!   and real sockets; [`routed_tcp`] is the socket instantiation of
//!   the routed adapter's dial/listen seam.
//! - [`gossip_snapshot`] captures a session's exact bytes for the `insta`
//!   pins.
//!
//! Not every binary uses every module; suppress unused-code warnings here
//! rather than peppering allows across modules.
#![allow(dead_code, unused_imports)]

pub mod action;
pub mod fault;
pub mod flaky;
pub mod gossip_snapshot;
pub mod oracle;
pub mod peer;
pub mod routed_tcp;
pub mod schedule;
pub mod sim;
pub mod tcp;
pub mod wire;
