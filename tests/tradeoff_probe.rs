//! One-shot validation instrument, ignore-gated: the solve-derived
//! trade-off predictions held against measured wire-time slowdowns.
//!
//! It validates, deterministically, that measured slowdowns stay at or
//! inside the wave form evaluated at the window the real derivation
//! grants — the quantity the committed trade-off table tabulates — and
//! runs only by explicit request:
//!
//!     cargo nextest run --release --test tradeoff_probe \
//!         --run-ignored all --no-capture
//!
//! Method (window_operator.rs's session shape, generalized over record
//! size), per record size `m`:
//!
//! 1. Measure the transfer-bound baseline (an unbounded budget) in exact
//!    one-way hops, self-calibrating the link's BDP in messages.
//! 2. Measure budgets spanning a constricted, a near-crossover, and a
//!    comfortable cell, all at the design corpus (62,500 divergent
//!    messages a side, the scale the per-scope envelope is pinned at).
//! 3. Assert the observed slowdown — hops(budget) / hops(unbounded) —
//!    at or inside the solve-derived wave form to within hop
//!    quantization; the closed-form estimate is printed beside it for
//!    the record.
//!
//! Deterministic counts only: under the paused clock virtual time
//! advances only while every task is blocked on the wire, so the hop
//! counts are exact and wall compute is excluded.

// Only the delayed wire is exercised here.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use borsh::{BorshDeserialize, BorshSerialize};
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::{envelope_and_wire_bytes, supply_decode_envelope_bytes, window_capacities};
use rumors::{Peer, Protocol, Rumors};

/// One-way delay for the virtual-time measurements (the timer grain).
const DELAY: Duration = Duration::from_millis(10);

/// Roomy buffering for corpus construction only.
const BUILD_CAPACITY: usize = 8 * 1024 * 1024;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// Messages each side originates alone: the design corpus (the scale
/// `SCOPE_ENVELOPE_BYTES` = 4,865 B is pinned at), so the closed form
/// is evaluated inside its own claimed regime.
const DIVERGENT: usize = 62_500;

/// An effectively unbounded budget: the transfer-bound baseline.
const UNBOUNDED: usize = 8 << 30;

fn diverged<T>(budget: usize, mint: &mut impl FnMut(&mut SmallRng) -> T) -> (Rumors<T>, Rumors<T>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    let left = Peer::seed().sync_memory_budget(budget).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_7ade_0ff1);
    let mut send = |rumors: &Rumors<T>, n: usize, rng: &mut SmallRng| {
        rumors::testing::commit((0..n).fold(rumors.batch(), |batch, _| batch.send(mint(rng))));
    };
    send(&left, COMMON, &mut rng);

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(BUILD_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip(&mut provider),
            Peer::<T>::bootstrap()
                .protocol(Protocol::V2)
                .join(&mut newcomer),
        );
        served.expect("serve bootstrap");
        joined
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .sync_memory_budget(budget)
            .into_rumors()
    });

    send(&left, DIVERGENT, &mut rng);
    send(&right, DIVERGENT, &mut rng);
    (left, right)
}

/// Serialized wire time of one session at `budget` over the given pipe,
/// in exact one-way hops.
///
/// Measured as virtual elapsed time under the paused clock, which
/// advances only while every task is blocked on the wire, so wall
/// compute is excluded and the count is deterministic (the hop-trace
/// principle: every wire event lands on an exact delay multiple).
fn wire_hops<T>(budget: usize, pipe: usize, mint: &mut impl FnMut(&mut SmallRng) -> T) -> u64
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    let (left, right) = diverged(budget, mint);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("build current-thread runtime");
    let (mut a_link, mut b_link) = latency::delayed_pair(pipe, DELAY);
    let elapsed = runtime.block_on(async {
        let start = tokio::time::Instant::now();
        let (a_result, b_result) =
            tokio::join!(left.gossip(&mut a_link), right.gossip(&mut b_link));
        a_result.expect("peer A gossip");
        b_result.expect("peer B gossip");
        start.elapsed()
    });
    assert_eq!(
        left.snapshot().hash(),
        right.snapshot().hash(),
        "the measured session must converge",
    );
    (elapsed.as_millis() / DELAY.as_millis()) as u64
}

fn run_cells<T>(
    label: &str,
    encoded_m: usize,
    pipe: usize,
    targets: &[f64],
    mint: &mut impl FnMut(&mut SmallRng) -> T,
) where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    let (envelope, _) = envelope_and_wire_bytes();
    let overhead = 28usize;
    let transfer = wire_hops(UNBOUNDED, pipe, mint);
    assert!(transfer >= 4, "degenerate transfer count {transfer}");
    let bdp_messages = 2.0 * DIVERGENT as f64 / transfer as f64;
    let bdp_bytes = bdp_messages * (overhead + encoded_m) as f64;
    eprintln!(
        "[{label}] m={encoded_m} pipe={pipe}: transfer {transfer} hops, \
         BDP {bdp_messages:.0} messages ({bdp_bytes:.0} bytes)",
    );
    for &target in targets {
        // Budget chosen so the closed form predicts `target`.
        let budget =
            (bdp_bytes * envelope as f64 / (target * (overhead + encoded_m) as f64)) as usize;
        let closed = (bdp_bytes * envelope as f64
            / (budget as f64 * (overhead + encoded_m) as f64))
            .max(1.0);
        let session_len = (COMMON + DIVERGENT) as u64;
        let caps = window_capacities(session_len, session_len, budget);
        let k_max = caps.iter().copied().max().unwrap_or(1);
        let exact = (bdp_messages / k_max as f64).max(1.0);
        let hops = wire_hops(budget, pipe, mint);
        let observed = hops as f64 / transfer as f64;
        let fans = supply_decode_envelope_bytes();
        eprintln!(
            "[{label}] budget {budget} (dispute share {}): K {k_max}, \
             closed-form estimate {closed:.2}x, solve-derived {exact:.2}x, \
             observed {observed:.2}x ({hops} hops vs {transfer})",
            budget.saturating_sub(fans),
        );
        // The comparison of record: the measured session stays at or
        // inside the wave form at the actually derived window, in hops
        // (the ratio's native integer unit, so quantization cannot
        // manufacture an excess).
        assert!(
            hops as f64 <= (exact * f64::from(u32::try_from(transfer).expect("small"))).ceil(),
            "[{label}] observed {hops} hops exceed the solve-derived wave envelope \
             ({exact:.2}x of {transfer} transfer hops): a finding, not noise",
        );
    }
}

/// The validation run (see the module docs).
///
/// Prints, per cell, the closed-form estimate, the wave form at the
/// actually derived window, and the observed hop-count slowdown; the
/// assertion of record holds the observation at or inside the
/// solve-derived wave form, in hops.
#[test]
#[ignore = "one-shot validation instrument: run explicitly with --run-ignored"]
fn tradeoff_closed_form_validation_run() {
    // m = 8: minimal u64 records (the table's first column). Targets
    // (denominated in the closed-form estimate, a budget-choosing
    // device only) span a constricted, a near-crossover, and a
    // comfortable cell at derived windows of a thousand scopes and up.
    run_cells(
        "u64",
        8,
        256 * 1024,
        &[4.0, 1.3, 0.4],
        &mut |rng: &mut SmallRng| rng.next_u64(),
    );

    // m = 172: the design record (the table's third column), one
    // constricted cell on a wider pipe so the window stays past the
    // near-root band.
    let mut design = |rng: &mut SmallRng| {
        let mut payload = vec![0u8; 168];
        rng.fill_bytes(&mut payload);
        payload
    };
    run_cells("design", 172, 1024 * 1024, &[3.0], &mut design);
}
