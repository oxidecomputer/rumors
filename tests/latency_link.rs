//! The benchmark-support latency link satisfies the link contract.
//!
//! `benches/support/latency.rs` builds the delayed-pipe link the latency
//! benchmarks sweep; these tests run it through the public
//! [`rumors::conformance`] suite so the sweep measures the protocols, not
//! an accidentally nonconforming transport. Delays live in virtual time
//! (paused-clock runtimes), so the nonzero-delay pass costs no wall time.

// The bench support module is compiled into this test verbatim; only the
// pair constructor is exercised here, the wire harness is bench-only.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

/// Small per-stream window so the suite's independence probe fills it
/// quickly: coupling hidden behind buffering must reveal itself.
const CAPACITY: usize = 64;

/// At zero delay the delayed pipe is a plain bounded pipe, and the link
/// contract — independent, receiver-paced streams, half-close, tolerated
/// accept cancellation — holds.
#[tokio::test(start_paused = true)]
async fn conforms_at_zero_delay() {
    rumors::conformance::check(async || latency::delayed_pair(CAPACITY, Duration::ZERO)).await;
}

/// The contract holds with latency injected: delaying every byte by a
/// nonzero one-way delay changes when bytes arrive, never whether streams
/// stay independent, receiver-paced, and half-close clean.
#[tokio::test(start_paused = true)]
async fn conforms_at_nonzero_delay() {
    rumors::conformance::check(async || latency::delayed_pair(CAPACITY, Duration::from_millis(3)))
        .await;
}
