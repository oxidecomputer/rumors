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
//! changes; the residual is now pure deterministic model error (see
//! [the determinism note](#determinism-no-per-sample-noise)), so it should be
//! small and stable, not a sampling average.
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
//! 1. Allocate `min(P, S+RA+RB)` ancestor parties and distribute the `S + RA +
//!    RB` ancestor inserts evenly across them. (Parties with zero inserts would
//!    be invisible to either side, so we cap.) Parties are anonymous Interval
//!    Tree Clock (ITC) regions forked from a single seed — there are no party
//!    *identifiers* on the wire (see [the version note](#version-sizing)).
//! 2. Both alice and bob `join` every ancestor party — each side's [`Version`]
//!    (an ITC event tree) now records one populated region per contributing
//!    background party.
//! 3. Partition the ancestor keys disjointly into `[shared | redact-by-A |
//!    redact-by-B]`. Alice redacts her slice; bob redacts his. Redactions
//!    therefore reference *existing* keys and never overlap, so the measurement
//!    reflects protocol behaviour rather than redaction-overlap statistics.
//! 4. Alice inserts `DA` fresh `()` units under her own party; bob inserts
//!    `DB`. The value type is `()` so we measure protocol overhead unmixed with
//!    application payload size.
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
//! performance shape, much of which is reproducible from the wire format alone.
//!
//! ## Determinism: no per-sample noise
//!
//! ITC parties are anonymous and forking is deterministic: there are no random
//! party identifiers any more. A given cell `(P, S, DA, DB, RA, RB)` therefore
//! produces a *byte-for-byte identical* session on every sample — same bytes,
//! same rounds, same initiator/responder assignment. `--samples > 1` adds no
//! information; it is retained only so old invocations keep working. The
//! prediction residual is consequently deterministic model error, not a
//! sampling average: there is nothing to average away.
//!
//! ## Role: the larger version initiates
//!
//! Once both sides have exchanged versions and found them unequal, the mirror
//! driver breaks the tie by comparing their *canonical ITC bytes*
//! lexicographically — the side with the larger bytes becomes the initiator
//! (see `mirror.rs`). This is a total, deterministic, but **non-closed-form**
//! tiebreak: it depends on the bit-packed encoding, not on the cell counts, and
//! it flips in structure-dependent ways as `DA`/`DB` change. The two roles have
//! sharply different cost profiles (below), so [`predict`] reconstructs the two
//! versions to read off the role exactly rather than guessing it from counts.
//!
//! ## Round trips: essentially constant, structurally logarithmic
//!
//! Each `exchange` call descends two heights into the 256-ary trie. For a union
//! of `N` random leaves, birthday-paradox depth is `O(log_256 N)`, so the round
//! count fits
//!
//! ```text
//! rounds = 2.36 + 0.5·𝟙[initiator] + 1.357 · log_256(N + 1)
//! ```
//!
//! The initiator runs one extra half-round-trip (its `Initiate`/`close` pair),
//! hence the `+0.5`. When the two versions match (only when `DA + DB + RA + RB
//! == 0`) both sides bail right after the connect exchange at exactly **1.5
//! rounds** (handshake write/read + connect write/read = 3 phase transitions).
//!
//! Practically: rounds plateau at ~3–4 across the calibration range. A network
//! must reach `N > 256` to add a level. This is the key advantage of the
//! 256-ary trie: convergence is latency-bounded by a handful of round trips,
//! not by `log₂ N`.
//!
//! ## Bytes: a small exact boundary, then two role-dependent regimes
//!
//! The fixed framing is exact and tiny:
//!
//! * **Connect frame**: each side ships its [`Version`] as a 4-byte
//!   length-delimited frame wrapping a borsh body (a `u32` length prefix + the
//!   bit-packed ITC event tree). With the 8-byte protocol handshake that is
//!   exactly `16 + |version|` bytes, where `|version|` is the serialized event
//!   tree (often just **1–30 bytes**; see below). On a connect-bail this is the
//!   *entire* cost: e.g. an empty version is `16 + 1 = 17` bytes per side.
//!
//! Past the connect, cost splits by role, because the initiator provides its
//! leaves *shallow* (early in the descent, before the frontier narrows) while
//! the responder provides them *deep*:
//!
//! * **Initiator** ≈ `36 + 40·D + |version|·(1 + D) + 0.65·U·log₂(1+U)` bytes,
//!   where `D` is its own outgoing leaves and `U = S + D_other + R_self` is the
//!   divergent set it must exchange `uncertain` hashes through. The frontier
//!   term is super-linear because deeper rounds add whole levels of hashes.
//! * **Responder** ≈ `68 + 67·D + |version|·(1 + D) + 31·S + 31·R_other` bytes.
//!   The `31` constants are one 32-byte blake3 hash apiece: the responder pays
//!   a hash exchange per shared element it descends past and per element the
//!   *other* side redacted (it routes the now-absent prefix to `requested`).
//!
//! In both regimes every provided leaf and the connect frame carry the side's
//! `Version`, hence the `|version|·(1 + D)` term (`1` connect + `D` leaves).
//!
//! ## Version sizing
//!
//! The old version-vector representation cost `44` bytes *per party* (a 32-byte
//! party-hash + `u64` counter + framing, in an OrdMap). ITC versions have **no
//! party identifiers at all**: a [`Version`] is a bit-packed event tree whose
//! size grows with the number of populated regions (`min(P, S+RA+RB)`, plus the
//! side's own region if it acted) and the Elias-gamma-coded event counts —
//! *sublinearly* and at well under a byte per region. Even at 32 regions the
//! serialized version is ~26 bytes, versus the `32·44 = 1408` bytes the old
//! model predicted. Because the size has no clean closed form, [`predict`]
//! reconstructs the version and measures `as_bytes().len()` directly.
//!
//! ## What scales with what
//!
//! * **Linear in the side's own outgoing elements** (`DA` for A, `DB` for B):
//!   ~40 bytes/leaf as initiator, ~67 as responder, plus the per-leaf version.
//! * **`O(S)` and `O(R_other)` for the responder** at ~31 bytes each (a hash);
//!   the initiator pays for these only through its sub-linear frontier term.
//! * **Logarithmically weak in the union size** for rounds.
//! * **Sub-linear, identifier-free in the party count `P`**: `P` only enlarges
//!   the bit-packed version, by a fraction of a byte per region — not the
//!   1.4 KiB/leaf the version-vector representation cost.
//!
//! ## Compression: now nearly a no-op
//!
//! Under the old version-vector wire, every leaf and connect frame reprinted
//! every known party's 32-byte hash — textbook repeating zstd dictionary
//! material, and compression was a large multiplier (ratios down to ~0.1 for
//! big batches). ITC versions removed those repeats entirely. What remains on
//! the wire is mostly high-entropy blake3 tree-descent hashes, which do not
//! compress, so the ratio (wire bytes / protocol bytes) now hovers near 1:
//!
//! | `P`  | mean ratio |
//! | ---- | ---------- |
//! |  1   | 1.06       |
//! |  8   | 0.99       |
//! |  32  | 0.92       |
//! |  64  | 0.91       |
//!
//! At small `P` the ratio is **above 1**: zstd's framing overhead exceeds what
//! it can save on a tiny, high-entropy payload. Only at large `P` or large
//! transmitted batches (`DA ≥ 8`, ratio ~0.88) does it claw back ~10%.
//! Compression no longer materially changes the bandwidth story, and never
//! changes the round count.
//!
//! # Validating the prediction
//!
//! For each row, compute `(bytes_a - pred_bytes_a)` and `(rounds_a -
//! pred_rounds_a)`. Because the workload is deterministic, these are stable
//! model errors, not noise. With the role and version sizes taken exactly from
//! a reconstruction, the residual reduces to the descent-cost model:
//!
//! * The **connect-bail** and **rounds** pieces are essentially exact.
//! * The **responder** byte model is tight (~2% median error): its cost is the
//!   clean per-leaf + per-hash structure above.
//! * The **initiator** byte model is looser (~7% median, with a heavier tail on
//!   high-`U` cells): its `uncertain`-hash frontier cost is genuinely
//!   round-by-round path-dependent and does not reduce to the cell counts. This
//!   residual is the irreducible "this is the model's approximation" term — it
//!   does not shrink with more samples.
//!
//! A run with `--max-shared 5 --max-distinct 5 --max-redacted 4 --max-parties
//! 6` sweeps enough of the space to see all of this.
//!
//! # When the protocol fits this workload model
//!
//! From the structure above, the protocol is well-suited to networks with:
//!
//! * Many parties — the ITC version is identifier-free and bit-packed, so party
//!   count costs a fraction of a byte per region rather than 44 bytes each.
//! * Rumor payloads at least a few hundred bytes (so the per-leaf overhead
//!   doesn't dominate the application data).
//! * Periodic reconciliation on a timer of seconds to minutes (so the
//!   handful of round trips per gossip are dwarfed by the gossip interval).
//! * Peers usually mostly in sync, occasionally diverging by a small delta
//!   (so linear-in-delta bandwidth is what's being paid, not linear-in-state).
//!
//! Compression, once a large multiplier, is now nearly a no-op: the ITC version
//! already removed the repeating-identifier bloat it used to recover.

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
use rumors::sync::Known;

#[derive(Parser, Debug)]
#[command(
    about = "Empirically measure rumors gossip bandwidth and round trips across a 5-D input space."
)]
struct Args {
    /// Output CSV path. The file is created (or truncated) and written
    /// row-by-row with a flush after each sample.
    #[arg(short, long)]
    output: PathBuf,

    /// Samples per cell. Now that ITC parties are anonymous and forking is
    /// deterministic, every sample of a cell is byte-for-byte identical, so
    /// this adds no information; it is retained only so old invocations keep
    /// working. Leave it at 1.
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
            bg.message_then(std::iter::repeat_n((), count), |k, _, _| keys.push(k));
            // Fork bg's observations into alice and bob; `learn` rejoins the
            // forked party region, and disjointness makes it infallible here.
            alice.join(bg.fork()).expect("disjoint background party");
            bob.join(bg.fork()).expect("disjoint background party");
        }
    }

    let s = shared as usize;
    let ra = redacted_a as usize;
    let redact_a: Vec<Key> = keys[s..s + ra].to_vec();
    let redact_b: Vec<Key> = keys[s + ra..].to_vec();

    alice.redact(redact_a);
    alice.message(std::iter::repeat_n((), distinct_a as usize));

    bob.redact(redact_b);
    bob.message(std::iter::repeat_n((), distinct_b as usize));

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

/// Structural wire-cost prediction for one `(P, S, DA, DB, RA, RB)` cell.
///
/// The workload is deterministic (see the module's determinism note), so this
/// is a point prediction, not a mean over samples. It is built in two layers:
///
/// 1. **Reconstructed-exact pieces.** The two sides' [`Version`]s are rebuilt
///    with [`build_pair`] (cheap: forks + ticks, no gossip) so the serialized
///    version sizes and the initiator/responder role come out *exactly*. Both
///    are needed and neither has a usable closed form: the version is a
///    bit-packed ITC event tree, and the role is a lexicographic comparison of
///    the two canonical version-byte strings (`mirror.rs`).
/// 2. **Modeled descent cost.** Given the role and version sizes, the protocol
///    byte/round costs are fit constants with the structural meanings spelled
///    out on each constant below.
///
/// # Wire ground truth
///
/// The mirror protocol is framed by `tokio_util::codec::LengthDelimitedCodec`
/// (4-byte big-endian length + borsh body), and a session opens with the 8-byte
/// protocol handshake. The first frame each side sends is its [`Version`]: a
/// `u32` length prefix + the bit-packed event tree. So the connect costs
/// exactly `16 + |version|` bytes (`8` handshake + `4` frame + `4` borsh len),
/// and on a connect-bail (`DA+DB+RA+RB == 0`, equal versions) that is the whole
/// session — `1.5` rounds (handshake + connect, three phase transitions).
///
/// # Modeled descent cost
///
/// Past the connect the cost depends sharply on role (the initiator provides
/// its leaves shallow and early; the responder provides them deep), so the two
/// sides are modeled by separate fits in [`side_bytes`]. Every provided leaf
/// and the connect frame carry the side's `Version`, so its serialized size
/// appears `1 + D` times. The responder's per-shared and per-other-redaction
/// constants are each one 32-byte blake3 hash; the initiator instead pays a
/// single super-linear `uncertain`-hash frontier term over the divergent set.
///
/// # Round count
///
/// Both roles fit `rounds = 2.36 + 0.5·𝟙[initiator] + 1.357·log_256(N+1)` for
/// union size `N` (256-ary descent, two heights per `exchange`; the initiator
/// runs one extra half-round-trip). Connect-bail is exactly `1.5`.
///
/// # Residual
///
/// Deterministic, not noise. The responder model is tight (~2% median); the
/// initiator's frontier term is the looser piece (~7% median, heavier tail) and
/// is the model's irreducible approximation — its true cost is round-by-round
/// path-dependent, not a function of the cell counts.
fn predict(
    parties: u32,
    shared: u32,
    distinct_a: u32,
    distinct_b: u32,
    redacted_a: u32,
    redacted_b: u32,
) -> Prediction {
    // Reconstruct the two sides deterministically to read off the exact
    // serialized version sizes and the exact role. This is the same
    // construction the measured run uses, minus the gossip.
    let (alice, bob) = build_pair(
        parties, shared, distinct_a, distinct_b, redacted_a, redacted_b,
    );
    let version_a = alice.version();
    let version_b = bob.version();
    let vlen_a = version_a.as_bytes().len() as f64;
    let vlen_b = version_b.as_bytes().len() as f64;

    let any_diff = distinct_a + distinct_b + redacted_a + redacted_b > 0;

    // EXACT: equal versions ⇒ both sides ship only their connect frame and stop.
    if !any_diff {
        return Prediction {
            bytes_a: CONNECT_FIXED + vlen_a,
            bytes_b: CONNECT_FIXED + vlen_b,
            rounds_a: BAIL_ROUNDS,
            rounds_b: BAIL_ROUNDS,
        };
    }

    // Distinct versions have distinct canonical bytes, so the comparison is
    // strict and total: the larger-bytes side is the initiator (`mirror.rs`).
    let alice_initiates = version_a.as_bytes() > version_b.as_bytes();

    let n = (shared + distinct_a + distinct_b + redacted_a + redacted_b) as f64;
    Prediction {
        bytes_a: side_bytes(
            distinct_a,
            distinct_b,
            shared,
            redacted_a,
            redacted_b,
            vlen_a,
            alice_initiates,
        ),
        bytes_b: side_bytes(
            distinct_b,
            distinct_a,
            shared,
            redacted_b,
            redacted_a,
            vlen_b,
            !alice_initiates,
        ),
        rounds_a: side_rounds(n, alice_initiates),
        rounds_b: side_rounds(n, !alice_initiates),
    }
}

/// Fixed per-session framing ahead of any version payload: the 8-byte protocol
/// handshake + the 4-byte length-delimited frame prefix + borsh's 4-byte `u32`
/// length on the `Version` body. The connect frame is `CONNECT_FIXED +
/// |version|` bytes.
const CONNECT_FIXED: f64 = 16.0;

/// Connect-bail rounds: handshake write/read + connect write/read = three
/// protocol-layer phase transitions = 1.5 rounds, identically on both sides.
const BAIL_ROUNDS: f64 = 1.5;

/// One side's transmitted bytes, given its role and the exact serialized size
/// of the `Version` it stamps on its connect frame and on every leaf it
/// provides. `d`/`r` are this side's own outgoing leaves and redactions;
/// `od`/`orr` are the other side's. Coefficients are fit to the deterministic
/// sweep; each is annotated with its structural meaning.
fn side_bytes(
    d: u32,
    od: u32,
    s: u32,
    r: u32,
    orr: u32,
    version_len: f64,
    is_initiator: bool,
) -> f64 {
    let d = d as f64;
    // The side's Version rides its connect frame once and every provided leaf.
    let version_total = version_len * (1.0 + d);

    if is_initiator {
        // Provides leaves shallow (cheap framing), then pays an `uncertain`-hash
        // frontier cost over the divergent set `U = S + D_other + R_self`. The
        // frontier term is super-linear: deeper rounds add whole hash levels.
        const INIT_BASE: f64 = 36.4;
        const INIT_PER_LEAF: f64 = 39.9;
        const INIT_PER_VERSION: f64 = 1.106;
        const INIT_FRONTIER: f64 = 0.649;
        let u = (s + od + r) as f64;
        INIT_BASE
            + INIT_PER_LEAF * d
            + INIT_PER_VERSION * version_total
            + INIT_FRONTIER * u * (1.0 + u).log2()
    } else {
        // Provides leaves deep (pricier framing per leaf), and exchanges one
        // 32-byte hash per shared element it descends past and per element the
        // *other* side redacted (routed to `requested`).
        const RESP_BASE: f64 = 68.4;
        const RESP_PER_LEAF: f64 = 66.7;
        const RESP_PER_VERSION: f64 = 0.969;
        const RESP_PER_SHARED: f64 = 30.8;
        const RESP_PER_OTHER_REDACT: f64 = 31.2;
        RESP_BASE
            + RESP_PER_LEAF * d
            + RESP_PER_VERSION * version_total
            + RESP_PER_SHARED * s as f64
            + RESP_PER_OTHER_REDACT * orr as f64
    }
}

/// One side's round count: 256-ary descent over a union of `n` leaves, with the
/// initiator running one extra half-round-trip (its `Initiate`/`close` pair).
fn side_rounds(n: f64, is_initiator: bool) -> f64 {
    const ROUNDS_RESP_BASE: f64 = 2.356;
    const ROUNDS_INIT_BONUS: f64 = 0.5;
    const ROUNDS_PER_LEVEL: f64 = 1.357;
    let base = ROUNDS_RESP_BASE + if is_initiator { ROUNDS_INIT_BONUS } else { 0.0 };
    base + ROUNDS_PER_LEVEL * (n + 1.0).log(256.0)
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
                    job.bob.gossip(&mut r, &mut w).expect("sync gossip bob");
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
                bob.gossip(&mut r, &mut w).expect("sync gossip bob");
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
        let _alice_out = alice.gossip(&mut r, &mut w).expect("sync gossip alice");
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
