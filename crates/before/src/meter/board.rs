//! The amplification board: a red-green resource-proportionality matrix.
//!
//! Sweeps the crate's public operation surface against the adversarial input
//! families in [`meter`](crate::meter) (plus a benign pseudo-random control)
//! and prints one verdict per operation × family cell. The target contract
//! being scored: no operation materializes transient state asymptotically
//! larger than its packed operands, and every operation is amortized
//! `O(n + m)` in the packed input bits — with no bound on value magnitude,
//! tree depth, or encoded size.
//!
//! # The criterion
//!
//! Each cell runs its operation at two input scales (the second twice the
//! first) and reads three deterministic meters over the body alone:
//!
//! - **peak heap bytes**, from a caller-installed counting allocator
//!   (see [`HeapMeter`]);
//! - **grown stack segments** ([`stack_segments`](crate::meter::stack_segments)),
//!   the honest stand-in for recursion-driven stack cost, which bypasses any
//!   heap meter;
//! - **big-integer limb operations**
//!   ([`limb_ops`](crate::meter::limb_ops)), only when the `limb-meter`
//!   feature compiles the counter into the arithmetic; arithmetic-width
//!   blowups are invisible to the other two meters.
//!
//! Per meter the board derives a **scaling exponent**
//! `log(m₂/m₁) / log(n₂/n₁)` (`n` = packed input bytes) and a **per-input-byte
//! constant** at the larger scale. A cell is **GREEN** iff every meter's
//! exponent is at most [`MAX_SCALING_EXPONENT`] *and* every constant is under
//! its pinned ceiling ([`MAX_HEAP_BYTES_PER_INPUT_BYTE`] over
//! [`HEAP_FLAT_ALLOWANCE_BYTES`], [`MAX_GROWN_STACK_SEGMENTS`],
//! [`MAX_LIMB_OPS_PER_INPUT_BYTE`]); **RED** otherwise, with the offending
//! meters named. Wall time is displayed per scale but never judged: it is the
//! one number here that is not deterministic.
//!
//! # Reading the numbers
//!
//! The board runs every cell in one process and resets the peak-heap counter
//! between cells, so a cell's heap number can include allocator noise from
//! the harness itself: the board's numbers are *indicative*. The enforced
//! record is the meter test binary (`tests/meter.rs`), whose scenarios run
//! one per process under nextest and pin exact envelopes. Zero-measurement
//! cells score exponent 0; a meter that moves from 0 to a nonzero count is
//! clamped through `max(m, 1)` before the ratio, so the exponent stays
//! finite.
//!
//! # Families
//!
//! The four adversarial shapes from [`meter`](crate::meter) — the dense
//! event spine, `bigroot`, `hugeleaf`, and the diverted id-spine pair — plus
//! `benign`: a fixed-seed pseudo-random population of forked, ticked clocks,
//! the control row that keeps the ceilings honest on organic inputs. Event
//! families exercise `Version` (and `Clock`) operations; the id pair
//! exercises `Party` (and `Clock`) operations; `benign` provides both. Where
//! an operation needs a `Party` and a `Version`, the board crosses
//! adversarial party × small version and small party × adversarial version.
//!
//! # Coverage: the not-applicable list
//!
//! Every public operation either has a board row or is listed here with the
//! reason it has no meaningful adversarial operand of its own:
//!
//! - **Delegations and aliases**: `Version::concurrent`/`Batch::concurrent`
//!   are one `partial_cmp` (the `cmp` row measures the walk; `concurrent`
//!   still gets its own row since it is the documented entry point);
//!   `Version::tick` is `Batch::tick` (the tick rows drive the batch);
//!   `Clock::send` is `Clock::tick` by definition; `clock | version` and
//!   `clock |= version` fold through the same `join_version` the `recv` row
//!   measures; `Clock::batch` operations are what the clock rows already
//!   route through; `Party::tick` is the mirror of `Version::tick` (the
//!   `tick_adv_party` row); `Debug` for all three types delegates to
//!   `Display`.
//! - **Folds of measured rows**: `Version::join_all`/`meet_all`,
//!   `Sum`/`FromIterator`, `Party::join_all`, `Clock::join_all` iterate the
//!   measured `join`/`meet` cells; `Party::forks`/`Clock::forks` iterate
//!   `fork`.
//! - **Bounded or trivial inputs**: `Version::new`/`Default`,
//!   `TryFrom<u64>`/tuple literals (word-sized literals),
//!   `Party::seed`/`is_seed`, `TryFrom<u8>`/`TryFrom<bool>`,
//!   `Clock::seed`/`TryFrom<(I, E)>`.
//! - **Moves, borrows, and byte copies**: `is_empty`, `as_bytes`,
//!   `encoded_bits`, `encode_to` (the `encode` row's path into a writer),
//!   `dangerously_alias` (a byte copy), `Clock::from_parts`/`into_parts`,
//!   `Clock::party`/`version`, `Version::batch`.
//! - **Derived pairings**: `Ranked::from` is the `rank` row plus a move; its
//!   comparisons are `Rank` comparisons plus byte equality;
//!   `Rank::checked_sub` and `Rank`'s ordering run inside the
//!   `distance`/`lag` rows.
//! - **The same comparisons under another name**: `causally`'s other
//!   constructors and `Range::placement_of` perform the identical causal
//!   comparisons the `causally_contains` row measures.
//! - **Wrappers**: the `serde`/`borsh` impls serialize as the canonical
//!   encoding and deserialize through the strict decoder — the
//!   `encode`/`decode` rows.
//! - **Test support**: `oracle`, `meter`, and the `error`/`iter` modules'
//!   data types perform no computation over packed inputs.

use std::any::Any;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::{causally, Clock, Party, Version};

// ─── the pinned ceilings ────────────────────────────────────────────────────

/// Green requires every meter's scaling exponent at or below this.
///
/// The contract is amortized-linear; 1.15 leaves room for measurement noise
/// (allocator rounding, `Vec` doubling) without admitting a real log factor
/// at these input sizes.
pub const MAX_SCALING_EXPONENT: f64 = 1.15;

/// Green requires peak transient heap at most this many bytes per packed
/// input byte, over the flat allowance.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 16.0;

/// Heap bytes ignored before the per-byte constant is computed: fixed-size
/// scaffolding (format machinery, hasher state, container headers) that does
/// not scale with the input.
pub const HEAP_FLAT_ALLOWANCE_BYTES: usize = 8_192;

/// Green requires at most this many grown stack segments, as an absolute
/// count: the target is walks that never grow the stack, so the ceiling is
/// flat, not per-byte.
pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

/// Green requires at most this many big-integer limb operations per packed
/// input byte (asserted only when the `limb-meter` feature is lit).
///
/// Calibrated against the benign control: an amortized-linear walk records a
/// handful of unit-limb operations per node (tens per packed byte at ~2 bits
/// per node, over a hundred for multi-walk operations like `distance`), and
/// that per-node arithmetic is exactly the contract's linear regime. The
/// ceiling sits above it; width blowups are caught by the exponent bound
/// long before the constant.
pub const MAX_LIMB_OPS_PER_INPUT_BYTE: f64 = 128.0;

// ─── family sizes at scale 1.0 ──────────────────────────────────────────────

/// Dense event spine depth at scale 1.0 (packed size ~4 KiB).
const DENSE_BASE_DEPTH: usize = 8_000;

/// Bigroot root magnitude in bits at scale 1.0.
const BIGROOT_BASE_MAGNITUDE_BITS: usize = 8_000;

/// Bigroot spine depth at scale 1.0 (packed size ~3 KiB with the magnitude).
const BIGROOT_BASE_DEPTH: usize = 2_000;

/// Hugeleaf magnitude in bits at scale 1.0 (packed size ~4 KiB).
const HUGELEAF_BASE_MAGNITUDE_BITS: usize = 16_000;

/// Id spine depth at scale 1.0 (packed pair ~6 KiB).
const ID_BASE_DEPTH: usize = 12_000;

/// Benign clock population at scale 1.0.
const BENIGN_BASE_CLOCKS: usize = 256;

/// Floor on every scaled size parameter, so extreme scale-down (the smoke
/// test) still builds valid shapes and a nonempty benign population.
const MIN_SIZE_PARAM: usize = 4;

/// Fixed seed for the benign family's pseudo-random construction: the
/// control row must be deterministic run to run.
const BENIGN_RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

// ─── caller-supplied heap meter ─────────────────────────────────────────────

/// The peak-heap meter the board reads, supplied by the binary that runs it.
///
/// A counting global allocator is per-binary state the library cannot own,
/// so the runner (the `amp_board` example, the smoke test) installs one and
/// passes readers in. All three read the runner's allocator: `reset_peak`
/// clears the peak high-water mark, `peak` reads it, `current` reads live
/// bytes (the baseline subtracted from the peak).
pub struct HeapMeter {
    /// Clear the peak high-water mark down to current usage.
    pub reset_peak: fn(),
    /// The peak live bytes since the last reset.
    pub peak: fn() -> usize,
    /// The currently live bytes.
    pub current: fn() -> usize,
}

// ─── board outcome ──────────────────────────────────────────────────────────

/// The board's bottom line: how many cells scored green and red.
#[derive(Debug, Clone, Copy)]
pub struct Summary {
    /// Cells within every ceiling and exponent bound.
    pub green: usize,
    /// Cells over at least one bound, i.e. amplification findings.
    pub red: usize,
}

// ─── input families ─────────────────────────────────────────────────────────

/// The five input families, one column group of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamilyKind {
    /// The dense event spine `S(d)`: node count and depth maximizer.
    Dense,
    /// `bigroot(B, d)`: a huge root magnitude over a long spine.
    Bigroot,
    /// `hugeleaf(B)`: one node, maximal bits per node.
    Hugeleaf,
    /// The diverted id-spine pair `I(d, ·)`: full-lockstep two-party walks.
    IdPair,
    /// The fixed-seed organic control population.
    Benign,
}

/// Every family, in display order.
const FAMILIES: [FamilyKind; 5] = [
    FamilyKind::Dense,
    FamilyKind::Bigroot,
    FamilyKind::Hugeleaf,
    FamilyKind::IdPair,
    FamilyKind::Benign,
];

/// One family instantiated at one scale: the packed operands every row's
/// `prepare` decodes fresh (outside measurement).
struct FamilyData {
    kind: FamilyKind,
    name: &'static str,
    /// The family's primary packed version (event families and benign).
    version: Option<Vec<u8>>,
    /// The comparison counterpart: `version` plus one seed tick, packed.
    version2: Option<Vec<u8>>,
    /// A disjoint packed party pair (the id pair and benign halves).
    parties: Option<(Vec<u8>, Vec<u8>)>,
}

impl FamilyData {
    /// Build a family's operands at `scale`, doubled `level` times.
    ///
    /// `level` 0 and 1 are the two measurement scales of every cell.
    fn build(kind: FamilyKind, scale: f64, level: u32) -> FamilyData {
        let size = |base: usize| -> usize {
            let scaled = ((base as f64) * scale).round() as usize;
            scaled.max(MIN_SIZE_PARAM) << level
        };
        match kind {
            FamilyKind::Dense => {
                Self::event(kind, "dense", super::dense(size(DENSE_BASE_DEPTH)).bytes)
            }
            FamilyKind::Bigroot => Self::event(
                kind,
                "bigroot",
                super::bigroot(size(BIGROOT_BASE_MAGNITUDE_BITS), size(BIGROOT_BASE_DEPTH)).bytes,
            ),
            FamilyKind::Hugeleaf => Self::event(
                kind,
                "hugeleaf",
                super::hugeleaf(size(HUGELEAF_BASE_MAGNITUDE_BITS)).bytes,
            ),
            FamilyKind::IdPair => FamilyData {
                kind,
                name: "id-pair",
                version: None,
                version2: None,
                parties: Some((
                    super::id_spine(size(ID_BASE_DEPTH), false).bytes,
                    super::id_spine(size(ID_BASE_DEPTH), true).bytes,
                )),
            },
            FamilyKind::Benign => Self::benign(size(BENIGN_BASE_CLOCKS)),
        }
    }

    /// Wrap a packed event shape and derive its ticked counterpart.
    fn event(kind: FamilyKind, name: &'static str, bytes: Vec<u8>) -> FamilyData {
        let v = decode_version(&bytes);
        let mut w = v;
        w.tick(&Party::seed());
        FamilyData {
            kind,
            name,
            version: Some(bytes),
            version2: Some(w.encode()),
            parties: None,
        }
    }

    /// Build the benign control: `n` clocks forked at random from a seed,
    /// each ticked one to three times, folded into one version and two
    /// disjoint half-population parties.
    fn benign(n: usize) -> FamilyData {
        let mut rng = XorShift(BENIGN_RNG_SEED);
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < n {
            let i = (rng.next() as usize) % clocks.len();
            let child = clocks[i].fork();
            clocks.push(child);
        }
        for clock in &mut clocks {
            for _ in 0..(rng.next() % 3 + 1) {
                clock.tick();
            }
        }
        let version = Version::join_all(clocks.iter().map(|c| c.version().clone()));
        let mut version2 = version.clone();
        version2.tick(&Party::seed());
        let mut parties = clocks.into_iter().map(|c| c.into_parts().0);
        let mut a = parties.next().expect("the population is nonempty");
        let mut b = parties
            .next()
            .expect("MIN_SIZE_PARAM keeps at least two clocks in the population");
        for (i, p) in parties.enumerate() {
            // Alternate the halves so both operand parties scatter across
            // the whole id tree rather than owning one contiguous region.
            let half = if i % 2 == 0 { &mut a } else { &mut b };
            half.join(p).expect("forked parties are pairwise disjoint");
        }
        FamilyData {
            kind: FamilyKind::Benign,
            name: "benign",
            version: Some(version.encode()),
            version2: Some(version2.encode()),
            parties: Some((a.encode(), b.encode())),
        }
    }

    /// The primary version, decoded fresh, with its packed byte length.
    fn version(&self) -> Option<(Version, usize)> {
        let bytes = self.version.as_ref()?;
        Some((decode_version(bytes), bytes.len()))
    }

    /// Both versions decoded fresh, with their combined packed byte length.
    fn version_pair(&self) -> Option<(Version, Version, usize)> {
        let (v, n) = self.version()?;
        let bytes2 = self.version2.as_ref()?;
        Some((v, decode_version(bytes2), n + bytes2.len()))
    }

    /// The disjoint party pair decoded fresh, with combined byte length.
    fn party_pair(&self) -> Option<(Party, Party, usize)> {
        let (a, b) = self.parties.as_ref()?;
        Some((decode_party(a), decode_party(b), a.len() + b.len()))
    }

    /// One clock per family: small party × adversarial version for the
    /// event families, adversarial party × small version for the id pair
    /// and the benign halves.
    fn clock(&self) -> Option<(Clock, usize)> {
        match self.kind {
            FamilyKind::IdPair => {
                let (a, _, _) = self.party_pair()?;
                let n = self.parties.as_ref().map(|(a, _)| a.len())?;
                Some((Clock::from_parts(a, Version::new()), n + 1))
            }
            _ => {
                let (v, n) = self.version()?;
                Some((Clock::from_parts(Party::seed(), v), n + 1))
            }
        }
    }

    /// Two joinable clocks (disjoint parties), with combined operand bytes.
    fn clock_pair(&self) -> Option<(Clock, Clock, usize)> {
        match self.kind {
            FamilyKind::IdPair => {
                let (a, b, n) = self.party_pair()?;
                Some((
                    Clock::from_parts(a, Version::new()),
                    Clock::from_parts(b, Version::new()),
                    n + 2,
                ))
            }
            FamilyKind::Benign => {
                let (a, b, np) = self.party_pair()?;
                let (v, w, nv) = self.version_pair()?;
                Some((Clock::from_parts(a, v), Clock::from_parts(b, w), np + nv))
            }
            _ => {
                let (v, w, n) = self.version_pair()?;
                let mut p = Party::seed();
                let q = p.fork();
                Some((Clock::from_parts(p, v), Clock::from_parts(q, w), n + 2))
            }
        }
    }
}

/// Decode packed bytes the board itself generated.
fn decode_version(bytes: &[u8]) -> Version {
    Version::decode(bytes).expect("board-generated version bytes are canonical")
}

/// Decode packed party bytes the board itself generated.
fn decode_party(bytes: &[u8]) -> Party {
    Party::decode(bytes).expect("board-generated party bytes are canonical")
}

/// A tiny xorshift64 generator: deterministic, dependency-free randomness
/// for the benign control family.
struct XorShift(u64);

impl XorShift {
    /// The next pseudo-random word.
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

// ─── operations ─────────────────────────────────────────────────────────────

/// One prepared cell run: the operand bytes it charges against and the body
/// to measure.
///
/// `prepare` builds (and decodes) operands outside measurement; the body's
/// result is boxed and kept alive until the meters are read, so peak heap
/// includes the fully materialized output.
struct Cell {
    /// The packed operand bytes the cost is normalized against.
    input_bytes: usize,
    /// The measured body; its result stays alive until the meters are read.
    #[allow(clippy::type_complexity)]
    body: Box<dyn FnOnce() -> Box<dyn Any>>,
}

impl Cell {
    /// Package a measured body with its operand byte count.
    fn new<R: Any>(input_bytes: usize, body: impl FnOnce() -> R + 'static) -> Cell {
        Cell {
            input_bytes,
            body: Box::new(move || Box::new(body())),
        }
    }
}

/// One board row: a public operation and how to instantiate it per family.
struct Op {
    /// The row label, `type_operation`.
    name: &'static str,
    /// Build the cell for one family, or `None` where the family provides
    /// no operand for this operation.
    prepare: fn(&FamilyData) -> Option<Cell>,
}

/// The operation table: every public operation with a meaningful packed
/// operand (the module doc lists the rest).
#[allow(clippy::too_many_lines)]
fn ops() -> Vec<Op> {
    vec![
        // ── Version ────────────────────────────────────────────────────
        Op {
            name: "version_decode",
            prepare: |f| {
                let bytes = f.version.clone()?;
                Some(Cell::new(bytes.len(), move || decode_version(&bytes)))
            },
        },
        Op {
            name: "version_encode",
            prepare: |f| {
                let (v, n) = f.version()?;
                Some(Cell::new(n, move || (v.encode(), v)))
            },
        },
        Op {
            name: "version_cmp",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || {
                    let ord: Option<Ordering> = v.partial_cmp(&w);
                    (ord, v, w)
                }))
            },
        },
        Op {
            name: "version_eq",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || (v == w, v, w)))
            },
        },
        Op {
            name: "version_concurrent",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || (v.concurrent(&w), v, w)))
            },
        },
        Op {
            name: "version_join",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || (&v | &w, v, w)))
            },
        },
        Op {
            name: "version_join_assign",
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || {
                    v |= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_meet",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || (&v & &w, v, w)))
            },
        },
        Op {
            name: "version_meet_assign",
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || {
                    v &= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_tick",
            prepare: |f| {
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(n + 1, move || {
                    v.tick(&party);
                    v
                }))
            },
        },
        Op {
            name: "version_tick_adv_party",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                Some(Cell::new(n + 1, move || {
                    v.tick(&a);
                    (v, a)
                }))
            },
        },
        Op {
            name: "version_batch_snapshot",
            prepare: |f| {
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(n + 1, move || {
                    let snap = {
                        let mut batch = v.batch();
                        batch.tick(&party);
                        batch.snapshot()
                    };
                    (snap, v)
                }))
            },
        },
        Op {
            name: "version_rank",
            prepare: |f| {
                let (v, n) = f.version()?;
                Some(Cell::new(n, move || (v.rank(), v)))
            },
        },
        Op {
            name: "version_distance",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || (v.distance(&w), v, w)))
            },
        },
        Op {
            name: "version_lag",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || (v.lag(&w), v, w)))
            },
        },
        Op {
            name: "version_min_ticks",
            prepare: |f| {
                let (v, n) = f.version()?;
                Some(Cell::new(n, move || (v.min_ticks(), v)))
            },
        },
        Op {
            name: "version_project",
            prepare: |f| match f.kind {
                // Adversarial party × small version.
                FamilyKind::IdPair => {
                    let (a, _, _) = f.party_pair()?;
                    let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                    let mut v = Version::new();
                    v.tick(&a);
                    Some(Cell::new(n + v.encode().len(), move || (&v / &a, v, a)))
                }
                // Small (half-interval) party × adversarial version.
                _ => {
                    let (v, n) = f.version()?;
                    let half = Party::seed().fork();
                    Some(Cell::new(n + 1, move || (&v / &half, v, half)))
                }
            },
        },
        Op {
            name: "version_display",
            prepare: |f| {
                let (v, n) = f.version()?;
                Some(Cell::new(n, move || (v.to_string(), v)))
            },
        },
        Op {
            name: "version_from_str",
            prepare: |f| {
                let (v, _) = f.version()?;
                let s = v.to_string();
                Some(Cell::new(s.len(), move || {
                    s.parse::<Version>()
                        .expect("a displayed version parses back")
                }))
            },
        },
        Op {
            name: "version_hash",
            prepare: |f| {
                let (v, n) = f.version()?;
                Some(Cell::new(n, move || {
                    let mut hasher = DefaultHasher::new();
                    v.hash(&mut hasher);
                    (hasher.finish(), v)
                }))
            },
        },
        Op {
            name: "causally_contains",
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, move || {
                    let hit = causally::since(&v).contains(&w);
                    (hit, v, w)
                }))
            },
        },
        // ── Party ──────────────────────────────────────────────────────
        Op {
            name: "party_decode",
            prepare: |f| {
                let (a, b) = f.parties.clone()?;
                Some(Cell::new(a.len() + b.len(), move || {
                    (decode_party(&a), decode_party(&b))
                }))
            },
        },
        Op {
            name: "party_encode",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                Some(Cell::new(n, move || (a.encode(), a)))
            },
        },
        Op {
            name: "party_fork",
            prepare: |f| {
                let (mut a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                Some(Cell::new(n, move || {
                    let child = a.fork();
                    (a, child)
                }))
            },
        },
        Op {
            name: "party_join",
            prepare: |f| {
                let (mut a, b, n) = f.party_pair()?;
                Some(Cell::new(n, move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "party_covers",
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                Some(Cell::new(n, move || (a.covers(&b), a, b)))
            },
        },
        Op {
            name: "party_disjoint",
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                Some(Cell::new(n, move || (a.is_disjoint(&b), a, b)))
            },
        },
        Op {
            name: "party_without",
            prepare: |f| {
                let (_, b, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(_, b)| b.len())?;
                Some(Cell::new(n + 1, move || (Party::seed().without(&b), b)))
            },
        },
        Op {
            name: "party_display",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                Some(Cell::new(n, move || (a.to_string(), a)))
            },
        },
        Op {
            name: "party_from_str",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let s = a.to_string();
                Some(Cell::new(s.len(), move || {
                    s.parse::<Party>().expect("a displayed party parses back")
                }))
            },
        },
        Op {
            name: "party_hash",
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                Some(Cell::new(n, move || {
                    let mut hasher = DefaultHasher::new();
                    a.hash(&mut hasher);
                    (hasher.finish(), a)
                }))
            },
        },
        // ── Clock ──────────────────────────────────────────────────────
        Op {
            name: "clock_decode",
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                Some(Cell::new(bytes.len(), move || {
                    Clock::decode(&bytes[..]).expect("an encoded clock decodes back")
                }))
            },
        },
        Op {
            name: "clock_encode",
            prepare: |f| {
                let (clock, n) = f.clock()?;
                Some(Cell::new(n, move || (clock.encode(), clock)))
            },
        },
        Op {
            name: "clock_tick",
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                Some(Cell::new(n, move || {
                    clock.tick();
                    clock
                }))
            },
        },
        Op {
            name: "clock_fork",
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                Some(Cell::new(n, move || {
                    let child = clock.fork();
                    (clock, child)
                }))
            },
        },
        Op {
            name: "clock_join",
            prepare: |f| {
                let (mut a, b, n) = f.clock_pair()?;
                Some(Cell::new(n, move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "clock_sync",
            prepare: |f| {
                let (mut a, mut b, n) = f.clock_pair()?;
                Some(Cell::new(n, move || {
                    let synced = a.sync(&mut b).is_ok();
                    (synced, a, b)
                }))
            },
        },
        Op {
            name: "clock_recv",
            prepare: |f| match f.kind {
                // Adversarial party × small received version.
                FamilyKind::IdPair => {
                    let (a, _, _) = f.party_pair()?;
                    let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                    let mut clock = Clock::from_parts(a, Version::new());
                    let msg = Version::try_from(1u64).expect("a one-tick version is valid");
                    Some(Cell::new(n + 2, move || {
                        clock.recv(&msg);
                        (clock, msg)
                    }))
                }
                // Small clock × adversarial received version.
                _ => {
                    let (v, n) = f.version()?;
                    let mut clock = Clock::seed();
                    Some(Cell::new(n + 2, move || {
                        clock.recv(&v);
                        (clock, v)
                    }))
                }
            },
        },
        Op {
            name: "clock_own_version",
            prepare: |f| {
                let (clock, n) = f.clock()?;
                Some(Cell::new(n, move || (clock.own_version(), clock)))
            },
        },
        Op {
            name: "clock_display",
            prepare: |f| {
                let (clock, n) = f.clock()?;
                Some(Cell::new(n, move || (clock.to_string(), clock)))
            },
        },
        Op {
            name: "clock_from_str",
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let s = clock.to_string();
                Some(Cell::new(s.len(), move || {
                    s.parse::<Clock>().expect("a displayed clock parses back")
                }))
            },
        },
        Op {
            name: "clock_hash",
            prepare: |f| {
                let (clock, n) = f.clock()?;
                Some(Cell::new(n, move || {
                    let mut hasher = DefaultHasher::new();
                    clock.hash(&mut hasher);
                    (hasher.finish(), clock)
                }))
            },
        },
    ]
}

// ─── measurement ────────────────────────────────────────────────────────────

/// One measured run of a cell body: every meter, plus wall time.
struct Sample {
    input_bytes: usize,
    peak_heap: usize,
    segments: u64,
    limb: Option<u64>,
    wall: Duration,
}

/// Run one prepared cell under all meters.
fn measure(heap: &HeapMeter, cell: Cell) -> Sample {
    super::reset_stack_segments();
    reset_limb();
    (heap.reset_peak)();
    let baseline = (heap.current)();
    let start = Instant::now();
    let result = (cell.body)();
    let wall = start.elapsed();
    let peak_heap = (heap.peak)().saturating_sub(baseline);
    let segments = super::stack_segments();
    let limb = read_limb();
    drop(result);
    Sample {
        input_bytes: cell.input_bytes,
        peak_heap,
        segments,
        limb,
        wall,
    }
}

/// Reset the limb counter when the `limb-meter` feature carries one.
#[cfg(feature = "limb-meter")]
fn reset_limb() {
    super::reset_limb_ops();
}

/// Without the `limb-meter` feature there is no counter to reset.
#[cfg(not(feature = "limb-meter"))]
fn reset_limb() {}

/// Read the limb counter, or `None` without the `limb-meter` feature.
#[cfg(feature = "limb-meter")]
fn read_limb() -> Option<u64> {
    Some(super::limb_ops())
}

/// Without the `limb-meter` feature the limb column is absent.
#[cfg(not(feature = "limb-meter"))]
fn read_limb() -> Option<u64> {
    None
}

/// The scaling exponent `log(m2/m1) / log(n2/n1)`, clamped finite.
///
/// A meter that reads zero at both scales scores 0; a zero at one scale is
/// clamped through `max(m, 1)` so the ratio stays defined. Degenerate input
/// sizes (`n2 <= n1`, possible only at extreme scale-down) score 0 rather
/// than dividing by a vanishing log.
fn exponent(m1: u64, m2: u64, n1: usize, n2: usize) -> f64 {
    if (m1 == 0 && m2 == 0) || n2 <= n1 {
        return 0.0;
    }
    let growth = (m2.max(1) as f64) / (m1.max(1) as f64);
    growth.ln() / ((n2 as f64) / (n1 as f64)).ln()
}

/// One evaluated cell: both samples, derived scores, and the verdict.
struct CellResult {
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
    heap_exp: f64,
    heap_per_byte: f64,
    seg_exp: f64,
    limb_exp: Option<f64>,
    limb_per_byte: Option<f64>,
    /// The meters over their bounds; empty means green.
    red: Vec<&'static str>,
}

/// Score a cell's two samples against the exponent bound and the ceilings.
fn evaluate(op: &'static str, family: &'static str, s1: Sample, s2: Sample) -> CellResult {
    let heap_exp = exponent(
        s1.peak_heap as u64,
        s2.peak_heap as u64,
        s1.input_bytes,
        s2.input_bytes,
    );
    let heap_per_byte =
        s2.peak_heap.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES) as f64 / s2.input_bytes as f64;
    let seg_exp = exponent(s1.segments, s2.segments, s1.input_bytes, s2.input_bytes);
    let (limb_exp, limb_per_byte) = match (s1.limb, s2.limb) {
        (Some(l1), Some(l2)) => (
            Some(exponent(l1, l2, s1.input_bytes, s2.input_bytes)),
            Some(l2 as f64 / s2.input_bytes as f64),
        ),
        _ => (None, None),
    };

    let mut red = Vec::new();
    if heap_exp > MAX_SCALING_EXPONENT {
        red.push("heap exponent");
    }
    if heap_per_byte > MAX_HEAP_BYTES_PER_INPUT_BYTE {
        red.push("heap constant");
    }
    if seg_exp > MAX_SCALING_EXPONENT {
        red.push("segments exponent");
    }
    if s2.segments > MAX_GROWN_STACK_SEGMENTS {
        red.push("segments count");
    }
    if limb_exp.is_some_and(|e| e > MAX_SCALING_EXPONENT) {
        red.push("limb exponent");
    }
    if limb_per_byte.is_some_and(|c| c > MAX_LIMB_OPS_PER_INPUT_BYTE) {
        red.push("limb constant");
    }

    CellResult {
        op,
        family,
        s1,
        s2,
        heap_exp,
        heap_per_byte,
        seg_exp,
        limb_exp,
        limb_per_byte,
        red,
    }
}

// ─── rendering ──────────────────────────────────────────────────────────────

/// Format a wall-time reading compactly in milliseconds.
fn wall(d: Duration) -> String {
    format!("{:.2}ms", d.as_secs_f64() * 1e3)
}

/// Render one result row.
fn row(out: &mut dyn Write, r: &CellResult) -> io::Result<()> {
    let verdict = if r.red.is_empty() { "GREEN" } else { "RED" };
    let limb = match (r.limb_exp, r.limb_per_byte) {
        (Some(e), Some(c)) => format!("limb[e{e:5.2} {c:>10.1}/B]"),
        _ => "limb[      off      ]".to_string(),
    };
    let reasons = if r.red.is_empty() {
        String::new()
    } else {
        format!("  <- {}", r.red.join(", "))
    };
    writeln!(
        out,
        "{verdict:<5} {op:<24} {family:<8} {n1:>8}->{n2:<8} B  \
         heap[e{he:5.2} {hc:>10.1}/B]  seg[e{se:5.2} {sc:>4}]  {limb}  \
         wall {w1:>9}->{w2:<9}{reasons}",
        op = r.op,
        family = r.family,
        n1 = r.s1.input_bytes,
        n2 = r.s2.input_bytes,
        he = r.heap_exp,
        hc = r.heap_per_byte,
        se = r.seg_exp,
        sc = r.s2.segments,
        w1 = wall(r.s1.wall),
        w2 = wall(r.s2.wall),
    )
}

/// Run the whole board and render the matrix to `out`.
///
/// `scale` multiplies every family's base size (1.0 is the seconds-scale
/// default; the smoke test passes a small fraction). Cells run at the scaled
/// size and its double. Red rows print first.
///
/// # Panics
///
/// Panics if `scale` is not strictly positive.
pub fn run(scale: f64, heap: &HeapMeter, out: &mut dyn Write) -> io::Result<Summary> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "amp-board: scale must be a positive finite number"
    );

    let families: Vec<(FamilyData, FamilyData)> = FAMILIES
        .iter()
        .map(|&kind| {
            (
                FamilyData::build(kind, scale, 0),
                FamilyData::build(kind, scale, 1),
            )
        })
        .collect();

    let mut results = Vec::new();
    for op in ops() {
        for (small, large) in &families {
            let Some(c1) = (op.prepare)(small) else {
                continue;
            };
            let c2 = (op.prepare)(large)
                .expect("a cell's applicability depends on the family, never the size");
            let s1 = measure(heap, c1);
            let s2 = measure(heap, c2);
            results.push(evaluate(op.name, small.name, s1, s2));
        }
    }

    writeln!(
        out,
        "amplification board: transient cost vs packed input bytes, each cell at two scales"
    )?;
    writeln!(
        out,
        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT} and constants within: \
         heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \
         segments <= {MAX_GROWN_STACK_SEGMENTS}, \
         limb <= {MAX_LIMB_OPS_PER_INPUT_BYTE} ops/B; wall time shown, never judged"
    )?;
    writeln!(out)?;

    let red: Vec<&CellResult> = results.iter().filter(|r| !r.red.is_empty()).collect();
    let green: Vec<&CellResult> = results.iter().filter(|r| r.red.is_empty()).collect();
    for r in red.iter().chain(green.iter()) {
        row(out, r)?;
    }

    writeln!(out)?;
    writeln!(
        out,
        "amp-board: {} green / {} red ({} cells)",
        green.len(),
        red.len(),
        results.len()
    )?;
    Ok(Summary {
        green: green.len(),
        red: red.len(),
    })
}
