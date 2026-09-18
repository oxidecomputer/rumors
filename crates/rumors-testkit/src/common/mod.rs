//! Shared infrastructure for integration tests.
//!
//! How the pieces compose:
//!
//! - [`crate::common::action`] generates single-peer `Insert`/`Redact` sequences that
//!   suites apply to one peer at a time, building each side's local
//!   history by hand.
//! - [`crate::common::schedule`] generates arbitrary multi-peer interleavings of peer
//!   events (inserts, redactions, gossip sessions, and — under the
//!   membership alphabet — mid-schedule bootstraps and retirements) and
//!   executes them deterministically, one session at a time
//!   ([`crate::common::schedule::executor`]), against real peers ([`crate::common::peer`]) over
//!   in-memory links, with [`crate::common::oracle`] computing the expected converged
//!   set for comparison.
//! - [`crate::common::observer`] shares message-observer read helpers and two-peer schedules.
//! - [`crate::common::sim`] runs whole plans concurrently instead: overlapping sessions
//!   on a multi-thread runtime, genuinely nondeterministic.
//! - [`crate::common::fault`] injects transport adversity; [`crate::common::flaky`] injects
//!   bookmark-storage adversity.
//! - [`crate::common::wire`] and [`crate::common::tcp`] carry the same sessions over in-memory links
//!   and real sockets; [`crate::common::count`] measures each direction of an in-memory
//!   session; [`crate::common::routed_tcp`] is the socket instantiation of the routed
//!   adapter's dial/listen seam.
//! - [`crate::common::window`] is the window-budget sweep dimension: generated per-peer
//!   window configurations (floor, tight budget, default) for the suites
//!   that sweep it.
//! - [`crate::common::gossip_snapshot`] captures a session's exact bytes for the `insta`
//!   pins.

pub mod action;
pub mod count;
pub mod fault;
pub mod flaky;
pub mod gossip_snapshot;
pub mod observer;
pub mod oracle;
pub mod overlap;
pub mod peer;
pub mod routed_tcp;
pub mod schedule;
pub mod shape;
pub mod sim;
pub mod tcp;
pub mod window;
pub mod wire;
