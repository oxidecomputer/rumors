//! One-off profiling harness for the perf-probe investigation.
//!
//! Rebuilds the bench suite's corpus shapes through the public API,
//! then spins each hot operation in its own `#[inline(never)]` loop so
//! a sampling profiler attributes cycles per operation. Run with
//! `--features oracle` to also print the oracle's timings for the same
//! plans (ratio anchor only).
//!
//! Usage: cargo run -p before --profile bench --example perf_probe --features oracle [n]

use before::{Clock, Party, Version};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::hint::black_box;
use std::time::Instant;

const SEED: u64 = 0x1737_C10C_C0DE;
const DISCARD: u8 = u8::MAX;

struct Plan {
    schedule: Vec<usize>,
    label: Vec<u8>,
    ticks: Vec<u32>,
}

fn plan(rng: &mut StdRng, n: usize, groups: u8) -> Plan {
    let schedule: Vec<usize> = (0..n - 1).map(|i| rng.gen_range(0..=i)).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.shuffle(rng);
    let mut label = vec![DISCARD; n];
    for (g, &m) in order.iter().take(groups as usize).enumerate() {
        label[m] = g as u8;
    }
    for &m in &order[groups as usize..] {
        label[m] = if rng.gen_bool(0.33) {
            DISCARD
        } else {
            rng.gen_range(0..groups)
        };
    }
    let ticks: Vec<u32> = (0..n).map(|_| rng.gen_range(0..4)).collect();
    Plan {
        schedule,
        label,
        ticks,
    }
}

fn impl_clocks(plan: &Plan, groups: u8) -> Vec<Clock> {
    let mut universe = vec![Clock::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    for (m, c) in universe.iter_mut().enumerate() {
        for _ in 0..plan.ticks[m] {
            c.tick();
        }
    }
    let mut slots: Vec<Option<Clock>> = universe.into_iter().map(Some).collect();
    (0..groups)
        .map(|g| {
            let members: Vec<Clock> = (0..slots.len())
                .filter(|&m| plan.label[m] == g)
                .filter_map(|m| slots[m].take())
                .collect();
            members
                .into_iter()
                .reduce(|mut acc, c| {
                    acc.join(c).map_err(|_| ()).expect("disjoint");
                    acc
                })
                .expect("nonempty group")
        })
        .collect()
}

/// The ownership-hole pair: the full universe's joined version against
/// one late-forked member's party.
///
/// A peer owning a vanishing custody fraction registering an event on
/// a fully-received version. The aliased party never re-enters
/// protocol use, so linearity holds for everything the loop observes.
fn hole_pair(plan: &Plan) -> (Party, Version) {
    let mut universe = vec![Clock::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    for (m, c) in universe.iter_mut().enumerate() {
        for _ in 0..plan.ticks[m] {
            c.tick();
        }
    }
    let probe = universe
        .last()
        .expect("nonempty universe")
        .dangerously_alias();
    let (party, _) = probe.into_parts();
    let full = universe
        .into_iter()
        .reduce(|mut acc, c| {
            acc.join(c).map_err(|_| ()).expect("disjoint");
            acc
        })
        .expect("nonempty universe");
    let (_, version) = full.into_parts();
    (party, version)
}

#[cfg(feature = "oracle")]
fn oracle_clocks(plan: &Plan, groups: u8) -> Vec<before::oracle::Clock> {
    use before::oracle;
    let mut universe = vec![oracle::Clock::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    for (m, c) in universe.iter_mut().enumerate() {
        for _ in 0..plan.ticks[m] {
            c.tick();
        }
    }
    let mut slots: Vec<Option<oracle::Clock>> = universe.into_iter().map(Some).collect();
    (0..groups)
        .map(|g| {
            let members: Vec<oracle::Clock> = (0..slots.len())
                .filter(|&m| plan.label[m] == g)
                .filter_map(|m| slots[m].take())
                .collect();
            members
                .into_iter()
                .reduce(|mut acc, c| {
                    acc.join(c).map_err(|_| ()).expect("disjoint");
                    acc
                })
                .expect("nonempty group")
        })
        .collect()
}

/// Runs `f` until ~`budget_ms` elapsed; returns (iters, ns/iter).
fn time_loop(budget_ms: u64, mut f: impl FnMut()) -> (u64, f64) {
    // Warm up briefly.
    for _ in 0..8 {
        f();
    }
    let start = Instant::now();
    let mut iters = 0u64;
    while start.elapsed().as_millis() < budget_ms as u128 {
        for _ in 0..16 {
            f();
        }
        iters += 16;
    }
    (iters, start.elapsed().as_nanos() as f64 / iters as f64)
}

#[inline(never)]
fn loop_version_tick(budget_ms: u64, party: &Party, version: &Version) -> (u64, f64) {
    time_loop(budget_ms, || {
        let mut v = version.clone();
        v.tick(party);
        black_box(&v);
    })
}

#[inline(never)]
fn loop_version_join(budget_ms: u64, a: &Version, b: &Version) -> (u64, f64) {
    time_loop(budget_ms, || {
        black_box(a | b);
    })
}

#[inline(never)]
fn loop_clock_join(budget_ms: u64, a: &Clock, b: &Clock) -> (u64, f64) {
    time_loop(budget_ms, || {
        let mut x = a.dangerously_alias();
        let y = b.dangerously_alias();
        x.join(y).map_err(|_| ()).expect("disjoint");
        black_box(&x);
    })
}

#[inline(never)]
fn loop_clock_decode(budget_ms: u64, bytes: &[u8]) -> (u64, f64) {
    time_loop(budget_ms, || {
        black_box(Clock::decode(bytes).unwrap());
    })
}

#[inline(never)]
fn loop_clock_encode(budget_ms: u64, clock: &Clock) -> (u64, f64) {
    time_loop(budget_ms, || {
        black_box(clock.encode());
    })
}

#[inline(never)]
fn loop_version_cmp(budget_ms: u64, a: &Version, b: &Version) -> (u64, f64) {
    time_loop(budget_ms, || {
        black_box(a.partial_cmp(b));
    })
}

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8192);
    let budget: u64 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    // Optional op filter: run only the named loop (for sampling profilers).
    let only: Option<String> = std::env::args().nth(3);
    let run = |name: &str| only.as_deref().is_none_or(|o| o == name);

    // Same salts as benches/clock.rs: tick uses salt 1 (1 group), join salt 3
    // (2 groups).
    let mut r1 = StdRng::seed_from_u64(SEED.wrapping_add(1));
    let p1 = plan(&mut r1, n, 1);
    let mut r3 = StdRng::seed_from_u64(SEED.wrapping_add(3));
    let p3 = plan(&mut r3, n, 2);

    let tick_clock = impl_clocks(&p1, 1).pop().unwrap();
    let bytes = tick_clock.encode();
    let (tick_party, tick_version) = tick_clock.into_parts();
    let join_pair = impl_clocks(&p3, 2);
    let (ja, jb) = (&join_pair[0], &join_pair[1]);

    // Leaf counts, read off the text rendering: every leaf is a digit run.
    let leaves = |v: &Version| {
        let s = v.to_string();
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .count()
    };
    println!(
        "n={n} clock bytes={} party bits={} version bits={} leaves={} join operands: {}b/{} leaves, {}b/{} leaves",
        bytes.len(),
        tick_party.encoded_bits(),
        tick_version.encoded_bits(),
        leaves(&tick_version),
        ja.version().encoded_bits(),
        leaves(ja.version()),
        jb.version().encoded_bits(),
        leaves(jb.version()),
    );

    if run("tick") {
        let (i, t) = loop_version_tick(budget, &tick_party, &tick_version);
        println!("version_tick   {t:>12.1} ns/op ({i} iters)");
    }
    if run("holetick") {
        let mut rh = StdRng::seed_from_u64(SEED.wrapping_add(1));
        let ph = plan(&mut rh, n, 1);
        let (hole_party, hole_version) = hole_pair(&ph);
        println!(
            "holetick operands: party bits={} version bits={}",
            hole_party.encoded_bits(),
            hole_version.encoded_bits(),
        );
        let (i, t) = loop_version_tick(budget, &hole_party, &hole_version);
        println!("version_holetick {t:>10.1} ns/op ({i} iters)");
    }
    if run("holeproj") {
        let mut rh = StdRng::seed_from_u64(SEED.wrapping_add(1));
        let ph = plan(&mut rh, n, 1);
        let (hole_party, hole_version) = hole_pair(&ph);
        let (i, t) = time_loop(budget, || {
            black_box((&hole_version / &hole_party).to_version());
        });
        println!("version_holeproj {t:>10.1} ns/op ({i} iters)");
    }
    if run("holecmp") {
        let mut rh = StdRng::seed_from_u64(SEED.wrapping_add(1));
        let ph = plan(&mut rh, n, 1);
        let (hole_party, hole_version) = hole_pair(&ph);
        // A second, byte-identical pair in distinct buffers: the
        // deterministic construction re-run (Party is deliberately not
        // Clone).
        let (party2, version2) = hole_pair(&ph);
        let own_a = &hole_version / &hole_party;
        let own_b = &version2 / &party2;
        // Equality of equal projections held in distinct buffers: the
        // full masked co-walk to exhaustion, no early exit.
        let (i, t) = time_loop(budget, || {
            black_box(own_a == own_b);
        });
        println!("version_holecmp {t:>11.1} ns/op ({i} iters)");
    }
    if run("vjoin") {
        let (i, t) = loop_version_join(budget, ja.version(), jb.version());
        println!("version_join   {t:>12.1} ns/op ({i} iters)");
    }
    if run("vcmp") {
        let (i, t) = loop_version_cmp(budget, ja.version(), jb.version());
        println!("version_cmp    {t:>12.1} ns/op ({i} iters)");
    }
    if run("cjoin") {
        let (i, t) = loop_clock_join(budget, ja, jb);
        println!("clock_join     {t:>12.1} ns/op ({i} iters)");
    }
    if run("decode") {
        let (i, t) = loop_clock_decode(budget, &bytes);
        println!("clock_decode   {t:>12.1} ns/op ({i} iters)");
    }
    if run("encode") {
        let (i, t) = loop_clock_encode(
            budget,
            &tick_clock_encode_target(&tick_party, &tick_version),
        );
        println!("clock_encode   {t:>12.1} ns/op ({i} iters)");
    }

    #[cfg(feature = "oracle")]
    if only.is_none() {
        let otick = oracle_clocks(&p1, 1).pop().unwrap();
        let (op1, ov1) = otick.into_parts();
        let opair = oracle_clocks(&p3, 2);
        let (oa, ob) = (opair[0].clone(), opair[1].clone());
        let (i, t) = time_loop(budget, || {
            let mut v = ov1.clone();
            v.tick(&op1);
            black_box(&v);
        });
        println!("oracle_tick+cl {t:>12.1} ns/op ({i} iters)");
        let (i, t) = time_loop(budget, || {
            black_box(ov1.clone());
        });
        println!("oracle_vclone  {t:>12.1} ns/op ({i} iters)");
        let (ova, ovb) = (oa.version(), ob.version());
        let (i, t) = time_loop(budget, || {
            black_box(ova.clone() | ovb.clone());
        });
        println!("oracle_vjoin   {t:>12.1} ns/op ({i} iters)");
        let (i, t) = time_loop(budget, || {
            let (mut a, b) = (oa.clone(), ob.clone());
            a.join(b).map_err(|_| ()).expect("disjoint");
            black_box(&a);
        });
        println!("oracle_cjoin   {t:>12.1} ns/op ({i} iters)");
    }
}

/// Reassemble a clock for the encode loop (the original was consumed by
/// `into_parts`).
fn tick_clock_encode_target(party: &Party, version: &Version) -> Clock {
    Clock::from_parts(party.dangerously_alias(), version.clone())
}
