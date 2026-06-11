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
//! 2. Both alice and bob gossip with every ancestor party (over pipes, the
//!    wire being the only merge) — each side's [`Version`] (an ITC event
//!    tree) now records one populated region per contributing background
//!    party.
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
//! | `vlen_a`, `vlen_b`                       | exact serialized [`Version`] size per side, in bytes  |
//! | `initiator_a`                            | `1` if A initiates, `0` if B does, empty on a bail    |
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
//! * **Connect frame**: each side ships one greeting as a 4-byte
//!   length-delimited frame whose borsh body is the [`Version`] (a `u32`
//!   length prefix + the bit-packed ITC event tree). With the 25-byte raw
//!   protocol preamble (magic, protocol version, network id, intent tag)
//!   that is exactly `33 + |version|` bytes (itemized at `CONNECT_FIXED`),
//!   where `|version|` is the serialized event tree (often just **1–30
//!   bytes**; see below). On a connect-bail this is the *entire* cost: e.g.
//!   an empty version is `33 + 1 = 34` bytes per side.
//!
//! Past the connect, cost splits by role, because the initiator provides its
//! leaves *shallow* (early in the descent, before the frontier narrows) while
//! the responder provides them *deep*:
//!
//! * **Initiator** ≈ `53 + 40·D + |version|·(1 + D) + 0.65·U·log₂(1+U)` bytes,
//!   where `D` is its own outgoing leaves and `U = S + D_other + R_self` is the
//!   divergent set it must exchange `uncertain` hashes through. The frontier
//!   term is super-linear because deeper rounds add whole levels of hashes.
//! * **Responder** ≈ `85 + 67·D + |version|·(1 + D) + 31·S + 31·R_other` bytes.
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
//! |  1   | 1.03       |
//! |  8   | 0.99       |
//! |  32  | 0.94       |
//! |  64  | 0.93       |
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
//! # Refitting the model
//!
//! The fixed connect framing is derived from the wire format (and pinned by
//! the gossip snapshots), but the descent-cost coefficients are empirical fits
//! that rot silently when the wire format changes. To refit, sweep then fit:
//!
//! ```text
//! cargo run --release --example network_profile -- --output profile.csv \
//!     --max-shared 5 --max-distinct 5 --max-redacted 4 --max-parties 6
//! cargo run --release --example network_profile -- --fit profile.csv
//! ```
//!
//! `--fit` re-derives every `side_bytes`/`side_rounds` coefficient by least
//! squares over the per-side observations (printed paste-ready), reports
//! residuals for both the shipped constants and the refit, verifies the
//! connect-bail prediction is still byte-exact, and prints the
//! compression-ratio table above.
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
    about = "Empirically measure rumors gossip bandwidth and round trips across a 6-D input space."
)]
struct Args {
    /// Output CSV path. The file is created (or truncated) and written
    /// row-by-row with a flush after each sample.
    #[arg(short, long, conflicts_with = "fit", required_unless_present = "fit")]
    output: Option<PathBuf>,

    /// Instead of running a sweep, refit the descent-cost model from a
    /// previously generated CSV: prints least-squares coefficients for
    /// `side_bytes`/`side_rounds` (paste-ready), residual summaries for the
    /// shipped constants and the refit, the connect-bail exactness check, and
    /// the compression-ratio-by-parties table the module docs quote.
    #[arg(long, value_name = "CSV")]
    fit: Option<PathBuf>,

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

/// Mint a genuine party-disjoint peer that inherits `parent`'s content.
///
/// A peer with its own disjoint Interval Tree Clock region — required of any
/// party that will independently `send`/`redact`, and of each anonymous
/// background party here — is minted by serving a bootstrap from `parent`
/// over a pair of pipes. The newcomer pulls `parent`'s whole tree through the
/// ordinary mirror descent and is handed a fresh disjoint party, forked in
/// the same critical section that snapshots the served tree. Serving from an
/// empty `parent` yields a disjoint empty peer in the same universe.
fn bootstrap_fork<T>(parent: &mut Known<T>) -> Known<T>
where
    T: borsh::BorshSerialize + borsh::BorshDeserialize + Clone + Send + Sync + 'static,
{
    let (mut p2n_r, mut p2n_w) = pipe().expect("pipe parent->newcomer");
    let (mut n2p_r, mut n2p_w) = pipe().expect("pipe newcomer->parent");
    thread::scope(|s| {
        let newcomer = s.spawn(move || {
            Known::<T>::bootstrap(&mut p2n_r, &mut n2p_w)
                .expect("bootstrap newcomer")
                .expect("provider served bootstrap")
        });
        parent
            .gossip(&mut n2p_r, &mut p2n_w)
            .expect("serve bootstrap");
        newcomer.join().expect("join bootstrap thread")
    })
}

/// One bidirectional gossip session between two locals over a pair of pipes,
/// each side on its own thread: how a background party's content is absorbed
/// into alice and bob now that the in-process merge is gone.
fn sync_gossip<T>(a: &mut Known<T>, b: &mut Known<T>)
where
    T: borsh::BorshSerialize + borsh::BorshDeserialize + Clone + Send + Sync + 'static,
{
    let (mut a2b_r, mut a2b_w) = pipe().expect("pipe a->b");
    let (mut b2a_r, mut b2a_w) = pipe().expect("pipe b->a");
    thread::scope(|s| {
        let b_thread = s.spawn(move || b.gossip(&mut a2b_r, &mut b2a_w).expect("gossip b"));
        a.gossip(&mut b2a_r, &mut a2b_w).expect("gossip a");
        b_thread.join().expect("join b thread");
    });
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
    let mut alice = bootstrap_fork(&mut seed);
    let mut bob = bootstrap_fork(&mut seed);

    let total_ancestor = shared + redacted_a + redacted_b;
    let mut keys: Vec<Key> = Vec::with_capacity(total_ancestor as usize);

    if total_ancestor > 0 {
        // Cap effective parties to total_ancestor: a party with 0 inserts is
        // structurally indistinguishable from "not present" in the network.
        let effective = parties.min(total_ancestor).max(1);
        let base = total_ancestor / effective;
        let remainder = total_ancestor % effective;
        for i in 0..effective {
            let count = (base + if i < remainder { 1 } else { 0 }) as usize;
            let mut bg = bootstrap_fork(&mut seed);
            let pre = bg.latest();
            {
                let mut batch = bg.batch();
                for _ in 0..count {
                    batch.send(());
                }
            }
            // The keys bg's batch just minted: the live leaves above bg's
            // pre-batch frontier, in the tree's deterministic order.
            keys.extend(
                bg.snapshot()
                    .range(rumors::causally::since(&pre))
                    .map(|(k, _, _)| k),
            );
            // Merge bg's content into alice and bob over the wire. bg is a
            // genuine disjoint party, so its leaves carry its own ITC region:
            // absorbing them raises each side's version to record one
            // populated region per background party. (At this point alice
            // and bob hold nothing of their own, so the bidirectional
            // session moves content in one direction only.)
            sync_gossip(&mut alice, &mut bg);
            sync_gossip(&mut bob, &mut bg);
        }
    }

    let s = shared as usize;
    let ra = redacted_a as usize;
    let redact_a: Vec<Key> = keys[s..s + ra].to_vec();
    let redact_b: Vec<Key> = keys[s + ra..].to_vec();

    {
        let mut batch = alice.batch();
        for key in redact_a {
            batch.redact(key);
        }
        for _ in 0..distinct_a {
            batch.send(());
        }
    }

    {
        let mut batch = bob.batch();
        for key in redact_b {
            batch.redact(key);
        }
        for _ in 0..distinct_b {
            batch.send(());
        }
    }

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
/// (4-byte big-endian length + borsh body), and a session opens with the
/// 25-byte raw preamble (magic, protocol version, network id, intent tag).
/// The first frame each side sends is its greeting: its [`Version`] alone (a
/// `u32` length prefix + the bit-packed event tree). So the connect costs
/// exactly `33 + |version|` bytes (itemized at `CONNECT_FIXED`), and on a
/// connect-bail (`DA+DB+RA+RB == 0`, equal versions) that is the whole
/// session — `1.5` rounds (preamble + connect, three phase transitions).
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
    let version_a = alice.latest();
    let version_b = bob.latest();
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
            vlen_a,
            vlen_b,
            alice_initiates: None,
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
        vlen_a,
        vlen_b,
        alice_initiates: Some(alice_initiates),
    }
}

/// The 25-byte raw preamble: `b"RUMORS"`, the `u16` protocol version, the
/// 16-byte network id, and the 1-byte intent tag (remain/retire).
const PREAMBLE: f64 = 25.0;
/// `LengthDelimitedCodec`'s 4-byte big-endian length prefix on the greeting.
const GREETING_FRAME_PREFIX: f64 = 4.0;
/// borsh's `u32` length prefix on the `Version` bytes inside the greeting
/// (since the preamble rework, the version is the whole greeting body).
const GREETING_VERSION_PREFIX: f64 = 4.0;

/// Fixed per-session framing: everything a side sends through the connect
/// besides the bit-packed event tree itself, so the connect costs
/// `CONNECT_FIXED + |version|` bytes. The layout is pinned byte-for-byte by
/// `tests/gossip_snapshot.rs` — its empty-pair snapshot shows each side
/// sending 25 + 9 bytes, i.e. `CONNECT_FIXED` (33) + a 1-byte empty version.
/// If a preamble or greeting field changes, that snapshot churns and this
/// sum must follow.
const CONNECT_FIXED: f64 = PREAMBLE + GREETING_FRAME_PREFIX + GREETING_VERSION_PREFIX;

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
        const INIT_BASE: f64 = 53.4;
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
        const RESP_BASE: f64 = 85.4;
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
    /// Exact serialized [`Version`] sizes, read off the reconstruction. Emitted
    /// into the CSV so `--fit` can rebuild the model features without rerunning
    /// the reconstruction.
    vlen_a: f64,
    vlen_b: f64,
    /// `None` on a connect-bail: equal versions, no descent, no initiator.
    alice_initiates: Option<bool>,
}

/// One side's observation from a descent (non-bail) CSV row, in the model's
/// own coordinates: `d`/`r` are this side's outgoing leaves and redactions,
/// `od`/`orr` the other side's (only their combinations `u` and `orr` are
/// retained).
#[derive(Copy, Clone, Debug)]
struct SideObs {
    /// This side's own outgoing leaves.
    d: f64,
    /// Shared elements.
    s: f64,
    /// Elements the *other* side redacted.
    orr: f64,
    /// Divergent set the initiator's frontier term ranges over: `s + od + r`.
    u: f64,
    /// Union size for the rounds model: `s + da + db + ra + rb`.
    n: f64,
    /// Exact serialized `Version` size for this side.
    vlen: f64,
    init: bool,
    bytes: f64,
    rounds: f64,
    /// Predictions of the constants currently compiled into this binary, so
    /// the report can show shipped-versus-refit drift.
    shipped_bytes: f64,
    shipped_rounds: f64,
}

/// Solve the least-squares problem `min ‖Xβ − y‖` via the normal equations
/// `(XᵀX)β = Xᵀy`, by Gauss-Jordan elimination with partial pivoting. The
/// systems here are at most 5×5, so numerical sophistication beyond pivoting
/// (or a linear-algebra dependency) would be wasted on an example.
fn least_squares(x: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let k = x[0].len();
    // Augmented [XᵀX | Xᵀy].
    let mut a = vec![vec![0.0f64; k + 1]; k];
    for (f, &yi) in x.iter().zip(y) {
        for i in 0..k {
            for j in 0..k {
                a[i][j] += f[i] * f[j];
            }
            a[i][k] += f[i] * yi;
        }
    }
    for col in 0..k {
        let pivot = (col..k)
            .max_by(|&r1, &r2| a[r1][col].abs().partial_cmp(&a[r2][col].abs()).unwrap())
            .unwrap();
        a.swap(col, pivot);
        let p = a[col][col];
        assert!(
            p.abs() > 1e-9,
            "singular normal equations: a regressor is degenerate in this CSV",
        );
        for v in &mut a[col][col..=k] {
            *v /= p;
        }
        // Copy the (tiny) normalized pivot row so eliminating the other rows
        // doesn't alias it.
        let pivot_row: Vec<f64> = a[col][col..=k].to_vec();
        for (row, r) in a.iter_mut().enumerate() {
            if row != col && r[col] != 0.0 {
                let factor = r[col];
                for (v, pv) in r[col..=k].iter_mut().zip(&pivot_row) {
                    *v -= factor * pv;
                }
            }
        }
    }
    (0..k).map(|i| a[i][k]).collect()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `(median, p90, max)` of `|pred − meas| / meas`.
fn rel_err_summary(pred: &[f64], meas: &[f64]) -> (f64, f64, f64) {
    let mut errs: Vec<f64> = pred
        .iter()
        .zip(meas)
        .map(|(p, m)| ((p - m) / m).abs())
        .collect();
    errs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pick = |q: f64| errs[((errs.len() - 1) as f64 * q).round() as usize];
    (pick(0.5), pick(0.9), *errs.last().unwrap())
}

/// Fit one model block and print paste-ready constants plus a residual
/// comparison between the constants compiled into this binary ("shipped") and
/// the fresh fit. `names` labels the coefficients positionally, matching the
/// regressor order `features` produces.
fn fit_block(
    title: &str,
    names: &[&str],
    obs: &[SideObs],
    features: impl Fn(&SideObs) -> Vec<f64>,
    measured: impl Fn(&SideObs) -> f64,
    shipped: impl Fn(&SideObs) -> f64,
) {
    println!("{title} ({} observations):", obs.len());
    if obs.len() < names.len() {
        println!("  too few observations to fit; run a larger sweep");
        return;
    }
    let x: Vec<Vec<f64>> = obs.iter().map(&features).collect();
    let y: Vec<f64> = obs.iter().map(&measured).collect();
    let beta = least_squares(&x, &y);
    for (name, b) in names.iter().zip(&beta) {
        println!("    const {name}: f64 = {b:.3};");
    }
    let refit: Vec<f64> = x.iter().map(|f| dot(f, &beta)).collect();
    let shipped: Vec<f64> = obs.iter().map(&shipped).collect();
    let (sm, sp90, smax) = rel_err_summary(&shipped, &y);
    let (rm, rp90, rmax) = rel_err_summary(&refit, &y);
    println!(
        "  |rel err| shipped: median {:5.2}%  p90 {:5.2}%  max {:6.2}%",
        sm * 100.0,
        sp90 * 100.0,
        smax * 100.0,
    );
    println!(
        "  |rel err| refit:   median {:5.2}%  p90 {:5.2}%  max {:6.2}%",
        rm * 100.0,
        rp90 * 100.0,
        rmax * 100.0,
    );
}

/// Refit the descent-cost model from a previously generated sweep CSV.
///
/// Connect-bail rows are *verified* (the prediction there is exact by
/// construction, so any residual means `CONNECT_FIXED` has drifted from the
/// wire format) and excluded from the fits. Every other row yields two
/// per-side observations.
fn run_fit(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let col = |name: &str| -> Result<usize, String> {
        headers.iter().position(|h| h == name).ok_or_else(|| {
            format!(
                "CSV is missing column `{name}`: regenerate it with --output \
                 (the fit needs the vlen_a/vlen_b/initiator_a columns)"
            )
        })
    };
    let c_parties = col("parties")?;
    let c_shared = col("shared")?;
    let c_da = col("distinct_a")?;
    let c_db = col("distinct_b")?;
    let c_ra = col("redacted_a")?;
    let c_rb = col("redacted_b")?;
    let c_bytes_a = col("bytes_a")?;
    let c_bytes_b = col("bytes_b")?;
    let c_comp_a = col("bytes_a_compressed")?;
    let c_comp_b = col("bytes_b_compressed")?;
    let c_rounds_a = col("rounds_a")?;
    let c_rounds_b = col("rounds_b")?;
    let c_vlen_a = col("vlen_a")?;
    let c_vlen_b = col("vlen_b")?;
    let c_init_a = col("initiator_a")?;

    let mut obs: Vec<SideObs> = Vec::new();
    let mut bail_rows = 0u64;
    let mut bail_bytes_max = 0.0f64;
    let mut bail_rounds_max = 0.0f64;
    // parties → (sum of per-side wire/protocol ratios, side count)
    let mut ratios: std::collections::BTreeMap<u64, (f64, u64)> = std::collections::BTreeMap::new();

    for record in reader.records() {
        let record = record?;
        let get = |i: usize| -> Result<f64, Box<dyn std::error::Error>> {
            Ok(record.get(i).ok_or("short CSV record")?.parse::<f64>()?)
        };
        let parties = get(c_parties)? as u64;
        let s = get(c_shared)?;
        let da = get(c_da)?;
        let db = get(c_db)?;
        let ra = get(c_ra)?;
        let rb = get(c_rb)?;
        let bytes_a = get(c_bytes_a)?;
        let bytes_b = get(c_bytes_b)?;
        let comp_a = get(c_comp_a)?;
        let comp_b = get(c_comp_b)?;
        let rounds_a = get(c_rounds_a)?;
        let rounds_b = get(c_rounds_b)?;
        let vlen_a = get(c_vlen_a)?;
        let vlen_b = get(c_vlen_b)?;

        let ratio = ratios.entry(parties).or_insert((0.0, 0));
        ratio.0 += comp_a / bytes_a + comp_b / bytes_b;
        ratio.1 += 2;

        let init_field = record.get(c_init_a).ok_or("short CSV record")?;
        if init_field.is_empty() {
            // Connect-bail: exact by construction, so verify instead of fit.
            bail_rows += 1;
            bail_bytes_max = bail_bytes_max
                .max((bytes_a - (CONNECT_FIXED + vlen_a)).abs())
                .max((bytes_b - (CONNECT_FIXED + vlen_b)).abs());
            bail_rounds_max = bail_rounds_max
                .max((rounds_a - BAIL_ROUNDS).abs())
                .max((rounds_b - BAIL_ROUNDS).abs());
            continue;
        }
        let alice_initiates = init_field == "1";
        let n = s + da + db + ra + rb;
        for (d, od, r, orr, vlen, init, bytes, rounds) in [
            (da, db, ra, rb, vlen_a, alice_initiates, bytes_a, rounds_a),
            (db, da, rb, ra, vlen_b, !alice_initiates, bytes_b, rounds_b),
        ] {
            obs.push(SideObs {
                d,
                s,
                orr,
                u: s + od + r,
                n,
                vlen,
                init,
                bytes,
                rounds,
                shipped_bytes: side_bytes(
                    d as u32, od as u32, s as u32, r as u32, orr as u32, vlen, init,
                ),
                shipped_rounds: side_rounds(n, init),
            });
        }
    }

    println!(
        "rows: {} connect-bail + {} descent ({} per-side observations)",
        bail_rows,
        obs.len() / 2,
        obs.len(),
    );
    println!(
        "connect-bail exactness: max |bytes residual| = {bail_bytes_max:.3}, \
         max |rounds residual| = {bail_rounds_max:.3}",
    );
    if bail_bytes_max > 0.0 || bail_rounds_max > 0.0 {
        println!(
            "  WARNING: the connect-bail prediction must be exact; \
             CONNECT_FIXED has drifted from the wire format"
        );
    }
    println!();

    let initiator: Vec<SideObs> = obs.iter().copied().filter(|o| o.init).collect();
    let responder: Vec<SideObs> = obs.iter().copied().filter(|o| !o.init).collect();

    fit_block(
        "initiator bytes",
        &[
            "INIT_BASE",
            "INIT_PER_LEAF",
            "INIT_PER_VERSION",
            "INIT_FRONTIER",
        ],
        &initiator,
        |o| vec![1.0, o.d, o.vlen * (1.0 + o.d), o.u * (1.0 + o.u).log2()],
        |o| o.bytes,
        |o| o.shipped_bytes,
    );
    println!();
    fit_block(
        "responder bytes",
        &[
            "RESP_BASE",
            "RESP_PER_LEAF",
            "RESP_PER_VERSION",
            "RESP_PER_SHARED",
            "RESP_PER_OTHER_REDACT",
        ],
        &responder,
        |o| vec![1.0, o.d, o.vlen * (1.0 + o.d), o.s, o.orr],
        |o| o.bytes,
        |o| o.shipped_bytes,
    );
    println!();
    fit_block(
        "rounds (both roles)",
        &["ROUNDS_RESP_BASE", "ROUNDS_INIT_BONUS", "ROUNDS_PER_LEVEL"],
        &obs,
        |o| vec![1.0, f64::from(o.init), (o.n + 1.0).log(256.0)],
        |o| o.rounds,
        |o| o.shipped_rounds,
    );
    println!();

    println!("compression ratio (wire bytes / protocol bytes), mean by parties:");
    for (p, (sum, count)) in &ratios {
        println!("  P = {:>3}: {:.2}", p, sum / *count as f64);
    }
    Ok(())
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
            while let Ok(mut job) = job_rx.recv() {
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
            let mut bob = bob;
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
    let mut alice = alice;
    let stats_a: StatsHandle = Arc::new(Mutex::new(SideStats::default()));
    {
        let (mut r, mut w) =
            build_io_stack(b_to_a_r, a_to_b_w, &stats_a, zstd_level).expect("build alice stack");
        alice.gossip(&mut r, &mut w).expect("sync gossip alice");
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
    if let Some(csv_path) = &args.fit {
        return run_fit(csv_path);
    }
    let output_path = args
        .output
        .clone()
        .expect("clap enforces exactly one of --output / --fit");
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
    // row is sent as a pre-formatted `[String; 22]` so all string formatting
    // happens in parallel on the producer side.
    let header: [&'static str; 22] = [
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
        "vlen_a",
        "vlen_b",
        "initiator_a",
    ];
    let (row_tx, row_rx) = mpsc::channel::<[String; 22]>();
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
                    format!("{}", p.vlen_a as u64),
                    format!("{}", p.vlen_b as u64),
                    p.alice_initiates
                        .map(|init| if init { "1" } else { "0" }.to_string())
                        .unwrap_or_default(),
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
