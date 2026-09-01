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

use rumors::{Peer, Rumors};

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
    left.send_all([0]).unwrap();

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(ROOMY_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip(&mut provider),
            Peer::<u64>::bootstrap().join(&mut newcomer),
        );
        served.expect("serve bootstrap");
        joined
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .into_rumors()
    });

    left.send_all(1..=64u64).unwrap();
    right.send_all(65..=128u64).unwrap();
    (left, right)
}

/// A paused-clock wire's reported cost includes the virtual wire stall.
///
/// A diverged session over a 10 ms one-way delay pays at least one
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

/// A paused-clock wire's virtual report is exact: it lands on the delay
/// lattice.
///
/// (Every pipe deadline is a whole number of delays past the
/// runtime's epoch.) The report includes the diverged session's
/// request/response stall, at least `2 * delay`.
#[test]
fn virtual_report_is_exact_on_the_delay_lattice() {
    let delay = Duration::from_millis(10);
    let (left, right) = diverged_pair();
    let mut wire = latency::DelayedWire::new(ROOMY_CAPACITY, delay);
    let (_pair, reported) = wire.round_trip_virtual(left, right);
    let hops = latency::hops_on_lattice(reported, delay);
    assert!(
        hops >= 2,
        "a diverged session pays at least one request/response of wire \
         stall: {hops} hops at one-way delay {delay:?}",
    );
}

/// The virtual report is deterministic: the same session shape measures
/// the same wire cost on every run — the load-independence the window suites
/// pin their bounds on, stated as run-to-run equality.
#[test]
fn virtual_report_is_deterministic() {
    let delay = Duration::from_millis(10);
    let measure = || {
        let (left, right) = diverged_pair();
        let mut wire = latency::DelayedWire::new(ROOMY_CAPACITY, delay);
        let (_pair, reported) = wire.round_trip_virtual(left, right);
        reported
    };
    let (first, second) = (measure(), measure());
    assert_eq!(
        first, second,
        "one session shape, one virtual wire cost: {first:?} vs {second:?}",
    );
}

/// A wall-clock wire refuses to report a virtual cost: its virtual clock
/// tracks the real one, so the component is wall time in disguise and
/// load-independence — the figure's contract — cannot be honored.
#[test]
#[should_panic(expected = "virtual wire cost is only meaningful on a paused clock")]
fn wall_clock_wire_refuses_virtual_report() {
    let delay = Duration::from_millis(2);
    let (left, right) = diverged_pair();
    let mut wire = latency::DelayedWire::new_wall_clock(ROOMY_CAPACITY, delay);
    let _ = wire.round_trip_virtual(left, right);
}

/// A wall-clock wire's reported cost never exceeds externally measured
/// elapsed time.
///
/// The report is one interval strictly contained in the
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
