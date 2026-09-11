//! Pipelining reduces round trips when the queues fit the disputed frontier.
//!
//! This fixture uses ordinary hashed paths, matching the production window's
//! sizing model. Artificially clustered deep trees can exceed those statistical
//! queue widths and serialize despite a generous memory budget. They must
//! still reconcile correctly, but this latency ceiling does not apply to them.

mod common;

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::{Peer, Rumors};

use common::window::WindowChoice;
use common::wire::bootstrap_fork_with_window;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// Messages each peer originates after the fork: enough divergence to
/// dispute nearly all 256 root-child scopes.
const DIVERGENT_PER_SIDE: usize = 512;

/// One-way link delay, in whole milliseconds (the timer wheel's grain).
const DELAY: Duration = Duration::from_millis(10);

/// Separate pipelined and serialized behavior on this hash-distributed fixture.
///
/// Leave room for modest changes in fixed protocol cost; the minimum-window
/// comparison below verifies that this ceiling still rejects serialization.
const HOP_BUDGET: u32 = 24;

/// Per-stream in-flight window: far above this session's transfers, so
/// only round-trip structure is measured (see the module docs).
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// The default window keeps this fixture below the latency ceiling, while
/// the same divergence at minimum queue widths exceeds it.
#[test]
fn window_pipelines_disputed_scopes() {
    let pipelined =
        latency::session_hops(LINK_CAPACITY, DELAY, diverged_pair(WindowChoice::Default));
    let serialized =
        latency::session_hops(LINK_CAPACITY, DELAY, diverged_pair(WindowChoice::Floor));
    eprintln!("default window: {pipelined} hops; minimum window: {serialized} hops");
    assert!(
        (2..HOP_BUDGET).contains(&pipelined),
        "default window took {pipelined} hops, expected a completed exchange below {HOP_BUDGET}",
    );
    assert!(
        serialized > HOP_BUDGET,
        "minimum window took {serialized} hops: the ceiling no longer distinguishes serialization",
    );
}

/// Two peers with shared history and concurrent additions, using one window choice.
fn diverged_pair(window: WindowChoice) -> (Rumors<u64>, Rumors<u64>) {
    let left = window.apply(Peer::seed()).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork_with_window(&left, window);
    send_random(&left, DIVERGENT_PER_SIDE, &mut rng);
    send_random(&right, DIVERGENT_PER_SIDE, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
    rumors.send_all((0..n).map(|_| rng.next_u64())).unwrap();
}
