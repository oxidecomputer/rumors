//! Measure-only distributional study of the stored integer code: histogram
//! the integers the skyline coding actually stores, then price every
//! candidate integer code closed-form on those histograms.
//!
//! This is the instrument behind the crate docs' integer-code figures (the
//! [`implementation`](before::implementation) essay's "Small values over
//! large" trade): the constants below are the as-run parameters of the
//! measurement quoted there. It changes nothing and asserts its own
//! taxonomy exactly (the reconciliation pin below), so a re-run at other
//! parameters is safe and cheap.
//!
//! # Emission-class taxonomy (derived from the code)
//!
//! A stored `Version` is one bit stream (`version/skyline.rs` module doc):
//! one preorder flag bit per node (`0` internal, `1` leaf, never
//! integer-coded), and one gamma code per leaf:
//!
//! - class FIRST: the first leaf in preorder stores its absolute height
//!   `v1 >= 0`, handed raw to `codec::encode_int` (which codes `m = v + 1`).
//! - class DELTA: every later leaf stores `z = zigzag(vi − vi−1)` with
//!   `k >= 0 -> 2k`, `k < 0 -> 2|k| − 1` (`version/skyline.rs`), handed to
//!   the same `encode_int`.
//!
//! `Party` id trees carry 2-bit presence tags per node and no integers
//! (`idbits`), so the whole integer-code question lives in the version
//! stream. The borsh leg writes the same stored bytes.
//!
//! # Mechanism
//!
//! No encoder instrumentation: the stored stream IS the encoder's output
//! (canonical uniqueness), so an offline walker re-parses each stored
//! version and histograms the decoded payload values per class. The walk
//! re-derives every gamma code length and asserts, per version:
//! `topology_bits + sum(len_gamma(value)) == Version::encoded_bits()` —
//! the reconciliation pin, exact by construction or the taxonomy is wrong.
//!
//! # Corpora
//!
//! - REALISTIC: the `space_consumption` simulation (same step functions,
//!   same per-run seeding), at the reduced parameters in the constants
//!   below (printed in the output); histograms every live stamp's version
//!   at every log-spaced checkpoint.
//! - ADVERSARIAL: the amplification board's shape corpus at scale 1.0,
//!   level 0, via `meter::board::study_family_versions` (the board's own
//!   bundles).

use std::collections::BTreeMap;

use before::meter::board;
use before::{Clock, Version};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

// ─── realistic-simulation parameters ────────────────────────────────────────
// Reduced relative to the `space_consumption` example's defaults; these are
// the as-run parameters of the crate docs' committed figures.

const RUNS: u64 = 10;
const DATA_ITERS: u64 = 20_000;
const PROCESS_ITERS: u64 = 10_000;
const ENTITIES: &[usize] = &[4, 8, 16, 32, 64, 128];
const POINTS_PER_DECADE: f64 = 8.0;

// ─── histograms ─────────────────────────────────────────────────────────────

/// Values as handed to the integer coder.
///
/// Values whose gamma mantissa fits 127 bits are kept exact; wider ones
/// (the hugeleaf-scale magnitudes) are binned by the bit length of
/// `m = v + 1`, which determines every candidate length except Rice's
/// (Rice diverges there and is flagged).
#[derive(Default, Clone)]
struct Hist {
    /// value -> count, exact, for values with bits(v+1) <= 127.
    small: BTreeMap<u128, u64>,
    /// bits(v+1) -> count for wider values.
    wide: BTreeMap<u64, u64>,
}

impl Hist {
    fn record_small(&mut self, v: u128) {
        *self.small.entry(v).or_insert(0) += 1;
    }
    fn record_wide(&mut self, mbits: u64) {
        *self.wide.entry(mbits).or_insert(0) += 1;
    }
    fn merge(&mut self, other: &Hist) {
        for (&v, &c) in &other.small {
            *self.small.entry(v).or_insert(0) += c;
        }
        for (&l, &c) in &other.wide {
            *self.wide.entry(l).or_insert(0) += c;
        }
    }
    fn count(&self) -> u64 {
        self.small.values().sum::<u64>() + self.wide.values().sum::<u64>()
    }
    /// Mass of values <= bound (wide values are all > any u128 bound used).
    fn mass_le(&self, bound: u128) -> u64 {
        self.small
            .iter()
            .take_while(|&(&v, _)| v <= bound)
            .map(|(_, &c)| c)
            .sum()
    }
    /// Total bits under a candidate code; None if the code diverges
    /// (Rice on a wide-binned value).
    fn price(&self, code: Code) -> Option<u128> {
        let mut total: u128 = 0;
        for (&v, &c) in &self.small {
            total += code.len_small(v)? as u128 * c as u128;
        }
        for (&l, &c) in &self.wide {
            total += code.len_wide(l)? as u128 * c as u128;
        }
        Some(total)
    }
}

/// Per-corpus accumulator: the two class histograms plus the structural
/// bookkeeping the reconciliation pin needs.
#[derive(Default, Clone)]
struct Corpus {
    first: Hist,
    delta: Hist,
    /// Total preorder flag bits (= node count) across versions.
    topo_bits: u128,
    /// Number of versions walked.
    versions: u64,
    /// Sum of `Version::encoded_bits()` (the actual encoder output).
    actual_bits: u128,
    /// Sum of per-version byte lengths (`ceil(bits/8)`, what rest storage pays).
    actual_bytes: u128,
    /// Sum over versions of `ceil((topo + priced_code_bits)/8)` per candidate,
    /// so the byte movement includes per-version padding. Indexed as CODES.
    code_bytes: Vec<u128>,
    /// Per-candidate divergence flag (Rice hit a wide value somewhere).
    code_diverged: Vec<bool>,
}

impl Corpus {
    fn new() -> Corpus {
        Corpus {
            code_bytes: vec![0; CODES.len()],
            code_diverged: vec![false; CODES.len()],
            ..Corpus::default()
        }
    }
    fn merge(&mut self, other: &Corpus) {
        self.first.merge(&other.first);
        self.delta.merge(&other.delta);
        self.topo_bits += other.topo_bits;
        self.versions += other.versions;
        self.actual_bits += other.actual_bits;
        self.actual_bytes += other.actual_bytes;
        for i in 0..CODES.len() {
            self.code_bytes[i] += other.code_bytes[i];
            self.code_diverged[i] |= other.code_diverged[i];
        }
    }
}

// ─── candidate code length functions (closed-form, on m = v + 1) ───────────

/// A candidate integer code, priced closed-form. `l` is bits(m), m = v+1.
#[derive(Clone, Copy, PartialEq)]
enum Code {
    Gamma,
    Delta,
    Omega,
    Zeta(u32),
    Rice(u32),
}

/// The candidate roster, gamma-as-built first (the ratio denominator).
const CODES: &[(&str, Code)] = &[
    ("gamma", Code::Gamma),
    ("delta", Code::Delta),
    ("omega", Code::Omega),
    ("zeta2", Code::Zeta(2)),
    ("zeta3", Code::Zeta(3)),
    ("zeta4", Code::Zeta(4)),
    ("rice0", Code::Rice(0)),
    ("rice1", Code::Rice(1)),
    ("rice2", Code::Rice(2)),
    ("rice3", Code::Rice(3)),
];

fn bits_of(m: u128) -> u32 {
    128 - m.leading_zeros()
}

impl Code {
    /// Code length for an exact value v (bits(v+1) <= 127).
    fn len_small(self, v: u128) -> Option<u64> {
        match self {
            Code::Rice(k) => {
                let q = v >> k;
                let q64: u64 = q.try_into().ok()?;
                Some(q64 + 1 + k as u64)
            }
            _ => self.len_wide(bits_of(v + 1) as u64),
        }
    }
    /// Code length from bits(m) alone; None where that underdetermines the
    /// length (Rice).
    fn len_wide(self, l: u64) -> Option<u64> {
        match self {
            Code::Gamma => Some(2 * l - 1),
            Code::Delta => {
                // gamma(l) then the l−1 mantissa bits of m.
                let lg = 63 - l.leading_zeros() as u64; // floor(log2 l), l >= 1
                Some((l - 1) + 2 * lg + 1)
            }
            Code::Omega => {
                // 1 terminator + the group for m (l bits) + groups for the
                // recursively coded lengths.
                if l == 1 {
                    return Some(1);
                }
                let mut total = 1 + l;
                let mut n = l - 1;
                while n > 1 {
                    let b = 64 - n.leading_zeros() as u64;
                    total += b;
                    n = b - 1;
                }
                Some(total)
            }
            Code::Zeta(k) => {
                // Boldi–Vigna zeta_k of m: unary(h+1) then truncated binary;
                // the short branch is taken exactly when k divides l−1.
                let k = k as u64;
                let h = (l - 1) / k;
                let s = (h + 1) * k;
                Some(h + 1 + s - if (l - 1).is_multiple_of(k) { 1 } else { 0 })
            }
            Code::Rice(_) => None,
        }
    }
}

// ─── the skyline stream walker ──────────────────────────────────────────────

/// Read bit `i` of an Msb0 packed stream.
fn bit(bytes: &[u8], i: u64) -> bool {
    // An in-range byte index fits `usize`: it indexes an allocated buffer.
    (bytes[(i / 8) as usize] >> (7 - i % 8)) & 1 == 1
}

/// Walk one stored version stream, recording payload values per class and
/// asserting the exact reconciliation identity.
fn walk(v: &Version, corpus: &mut Corpus) {
    let bytes = v.as_bytes();
    let bits = v.encoded_bits();
    let mut pos = 0u64;
    let mut pending = 1u64; // subtrees still owed
    let mut nodes = 0u128;
    let mut first = true;
    let mut gamma_bits = 0u128;
    // Per-candidate payload bits for this version (byte-ceiling accounting).
    let mut code_bits = vec![Some(0u128); CODES.len()];

    while pending > 0 {
        assert!(pos < bits, "walker ran past the stream: taxonomy wrong");
        let flag = bit(bytes, pos);
        pos += 1;
        nodes += 1;
        if !flag {
            pending += 1; // internal (`0`): two children replace one owed subtree
            continue;
        }
        pending -= 1;
        // Leaf (`1`) payload: one gamma code. Count the unary prefix.
        let mut k = 0u64;
        while !bit(bytes, pos) {
            pos += 1;
            k += 1;
            assert!(pos < bits, "truncated gamma: taxonomy wrong");
        }
        pos += 1; // the leading 1 of the mantissa
        gamma_bits += (2 * k + 1) as u128;
        let hist = if first {
            &mut corpus.first
        } else {
            &mut corpus.delta
        };
        if k <= 126 {
            let mut m: u128 = 1;
            for _ in 0..k {
                m = (m << 1) | (bit(bytes, pos) as u128);
                pos += 1;
            }
            let v = m - 1;
            hist.record_small(v);
            for (slot, (_, code)) in code_bits.iter_mut().zip(CODES) {
                *slot = match (*slot, code.len_small(v)) {
                    (Some(t), Some(l)) => Some(t + l as u128),
                    _ => None,
                };
            }
        } else {
            pos += k; // skip the wide mantissa remainder
            let l = k + 1;
            hist.record_wide(l);
            for (slot, (_, code)) in code_bits.iter_mut().zip(CODES) {
                *slot = match (*slot, code.len_wide(l)) {
                    (Some(t), Some(l)) => Some(t + l as u128),
                    _ => None,
                };
            }
        }
        first = false;
    }
    assert_eq!(pos, bits, "stream not exact: taxonomy wrong");
    // The reconciliation pin: topology + gamma payload = the whole stream.
    assert_eq!(
        nodes + gamma_bits,
        bits as u128,
        "topology + gamma bits != encoded_bits: taxonomy wrong"
    );
    corpus.topo_bits += nodes;
    corpus.versions += 1;
    corpus.actual_bits += bits as u128;
    corpus.actual_bytes += bits.div_ceil(8) as u128;
    for (i, bits) in code_bits.iter().enumerate() {
        match bits {
            Some(payload) => corpus.code_bytes[i] += (nodes + payload).div_ceil(8),
            None => corpus.code_diverged[i] = true,
        }
    }
}

// ─── the realistic simulation (space_consumption replicated, reduced) ──────

fn seed_for(tag: u64, n: usize, run: u64) -> u64 {
    const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
    GOLDEN ^ (tag << 56) ^ ((n as u64) << 32) ^ run
}

fn checkpoints(max: u64) -> Vec<u64> {
    let mut points = Vec::new();
    let mut last = 0u64;
    let k_max = ((max as f64).log10() * POINTS_PER_DECADE).ceil() as i64;
    for k in 0..=k_max {
        let x = (10f64.powf(k as f64 / POINTS_PER_DECADE).round() as u64).min(max);
        if x != last {
            points.push(x);
            last = x;
        }
    }
    if points.last() != Some(&max) {
        points.push(max);
    }
    points
}

fn build_population(n: usize) -> Vec<Clock> {
    let mut clocks = vec![Clock::seed()];
    while clocks.len() < n {
        let round = clocks.len();
        for i in 0..round {
            if clocks.len() >= n {
                break;
            }
            let child = clocks[i].fork();
            clocks.push(child);
        }
    }
    clocks
}

fn step_data(clocks: &mut Vec<Clock>, rng: &mut StdRng) {
    let parent = rng.gen_range(0..clocks.len());
    let child = clocks[parent].fork();
    clocks.push(child);
    let who = rng.gen_range(0..clocks.len());
    clocks[who].tick();
    let donor = clocks.swap_remove(rng.gen_range(0..clocks.len()));
    let target = rng.gen_range(0..clocks.len());
    clocks[target]
        .join(donor)
        .expect("clocks forked from one seed are disjoint");
}

fn step_process(clocks: &mut [Clock], n: usize, rng: &mut StdRng) {
    let who = rng.gen_range(0..n);
    clocks[who].tick();
    let sender = rng.gen_range(0..n);
    let mut receiver = rng.gen_range(0..n);
    if receiver == sender {
        receiver = (receiver + 1) % n;
    }
    let peeked = clocks[sender].version().clone();
    clocks[receiver] |= peeked;
}

/// One simulation run: histogram every live stamp's version at every
/// log-spaced checkpoint. Returns (all-checkpoints corpus, final-checkpoint
/// corpus).
fn simulate(tag: u64, n: usize, iters: u64, run: u64) -> (Corpus, Corpus) {
    let cps = checkpoints(iters);
    let mut rng = StdRng::seed_from_u64(seed_for(tag, n, run));
    let mut clocks = build_population(n);
    let mut all = Corpus::new();
    let mut fin = Corpus::new();
    let mut next_cp = 0usize;
    for iteration in 1..=iters {
        if tag == 0 {
            step_data(&mut clocks, &mut rng);
        } else {
            step_process(&mut clocks, n, &mut rng);
        }
        if next_cp < cps.len() && iteration == cps[next_cp] {
            for c in &clocks {
                walk(c.version(), &mut all);
                if iteration == iters {
                    walk(c.version(), &mut fin);
                }
            }
            next_cp += 1;
        }
    }
    (all, fin)
}

// ─── reporting ──────────────────────────────────────────────────────────────

fn pct(num: u64, den: u64) -> f64 {
    if den == 0 {
        return f64::NAN;
    }
    100.0 * num as f64 / den as f64
}

fn mass_table(label: &str, h: &Hist) {
    let n = h.count();
    let wide: u64 = h.wide.values().sum();
    println!(
        "  {label:<28} n={n:<12} P(v=0)={:>6.2}%  P(v<=1)={:>6.2}%  P(v<=3)={:>6.2}%  P(v<=15)={:>6.2}%  wide(>=2^127)={wide}",
        pct(h.mass_le(0), n),
        pct(h.mass_le(1), n),
        pct(h.mass_le(3), n),
        pct(h.mass_le(15), n),
    );
}

fn price_table(label: &str, c: &Corpus) {
    println!("\n== {label} ==");
    println!(
        "  versions={} topo_bits={} actual_bits={} actual_bytes={}",
        c.versions, c.topo_bits, c.actual_bits, c.actual_bytes
    );
    mass_table("FIRST (absolute height)", &c.first);
    mass_table("DELTA (zigzag magnitude)", &c.delta);

    let gamma_first = c.first.price(Code::Gamma).expect("gamma is total");
    let gamma_delta = c.delta.price(Code::Gamma).expect("gamma is total");
    // Reconciliation: histogram-priced gamma + topology == actual bits.
    let priced = gamma_first + gamma_delta + c.topo_bits;
    println!(
        "  reconciliation: priced(gamma)+topo = {} vs actual_bits = {} -> {}",
        priced,
        c.actual_bits,
        if priced == c.actual_bits {
            "EXACT"
        } else {
            "MISMATCH"
        }
    );
    let gamma_total = gamma_first + gamma_delta;
    println!(
        "  {:<7} {:>16} {:>8} {:>16} {:>8} {:>16} {:>8} {:>14} {:>8}",
        "code",
        "FIRST bits",
        "/gamma",
        "DELTA bits",
        "/gamma",
        "payload bits",
        "/gamma",
        "bytes@rest",
        "/gamma"
    );
    for (i, (name, code)) in CODES.iter().enumerate() {
        let f = c.first.price(*code);
        let d = c.delta.price(*code);
        let row = |x: Option<u128>, g: u128| match x {
            Some(x) => (format!("{x}"), format!("{:.4}", x as f64 / g as f64)),
            None => ("diverges".into(), "-".into()),
        };
        let (fs, fr) = row(f, gamma_first);
        let (ds, dr) = row(d, gamma_delta);
        let (ts, tr) = row(
            match (f, d) {
                (Some(f), Some(d)) => Some(f + d),
                _ => None,
            },
            gamma_total,
        );
        let (bs, br) = if c.code_diverged[i] {
            ("diverges".into(), "-".into())
        } else {
            (
                format!("{}", c.code_bytes[i]),
                format!("{:.4}", c.code_bytes[i] as f64 / c.actual_bytes as f64),
            )
        };
        println!("  {name:<7} {fs:>16} {fr:>8} {ds:>16} {dr:>8} {ts:>16} {tr:>8} {bs:>14} {br:>8}");
    }
    // Best fixed Rice parameter per class, from the histogram (adaptivity
    // caveat: a fixed k is a protocol constant).
    for (cls, h) in [("FIRST", &c.first), ("DELTA", &c.delta)] {
        let best = (0..=8u32)
            .filter_map(|k| h.price(Code::Rice(k)).map(|b| (k, b)))
            .min_by_key(|&(_, b)| b);
        match best {
            Some((k, b)) => println!(
                "  best rice for {cls}: k={k} at {b} bits ({:.4} of gamma)",
                b as f64
                    / (if cls == "FIRST" {
                        gamma_first
                    } else {
                        gamma_delta
                    }) as f64
            ),
            None => println!("  best rice for {cls}: diverges at every k (wide values)"),
        }
    }
}

fn main() {
    // ── adversarial corpus: the amplification board's bundles at scale 1.0 ──
    let mut adv = Corpus::new();
    println!("== ADVERSARIAL per-family (board bundles, scale 1.0, level 0) ==");
    for (name, versions) in board::study_family_versions(1.0) {
        let mut fam = Corpus::new();
        for bytes in &versions {
            let v = Version::decode(&bytes[..]).expect("board bundles are canonical");
            walk(&v, &mut fam);
        }
        let n_first = fam.first.count();
        let n_delta = fam.delta.count();
        println!(
            "  {name:<16} versions={:<6} first_n={n_first:<6} delta_n={n_delta:<9} P(delta=0)={:>6.2}% P(delta<=3)={:>6.2}%",
            fam.versions,
            pct(fam.delta.mass_le(0), n_delta),
            pct(fam.delta.mass_le(3), n_delta),
        );
        adv.merge(&fam);
    }
    price_table("ADVERSARIAL aggregate (all families)", &adv);

    // ── realistic corpus: the reduced-parameter simulation ──
    println!(
        "\n== REALISTIC simulation parameters: runs={RUNS} data_iters={DATA_ITERS} process_iters={PROCESS_ITERS} entities={ENTITIES:?} =="
    );
    for (tag, label, iters) in [
        (0u64, "data (dynamic)", DATA_ITERS),
        (1u64, "process (static)", PROCESS_ITERS),
    ] {
        let results: Vec<(Corpus, Corpus)> = ENTITIES
            .par_iter()
            .flat_map(|&n| {
                (0..RUNS)
                    .into_par_iter()
                    .map(move |run| simulate(tag, n, iters, run))
            })
            .collect();
        let mut all = Corpus::new();
        let mut fin = Corpus::new();
        for (a, f) in &results {
            all.merge(a);
            fin.merge(f);
        }
        price_table(&format!("REALISTIC {label}: all checkpoints"), &all);
        price_table(&format!("REALISTIC {label}: final checkpoint only"), &fin);
    }
}
