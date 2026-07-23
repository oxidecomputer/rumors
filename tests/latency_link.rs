//! The benchmark-support latency link satisfies the link contract.
//!
//! `benches/support/latency.rs` builds the delayed-pipe link the latency
//! benchmarks sweep; these tests run it through the public
//! [`rumors::conformance::link`] suite so the sweep measures the protocols, not
//! an accidentally nonconforming transport. Delays live in virtual time
//! (paused-clock runtimes), so the nonzero-delay pass costs no wall time.
//! The wire harness's cost-reporting contract is pinned here too, in both
//! clock modes.

// The bench support module is compiled into this test verbatim; the pair
// constructor and the wire harness are exercised, the rest is bench-only.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rumors::{Peer, Protocol, Rumors};

/// Small per-stream window so the suite's independence probe fills it
/// quickly: coupling hidden behind buffering must reveal itself.
const CAPACITY: usize = 64;

/// At zero delay the delayed pipe is a plain bounded pipe, and the link
/// contract — independent, receiver-paced streams, half-close, tolerated
/// accept cancellation — holds.
#[tokio::test(start_paused = true)]
async fn conforms_at_zero_delay() {
    rumors::conformance::link::check(async || latency::delayed_pair(CAPACITY, Duration::ZERO))
        .await;
}

/// The contract holds with latency injected: delaying every byte by a
/// nonzero one-way delay changes when bytes arrive, never whether streams
/// stay independent, receiver-paced, and half-close clean.
#[tokio::test(start_paused = true)]
async fn conforms_at_nonzero_delay() {
    rumors::conformance::link::check(async || {
        latency::delayed_pair(CAPACITY, Duration::from_millis(3))
    })
    .await;
}

/// Roomy per-stream pipe buffering for the cost-model pins: only the
/// harness's reporting is under test, never window throttling.
const ROOMY_CAPACITY: usize = 8 * 1024 * 1024;

/// Build a bootstrapped pair diverged by a handful of messages on each
/// side, so a session pays at least one request/response of wire stall.
fn diverged_pair() -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed().sync_window_floor().into_rumors();
    left.batch().send(0);

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(ROOMY_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip(&mut provider),
            Peer::<u64>::bootstrap_with_protocol(Protocol::V2, &mut newcomer),
        );
        served.expect("serve bootstrap");
        joined
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .into_rumors()
    });

    let mut batch = left.batch();
    (1..=64u64).for_each(|n| {
        batch.send(n);
    });
    drop(batch);
    let mut batch = right.batch();
    (65..=128u64).for_each(|n| {
        batch.send(n);
    });
    drop(batch);
    (left, right)
}

/// A paused-clock wire's reported cost includes the virtual wire stall: a
/// diverged session over a 10 ms one-way delay pays at least one
/// request/response, so the report is bounded below by `2 * delay` even
/// though no wall time elapses on the wire.
#[test]
fn paused_report_includes_virtual_stall() {
    let delay = Duration::from_millis(10);
    let (left, right) = diverged_pair();
    let mut wire = latency::DelayedWire::new(ROOMY_CAPACITY, delay);
    let (_pair, reported) = wire.round_trip(left, right);
    assert!(
        reported >= 2 * delay,
        "a diverged session pays at least one request/response of virtual \
         stall: reported {reported:?} at one-way delay {delay:?}",
    );
}

/// A wall-clock wire's reported cost never exceeds externally measured
/// elapsed time: the report is one interval strictly contained in the
/// caller's, so summing wall and virtual components (two measurements of
/// the same real interval) would breach this bound by double-counting.
#[test]
fn wall_clock_report_is_contained_in_real_elapsed_time() {
    let delay = Duration::from_millis(2);
    let (left, right) = diverged_pair();
    let mut wire = latency::DelayedWire::new_wall_clock(ROOMY_CAPACITY, delay);
    let external_start = std::time::Instant::now();
    let (_pair, reported) = wire.round_trip(left, right);
    let external = external_start.elapsed();
    assert!(
        reported <= external,
        "the reported duration is measured inside the call and must be \
         contained in the caller's measurement: reported {reported:?}, \
         external {external:?}",
    );
}
