//! Empirical network profile for the rumors gossip protocol.
//!
//! # Purpose
//!
//! This example characterizes the protocol's wire behaviour as a function of
//! the gossip state on each side. It runs many synthetic gossip sessions over
//! an in-memory pipe pair, with byte-counting and phase-change-tracking I/O
//! wrappers on both sides of an optional zstd compression layer, and emits one
//! CSV row per sample so the distribution can be analyzed offline.
//!
//! Each row also carries a *theoretical* prediction for the same cell from
//! [`predict`], computed from a structural model of the wire format. Comparing
//! `bytes_a` to `pred_bytes_a` (and the rounds equivalents) is the primary way
//! to validate that the model still matches the implementation when either
//! changes; the cell-mean residual should approach zero as samples grow, and
//! the per-sample residual should approach a small irreducible noise floor.
//!
//! # Input space
//!
//! Six dimensions, swept independently as powers of two:
//!
//! | dim    | meaning                                                              | sweep                      |
//! | ------ | -------------------------------------------------------------------- | -------------------------- |
//! | `P`    | distinct background parties whose inserts both sides have observed   | `{1, 2, …, 2^max_parties}` |
//! | `S`    | shared elements both sides hold under the same key                   | `{0, 1, …, 2^max_shared}`  |
//! | `DA`   | elements only A has (alice's own fresh inserts)                      | `{0, 1, …, 2^max_distinct}`|
//! | `DB`   | elements only B has (bob's own fresh inserts)                        | `{0, 1, …, 2^max_distinct}`|
//! | `RA`   | shared elements A has redacted, B has not yet learned about          | `{0, 1, …, 2^max_redacted}`|
//! | `RB`   | shared elements B has redacted, A has not yet learned about          | `{0, 1, …, 2^max_redacted}`|
//!
//! For each cell `(P, S, DA, DB, RA, RB)` the example constructs a fresh
//! synthetic state:
//!
//! 1. Allocate `min(P, S+RA+RB)` ancestor parties with random identifiers and
//!    distribute the `S + RA + RB` ancestor inserts evenly across them.
//!    (Parties with zero inserts would be invisible to either side, so we cap.)
//! 2. Both alice and bob process every ancestor party — each side's version
//!    vector now contains one entry per contributing background party.
//! 3. Partition the ancestor keys disjointly into `[shared | redact-by-A |
//!    redact-by-B]`. Alice redacts her slice; bob redacts his. Redactions
//!    therefore reference *existing* keys and never overlap, so the measurement
//!    reflects protocol behaviour rather than redaction-overlap statistics.
//! 4. Alice inserts `DA` fresh `()` units under her own party id; bob inserts
//!    `DB`. The value type is `()` so we measure protocol overhead unmixed with
//!    application payload size.
//!
//! Per sample, the ancestor party names, alice's name, and bob's name are all
//! drawn freshly from a per-job RNG seed derived from `--seed` plus the cell
//! and sample index. Repeated samples therefore vary the hash distribution at
//! every level of the tree without changing the structural counts.
//!
//! # Output
//!
//! Per row:
//!
//! | column                                   | meaning                                                |
//! | ---------------------------------------- | ------------------------------------------------------ |
//! | `bytes_a`, `bytes_b`                     | bytes each side transmitted at the protocol layer     |
//! | `bytes_a_compressed`, `bytes_b_compressed` | bytes each side put on the pipe after zstd            |
//! | `transitions_a`, `transitions_b`         | phase-change count between R and W on each side       |
//! | `rounds_a`, `rounds_b`                   | `transitions / 2`; may be fractional at protocol end  |
//! | `pred_bytes_*`, `pred_rounds_*`          | structural-model prediction for the same cell         |
//!
//! Phase transitions are tracked at the *protocol* (pre-compression) layer
//! because that's where the logical round-trips live; the compression layer
//! reshuffles bytes but never the round count.
//!
//! # I/O stack
//!
//! Inside-out, per side:
//!
//! ```text
//! protocol  <->  protocol-layer counter  <->  zstd encoder/decoder
//!           <->  wire-layer counter      <->  std::io::pipe
//! ```
//!
//! The protocol uses `rumors::sync::Known::gossip` so we drive synchronous
//! `Read` + `Write` halves of a `std::io::pipe`; one helper thread runs bob's
//! side per sample, alice's runs on the rayon worker thread. zstd's
//! `Encoder::auto_finish` writes a closing block on drop so the peer's decoder
//! sees a clean EOF when gossip completes.
//!
//! # What the measurements demonstrate about the protocol
//!
//! Running the full sweep and analyzing the CSV reveals the protocol's
//! performance shape, much of which is reproducible from the wire format alone:
//!
//! ## Round trips: essentially constant, structurally logarithmic
//!
//! Each `exchange` call in the mirror protocol descends two heights into the
//! 256-ary trie. For a union of `N` random leaves, birthday-paradox depth is
//! `O(log_256 N)`, so the round count fits
//!
//! ```text
//! rounds = 1.82 + 1.013 · log_256(N + 1) − 0.24 · 𝟙[side is "quiet"]
//! ```
//!
//! where the quiet indicator captures the side with `DA == 0 ∧ RB == 0` (the
//! one that has neither fresh inserts to push nor remote redactions to learn
//! about) terminating one round earlier. The 1.013 coefficient is within
//! fitting precision of the theoretical 1.0; the formula is structural.
//!
//! Practically: rounds plateau at ~2.5 across the calibration range (1 ≤ N ≤
//! a few hundred). A network must reach `N > 256` to see a third round and
//! `N > 65 536` to see a fourth. This is the key advantage of the 256-ary
//! trie: convergence is latency-bounded by 2–4 round trips, not by `log₂ N`.
//!
//! ## Bytes: exact at the boundary, linear in the active leaves
//!
//! Two structural pieces are reproducible exactly from the wire format:
//!
//! * **Connect frame**: each side ships its `Version<Bytes>` as a 4-byte
//!   length-delimited frame containing a borsh OrdMap of `(Bytes(party_hash),
//!   u64(counter))` pairs. Total `8 + 44 · np` bytes, where `np` is the
//!   distinct-party count in that side's version vector. When the two
//!   versions match (only happens when `DA + DB + RA + RB == 0`), both sides
//!   bail at this point — bytes are exactly `8 + 44 · np`, rounds exactly 0.5.
//!
//! * **Per-transmitted-leaf Version cost**: every leaf shipped in `providing`
//!   carries the originator's insert-time `Version<P>`, exactly `4 + 44 · np`
//!   bytes per leaf. Empirically this fits with coefficient 1.014 ± 0.05,
//!   matching the theoretical 1.0 to fitting precision.
//!
//! The remaining ~48 bytes per A-originated leaf (path-compressed prefix,
//! `providing`'s OrdMap key, framing share) and the per-shared-element
//! descent step (~17 bytes) are fit constants, but each has a structural
//! interpretation traceable to the wire-format docs on the mirror protocol's
//! message types.
//!
//! ## What scales with what
//!
//! * **Linear in the side's own outgoing elements** (`DA` for A, `DB` for B).
//!   Each fresh leaf alice originates costs alice `(4 + 44·np) + ~48` bytes;
//!   bob pays only ~2 bytes per A-originated leaf.
//! * **Logarithmically weak in the union size** for rounds: rounds barely
//!   move as the union grows by a factor of 100.
//! * **`O(np)` in the version-vector size**, which is `min(P, S+RA+RB) + 𝟙[acted]`.
//!   At `np = 32` the connect frame alone is 1.4 KiB; every transmitted leaf
//!   adds another 1.4 KiB.
//! * **`O(1)` in shared elements** for rounds (S only affects the union size
//!   under the log).
//! * **`O(1)` in redactions** for rounds, and only `~16` bytes per element
//!   for bytes (descent through an empty-now subtree).
//!
//! ## Compression: dominated by repeating party identifiers
//!
//! Party identifiers are 32-byte blake3 hashes. Each transmitted leaf
//! reprints every known party's hash in its `Version<P>` payload, and so does
//! every connect frame. With `P = 32`, the version vector for a typical leaf
//! is ~1.4 KiB of repeating 32-byte literals — textbook zstd dictionary
//! material.
//!
//! Empirically, the compression ratio (wire bytes / protocol bytes) falls
//! monotonically as `P` grows:
//!
//! | `P`  | mean ratio |
//! | ---- | ---------- |
//! |  1   | 0.64       |
//! |  4   | 0.47       |
//! |  16  | 0.36       |
//! |  32  | 0.36       |
//!
//! For larger transmitted batches (`DA ≥ 8` at `P = 32`), the ratio drops to
//! ~0.1: the protocol-layer overhead is almost entirely compressed away,
//! leaving only the high-entropy tree-descent hashes and the application
//! payload (incompressible by assumption) on the wire.
//!
//! Compression does not affect the round count.
//!
//! # Validating the prediction
//!
//! For each row, compute `(bytes_a - pred_bytes_a)` and `(rounds_a -
//! pred_rounds_a)`. The per-sample distribution has an irreducible noise
//! floor (~190 bytes / ~0.28 rounds RMS) driven by random hash distribution,
//! since which side is initiator vs. responder is decided by the
//! lexicographic order of version-vector OrdMaps. Cell-mean residuals
//! (averaging over the per-sample noise) should approach zero as samples
//! grow; that's the convergence test the prediction is built for.
//!
//! A run with `--samples 4 --max-shared 4 --max-distinct 4 --max-redacted 3
//! --max-parties 5` produces enough cells across `P ∈ {1, …, 32}` to confirm
//! both the structural pieces (exact agreement on connect-bail) and the
//! near-zero bias of the full formula.
//!
//! # When the protocol fits this workload model
//!
//! From the structure above, the protocol is well-suited to networks with:
//!
//! * Tens to low hundreds of stable parties (so `44·np` stays small or
//!   compresses well).
//! * Rumor payloads at least a few hundred bytes (so the per-leaf overhead
//!   doesn't dominate the application data).
//! * Periodic reconciliation on a timer of seconds to minutes (so the
//!   2–3 round trips per gossip are dwarfed by the gossip interval).
//! * Peers usually mostly in sync, occasionally diverging by a small delta
//!   (so linear-in-delta bandwidth is what's being paid, not linear-in-state).
//!
//! Compression is a large multiplier for any network with many parties: it
//! turns the `O(P²)` interaction between party count and transmitted leaves
//! into effectively `O(payload + small constant)`.

use std::{
    fs::File,
    io::{self, BufWriter, Read, Write, pipe},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Parser, ValueEnum};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rand::{SeedableRng, rngs::SmallRng};
use rayon::prelude::*;
use rumors::Key;
use rumors::sync::{Known, ignore};

#[derive(Parser, Debug)]
#[command(
    about = "Empirically measure rumors gossip bandwidth and round trips across a 5-D input space."
)]
struct Args {
    /// Output CSV path. The file is created (or truncated) and written
    /// row-by-row with a flush after each sample.
    #[arg(short, long)]
    output: PathBuf,

    /// Samples per cell. Each sample uses fresh random party identifiers,
    /// derived deterministically from `--seed` + (cell index, sample index).
    #[arg(long, default_value_t = 1)]
    samples: u32,

    /// Max exponent for the `shared` axis. Sweep is {0, 1, 2, …, 2^N}.
    #[arg(long, default_value_t = 6)]
    max_shared: u32,

    /// Max exponent for each `distinct_*` axis. Sweep is {0, 1, 2, …, 2^N}
    /// independently on each side.
    #[arg(long, default_value_t = 6)]
    max_distinct: u32,

    /// Max exponent for each `redacted_*` axis. Sweep is {0, 1, 2, …, 2^N}
    /// independently on each side.
    #[arg(long, default_value_t = 6)]
    max_redacted: u32,

    /// Max exponent for the `parties` axis. Sweep is {1, 2, 4, …, 2^N}
    /// (always at least one ancestor party). Default 5 → up to 32 parties.
    #[arg(long, default_value_t = 5)]
    max_parties: u32,

    /// RNG seed for reproducibility. Defaults to wall-clock nanoseconds.
    #[arg(long)]
    seed: Option<u64>,

    /// zstd compression level for the wire layer. Default 3 matches the zstd
    /// CLI default and trades CPU for ratio in the usual sweet spot.
    #[arg(long, default_value_t = 3)]
    zstd_level: i32,

    /// Threading strategy for the per-sample peer (bob).
    ///
    /// `spawn`: `thread::spawn` a fresh OS thread per sample (one-shot).
    /// `pool`: reuse a long-lived bob worker per rayon worker via thread-local
    /// channels. Avoids per-sample thread creation/teardown.
    #[arg(long, value_enum, default_value_t = Mode::Pool)]
    mode: Mode,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Mode {
    /// Fresh `thread::spawn` per sample.
    Spawn,
    /// Long-lived bob worker per rayon thread, fed via channels.
    Pool,
}

/// Per-side measurement state. Tracks bytes at both the protocol (uncompressed)
/// and wire (compressed) layers, plus phase transitions on the protocol layer.
///
/// Only ever touched by a single thread (the rayon worker for alice, the bob
/// worker for bob), but the wrapped handle has to be `Send` because
/// [`rumors::sync::Known::gossip`]'s `R`/`W` parameters require it. We use
/// `Arc<Mutex<…>>`: in single-threaded use the mutex is always uncontended,
/// so each lock is a single uncontended atomic op — measurably negligible
/// against the I/O the example is profiling.
#[derive(Default, Clone, Copy)]
struct SideStats {
    proto_bytes_read: u64,
    proto_bytes_written: u64,
    wire_bytes_read: u64,
    wire_bytes_written: u64,
    transitions: u64,
    last_op: Option<Op>,
}

/// Shared handle to one side's stats. The two counting wrappers (read + write)
/// and the rig that reads the final counters hold clones.
type StatsHandle = Arc<Mutex<SideStats>>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Op {
    Read,
    Write,
}

/// Which side of the zstd codec a counter sits on. The protocol layer also
/// owns the phase-transition tracking; the wire layer just tallies bytes.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Layer {
    Protocol,
    Wire,
}

struct CountingRead<R> {
    inner: R,
    stats: StatsHandle,
    layer: Layer,
}

impl<R: Read> Read for CountingRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        let mut s = self.stats.lock().unwrap();
        match self.layer {
            Layer::Protocol => {
                if matches!(s.last_op, Some(Op::Write)) {
                    s.transitions += 1;
                }
                s.last_op = Some(Op::Read);
                s.proto_bytes_read += n as u64;
            }
            Layer::Wire => {
                s.wire_bytes_read += n as u64;
            }
        }
        Ok(n)
    }
}

struct CountingWrite<W> {
    inner: W,
    stats: StatsHandle,
    layer: Layer,
}

impl<W: Write> Write for CountingWrite<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        let mut s = self.stats.lock().unwrap();
        match self.layer {
            Layer::Protocol => {
                if matches!(s.last_op, Some(Op::Read)) {
                    s.transitions += 1;
                }
                s.last_op = Some(Op::Write);
                s.proto_bytes_written += n as u64;
            }
            Layer::Wire => {
                s.wire_bytes_written += n as u64;
            }
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// {0, 1, 2, 4, …, 2^max_exp}. The leading 0 lets us read off the
/// O(1)-in-RA/RB/S claims at the baseline.
fn axis(max_exp: u32) -> Vec<u32> {
    let mut v = Vec::with_capacity(max_exp as usize + 2);
    v.push(0);
    for k in 0..=max_exp {
        v.push(1u32 << k);
    }
    v
}

/// Build the (alice, bob) `Known<()>` pair for one sample. Ancestor elements
/// are distributed as evenly as possible across `parties` distinct background
/// parties, each of which inserts its share and is then processed into both
/// alice's and bob's local views. Redactions reference *existing* ancestor
/// keys and the two sides' redaction sets are disjoint.
///
/// If the requested `parties` exceeds the number of ancestor elements, the
/// effective party count is capped: a "party" with zero inserts never
/// appears in any version vector, so it can't be observed.
fn build_pair(
    parties: u32,
    shared: u32,
    distinct_a: u32,
    distinct_b: u32,
    redacted_a: u32,
    redacted_b: u32,
) -> (Known<()>, Known<()>) {
    // Every party in one sample descends from a single universe seed, so they
    // are pairwise disjoint and can `learn` from one another (the Law of
    // Disjointness). ITC parties are anonymous — the old random party *names*
    // are gone, and forking is deterministic, so a given cell's structure is
    // now identical across samples.
    let mut seed: Known<()> = Known::seed();
    let mut alice = seed.fork();
    let mut bob = seed.fork();

    let total_ancestor = shared + redacted_a + redacted_b;
    // The sync callback bound is `Send + 'a`, so the closure borrows `keys`
    // directly for the duration of each `message` call.
    let mut keys: Vec<Key> = Vec::with_capacity(total_ancestor as usize);

    if total_ancestor > 0 {
        // Cap effective parties to total_ancestor: a party with 0 inserts is
        // structurally indistinguishable from "not present" in the network.
        let effective = parties.min(total_ancestor).max(1);
        let base = total_ancestor / effective;
        let remainder = total_ancestor % effective;
        for i in 0..effective {
            let count = (base + if i < remainder { 1 } else { 0 }) as usize;
            let mut bg = seed.fork();
            bg.message(std::iter::repeat_n((), count), |k, _, _| keys.push(k));
            // Fork bg's observations into alice and bob; `learn` rejoins the
            // forked party region, and disjointness makes it infallible here.
            alice
                .learn(bg.fork(), ignore)
                .expect("disjoint background party");
            bob.learn(bg.fork(), ignore)
                .expect("disjoint background party");
        }
    }

    let s = shared as usize;
    let ra = redacted_a as usize;
    let redact_a: Vec<Key> = keys[s..s + ra].to_vec();
    let redact_b: Vec<Key> = keys[s + ra..].to_vec();

    alice.redact(redact_a);
    alice.message(std::iter::repeat_n((), distinct_a as usize), ignore);

    bob.redact(redact_b);
    bob.message(std::iter::repeat_n((), distinct_b as usize), ignore);

    (alice, bob)
}

#[derive(Copy, Clone, Debug)]
struct Measure {
    /// Bytes alice transmitted at the protocol layer (before zstd compression).
    proto_bytes_a: u64,
    proto_bytes_b: u64,
    /// Bytes that actually traversed the pipe (after zstd compression).
    wire_bytes_a: u64,
    wire_bytes_b: u64,
    transitions_a: u64,
    transitions_b: u64,
}

/// Structurally-derived mean-prediction formula for one (S, DA, DB, RA, RB) cell.
///
/// Pieces marked EXACT are derived from the wire format and the construction
/// (no fitting). Other coefficients are fit to the calibration sweep, but each
/// has a structural meaning explained inline.
///
/// # Wire ground truth
///
/// The mirror protocol is framed by `tokio_util::codec::LengthDelimitedCodec`:
/// every frame is a 4-byte big-endian length followed by a borsh body.
///
/// First exchange (always sent): each side ships its `Version<Bytes>`. With
/// `imbl_borsh`'s OrdMap encoding, that is `u32(len)` followed by
/// `(Bytes(party_hash), u64(counter))` pairs. Party identifiers are hashed to
/// 32 bytes before insertion (see `Tree::for_party`), so every party entry is
/// `4 + 32 + 8 = 44` bytes, and the OrdMap-len prefix is `4` bytes. Including
/// the frame prefix, the connect-out costs exactly `8 + 44·np` bytes per side,
/// where `np` is the number of parties in that side's version vector.
///
/// Under the present construction, with `P` background parties each
/// contributing roughly `(S+RA+RB)/P` ancestor inserts:
///   * Each background party with ≥ 1 insert contributes one entry to both
///     alice's and bob's version vectors. So the effective background-party
///     count is `min(P, S+RA+RB)` (a party with zero inserts is invisible).
///   * Alice adds her own party-counter iff she performs any action herself,
///     i.e. iff `DA + RA > 0` (insert or redact); similarly bob.
///   * `np_self = min(P, S+RA+RB) + 𝟙[self acted]`.
///
/// If `DA + DB + RA + RB == 0`, both versions are identical and the protocol
/// bails after the connect exchange — total bytes are EXACTLY `8 + 44·np` per
/// side, and exactly 1 phase transition (= 0.5 rounds) per side.
///
/// # Per-leaf cost
///
/// When alice originates a leaf and ships it in `providing`, the leaf body
/// carries her full `Version<P>` at insert time — which has `np_a` parties.
/// That's `4 + 44·np_a` bytes per leaf (the same OrdMap shape, no outer frame
/// because the leaf is nested inside a larger message). Empirically this term
/// fits with coefficient **1.014 ± 0.05**, matching the theoretical 1.0 to
/// fitting precision; we lock it to 1.0.
///
/// The other ~48 bytes per A-leaf are framing share, the leaf's path-compressed
/// `prefix_len + prefix_bytes`, and the OrdMap key in `providing`. The exact
/// split varies by tree height at which the leaf is sent, so we fit one
/// constant rather than enumerate.
///
/// # Round count
///
/// The tree is 256-ary with depth 32. The protocol descends two levels per
/// `exchange` call and short-circuits when both sides have nothing left to
/// reconcile. For a union of `N` random leaves, the expected descent depth
/// before all paths diverge is `O(log_256(N))` (birthday paradox), so rounds
/// scale as **1.82 + 1.013 · log_256(N+1)** on a typical-role side. The 1.013
/// fit coefficient is within fitting precision of the theoretical 1.0; the
/// 1.82 base reflects the connect + open-initiator round-trip pair (≈ 4
/// transitions = 2 rounds, minus role-asymmetry averaging).
///
/// One side ("quiet": originates nothing and faces no remote redactions) can
/// short-circuit one round earlier: `DA == 0 && RB == 0` removes 0.24 rounds
/// from alice's expectation (symmetrically for bob).
///
/// # What residual noise remains
///
/// Per-sample bytes have ~190-byte RMS *irreducible* noise from party-id–driven
/// hash distribution: which side becomes initiator vs. responder is decided by
/// the lexicographic order of version-vector OrdMaps, which is essentially a
/// coin flip per random-party-id sample. Mean per cell converges (the formula
/// is unbiased), but individual sample residuals do not vanish.
fn predict(
    parties: u32,
    shared: u32,
    distinct_a: u32,
    distinct_b: u32,
    redacted_a: u32,
    redacted_b: u32,
) -> Prediction {
    let s = shared as f64;
    let da = distinct_a as f64;
    let db = distinct_b as f64;
    let ra = redacted_a as f64;
    let rb = redacted_b as f64;

    // Number of distinct parties in each side's version vector. EXACT under
    // the test-rig construction: a background party with ≥ 1 insert appears
    // in both alice's and bob's version; one with zero inserts is invisible.
    let total_ancestor = shared + redacted_a + redacted_b;
    let effective_parties = if total_ancestor == 0 {
        0
    } else {
        parties.min(total_ancestor)
    };
    let np_a = effective_parties + u32::from(redacted_a + distinct_a > 0);
    let np_b = effective_parties + u32::from(redacted_b + distinct_b > 0);

    // EXACT: connect-out frame = 4-byte LengthDelimitedCodec prefix + Version<Bytes>
    //   Version<Bytes> = 4 (OrdMap len) + (4 + 32 + 8) per party
    let ver_a = 8.0 + 44.0 * (np_a as f64);
    let ver_b = 8.0 + 44.0 * (np_b as f64);

    let any_diff = distinct_a + distinct_b + redacted_a + redacted_b > 0;

    // EXACT: matching versions cause both sides to bail after the connect exchange.
    if !any_diff {
        return Prediction {
            bytes_a: ver_a,
            bytes_b: ver_b,
            rounds_a: 0.5,
            rounds_b: 0.5,
        };
    }

    // EXACT structural per-leaf-version cost: each leaf in `providing` carries
    // a `Version<P>` with np_self parties, serialized as 4 + 44·np_self bytes.
    let leaf_ver_a = 4.0 + 44.0 * (np_a as f64);
    let leaf_ver_b = 4.0 + 44.0 * (np_b as f64);

    // Per-leaf overhead beyond the version (prefix_len + path-compressed prefix
    // + share of framing/OrdMap key in `providing`). Fit to the cal data; ~48
    // bytes captures the average across heights where leaves get shipped.
    const PER_LEAF_OVERHEAD: f64 = 48.0;

    // Per-shared-element descent step in the union subtree (~17 bytes; the
    // exchange carries one Hash entry plus OrdMap-key + prefix overhead).
    const PER_SHARED: f64 = 17.0;
    // Per-element descent into a B-redacted (or A-redacted, symmetrically)
    // subtree — slightly cheaper than a fully shared one because the receiver
    // routes the prefix to `requested` rather than carrying a full Node.
    const PER_REDACT_OTHER: f64 = 16.0;

    // Protocol startup beyond connect (Initiate/Opening pair + first descent).
    const PROTO_BASE: f64 = 30.0;

    // The remaining tiny coefficients are role-asymmetry residuals after the
    // main structural terms. Empirically they each contribute < 6 bytes; we
    // keep them so the model nails the asymmetric cells.
    const SMALL_OTHER_PER_DISTINCT: f64 = 2.0; // per element on the OTHER side
    const SMALL_OWN_REDACT: f64 = 1.0; // per own redaction announced
    const PRESENCE_BUMP: f64 = 5.0; // when an indicator subtree opens

    let proto_a = PROTO_BASE
        + da * (leaf_ver_a + PER_LEAF_OVERHEAD)
        + s * PER_SHARED
        + rb * PER_REDACT_OTHER
        + SMALL_OTHER_PER_DISTINCT * db
        + SMALL_OWN_REDACT * ra
        + if redacted_a > 0 { PRESENCE_BUMP } else { 0.0 }
        + if shared > 0 { PRESENCE_BUMP } else { 0.0 };
    let proto_b = PROTO_BASE
        + db * (leaf_ver_b + PER_LEAF_OVERHEAD)
        + s * PER_SHARED
        + ra * PER_REDACT_OTHER
        + SMALL_OTHER_PER_DISTINCT * da
        + SMALL_OWN_REDACT * rb
        + if redacted_b > 0 { PRESENCE_BUMP } else { 0.0 }
        + if shared > 0 { PRESENCE_BUMP } else { 0.0 };

    // Rounds: 256-ary tree descent + role asymmetry.
    let n_total = (shared + distinct_a + distinct_b + redacted_a + redacted_b) as f64;
    let log256 = (n_total + 1.0).ln() / 256.0_f64.ln();
    const ROUNDS_BASE: f64 = 1.82;
    const ROUNDS_PER_LEVEL: f64 = 1.013; // structural 1.0 within fit precision
    const QUIET_DISCOUNT: f64 = 0.24;

    let quiet_a = distinct_a == 0 && redacted_b == 0;
    let quiet_b = distinct_b == 0 && redacted_a == 0;

    let rounds_a =
        ROUNDS_BASE + ROUNDS_PER_LEVEL * log256 - if quiet_a { QUIET_DISCOUNT } else { 0.0 };
    let rounds_b =
        ROUNDS_BASE + ROUNDS_PER_LEVEL * log256 - if quiet_b { QUIET_DISCOUNT } else { 0.0 };

    Prediction {
        bytes_a: ver_a + proto_a,
        bytes_b: ver_b + proto_b,
        rounds_a,
        rounds_b,
    }
}

#[derive(Copy, Clone, Debug)]
struct Prediction {
    bytes_a: f64,
    bytes_b: f64,
    rounds_a: f64,
    rounds_b: f64,
}

/// Build one side's I/O stack: counted-protocol-layer wrapping a zstd codec
/// wrapping the counted wire layer wrapping the raw pipe halves.
#[allow(clippy::type_complexity)]
fn build_io_stack(
    raw_read: std::io::PipeReader,
    raw_write: std::io::PipeWriter,
    stats: &StatsHandle,
    zstd_level: i32,
) -> io::Result<(
    CountingRead<
        zstd::stream::read::Decoder<'static, io::BufReader<CountingRead<std::io::PipeReader>>>,
    >,
    CountingWrite<zstd::stream::AutoFinishEncoder<'static, CountingWrite<std::io::PipeWriter>>>,
)> {
    // Read path (bottom up): raw pipe -> wire counter -> zstd decoder -> protocol counter
    let wire_read = CountingRead {
        inner: raw_read,
        stats: stats.clone(),
        layer: Layer::Wire,
    };
    let decoder = zstd::stream::read::Decoder::new(wire_read)?;
    let proto_read = CountingRead {
        inner: decoder,
        stats: stats.clone(),
        layer: Layer::Protocol,
    };

    // Write path (bottom up): raw pipe -> wire counter -> zstd encoder -> protocol counter
    let wire_write = CountingWrite {
        inner: raw_write,
        stats: stats.clone(),
        layer: Layer::Wire,
    };
    // `auto_finish` ensures the encoder writes the final frame on drop, so the
    // other side's decoder sees a clean EOF when gossip returns.
    let encoder = zstd::stream::write::Encoder::new(wire_write, zstd_level)?.auto_finish();
    let proto_write = CountingWrite {
        inner: encoder,
        stats: stats.clone(),
        layer: Layer::Protocol,
    };

    Ok((proto_read, proto_write))
}

/// Work item handed to a long-lived bob worker. Bob's stats are constructed
/// *inside* the worker thread (so they can live in an `Rc<RefCell<…>>`) and
/// shipped back via the done channel.
struct BobJob {
    bob: Known<()>,
    read: std::io::PipeReader,
    write: std::io::PipeWriter,
    zstd_level: i32,
}

/// Long-lived bob worker: one OS thread, fed by an mpsc channel. The worker
/// loops until the sender is dropped (i.e. the owning rayon thread exits and
/// its thread-local store is torn down).
///
/// Submit-and-wait is split into two methods so the caller can run alice
/// concurrently with the bob worker in between.
struct BobWorker {
    job_tx: mpsc::Sender<BobJob>,
    done_rx: mpsc::Receiver<SideStats>,
}

impl BobWorker {
    fn new() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<BobJob>();
        let (done_tx, done_rx) = mpsc::channel::<SideStats>();
        thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let stats: StatsHandle = Arc::new(Mutex::new(SideStats::default()));
                {
                    let (mut r, mut w) =
                        build_io_stack(job.read, job.write, &stats, job.zstd_level)
                            .expect("build bob stack");
                    job.bob
                        .gossip(&mut r, &mut w, ignore)
                        .expect("sync gossip bob");
                }
                let result = *stats.lock().unwrap();
                // Recv side will be dropped on shutdown; ignore that error.
                let _ = done_tx.send(result);
            }
        });
        Self { job_tx, done_rx }
    }

    fn submit(&self, job: BobJob) {
        self.job_tx.send(job).expect("submit bob job");
    }

    fn wait(&self) -> SideStats {
        self.done_rx.recv().expect("wait bob done")
    }
}

thread_local! {
    static BOB_WORKER: BobWorker = BobWorker::new();
}

#[allow(clippy::too_many_arguments)]
fn run_sample(
    // Vestigial since party identities are no longer randomized (ITC parties
    // are anonymous and forking is deterministic); retained so the per-sample
    // seeding call chain stays intact.
    _rng: &mut SmallRng,
    parties: u32,
    shared: u32,
    distinct_a: u32,
    distinct_b: u32,
    redacted_a: u32,
    redacted_b: u32,
    zstd_level: i32,
    mode: Mode,
) -> Measure {
    let (alice, bob) = build_pair(
        parties, shared, distinct_a, distinct_b, redacted_a, redacted_b,
    );

    let (a_to_b_r, a_to_b_w) = pipe().expect("pipe a->b");
    let (b_to_a_r, b_to_a_w) = pipe().expect("pipe b->a");

    // Hand bob's half off to a peer thread. Alice runs on the current
    // (rayon worker) thread; we then wait for bob to finish. Bob's stats are
    // constructed on the bob thread (Rc is !Send) and shipped back.
    let bob_handle: BobHandle = match mode {
        Mode::Spawn => BobHandle::Spawn(thread::spawn(move || -> SideStats {
            let stats: StatsHandle = Arc::new(Mutex::new(SideStats::default()));
            {
                let (mut r, mut w) = build_io_stack(a_to_b_r, b_to_a_w, &stats, zstd_level)
                    .expect("build bob stack");
                bob.gossip(&mut r, &mut w, ignore).expect("sync gossip bob");
            }
            *stats.lock().unwrap()
        })),
        Mode::Pool => {
            BOB_WORKER.with(|w| {
                w.submit(BobJob {
                    bob,
                    read: a_to_b_r,
                    write: b_to_a_w,
                    zstd_level,
                });
            });
            BobHandle::Pool
        }
    };

    // Alice's stack: reads from b_to_a, writes to a_to_b.
    let stats_a: StatsHandle = Arc::new(Mutex::new(SideStats::default()));
    {
        let (mut r, mut w) =
            build_io_stack(b_to_a_r, a_to_b_w, &stats_a, zstd_level).expect("build alice stack");
        let _alice_out = alice
            .gossip(&mut r, &mut w, ignore)
            .expect("sync gossip alice");
    }
    let sa = *stats_a.lock().unwrap();

    let sb = match bob_handle {
        BobHandle::Spawn(h) => h.join().expect("bob thread join"),
        BobHandle::Pool => BOB_WORKER.with(|w| w.wait()),
    };

    Measure {
        proto_bytes_a: sa.proto_bytes_written,
        proto_bytes_b: sb.proto_bytes_written,
        wire_bytes_a: sa.wire_bytes_written,
        wire_bytes_b: sb.wire_bytes_written,
        transitions_a: sa.transitions,
        transitions_b: sb.transitions,
    }
}

enum BobHandle {
    Spawn(thread::JoinHandle<SideStats>),
    Pool,
}

/// Deterministically derive a per-job 64-bit seed from the run seed and
/// the (cell_idx, sample_idx) coordinates. Avoids `DefaultHasher`, which
/// is randomized per-process and would break reproducibility across runs.
fn job_seed(run_seed: u64, cell_idx: usize, sample_idx: u32) -> u64 {
    // Three odd multipliers from SplitMix64 / Murmur3 finalizer constants;
    // mixing is plenty for "different jobs get different RNG streams".
    run_seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((cell_idx as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((sample_idx as u64).wrapping_mul(0x94D0_49BB_1331_11EB))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    });

    let shared_axis = axis(args.max_shared);
    let distinct_axis = axis(args.max_distinct);
    let redacted_axis = axis(args.max_redacted);
    // Parties: powers of 2 starting at 1 (1, 2, 4, …, 2^max_parties), no zero.
    let parties_axis: Vec<u32> = (0..=args.max_parties).map(|k| 1u32 << k).collect();

    let mut cells: Vec<(u32, u32, u32, u32, u32, u32)> = Vec::with_capacity(
        parties_axis.len()
            * shared_axis.len()
            * distinct_axis.len().pow(2)
            * redacted_axis.len().pow(2),
    );
    for &p in &parties_axis {
        for &s in &shared_axis {
            for &da in &distinct_axis {
                for &db in &distinct_axis {
                    for &ra in &redacted_axis {
                        for &rb in &redacted_axis {
                            cells.push((p, s, da, db, ra, rb));
                        }
                    }
                }
            }
        }
    }
    let n_cells = cells.len();
    let total_samples = (n_cells as u64) * (args.samples as u64);

    eprintln!(
        "grid: {} cells x {} samples = {} runs (seed = {})",
        n_cells, args.samples, total_samples, seed,
    );

    // Dedicated writer thread, fed by an mpsc channel. Per-row mutex
    // contention on the BufWriter (and the flush(2) syscall held under that
    // lock) is the main remaining serialization point for the par_iter; with
    // a single owner thread the rayon workers just enqueue and move on. Each
    // row is sent as a pre-formatted `[String; 19]` so all string formatting
    // happens in parallel on the producer side.
    let header: [&'static str; 19] = [
        "parties",
        "shared",
        "distinct_a",
        "distinct_b",
        "redacted_a",
        "redacted_b",
        "sample_idx",
        "bytes_a",
        "bytes_b",
        "bytes_a_compressed",
        "bytes_b_compressed",
        "transitions_a",
        "transitions_b",
        "rounds_a",
        "rounds_b",
        "pred_bytes_a",
        "pred_bytes_b",
        "pred_rounds_a",
        "pred_rounds_b",
    ];
    let (row_tx, row_rx) = mpsc::channel::<[String; 19]>();
    let output_path = args.output.clone();
    let writer_thread: thread::JoinHandle<io::Result<()>> = thread::spawn(move || {
        // 256 KiB BufWriter holds ~1500 rows at typical row widths, so a flush
        // cycle becomes one write(2) instead of one per row. FLUSH_EVERY caps
        // tail-visibility lag at ~a second of throughput on a fast run while
        // still amortizing the syscall over a meaningful batch.
        const BUF_CAPACITY: usize = 256 * 1024;
        const FLUSH_EVERY: usize = 1024;
        let file = File::create(&output_path)?;
        let mut csv_writer = csv::Writer::from_writer(BufWriter::with_capacity(BUF_CAPACITY, file));
        csv_writer.write_record(header).map_err(io::Error::other)?;
        csv_writer.flush()?;
        let mut since_flush = 0usize;
        while let Ok(record) = row_rx.recv() {
            csv_writer.write_record(&record).map_err(io::Error::other)?;
            since_flush += 1;
            if since_flush >= FLUSH_EVERY {
                csv_writer.flush()?;
                since_flush = 0;
            }
        }
        csv_writer.flush()?;
        Ok(())
    });

    let pb = ProgressBar::new(total_samples);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40}] {pos}/{len} ({eta})",
        )?
        .progress_chars("=>-"),
    );

    let jobs: Vec<(usize, u32)> = (0..n_cells)
        .flat_map(|i| (0..args.samples).map(move |s| (i, s)))
        .collect();

    jobs.par_iter()
        .progress_with(pb)
        .for_each(|&(cell_idx, sample_idx)| {
            let (parties, s, da, db, ra, rb) = cells[cell_idx];
            let mut rng = SmallRng::seed_from_u64(job_seed(seed, cell_idx, sample_idx));
            let m = run_sample(
                &mut rng,
                parties,
                s,
                da,
                db,
                ra,
                rb,
                args.zstd_level,
                args.mode,
            );
            let rounds_a = m.transitions_a as f64 / 2.0;
            let rounds_b = m.transitions_b as f64 / 2.0;
            let p = predict(parties, s, da, db, ra, rb);

            row_tx
                .send([
                    parties.to_string(),
                    s.to_string(),
                    da.to_string(),
                    db.to_string(),
                    ra.to_string(),
                    rb.to_string(),
                    sample_idx.to_string(),
                    m.proto_bytes_a.to_string(),
                    m.proto_bytes_b.to_string(),
                    m.wire_bytes_a.to_string(),
                    m.wire_bytes_b.to_string(),
                    m.transitions_a.to_string(),
                    m.transitions_b.to_string(),
                    rounds_a.to_string(),
                    rounds_b.to_string(),
                    format!("{:.3}", p.bytes_a),
                    format!("{:.3}", p.bytes_b),
                    format!("{:.4}", p.rounds_a),
                    format!("{:.4}", p.rounds_b),
                ])
                .expect("send row to writer thread");
        });

    // Drop the producer so the writer thread sees the channel close, then
    // join it to surface any I/O errors.
    drop(row_tx);
    writer_thread
        .join()
        .map_err(|_| "writer thread panicked")??;
    Ok(())
}
