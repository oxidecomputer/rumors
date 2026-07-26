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
//! # The product over three axes
//!
//! The board is a generalized cartesian product over three declarative
//! axes, and coverage holds structurally rather than by per-cell wiring:
//!
//! - **Shapes** (the family list): each shape builds an *operand bundle* —
//!   a version, a disjoint party pair, a designated event × id cross, a
//!   fold population — and a uniform post-pass derives the rest (a
//!   cross shape's version is its event side; its id side becomes a
//!   party pair through the disjoint-mount adapter; every version gains
//!   its ticked counterpart and rank pair). A shape reaches every
//!   operation its bundle supplies: adding a shape grows the product
//!   without naming any operation.
//! - **Operations** (the row table): each row declares the bundle slots
//!   its signature consumes and prepares its cell from them alone —
//!   never from the shape's identity — so adding an operation grows the
//!   product across every shape that supplies its operands.
//! - **Currencies** ([`ByCurrency`]): every judged quantity — the
//!   declarations, the readings, the scores — carries one field per
//!   metering currency, and the judgment iterates the axis itself.
//!   Adding a currency is a compile error at every declaration site
//!   until each operation answers its floor-or-NA question ([`Floors`]),
//!   so a meter can never be half-wired onto the board.
//!
//! The deterministic board always runs the whole product (the smoke test
//! pins the count); the wall-clock mirror selects with [`BenchMode`].
//!
//! # The criterion
//!
//! Each cell runs its operation at two input scales (the second twice the
//! first) and reads four deterministic meters over the body alone:
//!
//! - **peak heap bytes**, from a caller-installed counting allocator
//!   (see [`HeapMeter`]);
//! - **grown stack segments** ([`stack_segments`](crate::meter::stack_segments)),
//!   the honest stand-in for recursion-driven stack cost, which bypasses any
//!   heap meter;
//! - **big-integer limb operations**
//!   ([`limb_ops`](crate::meter::limb_ops)), only when the `limb-meter`
//!   feature compiles the counter into the arithmetic; arithmetic-width
//!   blowups are invisible to the other two meters;
//! - **packed-stream scan bits**
//!   ([`scan_bits`](crate::meter::scan_bits)), only when the `scan-meter`
//!   feature compiles the counter into the stream primitives; traversal
//!   work over the packed forms — the id-side walks and folds above all —
//!   allocates nothing, recurses nothing, and does no `Base` arithmetic, so
//!   this is the one column that sees it.
//!
//! Per meter the board derives a **scaling exponent**
//! `log(m₂/m₁) / log(n₂/n₁)` (`n` = the cell's denominator bytes, below —
//! every exponent, on every column) and a **per-denominator-byte constant**
//! at the larger scale (the one constant not per denominator byte: the text
//! rows' limb constant is per `R` unit, below). A cell is
//! **GREEN** iff every meter's exponent is at most [`MAX_SCALING_EXPONENT`],
//! every constant is under its pinned ceiling
//! ([`MAX_HEAP_BYTES_PER_INPUT_BYTE`] over [`HEAP_FLAT_ALLOWANCE_BYTES`],
//! [`MAX_GROWN_STACK_SEGMENTS`], [`MAX_LIMB_OPS_PER_INPUT_BYTE`],
//! [`MAX_SCAN_BITS_PER_INPUT_BYTE`] — or, on
//! the text rows' limb column, [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`]), and
//! every committed liveness floor is met; **RED** otherwise, with the
//! offending meters named.
//!
//! # Liveness floors
//!
//! A ceiling over a counter proves the *instrumented* work is small, not
//! that the operation is cheap: three of the four judged columns are sensors
//! inside the implementation, and an implementation change can re-route work
//! around them, leaving the ceiling green over a counter that reads nothing.
//! Every cell therefore carries, per judged column, a [`Liveness`]
//! declaration the type demands at construction: either a **floor** — the
//! least the counter must read if the meter is watching the work, derived
//! from what the operation must do, never from how it does it — or an
//! explicit **not-applicable** with the reason no floor can bind. Floors
//! bind in the same pass the ceilings do, at both scales; a counter reading
//! below its floor is red, named as the column's floor with the vacuity
//! mechanism (the meter is not watching that work). The declarations render
//! per cell (`flr[...]`) and their derivations print as a legend above the
//! matrix. The conventions:
//!
//! - **Scan** is the universal leg: an operation that must examine its
//!   packed operands scans at least
//!   [`SCAN_FLOOR_BITS_PER_INPUT_BYTE`] bit per packed byte (an eighth of
//!   the stored bits); operations that may legitimately exit at the first
//!   divergence still read the root codes, floored at
//!   [`SCAN_TOUCH_FLOOR_BITS`]. Not-applicable is reserved for operations
//!   whose contract is a wholesale byte move or compare (encode, hash,
//!   same-form equality) or whose operands have no packed stream at all
//!   (the rank pair).
//! - **Limb** floors bind where big-integer arithmetic is semantically
//!   mandatory: an operation that must materialize or fold a magnitude
//!   wider than [`MACHINE_WORD_MAGNITUDE_BITS`] touches at least one limb
//!   per 64 bits of that magnitude. Narrow-magnitude cells are
//!   not-applicable (machine words suffice), as are operations whose
//!   contract forces no arithmetic at all.
//! - **Heap** floors bind on the codec and text rows, whose results must
//!   materialize at least their packed bytes; everywhere else allocation is
//!   not semantically forced (and the heap meter reads the process
//!   allocator, which no re-routing inside the crate can bypass).
//! - **Segments** is ceiling-only by policy: the target is walks that never
//!   grow the stack, so its honest floor is zero and a zero floor asserts
//!   nothing.
//!
//! A floor trip is a designed stop-and-look: an implementation that
//! legitimately does less work lowers the floor deliberately, in a change
//! whose diff shows the new derivation.
//!
//! Floors detect *total* vacuity, not partial rerouting: the scan floor is
//! deliberately an eighth of an honest walk's reading, so an implementation
//! that routes exactly the floor through metered primitives and the rest
//! around them still reads green. That is the derivation rule's designed
//! limit — a floor states what the operation *must* do, and partial
//! rerouting still does it — so the floors are a bypass tripwire, never a
//! full-liveness proof; the leg that bounds work no counter sees is the
//! time exponent judged over the bench suite (below).
//!
//! Four cells are watched by neither leg, an exposure accepted here so it
//! is stated rather than silent: `version_hash`, `party_hash`,
//! `clock_hash`, and `version_eq` on the benign family. Hashing folds the
//! stored canonical bytes wholesale, and same-form equality compares them
//! wholesale, below every metered primitive — no stream walk, no forced
//! arithmetic, no forced allocation — so every floor column is honestly
//! not-applicable, and the benign operands are small enough (a few hundred
//! packed bytes across both scales) that the body never reaches the bench
//! judge's 10 µs judgment floor. The exposure is bounded by exactly those
//! two facts: sub-10 µs of word arithmetic per call over a
//! few-hundred-byte operand, with the same rows under the time leg on
//! every larger family. `version_eq`'s exposure differs from the hash
//! rows' in one respect its NA reason states on the board face: eq
//! operands grow without bound, so the time leg — under its own sub-floor
//! discipline — is the one backstop that the compare stays linear.
//!
//! # Determinism and the time leg
//!
//! Every quantity the board judges or renders is a deterministic counter,
//! so two board runs at the same scale are byte-identical under any
//! machine load: the board reads no clock, conditions nothing on timing,
//! and comparing runs needs no exclusion rules. The claim is enforced,
//! not assumed, on two legs: the runner itself measures every cell twice
//! in process and panics on any counter or denominator disagreement
//! ([`run`]'s self-verification), and the `amp-board-determinism` recipe
//! byte-compares two whole renders across processes. Time still has its own
//! judged leg — wall time is the one implementation-agnostic witness for
//! *time*, exactly as heap is for space: a kernel doing quadratic work in
//! plain machine-word arithmetic (no allocation, no recursion, no metered
//! reads) is invisible to all four counters and visible only to a clock —
//! but the clock lives where timing discipline does. The bench judge
//! (`tools/benchjudge`, `just bench-judge`) fits each cell's time exponent
//! across two scales of the board benches' criterion medians (warmup,
//! sampling, and outlier rejection are criterion's), denominated against
//! the same per-cell bytes as the board's own exponents
//! ([`BenchCell::denominator_bytes`]), and holds every judged cell to a
//! ceiling generous to scheduler noise and impassable for a quadratic's
//! ~2.0.
//!
//! # Acceptance scales
//!
//! Every cell runs at a size scale; the inner loop uses the default
//! (scale 1, seconds of runtime). Segment counts have a ~1 MiB growth
//! threshold, so recursion-frame amplifiers whose onset sits above the
//! default depths read false green there — **campaign acceptance is
//! therefore all cells green at BOTH the default scale and the record
//! scale [`RECORD_SCALE`] (`just amp-board-record`), three identical runs
//! each, plus the bench judge green across the same two scales**. Record
//! runs are acceptance-time only; the enforced
//! per-operation record remains the process-isolated envelope suite in
//! `tests/meter.rs`.
//!
//! # Denomination
//!
//! Most cells charge cost against **packed input bytes** alone. Two cell
//! classes have *mandatory* output asymptotically larger than any constant
//! times their input, so an input-only ceiling on them is unsatisfiable by
//! construction — and an unsatisfiable criterion degenerates into exemption
//! holes. Those cells are denominated against **total I/O bytes** `n_io`,
//! with the output side read back from the operation's actual result, never
//! assumed from its inputs:
//!
//! - **Text rows** (`Display` and `FromStr` for all three types): `n_io` is
//!   packed input + text output (`Display`) or text input + packed output
//!   (`FromStr`). Their heap and segment columns are judged per `n_io` byte
//!   at the unchanged ceilings. Their **limb column** is judged on two legs
//!   that exclude different converters. The *exponent* leg — against `n_io`,
//!   like every exponent — is what excludes quadratic conversion: any
//!   schoolbook converter's limb work is `Θ(digits × limbs)`, quadratic in
//!   the value bits however wide its chunks, and reads ~2 there \[measured\].
//!   The *constant* leg — against the radix-work denominator
//!   `R = n_io + Σ digitsᵢ × limbsᵢ` over the values the text spells, the
//!   schoolbook cost law itself, at the divide-and-conquer target
//!   [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`] — is what excludes a wasteful
//!   constant: a digit-by-digit schoolbook probe scores ~1 limb per `R`
//!   unit, ~4× over it \[measured — the test suite's tripwire\]. Only a
//!   converter whose recorded limb work is near-linear in `n_io` with a
//!   D&C-class constant reads green.
//! - **Output-dominated projection** (`version_project` and
//!   `clock_own_version` on the comb × scattered-party cross): `n_io` is
//!   packed input + packed output. The cross exists because a scattered
//!   party keeps a wide magnitude per kept tooth — `Θ(e·k)` mandatory output
//!   bits — and a packed output cannot be padded, so `n_io` is the honest
//!   denominator on all three columns at the unchanged ceilings.
//!
//! The **output-honesty assertion** closes the pad-the-output door on the
//! text side: any text stream entering a denominator must satisfy
//! `text_bytes ≤` [`TEXT_BYTES_PER_RADIX_UNIT`] `× radix units` of the
//! values it spells, checked against the actual bytes.
//!
//! **Do not re-denominate** (these stay input-denominated): both binary
//! codec directions (the coding is canonical 1:1, so input bytes are the
//! honest bound); every scalar, comparison, and query row (word-sized or
//! borrowed results); and the packed-output mutator rows (`join`, `meet`,
//! `tick`, `batch_snapshot`, `fork`, `recv`, `sync`, `without`, and every
//! projection cell outside the output-domination cross) — their input denomination rests on output
//! coding ≤ inputs + O(1) per overlay boundary, which is pinned for
//! join/meet as the 1-Lipschitz proptest in
//! [`tier2`](crate::meter::tier2)'s test suite rather than assumed.
//!
//! **Rank operands** (`rank_pair_ops`, `rank_sum`) have no packed encoding
//! to charge against; their denominator of record is the operands' **value
//! content** `bits(num) + exp` in bytes. That content is wire-bounded:
//! every public construction path (the `rank`/`distance`/`lag` folds)
//! emits a rank whose numerator width and exponent are each linear in the
//! packed bits the fold read, so a ceiling per content byte is a ceiling
//! per wire byte up to the fold's own constant.
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
//! Every shape from [`meter`](crate::meter) reaches every operation its
//! operand bundle supplies (the product section above): the event shapes —
//! the dense spine, `bigroot`, `hugeleaf`, the boundary comb (`cliff`, at
//! `k = n` so its value content grows quadratically in its packed input),
//! and `harmonic` — carry a version; the diverted id-spine pair carries a
//! disjoint party pair; the eleven cross shapes (`comb-scatter` and the
//! ten tick-walk crosses) carry a version, a mounted party pair, and a
//! clock; `benign` — a fixed-seed pseudo-random population of forked,
//! ticked clocks, the control row that keeps the ceilings honest on
//! organic inputs — carries everything. Where an operation needs a
//! `Party` and a `Version`, the board crosses adversarial party × small
//! version, small party × adversarial version, and — on the cross
//! shapes — the designated adversarial × adversarial pairing.
//!
//! Three shapes carry a genre note beyond their variant docs:
//!
//! - `comb-scatter`: the projection cross (boundary-comb version ×
//!   scattered party) whose mandatory output dominates its input — the
//!   case the small-operand crosses cannot exhibit; its two projection
//!   cells are the board's only I/O-denominated non-text cells.
//! - `harmonic` (`meter::harmonic`, a 1-leaf at every depth), built
//!   against the linear-functional rows (`rank`/`distance`/`lag`/
//!   `min_ticks`) and the rank rows (`rank_pair_ops`, `rank_sum`): its
//!   rank's numerator is as
//!   wide as the depth already walked at every level, so a fold that
//!   re-shifts its accumulated numerator per level reads limb exponent ~2
//!   here while `dense` (a one-bit numerator) stays the linear control.
//!   The query kernels' rank fold telescopes through height deltas, and
//!   `version_rank × harmonic` reads the control's linear signature
//!   \[measured — limb exponent 1.00, constant within 2% of `dense`, both
//!   scales\]: the column is the tripwire that goes red under the
//!   re-shifting genre.
//! - `scatter`, whose bundle carries fold operands alone, for the two
//!   fold rows (`version_join_all`,
//!   `party_join_all`; both also keep a `benign` control cell, folding the
//!   organic population in construction order): balanced-forked
//!   single-tick operands ordered evens before odds, so a sequential
//!   fold's accumulator holds every other leaf and never coalesces — the
//!   shape that reads exponent ~2 under a left fold. Both rows run the
//!   balanced binary-counter reduction (every input passes through
//!   O(log n) joins), and what the cells show is its log factor — on the
//!   version fold's limb and scan columns, and on the party fold's scan
//!   column alone (its walk allocates nothing, recurses nothing, and does
//!   no arithmetic, so scan is the only deterministic meter that sees
//!   it): exponents ~1.1 and constants that grow with scale, marginally
//!   over the amortized-linear bounds at some scales \[measured — both
//!   scales\]. The `benign` controls read the same
//!   signature as `scatter`, so the marginal red is the reduction's own
//!   n·log n cost, not the adversarial ordering's.
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
//!   `Batch`'s operator matrix (`|`, `&`, and their assign forms, over
//!   every borrow shape) routes through the same `join_view`/`meet_view`
//!   emitters and cmp walk the `join`/`meet`/`cmp` rows measure — the
//!   batch is a decode-once wrapper, not a second implementation;
//!   `Clock::send` is `Clock::tick` by definition; `clock | version` and
//!   `clock |= version` fold through the same `join_version` the `recv` row
//!   measures; `Clock::batch` operations are what the clock rows already
//!   route through; `Party::tick` is the mirror of `Version::tick` (the
//!   `tick_adv_party` row); `Debug` for all three types delegates to
//!   `Display`.
//! - **Folds over measured operations**: `Version::join_all` has its own
//!   row (the `scatter` cell plus the `benign` control), and
//!   `Version::Sum`/`FromIterator` are that
//!   fold by definition; `Party::join_all` likewise (the party fold's
//!   cells); `Clock::join_all` is the party fold and the version
//!   fold run side by side, so the two fold rows price both of its
//!   halves. `Version::meet_all` stays NA by mechanism: meet only shrinks,
//!   so its running accumulator is bounded by the *smaller* operand at
//!   every step — the fold does at most `Σ min(|acc|, |vᵢ|)` work and
//!   cannot exhibit the growing-accumulator genre the join folds do.
//!   `Party::forks`/`Clock::forks` iterate the measured `fork`, each step
//!   on the freshly-split half (shrinking operands, same argument); a
//!   `Forks` iterator dropped mid-run rejoins its unclaimed remainder in
//!   one `join` per remaining level of the fork tree — O(log n) of the
//!   measured `join` on shrinking operands.
//! - **Bounded or trivial inputs**: `Version::new`/`Default`,
//!   `TryFrom<u64>`/tuple literals (word-sized literals),
//!   `Party::seed`/`is_seed`, `TryFrom<u8>`/`TryFrom<bool>`,
//!   `Clock::seed`/`TryFrom<(I, E)>`.
//! - **Moves, borrows, and byte copies**: `is_empty`, `as_bytes`,
//!   `encoded_bits`, `encode_to` (the `encode` row's path into a writer),
//!   `dangerously_alias` (a byte copy), `Clock::from_parts`/`into_parts`,
//!   `Clock::party`/`version`, `Version::batch`,
//!   `Ranked::rank`/`version`/`into_parts` (borrows and moves); `Clone`
//!   (`Version`, `Rank`, `Ranked`) copies the stored bits or value content
//!   wholesale, with no walk or arithmetic in the contract; `Party`'s and
//!   `Clock`'s derived `PartialEq`/`Eq` are one bit-slice compare of the
//!   stored canonical bits (`Version`'s `==` is the same wholesale compare
//!   — the stored coding is canonical, so byte equality is causal equality
//!   and no walk is in the contract; it keeps the `version_eq` row because
//!   its operands grow without bound, the row's time leg holding the
//!   compare linear); the consuming array splits
//!   (`From<Party> for [Party; N]`, `From<Clock> for [Clock; N]`) are the
//!   `forks` machinery above plus `N` moves.
//! - **Derived pairings**: `Ranked::from` is the `rank` row plus a move; its
//!   comparisons are `Rank` comparisons plus byte equality; `Rank::cmp`,
//!   `checked_sub`, and `+` have their own row (`rank_pair_ops`, on the
//!   mismatched-exponent pair, value-content-denominated per the
//!   Denomination section); `Rank::AddAssign` is `+` in place, and the
//!   `Sum` fold has its own row (`rank_sum`, the mixed high-first
//!   population, denominated the same way); `Rank`'s `Display` (its `Debug` delegates)
//!   prints the `rank` row's output — a derived value with no packed
//!   encoding to normalize against, rendered by the same per-`Base` decimal
//!   print the `version_display` row drives.
//! - **The same comparisons under another name**: `causally`'s other
//!   constructors, `Range::placement_of`, and `Range`'s refinement methods
//!   perform the identical causal comparisons the `causally_contains` row
//!   measures; `Range`'s bound accessors (including its `RangeBounds`
//!   view) are borrows.
//! - **Wrappers**: the `serde`/`borsh` impls serialize as the canonical
//!   encoding and deserialize through the strict decoder — the
//!   `encode`/`decode` rows.
//! - **Test support**: `oracle` and the `error`/`iter` modules' data types
//!   perform no computation over packed inputs; `meter`'s own surface —
//!   the generators, the counters, this board — is the measurement
//!   instrument itself, feature-gated out of production builds. The
//!   `skyline`/`accum` kernels `meter` re-exports are the implementation
//!   under every public operation, public only so the envelope suite can
//!   pin their internals: every cell of this board already times them at
//!   the public boundary, their resources are pinned by the envelope
//!   scenarios in `tests/meter.rs`, and their agreement with the
//!   recursive oracle is pinned by their differential suites.

mod currency;
#[cfg(test)]
mod tests;

pub use currency::{ByCurrency, Currency, Floors, Liveness};

use std::any::Any;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::rc::Rc;

use crate::codec::{self, Base};
use crate::{causally, Clock, Party, Rank, Version};

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

/// Green requires at most this many packed-stream scan bits per denominator
/// byte (asserted only when the `scan-meter` feature is lit).
///
/// Calibrated against the benign control and the green adversarial
/// families: a single walk over packed operands scans ~8 bits per byte,
/// multi-walk operations (`distance` runs join, meet, and two rank folds;
/// `sync` joins in both directions) scan a small multiple, and the text
/// parsers re-scan their packed output through the strict validator. The
/// worst honest reader measured is well under 64; the ceiling sits at 96 so
/// only a walk that re-scans state growing with the input — the fold genre —
/// goes red on this column.
pub const MAX_SCAN_BITS_PER_INPUT_BYTE: f64 = 96.0;

/// Scan liveness floor: an operation that must examine its packed operands
/// scans at least this many bits per packed input byte.
///
/// One bit per byte is an eighth of the stored bits: far below any honest
/// full walk (measured ~8 bits per byte across the board), and far above a
/// counter that has stopped watching (which reads ~0).
pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

/// Scan liveness floor for legitimate early-exit operations: even an
/// immediate divergence answer reads the operands' root codes.
pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;

/// Magnitudes at most this wide may legitimately be handled in machine
/// words; wider values force big-integer arithmetic, so limb floors bind
/// only on them.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// Text rows only: green requires at most this many limb operations per
/// radix-work unit `R = n_io + Σ digitsᵢ × limbsᵢ` (κ).
///
/// κ carries the text limb column's *constant* leg only; the *exponent* leg
/// is judged against `n_io`, never against `R` — `R` is the schoolbook cost
/// law itself, so an exponent against it reads a flat ~1 on exactly the
/// quadratic converters the bound exists to catch. The legs exclude
/// different converters, and a constant ceiling cannot enforce a complexity
/// class: a `u32`-chunked schoolbook converter scores ~0.11 limb per `R`
/// unit — under κ, and wider chunks only lower it — while its limb work
/// stays quadratic in the value bits and reads exponent ~2 against `n_io`
/// \[measured — the chunked tripwire in the test suite\]; the exponent leg
/// is what excludes it. What κ excludes is a wasteful constant: it sits 4×
/// under a digit-by-digit schoolbook probe's measured ~1 limb per `R` unit,
/// and the production parser — radix conversion delegated to the backend's
/// divide-and-conquer parser, one width-proportional limb record per
/// materialized value — reads far under it on the conversion-dominated
/// families \[measured — the delegating-parser pin in the test suite\].
/// The organic and dense `FromStr` cells still read over κ on per-value
/// gamma-encode arithmetic (an honest per-node cost `R` under-weights on
/// small values); those cells' owner re-derives κ against the skyline text
/// kernels' observed meter at record scale before any envelope enforces
/// it. The test suite pins three legs — the schoolbook probe exceeds κ;
/// the delegating parser stays under κ over a liveness floor; the chunked
/// probe slips under κ and trips the exponent leg — so none can silently
/// soften.
pub const MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT: f64 = 0.25;

/// Any text stream entering a denominator must hold at most this many bytes
/// per radix unit of the values it spells (the output-honesty ceiling).
///
/// Denominating against I/O bytes opens a door: pad the output, inflate the
/// denominator, read green. The ceiling closes it \[derived\], and its
/// basis is the radix-unit sum (`Σ digits × limbs`, computed from the
/// values outside the render) rather than wire bits, because the skyline
/// wire coding spends O(1) bits on a leaf whose spelled value is wide —
/// no constant per wire bit bounds honest text. Per rendered value the
/// grammar spends its exact decimal digits (`digits ≤ digits × limbs`, one
/// radix unit minimum per value) plus at most 6 syntax bytes (`(`, `)`,
/// and two `, ` separators), and every value contributes at least one
/// radix unit — so honest text stays under 7 bytes per radix unit and
/// padding trips the assertion.
pub const TEXT_BYTES_PER_RADIX_UNIT: f64 = 8.0;

/// The acceptance scale of record: the size multiplier of the record-mode
/// board run (`just amp-board-record`).
///
/// The default-scale board under-detects segment amplifiers: stacker grows
/// a segment only past ~1 MiB of frames, so a recursion-frame amplifier
/// whose onset sits above the default depths reads a false green there.
/// ×4 is the witnessed calibration floor — the scale at which every known
/// segment-onset amplifier read red under pre-fix code — so acceptance runs
/// pin it. **Campaign acceptance is all cells green at BOTH the default
/// scale and this one, three identical runs each**; a record run is
/// acceptance-time only (the inner loop stays at the default scale, and the
/// enforced per-operation record remains the envelope suite in
/// `tests/meter.rs` regardless of board onset).
pub const RECORD_SCALE: f64 = 4.0;

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

/// Boundary-comb tooth magnitude (bits) and tooth count at scale 1.0
/// (packed size ~4 KiB); one parameter drives both, mirroring the meter
/// suite's `k = n` convention.
///
/// Scaling `k` with `n` is the separating choice: it keeps the comb's
/// absolute value content growing quadratically in the packed input, so a
/// sweep that materializes running leaf values in a plain big integer
/// reads a superlinear exponent here instead of hiding a `k`-sized
/// constant under a fixed magnitude.
const CLIFF_BASE_SCALE: usize = 128;

/// Comb-scatter tooth count at scale 1.0 (packed cross ~32 KiB).
///
/// Scale drives the tooth count (and with it the scattered party's fragment
/// count, half the teeth); the tooth magnitude stays at
/// [`CROSS_TOOTH_MAGNITUDE_BITS`], so the operands grow linearly and the
/// output-domination ratio holds at every scale.
const CROSS_BASE_TEETH: usize = 128;

/// Comb-scatter tooth magnitude in bits (fixed across scales).
const CROSS_TOOTH_MAGNITUDE_BITS: usize = 1_000;

/// Harmonic spine depth at scale 1.0 (packed size ~6 KiB, matching the
/// dense spine's depth).
const HARMONIC_BASE_DEPTH: usize = 8_000;

/// Scatter population at scale 1.0: balanced-forked parties, one tick
/// each (~10 KiB of packed single-tick versions).
const SCATTER_BASE_CLOCKS: usize = 1_024;

/// Nested-full-sibling depth at scale 1.0 (packed pair ~1.5 KiB).
///
/// Deep enough that a per-level re-scan genre reads its exponent
/// across the level doubling, small enough that the quadratic pin
/// stays inside the board's runtime budget at the record scale.
const NESTED_BASE_DEPTH: usize = 1_500;

/// Nested-wide depth and root-magnitude bits at scale 1.0 (equal, so
/// the doubling scales width and depth together — the cross's cost
/// genre is their product; packed pair ~1.5 KiB).
///
/// Small enough that even a width × depth kernel stays inside the
/// record-scale runtime budget; the red reading rides the exponent
/// leg, not the constant ceiling.
const NESTED_WIDE_BASE: usize = 1_000;

/// Mirror-wide depth and tail-magnitude bits at scale 1.0 (equal, as
/// above; packed pair ~1 KiB). The memo arm's chains grow steeper than
/// the right-full arm's, so the base sits lower.
const MIRROR_WIDE_BASE: usize = 500;

/// Mirror-narrow depth at scale 1.0 (packed pair ~1.5 KiB): the
/// nested-full base, mirrored — the memo machinery at the same depth
/// the right-full cells walk.
const MIRROR_NARROW_BASE_DEPTH: usize = 1_500;

/// Staircase depth at scale 1.0 (packed pair ~2 KiB): deep enough that
/// per-level minimum bookkeeping would read its exponent across the
/// doubling, all values word-scale.
const STAIRCASE_BASE_DEPTH: usize = 1_500;

/// Reveal-comb site count and plateau-magnitude bits at scale 1.0
/// (equal; packed pair ~1 KiB).
///
/// One parameter drives both, so the doubling scales the site count
/// and the circulated width together — the cycle's cost genre is
/// their product. The close-reveal cycle's per-site cost is steeper
/// than the mirror families' chains, so the base sits at the
/// mirror-wide level.
const REVEAL_COMB_BASE: usize = 500;

/// Pure-comb level count and leaf-magnitude bits at scale 1.0 (equal,
/// as above; packed pair ~1 KiB).
///
/// The base watermark stack's own cycle runs at ~2 wide folds per
/// level — a tenth of the reveal comb's constant — so the base sits
/// higher for comparable work.
const PURE_COMB_BASE: usize = 1_000;

/// Ascending-cliff spine length and leaf-magnitude bits at scale 1.0
/// (equal, so the doubling scales the hop count and the residue width
/// together — the cascade's cost genre is their product; packed pair
/// ~1 KiB).
///
/// The cascade runs at ~4 touches per input byte on the cured fold
/// direction — the leveled control's constant — so the base sits at
/// the pure-comb level for comparable work.
const ASCEND_CLIFF_BASE: usize = 1_000;

/// Ticks behind the integer (exponent-zero) rank of the `rank_pair_ops`
/// row: small, so the pair's cost is carried entirely by the mismatch.
const RANK_PAIR_INTEGER_TICKS: u64 = 3;

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

/// The input families, one column group of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamilyKind {
    /// The dense event spine `S(d)`: node count and depth maximizer.
    Dense,
    /// `bigroot(B, d)`: a huge root magnitude over a long spine.
    Bigroot,
    /// `hugeleaf(B)`: one node, maximal bits per node.
    Hugeleaf,
    /// The boundary comb `C(k, n)` at `k = n`: leaf values oscillating
    /// across a `2^k` carry cliff, every crossing paid by a stored code.
    Cliff,
    /// The diverted id-spine pair `I(d, ·)`: full-lockstep two-party walks.
    IdPair,
    /// The output-domination cross: boundary comb × scattered party.
    CombScatter,
    /// The harmonic spine `H(d)`: the rank fold's wide-numerator
    /// adversary, designed against the linear-functional rows and the
    /// rank pair.
    Harmonic,
    /// The scatter-ordered fold population: balanced-forked single-tick
    /// operands whose join accumulator never coalesces; its bundle
    /// carries fold operands alone, so only the fold rows apply.
    Scatter,
    /// The nested-full-sibling cross `N(d)` × the dense spine `S(d)`.
    ///
    /// Every level a right-full shortcut site, the deepest stacking of
    /// the walk's deferred right-full decisions and raise bookkeeping
    /// on narrow values — the designated cross of the two tick rows.
    NestedFull,
    /// The wide right-full cross: `bigroot(b, d)` × `N(d)`.
    ///
    /// The stream's first payload is coded absolute, so the deepest
    /// subtree's net movement carries the root's full magnitude and
    /// every level's bookkeeping meets it — width × depth through the
    /// right-full arm. The designated cross of the two tick rows.
    NestedWide,
    /// The wide left-full (memo) cross: `wide_tail(b, d)` × `M(d)`.
    ///
    /// Every proper subtree nets the tail's full magnitude while every
    /// level is a memoized pre-scan site — width × depth through the
    /// left-full arm and the pre-scan's own chains. The designated cross
    /// of the two tick rows.
    MirrorWide,
    /// The narrow left-full (memo) cross: `wide_tail(1, d)` × `M(d)`.
    ///
    /// The memoized pre-scan machinery itself, all values word-scale.
    /// The designated cross of the two tick rows.
    MirrorNarrow,
    /// The descending staircase `D(d)` × the unary id spine `I(d)`.
    ///
    /// Every consumed leaf undercuts every open range's minimum —
    /// full-penetration minimum updates at every level, all values
    /// word-scale. The designated cross of the two tick rows.
    Staircase,
    /// The reveal-comb cross: `reveal_comb(s, s)` × its own id.
    ///
    /// `s` sibling left-full sites share one `2^s`-wide minimum over a
    /// zero floor, and the left-leaning spine closes each site's frame
    /// back into the floor frame between consecutive consumes: the
    /// width-`s` boundary difference is minted at every consume and
    /// popped at every close — the unfunded width circulation, in the
    /// touch currency these columns do not carry (the gate pins in
    /// `tests/meter.rs` enforce it; the bench mirror's time leg sees
    /// it). The designated cross of the two tick rows.
    RevealComb,
    /// The reveal-comb control: `reveal_comb_hifloor(s, s)` × the
    /// reveal-comb id.
    ///
    /// Identical forest and close-reveal cycle with the floor raised
    /// to `2^s − 2`, so the circulated boundary difference is O(1)
    /// wide: the gap control. The designated cross of the two tick rows.
    RevealHifloor,
    /// The pure-comb cross: `pure_comb(s, s)` × its own id.
    ///
    /// The reveal comb's cycle with no left-full site anywhere — no
    /// memo, no pre-scan, no site consume: the base watermark stack's
    /// own arm-move + close-pop width circulation, isolated from the
    /// frame ledger. The designated cross of the two tick rows.
    PureComb,
    /// The ascending-cliff cross: `ascend_cliff(s, s)` × its own id.
    ///
    /// `s` ascending wide leaves stack `s − 1` nonzero unit boundary
    /// differences and a terminal 0-cliff drives one width-`s` undercut
    /// residue through all of them — the cascade whose per-hop fold
    /// direction the gate pins in `tests/meter.rs` price in the touch
    /// currency these columns do not carry. The designated cross of the
    /// two tick rows.
    AscendCliff,
    /// The ascending-cliff control: `ascend_cliff_plateau(s, s)` × the
    /// ascending-cliff id.
    ///
    /// Identical spine, arming schedule, and cliff undercut with every
    /// leaf leveled, so the difference stack is one compressed zero run
    /// the residue passes whole in O(1): the hop-schedule control.
    /// The designated cross of the two tick rows.
    AscendPlateau,
    /// The fixed-seed organic control population.
    Benign,
}

/// Every family, in display order.
const FAMILIES: [FamilyKind; 19] = [
    FamilyKind::Dense,
    FamilyKind::Bigroot,
    FamilyKind::Hugeleaf,
    FamilyKind::Cliff,
    FamilyKind::IdPair,
    FamilyKind::CombScatter,
    FamilyKind::Harmonic,
    FamilyKind::Scatter,
    FamilyKind::NestedFull,
    FamilyKind::NestedWide,
    FamilyKind::MirrorWide,
    FamilyKind::MirrorNarrow,
    FamilyKind::Staircase,
    FamilyKind::RevealComb,
    FamilyKind::RevealHifloor,
    FamilyKind::PureComb,
    FamilyKind::AscendCliff,
    FamilyKind::AscendPlateau,
    FamilyKind::Benign,
];

/// One shape instantiated at one scale: the operand bundle every row's
/// `prepare` decodes fresh (outside measurement).
///
/// The bundle is the shape axis's declaration: each slot a shape fills
/// flows to every operation whose signature consumes it, so a shape's
/// reach is structural — build a shape with a version and it appears on
/// every version row; give it an id side and it appears on every party
/// row through the disjoint-mount adapter. The derived slots (`version2`,
/// `rank_pair`, a cross shape's `version` and `parties`) are filled by
/// one uniform post-pass in [`FamilyData::build`], never per shape.
struct FamilyData {
    kind: FamilyKind,
    name: &'static str,
    /// The shape's primary packed version (a cross shape's event side).
    version: Option<Vec<u8>>,
    /// The comparison counterpart, derived uniformly: `version` plus one
    /// seed tick, packed.
    version2: Option<Vec<u8>>,
    /// A disjoint packed party pair within one universe: natural for the
    /// id pair and the benign halves, minted by the disjoint-mount
    /// adapter from a cross shape's id side.
    parties: Option<(Vec<u8>, Vec<u8>)>,
    /// The designated packed (event version, id party) cross: the pairing
    /// the shape was built around, driving the tick rows' walk floors and
    /// the clock rows' operand choice.
    ///
    /// Each cross shape's variant doc states the arm and cost genre its
    /// cross drives.
    cross: Option<(Vec<u8>, Vec<u8>)>,
    /// Whether the cross's mandatory projection output dominates its
    /// input (the comb-scatter shape): the projection rows I/O-denominate
    /// exactly these cells (the module doc's Denomination section).
    output_dominated: bool,
    /// The packed fold operands (versions, parties), consumed by the two
    /// fold rows alone: the scatter shape's adversarially ordered
    /// population and the benign shape's organic control.
    #[allow(clippy::type_complexity)]
    fold: Option<(Vec<Vec<u8>>, Vec<Vec<u8>>)>,
    /// The mismatched rank pair, derived from `version` in the post-pass.
    ///
    /// Precomputed here (shape-derived rank, small integer rank) so the
    /// `rank_pair_ops` and `rank_sum` prepares clone their operands instead
    /// of re-running the rank fold:
    /// the bench harness calls prepare once per timed iteration, and the
    /// fold costs orders of magnitude more than the pair operations it
    /// feeds.
    rank_pair: Option<(Rank, Rank)>,
}

impl FamilyData {
    /// A bundle with every slot empty, for a build arm to fill with what
    /// the shape honestly has.
    fn bare(kind: FamilyKind, name: &'static str) -> FamilyData {
        FamilyData {
            kind,
            name,
            version: None,
            version2: None,
            parties: None,
            cross: None,
            output_dominated: false,
            fold: None,
            rank_pair: None,
        }
    }

    /// Build a shape's operand bundle at `scale`, doubled `level` times.
    ///
    /// `level` 0 and 1 are the two measurement scales of every cell. The
    /// arm fills the slots the shape natively has; the post-pass below
    /// derives the rest uniformly (a cross shape's version is its event
    /// side, its party pair is the disjoint-mount adapter over its id
    /// side; every version gains its ticked counterpart and its rank
    /// pair), so a new shape reaches every operation its bundle supplies
    /// without naming any.
    fn build(kind: FamilyKind, scale: f64, level: u32) -> FamilyData {
        let size = |base: usize| -> usize {
            let scaled = ((base as f64) * scale).round() as usize;
            scaled.max(MIN_SIZE_PARAM) << level
        };
        let mut data = match kind {
            FamilyKind::Dense => Self::event(
                kind,
                "dense",
                super::dense(size(DENSE_BASE_DEPTH)).version().encode(),
            ),
            FamilyKind::Bigroot => Self::event(
                kind,
                "bigroot",
                super::bigroot(size(BIGROOT_BASE_MAGNITUDE_BITS), size(BIGROOT_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyKind::Hugeleaf => Self::event(
                kind,
                "hugeleaf",
                super::hugeleaf(size(HUGELEAF_BASE_MAGNITUDE_BITS))
                    .version()
                    .encode(),
            ),
            FamilyKind::Cliff => {
                let scale = size(CLIFF_BASE_SCALE);
                Self::event(
                    kind,
                    "cliff",
                    super::cliff_comb(scale, scale).version().encode(),
                )
            }
            FamilyKind::IdPair => {
                let mut data = Self::bare(kind, "id-pair");
                data.parties = Some((
                    super::id_spine(size(ID_BASE_DEPTH), false).bytes,
                    super::id_spine(size(ID_BASE_DEPTH), true).bytes,
                ));
                data
            }
            FamilyKind::CombScatter => {
                let teeth = size(CROSS_BASE_TEETH);
                let mut data = Self::bare(kind, "comb-scatter");
                data.cross = Some((
                    super::cliff_comb(CROSS_TOOTH_MAGNITUDE_BITS, teeth)
                        .version()
                        .encode(),
                    super::scattered_id(teeth / 2).bytes,
                ));
                data.output_dominated = true;
                data
            }
            FamilyKind::Harmonic => Self::event(
                kind,
                "harmonic",
                super::harmonic(size(HARMONIC_BASE_DEPTH))
                    .version()
                    .encode(),
            ),
            FamilyKind::Scatter => Self::scatter(size(SCATTER_BASE_CLOCKS)),
            FamilyKind::NestedFull => {
                let d = size(NESTED_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "nested-full",
                    super::dense(d).version().encode(),
                    super::nested_full_id(d).bytes,
                )
            }
            FamilyKind::NestedWide => {
                let s = size(NESTED_WIDE_BASE);
                Self::cross_family(
                    kind,
                    "nested-wide",
                    super::bigroot(s, s).version().encode(),
                    super::nested_full_id(s).bytes,
                )
            }
            FamilyKind::MirrorWide => {
                let s = size(MIRROR_WIDE_BASE);
                Self::cross_family(
                    kind,
                    "mirror-wide",
                    super::wide_tail(s, s).version().encode(),
                    super::nested_left_full_id(s).bytes,
                )
            }
            FamilyKind::MirrorNarrow => {
                let d = size(MIRROR_NARROW_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "mirror-narrow",
                    super::wide_tail(1, d).version().encode(),
                    super::nested_left_full_id(d).bytes,
                )
            }
            FamilyKind::Staircase => {
                let d = size(STAIRCASE_BASE_DEPTH);
                Self::cross_family(
                    kind,
                    "staircase",
                    super::staircase(d).version().encode(),
                    super::id_spine(d, false).bytes,
                )
            }
            FamilyKind::RevealComb => {
                let s = size(REVEAL_COMB_BASE);
                Self::cross_family(
                    kind,
                    "reveal-comb",
                    super::reveal_comb(s, s).version().encode(),
                    super::reveal_comb_id(s).bytes,
                )
            }
            FamilyKind::RevealHifloor => {
                let s = size(REVEAL_COMB_BASE);
                Self::cross_family(
                    kind,
                    "reveal-hifloor",
                    super::reveal_comb_hifloor(s, s).version().encode(),
                    super::reveal_comb_id(s).bytes,
                )
            }
            FamilyKind::PureComb => {
                let s = size(PURE_COMB_BASE);
                Self::cross_family(
                    kind,
                    "pure-comb",
                    super::pure_comb(s, s).version().encode(),
                    super::pure_comb_id(s).bytes,
                )
            }
            FamilyKind::AscendCliff => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    "ascend-cliff",
                    super::ascend_cliff(s, s).version().encode(),
                    super::ascend_cliff_id(s).bytes,
                )
            }
            FamilyKind::AscendPlateau => {
                let s = size(ASCEND_CLIFF_BASE);
                Self::cross_family(
                    kind,
                    "ascend-plateau",
                    super::ascend_cliff_plateau(s, s).version().encode(),
                    super::ascend_cliff_id(s).bytes,
                )
            }
            FamilyKind::Benign => Self::benign(size(BENIGN_BASE_CLOCKS)),
        };
        // ── the bundle post-pass: the derived slots, uniform across shapes ──
        // A cross shape's primary version is its event side.
        if data.version.is_none() {
            data.version = data.cross.as_ref().map(|(v, _)| v.clone());
        }
        // Every version gains its ticked comparison counterpart and its
        // mismatched rank pair (shape-derived rank against a small integer
        // rank, the pair whose exponent mismatch the rank rows price).
        if let Some(bytes) = &data.version {
            let v = decode_version(bytes);
            let mut w = v.clone();
            w.tick(&Party::seed());
            data.version2 = Some(w.encode());
            let b = Version::try_from(RANK_PAIR_INTEGER_TICKS)
                .expect("a small integer version is valid")
                .rank();
            data.rank_pair = Some((v.rank(), b));
        }
        // A cross shape's id side becomes a disjoint party pair through
        // the mount adapter.
        if data.parties.is_none() {
            if let Some((_, id)) = &data.cross {
                data.parties = Some(disjoint_mounted_pair(id));
            }
        }
        data
    }

    /// Build the scatter fold population: `n` balanced-forked parties, one
    /// tick each, ordered evens before odds so a sequential fold's
    /// accumulator holds every other leaf and never coalesces.
    fn scatter(n: usize) -> FamilyData {
        let mut parties = vec![Party::seed()];
        while parties.len() < n {
            let mut next = Vec::with_capacity(parties.len() * 2);
            for mut p in parties {
                let q = p.fork();
                next.push(p);
                next.push(q);
            }
            parties = next;
        }
        // Dropping the tail keeps `n` honest at non-power-of-two scales;
        // a dropped party's region simply goes unowned.
        parties.truncate(n);
        let scatter_order = |v: Vec<Vec<u8>>| -> Vec<Vec<u8>> {
            let (evens, odds): (Vec<_>, Vec<_>) =
                v.into_iter().enumerate().partition(|(i, _)| i % 2 == 0);
            evens
                .into_iter()
                .chain(odds)
                .map(|(_, bytes)| bytes)
                .collect()
        };
        let versions = scatter_order(
            parties
                .iter()
                .map(|p| {
                    let mut v = Version::new();
                    v.tick(p);
                    v.encode()
                })
                .collect(),
        );
        let parties = scatter_order(parties.iter().map(Party::encode).collect());
        let mut data = Self::bare(FamilyKind::Scatter, "scatter");
        data.fold = Some((versions, parties));
        data
    }

    /// Wrap a cross shape: a packed (event, id) pair built as one
    /// adversarial pairing.
    ///
    /// The cross drives the tick rows' walk floors and the clock rows'
    /// operand choice directly; the post-pass derives the shape's version
    /// (the event side) and its disjoint party pair (the mounted id side),
    /// so the shape also reaches every version and party row.
    fn cross_family(
        kind: FamilyKind,
        name: &'static str,
        version: Vec<u8>,
        id: Vec<u8>,
    ) -> FamilyData {
        let mut data = Self::bare(kind, name);
        data.cross = Some((version, id));
        data
    }

    /// Wrap an event shape's wire bytes.
    fn event(kind: FamilyKind, name: &'static str, bytes: Vec<u8>) -> FamilyData {
        let mut data = Self::bare(kind, name);
        data.version = Some(bytes);
        data
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
        // The fold rows' organic control: the population's own versions and
        // parties in construction order (the adversarial ordering belongs to
        // the scatter family alone).
        let fold = Some((
            clocks.iter().map(|c| c.version().encode()).collect(),
            clocks.iter().map(|c| c.party().encode()).collect(),
        ));
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
        let mut data = Self::bare(FamilyKind::Benign, "benign");
        data.version = Some(version.encode());
        data.parties = Some((a.encode(), b.encode()));
        data.fold = fold;
        data
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

    /// The designated cross decoded fresh (event version, id party), with
    /// combined packed byte length.
    fn cross(&self) -> Option<(Version, Party, usize)> {
        let (v, p) = self.cross.as_ref()?;
        Some((decode_version(v), decode_party(p), v.len() + p.len()))
    }

    /// One clock per shape, from the bundle's slots.
    ///
    /// A cross shape pairs its own id and event sides; a version-bearing
    /// shape pairs the seed party with the adversarial version; a
    /// party-only shape pairs the adversarial party with the empty
    /// version.
    fn clock(&self) -> Option<(Clock, usize)> {
        if let Some((v, p, n)) = self.cross() {
            return Some((Clock::from_parts(p, v), n));
        }
        if let Some((v, n)) = self.version() {
            return Some((Clock::from_parts(Party::seed(), v), n + 1));
        }
        let (a, _, _) = self.party_pair()?;
        let n = self.parties.as_ref().map(|(a, _)| a.len())?;
        Some((Clock::from_parts(a, Version::new()), n + 1))
    }

    /// Two joinable clocks (disjoint parties), with combined operand
    /// bytes, from the bundle's slots.
    ///
    /// A shape with both a party pair and versions crosses them; a
    /// party-only pair rides empty versions; a version-only shape forks
    /// a seed pair around its version pair.
    fn clock_pair(&self) -> Option<(Clock, Clock, usize)> {
        match (self.parties.is_some(), self.version.is_some()) {
            (true, true) => {
                let (a, b, np) = self.party_pair()?;
                let (v, w, nv) = self.version_pair()?;
                Some((Clock::from_parts(a, v), Clock::from_parts(b, w), np + nv))
            }
            (true, false) => {
                let (a, b, n) = self.party_pair()?;
                Some((
                    Clock::from_parts(a, Version::new()),
                    Clock::from_parts(b, Version::new()),
                    n + 2,
                ))
            }
            (false, true) => {
                let (v, w, n) = self.version_pair()?;
                let mut p = Party::seed();
                let q = p.fork();
                Some((Clock::from_parts(p, v), Clock::from_parts(q, w), n + 2))
            }
            (false, false) => None,
        }
    }
}

/// The disjoint-mount adapter: lift one packed id shape into a disjoint
/// party pair inside a single universe.
///
/// The pair mounts the shape under opposite children of a fresh root —
/// `(shape, ·)` and `(·, shape)` — so the halves are disjoint by
/// construction and joining them merely reunites the root's two subtrees:
/// two independently-generated id shapes are never asked to share a
/// universe (linearity of parties is the invariant everything rests on —
/// the crate docs' safety rules). Each half is the shape itself one level
/// deeper, so party cells on a mounted shape measure the shape plus one
/// root tag. Runs at bundle build, outside any measurement, and asserts
/// the disjointness it mints.
fn disjoint_mounted_pair(id: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let shape = decode_party(id);
    let mount = |left: bool| -> Vec<u8> {
        let mut bits = codec::Bits::with_capacity(shape.as_bits().len() + 2);
        bits.push(left);
        bits.push(!left);
        bits.extend_from_bitslice(shape.as_bits());
        codec::zero_dead_bits(&mut bits);
        bits.into_vec()
    };
    let (a, b) = (mount(true), mount(false));
    assert!(
        decode_party(&a).is_disjoint(&decode_party(&b)),
        "the disjoint-mount adapter must mint a disjoint pair"
    );
    (a, b)
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

// ─── liveness floors ────────────────────────────────────────────────────────

// The floor derivations and not-applicable reasons, shared across rows so
// the rendered legend stays small and uniform.

/// Scan floor: the operation must examine its packed operands in full.
const WHY_SCAN_EXAMINES: &str =
    "must examine its packed operands: at least one scanned bit per packed byte";
/// Scan floor: early exit is legitimate, but the root codes are still read.
const WHY_SCAN_TOUCH: &str =
    "may answer at the first divergence: still reads the operands' root codes";
/// Scan NA (`version_eq`'s own): same-form equality is decided on the
/// stored canonical bytes wholesale, and the operands grow without bound.
const NA_SCAN_EQ_BYTES: &str = "decides same-form equality on the stored canonical bytes \
     wholesale (the compare may legitimately stop at the first differing byte): no stream walk \
     is in the contract; unlike the hash rows' small-operand exposure, eq operands grow without \
     bound, so the bench judge's time leg is the backstop that the compare stays linear";
/// Scan NA: the contract is a wholesale byte move.
const NA_SCAN_BYTE_COPY: &str =
    "moves or hashes the stored canonical bytes wholesale: no stream walk is in the contract";
/// Scan NA: the operands carry no packed stream.
const NA_SCAN_NO_STREAM: &str = "operands are decoded rank values: no packed stream exists";
/// Scan NA: a trivial (seed) operand stores no bits to scan.
const NA_SCAN_SEED_PARTY: &str = "the forked party is the seed: its packed form is empty";
/// Limb floor: wide magnitudes are folded limb by limb.
const WHY_LIMB_WIDE: &str = "a magnitude wider than the machine-word bound must be materialized \
     or folded limb by limb: one op per 64 magnitude bits";
/// Limb floor: the tick walk decodes every stored wide payload code.
const WHY_LIMB_TICK_STREAM: &str = "every payload code of the stored stream wider than the \
     machine-word bound must be decoded limb by limb: one op per 64 code bits (the stream's \
     own codes, not the decoded tree's values — a plateau of equal wide leaves stores its \
     width once)";
/// Limb floor: the rank pair's sum spans the wider operand's content.
const WHY_LIMB_RANK_PAIR: &str = "the mismatched pair's sum carries a numerator as wide as the \
     wider operand's value content: one limb write per 64 content bits";
/// Limb floor: the rank fold's sum spans its widest summand's content.
const WHY_LIMB_RANK_SUM: &str = "the fold's sum carries a numerator as wide as its widest \
     summand's value content: one limb write per 64 content bits";
/// Limb NA: every operand magnitude fits machine words.
const NA_LIMB_NARROW: &str =
    "no operand magnitude exceeds the machine-word bound: word arithmetic suffices";
/// Limb NA: the contract forces no arithmetic.
const NA_LIMB_NOT_FORCED: &str =
    "magnitudes may be moved or compared without arithmetic: no limb work is in the contract";
/// Limb NA: id trees have no magnitudes at all.
const NA_LIMB_ID_TREE: &str = "id trees store no magnitudes: there is no arithmetic to meter";
/// Limb NA: the contract is a machine-word fold.
const NA_LIMB_WORD_FOLD: &str = "a machine-word fold: no big-integer arithmetic in the contract";
/// Limb NA: the work runs below the shim, in the dependency.
const NA_LIMB_DEPENDENCY: &str = "the decimal conversion runs inside the bignum dependency, \
     below the limb shim: the bench judge's time leg, and its wide-display pair at \
     conversion-dominated widths, judge this row";
/// Heap floor: the result materializes at least its packed bytes.
const WHY_HEAP_MATERIALIZES: &str =
    "materializes a result at least as large as the packed bytes it codes";
/// Heap floor (deterministic-liveness): a forked child copies the version.
const WHY_HEAP_FORK_CHILD: &str = "deterministic-liveness: the forked child carries its own \
     copy of the version's packed bits today; a shared-buffer representation would lower this \
     floor deliberately";
/// Heap NA: allocation is not semantically forced.
const NA_HEAP_IN_PLACE: &str = "may compute in place or return word-scale results: allocation \
     is not semantically forced (the process allocator itself cannot be re-routed around)";
/// Scan floor: the tick walk examines its whole input.
const WHY_SCAN_TICK_WALK: &str = "the paired fill walk examines every topology bit and payload \
     code of both operands at least once: 8 bits per input byte, with the measured tick-walk \
     constants 2–5× above";
/// The tick-cross scan floor: full examination of every input bit.
const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

/// A full-examination scan floor over `packed_bytes` of operand.
fn scan_examines(packed_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: (packed_bytes as f64 * SCAN_FLOOR_BITS_PER_INPUT_BYTE) as u64,
        why: WHY_SCAN_EXAMINES,
    }
}

/// The early-exit scan floor: the root codes are always read.
fn scan_touch() -> Liveness {
    Liveness::Floor {
        min: SCAN_TOUCH_FLOOR_BITS,
        why: WHY_SCAN_TOUCH,
    }
}

/// A wide-magnitude limb floor, or NA when every magnitude fits machine
/// words.
fn limb_wide(mandatory_limbs: u64) -> Liveness {
    if mandatory_limbs == 0 {
        Liveness::NotApplicable {
            reason: NA_LIMB_NARROW,
        }
    } else {
        Liveness::Floor {
            min: mandatory_limbs,
            why: WHY_LIMB_WIDE,
        }
    }
}

/// A materialization heap floor over `packed_bytes`.
fn heap_materializes(packed_bytes: usize) -> Liveness {
    Liveness::Floor {
        min: packed_bytes as u64,
        why: WHY_HEAP_MATERIALIZES,
    }
}

/// The fork rows' heap declaration: the child clock's version copy floors
/// the heap at the version's whole stored bytes, or NA when the version is
/// word-scale (the id-pair cross forks around an empty version).
fn heap_fork_child(version: &Version) -> Liveness {
    let stored_bytes = (version.encoded_bits() / 8) as u64;
    if stored_bytes == 0 {
        na(NA_HEAP_IN_PLACE)
    } else {
        Liveness::Floor {
            min: stored_bytes,
            why: WHY_HEAP_FORK_CHILD,
        }
    }
}

/// Shorthand for a not-applicable declaration.
fn na(reason: &'static str) -> Liveness {
    Liveness::NotApplicable { reason }
}

/// Segments NA: the policy declaration every cell carries on the segments
/// currency.
const NA_SEG_CEILING_ONLY: &str = "ceiling-only by policy: the target is walks that never grow \
     the stack, so the honest floor is zero and a zero floor asserts nothing";

/// The segments currency's declaration: ceiling-only by policy, on every
/// cell.
fn seg_ceiling_only() -> Liveness {
    na(NA_SEG_CEILING_ONLY)
}

/// The floors of the many rows that must walk their operands but are forced
/// into neither allocation nor arithmetic: scan floored, heap and limb NA.
fn walk_floors(packed_bytes: usize) -> Floors {
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
        limb: na(NA_LIMB_NOT_FORCED),
        segments: seg_ceiling_only(),
        scan: scan_examines(packed_bytes),
    }
}

/// The tick-cross rows' floors: full-examination scan, per-stored-code
/// limb, in-place heap.
///
/// The paired fill walk examines every bit of both packed operands (a
/// full-examination scan floor, 8 bits per byte — the measured
/// tick-walk constants sit 2–5× above it), and every wide payload code
/// of the version's own stored stream must be decoded limb by limb
/// (the mandatory limb floor; NA on the word-scale families). The limb
/// floor derives from the stream's codes, not the decoded tree's
/// min-lifted bases: a plateau of equal wide leaves stores its width
/// once and steps by unit deltas after, and the walk provably need not
/// materialize each leaf's absolute value — a tree-derived floor would
/// demand limb work no conforming walk does.
fn tick_walk_floors(version: &Version, packed_bytes: usize) -> Floors {
    let limbs = mandatory_limbs_stream(version);
    Floors {
        heap: na(NA_HEAP_IN_PLACE),
        limb: if limbs == 0 {
            na(NA_LIMB_NARROW)
        } else {
            Liveness::Floor {
                min: limbs,
                why: WHY_LIMB_TICK_STREAM,
            }
        },
        segments: seg_ceiling_only(),
        scan: Liveness::Floor {
            min: (packed_bytes as u64).saturating_mul(TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE),
            why: WHY_SCAN_TICK_WALK,
        },
    }
}

/// The mandatory limb count of a version's stored stream: one limb per
/// 64 bits of every payload code wider than
/// [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// A walk over the stream must decode each stored code to fold it, and
/// decoding a wide code cannot touch fewer limbs than the code has;
/// narrower codes may legitimately live in machine words and count
/// zero. Unlike [`mandatory_limbs_version`], this counts the stream's
/// own delta codes, never the decoded tree's absolute values: it is
/// the honest floor for operations that read the stored form as-is.
/// Iterative over the packed form, outside any measurement.
fn mandatory_limbs_stream(v: &Version) -> u64 {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut limbs = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = bits[pos];
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let width = code.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
}

/// The mandatory limb count of a version's stored magnitudes: one limb per
/// 64 bits of every base wider than [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// Materializing or folding such a value cannot touch fewer limbs than the
/// value has, whatever the representation; narrower values may legitimately
/// live in machine words and count zero. The walk mirrors
/// [`radix_units_version`]: iterative over the packed form, outside any
/// measurement.
fn mandatory_limbs_version(v: &Version) -> u64 {
    let mut limbs = 0u64;
    for base in stored_bases(v) {
        let width = base.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
}

/// The min-lifted stored bases of a version's canonical event tree, in
/// preorder: the values the paper notation renders and any base-per-node
/// representation must hold.
///
/// Reconstructed from the stored skyline stream in three linear passes
/// (absolute leaf heights, bottom-up subtree floors, per-node relative
/// bases), entirely outside any measurement.
fn stored_bases(v: &Version) -> Vec<Base> {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    // Pass 1: topology flags and absolute leaf heights.
    let mut pos = 0usize;
    let mut topology: Vec<bool> = Vec::new();
    let mut heights: Vec<Base> = Vec::new();
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let internal = bits[pos];
        pos += 1;
        topology.push(internal);
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let value = match heights.last() {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev.clone() - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        heights.push(value);
    }
    // Pass 2: per-node floors (minimum leaf height in the subtree),
    // bottom-up over the preorder topology.
    let nodes = topology.len();
    let mut floors: Vec<Base> = vec![Base::ZERO; nodes];
    let mut open: Vec<(usize, Option<Base>)> = Vec::new();
    let mut next_leaf = 0usize;
    for (index, &internal) in topology.iter().enumerate() {
        if internal {
            open.push((index, None));
            continue;
        }
        floors[index] = heights[next_leaf].clone();
        next_leaf += 1;
        let mut summary = floors[index].clone();
        loop {
            match open.pop() {
                None => break,
                Some((parent, None)) => {
                    open.push((parent, Some(summary)));
                    break;
                }
                Some((parent, Some(left))) => {
                    let floor = if left <= summary { left } else { summary };
                    floors[parent] = floor.clone();
                    summary = floor;
                }
            }
        }
    }
    // Pass 3: each node's stored base is its floor minus its parent's.
    let mut bases = Vec::with_capacity(nodes);
    let mut parent_floors: Vec<Base> = vec![Base::ZERO];
    for (index, &internal) in topology.iter().enumerate() {
        let parent = parent_floors
            .pop()
            .expect("preorder supplies one inherited floor per node");
        bases.push(floors[index].clone() - &parent);
        if internal {
            parent_floors.push(floors[index].clone());
            parent_floors.push(floors[index].clone());
        }
    }
    bases
}

// ─── operations ─────────────────────────────────────────────────────────────

/// One prepared cell run: the operand bytes it charges against, the
/// denomination rule, and the body to measure.
///
/// `prepare` builds (and decodes) operands outside measurement; the body's
/// result is boxed and kept alive until the meters are read, so peak heap
/// includes the fully materialized output.
struct Cell {
    /// The packed (or, on `FromStr` rows, text) operand bytes.
    input_bytes: usize,
    /// How the meters are denominated (the module doc's criterion).
    denom: Denom,
    /// The cell's liveness declarations, one per floored column.
    floors: Floors,
    /// The measured body; its result stays alive until the meters are read.
    #[allow(clippy::type_complexity)]
    body: Box<dyn FnOnce() -> Box<dyn Any>>,
}

/// A cell's denomination rule (see the module doc's list of which rows get
/// which).
enum Denom {
    /// Input bytes alone: the default, and the only rule most rows may use.
    Input,
    /// Total I/O bytes: input plus the actual output, read back from the
    /// measured result after the meters are captured.
    Io(IoSpec),
}

/// The I/O-denomination data for a mandatory-output cell.
struct IoSpec {
    /// Read the actual output's byte size from the boxed result.
    output_bytes: fn(&dyn Any) -> usize,
    /// The text rows' extra terms; `None` for packed-output cells.
    text: Option<TextSpec>,
}

/// The text rows' radix-work term and output-honesty data.
struct TextSpec {
    /// `Σ digitsᵢ × limbsᵢ` over the values the text spells; the limb column
    /// is judged against `R = n_io +` this, at the κ ceiling, and the
    /// output-honesty ceiling is asserted against the same units.
    radix_units: u64,
    /// Whether the measured *output* is the text side (`Display`); the
    /// honesty assertion then runs against the actual output bytes.
    /// `FromStr`'s text is input and is asserted at prepare.
    output_is_text: bool,
}

impl Cell {
    /// Package an input-denominated body with its operand byte count and its
    /// liveness declarations.
    fn new<R: Any>(input_bytes: usize, floors: Floors, body: impl FnOnce() -> R + 'static) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Input,
            floors,
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Package an I/O-denominated packed-output body: the output side of
    /// `n_io` is read back from the actual result.
    fn io<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec {
                output_bytes,
                text: None,
            }),
            floors,
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Package a text-row body: I/O-denominated, with the limb column judged
    /// against the radix-work denominator.
    fn text<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        spec: TextSpec,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec {
                output_bytes,
                text: Some(spec),
            }),
            floors,
            body: Box::new(move || Box::new(body())),
        }
    }
}

/// Assert the output-honesty ceiling on one text stream.
///
/// Every text stream entering a denominator passes through here: text bytes
/// at most [`TEXT_BYTES_PER_RADIX_UNIT`] per radix unit of the values the
/// text spells, so padding the text side of `n_io` trips the run instead
/// of greening a cell.
fn assert_honest_text(what: &'static str, text_bytes: usize, radix_units: u64) {
    assert!(
        text_bytes as f64 <= TEXT_BYTES_PER_RADIX_UNIT * radix_units as f64,
        "output honesty: {what}: {text_bytes} text bytes exceed \
         {TEXT_BYTES_PER_RADIX_UNIT} per radix unit over {radix_units} units"
    );
}

/// `Σ digits × limbs` over the decimal values an event tree's text spells:
/// every node's stored base, exactly what `Display` renders and `FromStr`
/// parses.
///
/// `digits` is the value's exact decimal length; `limbs` its 64-bit limb
/// count (at least 1, so single-digit zeros still cost a unit). The walk is
/// iterative over the packed form; only the per-value `digits × limbs`
/// products enter the denominator, so the term prices schoolbook conversion
/// work without assuming any converter.
fn radix_units_version(v: &Version) -> u64 {
    let mut units = 0u64;
    for base in stored_bases(v) {
        let digits = base.to_string().len() as u64;
        let limbs = base.bits().div_ceil(64).max(1);
        units += digits * limbs;
    }
    units
}

/// `Σ digits × limbs` over an id tree's text: one unit per rendered `0`/`1`
/// token (terminals and absent children), each a single digit of a
/// single-limb value.
fn radix_units_party(p: &Party) -> u64 {
    let bits = p.as_bits();
    if bits.is_empty() {
        return 1; // the empty id renders one `0` token
    }
    let mut pos = 0usize;
    let mut pending = 1u64;
    let mut units = 0u64;
    while pending > 0 {
        pending -= 1;
        let left = bits[pos];
        let right = bits[pos + 1];
        pos += 2;
        if !left && !right {
            units += 1; // a terminal renders `1`
            continue;
        }
        for present in [left, right] {
            if present {
                pending += 1;
            } else {
                units += 1; // an absent child renders `0`
            }
        }
    }
    units
}

/// `Σ digits × limbs` over a clock's text: its party's and version's terms.
fn radix_units_clock(c: &Clock) -> u64 {
    radix_units_party(c.party()) + radix_units_version(c.version())
}

/// The packed byte size of a version produced by a measured body.
fn version_output_bytes(v: &Version) -> usize {
    v.encoded_bits().div_ceil(8)
}

/// One board row: a public operation and how to instantiate it per family.
struct Op {
    /// The row label, `type_operation`.
    name: &'static str,
    /// The signature group the row belongs to on the operation axis.
    group: OpGroup,
    /// Build the cell for one shape, or `None` where the shape's bundle
    /// supplies no operand for this operation's signature.
    prepare: fn(&FamilyData) -> Option<Cell>,
}

/// The operation axis's signature groups.
///
/// A group names the operand signature a row consumes; the bench mirror's
/// pinned subset pairs each shape with the groups it was designed to
/// stress ([`designed`]), so the subset is a rule over the same two axes
/// the board's product runs on, never a second cell list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpGroup {
    /// Rows over a shape's version operands (codec, comparison, merge,
    /// text, hash rows).
    Version,
    /// The linear-functional query rows: `rank`, `distance`, `lag`,
    /// `min_ticks`.
    Measure,
    /// The rank-value rows: `rank_pair_ops`, `rank_sum`.
    Rank,
    /// The tick rows, driven through a cross shape's designated pairing.
    Tick,
    /// The projection rows: `version_project`, `clock_own_version`.
    Projection,
    /// The fold rows: `version_join_all`, `party_join_all`.
    Fold,
    /// Rows over a shape's disjoint party pair.
    Party,
    /// Rows over a shape's clock (the tick and projection clock rows
    /// carry their own groups above).
    Clock,
}

/// The shape × operation-group pairings each shape was designed to
/// stress: the bench mirror's diagonal.
///
/// Declared per shape, on the shape axis, so the pinned bench subset is
/// derived — a shape added to the axis must answer which groups it was
/// built against (the exhaustive match), and the subset follows. The
/// deterministic board itself never consults this: it runs the whole
/// product.
fn designed(kind: FamilyKind, group: OpGroup) -> bool {
    match kind {
        // The original full-surface adversaries and the organic control,
        // plus the two population shapes (whose bundles already narrow
        // them to their party/fold rows).
        FamilyKind::Dense | FamilyKind::Benign | FamilyKind::IdPair | FamilyKind::Scatter => true,
        // The magnitude shapes predate the rank rows' mismatch pair and
        // were never its designed adversary.
        FamilyKind::Bigroot | FamilyKind::Hugeleaf | FamilyKind::Cliff => group != OpGroup::Rank,
        // The rank fold's wide-numerator adversary.
        FamilyKind::Harmonic => matches!(group, OpGroup::Measure | OpGroup::Rank),
        // The output-domination cross.
        FamilyKind::CombScatter => group == OpGroup::Projection,
        // The tick-walk crosses.
        FamilyKind::NestedFull
        | FamilyKind::NestedWide
        | FamilyKind::MirrorWide
        | FamilyKind::MirrorNarrow
        | FamilyKind::Staircase
        | FamilyKind::RevealComb
        | FamilyKind::RevealHifloor
        | FamilyKind::PureComb
        | FamilyKind::AscendCliff
        | FamilyKind::AscendPlateau => group == OpGroup::Tick,
    }
}

/// The operation table: every public operation with a meaningful packed
/// operand (the module doc lists the rest).
#[allow(clippy::too_many_lines)]
fn ops() -> Vec<Op> {
    vec![
        // ── Version ────────────────────────────────────────────────────
        Op {
            name: "version_decode",
            group: OpGroup::Version,
            prepare: |f| {
                let bytes = f.version.clone()?;
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
                    limb: limb_wide(mandatory_limbs_version(&decode_version(&bytes))),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(bytes.len()),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    decode_version(&bytes)
                }))
            },
        },
        Op {
            name: "version_encode",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                };
                Some(Cell::new(n, floors, move || (v.encode(), v)))
            },
        },
        Op {
            name: "version_cmp",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    let ord: Option<Ordering> = v.partial_cmp(&w);
                    (ord, v, w)
                }))
            },
        },
        Op {
            name: "version_eq",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_EQ_BYTES),
                };
                Some(Cell::new(n, floors, move || (v == w, v, w)))
            },
        },
        Op {
            name: "version_concurrent",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    (v.concurrent(&w), v, w)
                }))
            },
        },
        Op {
            name: "version_join",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || (&v | &w, v, w)))
            },
        },
        Op {
            name: "version_join_assign",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    v |= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_meet",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || (&v & &w, v, w)))
            },
        },
        Op {
            name: "version_meet_assign",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    v &= &w;
                    (v, w)
                }))
            },
        },
        Op {
            name: "version_tick",
            group: OpGroup::Tick,
            prepare: |f| {
                // The tick-walk families carry their own (event, id)
                // pair; every other family ticks its version with the
                // seed.
                if let Some((mut v, party, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    return Some(Cell::new(n, floors, move || {
                        v.tick(&party);
                        (v, party)
                    }));
                }
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(n + 1, walk_floors(n), move || {
                    v.tick(&party);
                    v
                }))
            },
        },
        Op {
            name: "version_tick_adv_party",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                Some(Cell::new(n + 1, walk_floors(n), move || {
                    v.tick(&a);
                    (v, a)
                }))
            },
        },
        Op {
            name: "version_batch_snapshot",
            group: OpGroup::Version,
            prepare: |f| {
                let (mut v, n) = f.version()?;
                let party = Party::seed();
                Some(Cell::new(n + 1, walk_floors(n), move || {
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
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_wide(mandatory_limbs_version(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || (v.rank(), v)))
            },
        },
        Op {
            name: "rank_pair_ops",
            group: OpGroup::Rank,
            prepare: |f| {
                // The mismatched pair: a family-derived rank (maximal
                // exponent on the spines) against a small integer rank, on
                // the spine families that maximize the mismatch plus the
                // benign control. Ranks are built at family construction,
                // outside measurement; the denominator is the pair's value
                // content (the module doc's rank denomination).
                let (a, b) = f.rank_pair.clone()?;
                let n = (a.content_bits() + b.content_bits()).div_ceil(8) as usize;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: Liveness::Floor {
                        min: a.content_bits().max(b.content_bits()).div_ceil(64),
                        why: WHY_LIMB_RANK_PAIR,
                    },
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                };
                Some(Cell::new(n, floors, move || {
                    let ord = a.cmp(&b);
                    // One direction of the pair dominates; keep whichever
                    // difference exists so the subtraction always runs.
                    let diff = a.checked_sub(&b).or_else(|| b.checked_sub(&a));
                    let sum = &a + &b;
                    (ord, diff, sum, a, b)
                }))
            },
        },
        Op {
            name: "rank_sum",
            group: OpGroup::Rank,
            prepare: |f| {
                // The mixed fold: the family-derived rank (maximal exponent
                // on the spines) summed high-first with one small integer
                // rank per packed byte of the family's measure operand, so
                // both sides of the value content scale together. High-first
                // is the adversarial order of record: `Sum` accepts arbitrary
                // order, and under a fold that re-normalizes per element it
                // is the order that makes every later add a full-width
                // operation. The denominator is the summands' total value
                // content (the module doc's rank denomination).
                let (a, _) = f.rank_pair.clone()?;
                let (_, k) = f.version()?;
                let ones: Vec<Rank> = (0..k)
                    .map(|i| {
                        Version::try_from(i as u64 % 7 + 1)
                            .expect("a small integer version is valid")
                            .rank()
                    })
                    .collect();
                let n = (a.content_bits().div_ceil(8) as usize)
                    + ones
                        .iter()
                        .map(|r| r.content_bits().div_ceil(8) as usize)
                        .sum::<usize>();
                let wide = a.content_bits();
                let limb = if wide > MACHINE_WORD_MAGNITUDE_BITS {
                    Liveness::Floor {
                        min: wide.div_ceil(64),
                        why: WHY_LIMB_RANK_SUM,
                    }
                } else {
                    na(NA_LIMB_NARROW)
                };
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb,
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_NO_STREAM),
                };
                Some(Cell::new(n, floors, move || {
                    std::iter::once(a).chain(ones).sum::<Rank>()
                }))
            },
        },
        Op {
            name: "version_distance",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_wide(mandatory_limbs_version(&v) + mandatory_limbs_version(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || (v.distance(&w), v, w)))
            },
        },
        Op {
            name: "version_lag",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: limb_wide(mandatory_limbs_version(&v) + mandatory_limbs_version(&w)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || (v.lag(&w), v, w)))
            },
        },
        Op {
            name: "version_min_ticks",
            group: OpGroup::Measure,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_WORD_FOLD),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || (v.min_ticks(), v)))
            },
        },
        Op {
            name: "version_join_all",
            group: OpGroup::Fold,
            prepare: |f| {
                let (versions, _) = f.fold.as_ref()?;
                let n = versions.iter().map(Vec::len).sum();
                let versions: Vec<Version> = versions.iter().map(|b| decode_version(b)).collect();
                Some(Cell::new(n, walk_floors(n), move || {
                    Version::join_all(versions)
                }))
            },
        },
        Op {
            name: "version_project",
            group: OpGroup::Projection,
            prepare: |f| {
                // Adversarial × adversarial with mandatory dominating
                // output: the declared output-domination cross,
                // I/O-denominated.
                if f.output_dominated {
                    let (v_bytes, p_bytes) = f.cross.as_ref()?;
                    let n = v_bytes.len() + p_bytes.len();
                    let v = decode_version(v_bytes);
                    let p = decode_party(p_bytes);
                    return Some(Cell::io(
                        n,
                        walk_floors(n),
                        |r| {
                            let (out, _, _) = r
                                .downcast_ref::<(Version, Version, Party)>()
                                .expect("the cross projection body yields (out, v, p)");
                            version_output_bytes(out)
                        },
                        move || (&v / &p, v, p),
                    ));
                }
                // A cross shape without output domination projects its
                // event side through its id side, input-denominated (the
                // module doc's do-not-re-denominate list).
                if let Some((v, p, n)) = f.cross() {
                    return Some(Cell::new(n, walk_floors(n), move || (&v / &p, v, p)));
                }
                // Small (half-interval) party × adversarial version.
                if f.version.is_some() {
                    let (v, n) = f.version()?;
                    let half = Party::seed().fork();
                    return Some(Cell::new(n + 1, walk_floors(n), move || {
                        (&v / &half, v, half)
                    }));
                }
                // Adversarial party × small version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut v = Version::new();
                v.tick(&a);
                let input = n + v.encode().len();
                Some(Cell::new(input, walk_floors(input), move || {
                    (&v / &a, v, a)
                }))
            },
        },
        Op {
            name: "version_display",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let spec = TextSpec {
                    radix_units: radix_units_version(&v),
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_DEPENDENCY),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Version)>()
                            .expect("the display body yields (text, v)")
                            .0
                            .len()
                    },
                    spec,
                    move || (v.to_string(), v),
                ))
            },
        },
        Op {
            name: "version_from_str",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, _) = f.version()?;
                let s = v.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_version(&v),
                    output_is_text: false,
                };
                assert_honest_text("version_from_str input", s.len(), spec.radix_units);
                let packed = version_output_bytes(&v);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: limb_wide(mandatory_limbs_version(&v)),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        version_output_bytes(
                            r.downcast_ref::<Version>()
                                .expect("the parse body yields a version"),
                        )
                    },
                    spec,
                    move || {
                        s.parse::<Version>()
                            .expect("a displayed version parses back")
                    },
                ))
            },
        },
        Op {
            name: "version_hash",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, n) = f.version()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    v.hash(&mut hasher);
                    (hasher.finish(), v)
                }))
            },
        },
        Op {
            name: "causally_contains",
            group: OpGroup::Version,
            prepare: |f| {
                let (v, w, n) = f.version_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    let hit = causally::since(&v).contains(&w);
                    (hit, v, w)
                }))
            },
        },
        // ── Party ──────────────────────────────────────────────────────
        Op {
            name: "party_decode",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b) = f.parties.clone()?;
                let n = a.len() + b.len();
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || {
                    (decode_party(&a), decode_party(&b))
                }))
            },
        },
        Op {
            name: "party_encode",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                };
                Some(Cell::new(n, floors, move || (a.encode(), a)))
            },
        },
        Op {
            name: "party_fork",
            group: OpGroup::Party,
            prepare: |f| {
                let (mut a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: if a.is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                };
                Some(Cell::new(n, floors, move || {
                    let child = a.fork();
                    (a, child)
                }))
            },
        },
        Op {
            name: "party_join",
            group: OpGroup::Party,
            prepare: |f| {
                let (mut a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "party_join_all",
            group: OpGroup::Fold,
            prepare: |f| {
                let (_, parties) = f.fold.as_ref()?;
                let n = parties.iter().map(Vec::len).sum();
                let mut parties = parties.iter().map(|b| decode_party(b));
                let acc = parties.next().expect("the scatter population is nonempty");
                let rest: Vec<Party> = parties.collect();
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || {
                    let mut acc = acc;
                    acc.join_all(rest)
                        .expect("fold operands are forked parties, pairwise disjoint");
                    acc
                }))
            },
        },
        Op {
            name: "party_covers",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_touch(),
                };
                Some(Cell::new(n, floors, move || (a.covers(&b), a, b)))
            },
        },
        Op {
            name: "party_disjoint",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, b, n) = f.party_pair()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n, floors, move || (a.is_disjoint(&b), a, b)))
            },
        },
        Op {
            name: "party_without",
            group: OpGroup::Party,
            prepare: |f| {
                let (_, b, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(_, b)| b.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::new(n + 1, floors, move || {
                    (Party::seed().without(&b), b)
                }))
            },
        },
        Op {
            name: "party_display",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let spec = TextSpec {
                    radix_units: radix_units_party(&a),
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Party)>()
                            .expect("the display body yields (text, party)")
                            .0
                            .len()
                    },
                    spec,
                    move || (a.to_string(), a),
                ))
            },
        },
        Op {
            name: "party_from_str",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let s = a.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_party(&a),
                    output_is_text: false,
                };
                assert_honest_text("party_from_str input", s.len(), spec.radix_units);
                let packed = a.encoded_bits().div_ceil(8);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        r.downcast_ref::<Party>()
                            .expect("the parse body yields a party")
                            .encoded_bits()
                            .div_ceil(8)
                    },
                    spec,
                    move || s.parse::<Party>().expect("a displayed party parses back"),
                ))
            },
        },
        Op {
            name: "party_hash",
            group: OpGroup::Party,
            prepare: |f| {
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_ID_TREE),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    a.hash(&mut hasher);
                    (hasher.finish(), a)
                }))
            },
        },
        // ── Clock ──────────────────────────────────────────────────────
        Op {
            name: "clock_decode",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let bytes = clock.encode();
                let floors = Floors {
                    heap: heap_materializes(bytes.len()),
                    limb: limb_wide(mandatory_limbs_version(clock.version())),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(bytes.len()),
                };
                Some(Cell::new(bytes.len(), floors, move || {
                    Clock::decode(&bytes[..]).expect("an encoded clock decodes back")
                }))
            },
        },
        Op {
            name: "clock_encode",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                };
                Some(Cell::new(n, floors, move || (clock.encode(), clock)))
            },
        },
        Op {
            name: "clock_tick",
            group: OpGroup::Tick,
            prepare: |f| {
                // The tick-walk families tick their own (id, event)
                // clock; they reach no other clock row.
                if let Some((v, p, n)) = f.cross() {
                    let floors = tick_walk_floors(&v, n);
                    let mut clock = Clock::from_parts(p, v);
                    return Some(Cell::new(n, floors, move || {
                        clock.tick();
                        clock
                    }));
                }
                let (mut clock, n) = f.clock()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    clock.tick();
                    clock
                }))
            },
        },
        Op {
            name: "clock_fork",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut clock, n) = f.clock()?;
                let floors = Floors {
                    heap: heap_fork_child(clock.version()),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: if clock.party().is_seed() {
                        na(NA_SCAN_SEED_PARTY)
                    } else {
                        scan_touch()
                    },
                };
                Some(Cell::new(n, floors, move || {
                    let child = clock.fork();
                    (clock, child)
                }))
            },
        },
        Op {
            name: "clock_join",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut a, b, n) = f.clock_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    let joined = a.join(b).is_ok();
                    (joined, a)
                }))
            },
        },
        Op {
            name: "clock_sync",
            group: OpGroup::Clock,
            prepare: |f| {
                let (mut a, mut b, n) = f.clock_pair()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    let synced = a.sync(&mut b).is_ok();
                    (synced, a, b)
                }))
            },
        },
        Op {
            name: "clock_recv",
            group: OpGroup::Clock,
            prepare: |f| {
                // Small clock × adversarial received version.
                if let Some((v, n)) = f.version() {
                    let mut clock = Clock::seed();
                    return Some(Cell::new(n + 2, walk_floors(n), move || {
                        clock.recv(&v);
                        (clock, v)
                    }));
                }
                // Adversarial party × small received version.
                let (a, _, _) = f.party_pair()?;
                let n = f.parties.as_ref().map(|(a, _)| a.len())?;
                let mut clock = Clock::from_parts(a, Version::new());
                let msg = Version::try_from(1u64).expect("a one-tick version is valid");
                Some(Cell::new(n + 2, walk_floors(n), move || {
                    clock.recv(&msg);
                    (clock, msg)
                }))
            },
        },
        Op {
            name: "clock_own_version",
            group: OpGroup::Projection,
            prepare: |f| {
                // Adversarial × adversarial with mandatory dominating
                // output: a clock holding the cross's event side whose
                // party is its id side, I/O-denominated (the module doc's
                // output-domination cross).
                if f.output_dominated {
                    let (v_bytes, p_bytes) = f.cross.as_ref()?;
                    let n = v_bytes.len() + p_bytes.len();
                    let clock = Clock::from_parts(decode_party(p_bytes), decode_version(v_bytes));
                    return Some(Cell::io(
                        n,
                        walk_floors(n),
                        |r| {
                            let (out, _) = r
                                .downcast_ref::<(Version, Clock)>()
                                .expect("the own_version body yields (out, clock)");
                            version_output_bytes(out)
                        },
                        move || (clock.own_version(), clock),
                    ));
                }
                let (clock, n) = f.clock()?;
                Some(Cell::new(n, walk_floors(n), move || {
                    (clock.own_version(), clock)
                }))
            },
        },
        Op {
            name: "clock_display",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let spec = TextSpec {
                    radix_units: radix_units_clock(&clock),
                    output_is_text: true,
                };
                let floors = Floors {
                    heap: heap_materializes(n),
                    limb: na(NA_LIMB_DEPENDENCY),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(n),
                };
                Some(Cell::text(
                    n,
                    floors,
                    |r| {
                        r.downcast_ref::<(String, Clock)>()
                            .expect("the display body yields (text, clock)")
                            .0
                            .len()
                    },
                    spec,
                    move || (clock.to_string(), clock),
                ))
            },
        },
        Op {
            name: "clock_from_str",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, _) = f.clock()?;
                let s = clock.to_string();
                let spec = TextSpec {
                    radix_units: radix_units_clock(&clock),
                    output_is_text: false,
                };
                assert_honest_text("clock_from_str input", s.len(), spec.radix_units);
                let packed = clock.encoded_bits().div_ceil(8);
                let floors = Floors {
                    heap: heap_materializes(packed),
                    limb: limb_wide(mandatory_limbs_version(clock.version())),
                    segments: seg_ceiling_only(),
                    scan: scan_examines(packed),
                };
                Some(Cell::text(
                    s.len(),
                    floors,
                    |r| {
                        r.downcast_ref::<Clock>()
                            .expect("the parse body yields a clock")
                            .encoded_bits()
                            .div_ceil(8)
                    },
                    spec,
                    move || s.parse::<Clock>().expect("a displayed clock parses back"),
                ))
            },
        },
        Op {
            name: "clock_hash",
            group: OpGroup::Clock,
            prepare: |f| {
                let (clock, n) = f.clock()?;
                let floors = Floors {
                    heap: na(NA_HEAP_IN_PLACE),
                    limb: na(NA_LIMB_NOT_FORCED),
                    segments: seg_ceiling_only(),
                    scan: na(NA_SCAN_BYTE_COPY),
                };
                Some(Cell::new(n, floors, move || {
                    let mut hasher = DefaultHasher::new();
                    clock.hash(&mut hasher);
                    (hasher.finish(), clock)
                }))
            },
        },
    ]
}

// ─── measurement ────────────────────────────────────────────────────────────

/// One measured run of a cell body: every meter and its denominators.
struct Sample {
    /// The denominator of every column's exponent and of the heap and
    /// segment constants: packed input bytes, or `n_io` for the
    /// I/O-denominated cells.
    denom_bytes: usize,
    /// The limb *constant*'s denominator: `denom_bytes`, or `R` for the
    /// text rows (the limb exponent is judged against `denom_bytes` on
    /// every row).
    limb_denom: u64,
    /// Whether the limb column is judged at the text ceiling κ.
    text_row: bool,
    /// The cell's liveness declarations; each sample carries its own since
    /// floors scale with the sample's operands.
    floors: Floors,
    /// Every currency's counter reading over the body; `None` where the
    /// counter is not compiled in (the feature-gated limb and scan
    /// columns render `off` and are exempt from judgment).
    readings: ByCurrency<Option<u64>>,
}

/// Run one prepared cell under all meters.
///
/// The denominators are settled after the meters are read and before the
/// result is dropped: an I/O-denominated cell's output side comes from the
/// actual result (never from a prediction), and a text output is checked
/// against the honesty ceiling right here.
fn measure(heap: &HeapMeter, op: &'static str, cell: Cell) -> Sample {
    super::reset_stack_segments();
    reset_limb();
    reset_scan();
    (heap.reset_peak)();
    let baseline = (heap.current)();
    let result = (cell.body)();
    let peak_heap = (heap.peak)().saturating_sub(baseline);
    let segments = super::stack_segments();
    let limb = read_limb();
    let scan = read_scan();
    let (denom_bytes, limb_denom, text_row) = match cell.denom {
        Denom::Input => (cell.input_bytes, cell.input_bytes as u64, false),
        Denom::Io(spec) => {
            let output_bytes = (spec.output_bytes)(result.as_ref());
            let n_io = cell.input_bytes + output_bytes;
            match spec.text {
                None => (n_io, n_io as u64, false),
                Some(text) => {
                    if text.output_is_text {
                        assert_honest_text(op, output_bytes, text.radix_units);
                    }
                    (n_io, n_io as u64 + text.radix_units, true)
                }
            }
        }
    };
    drop(result);
    Sample {
        denom_bytes,
        limb_denom,
        text_row,
        floors: cell.floors,
        readings: ByCurrency {
            heap: Some(peak_heap as u64),
            segments: Some(segments),
            limb,
            scan,
        },
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

/// Reset the scan counter when the `scan-meter` feature carries one.
#[cfg(feature = "scan-meter")]
fn reset_scan() {
    super::reset_scan_bits();
}

/// Without the `scan-meter` feature there is no counter to reset.
#[cfg(not(feature = "scan-meter"))]
fn reset_scan() {}

/// Read the scan counter, or `None` without the `scan-meter` feature.
#[cfg(feature = "scan-meter")]
fn read_scan() -> Option<u64> {
    Some(super::scan_bits())
}

/// Without the `scan-meter` feature the scan column is absent.
#[cfg(not(feature = "scan-meter"))]
fn read_scan() -> Option<u64> {
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

/// A liveness-floor trip's rendered message: the column and the vacuity
/// mechanism.
const HEAP_FLOOR_TRIP: &str =
    "heap floor: counter reads below floor: the meter is not watching this work";
/// The segments column's floor-trip message (unreachable while segments is
/// ceiling-only by policy; the judgment loop still carries it so a future
/// segments floor binds without a code change).
const SEG_FLOOR_TRIP: &str =
    "segments floor: counter reads below floor: the meter is not watching this work";
/// The limb column's floor-trip message.
const LIMB_FLOOR_TRIP: &str =
    "limb floor: counter reads below floor: the meter is not watching this work";
/// The scan column's floor-trip message.
const SCAN_FLOOR_TRIP: &str =
    "scan floor: counter reads below floor: the meter is not watching this work";

/// Whether `count` sits below a committed floor (an NA declaration never
/// trips).
fn below_floor(liveness: Liveness, count: u64) -> bool {
    match liveness {
        Liveness::Floor { min, .. } => count < min,
        Liveness::NotApplicable { .. } => false,
    }
}

/// Panic unless two in-process measurements of one cell agree on every
/// counter reading and denominator.
///
/// The in-process leg of the board's determinism tripwire ([`run`]'s
/// self-verification); the cross-process leg is the `amp-board-determinism`
/// recipe, which byte-compares two whole renders.
fn assert_deterministic(op: &str, family: &str, a: &Sample, b: &Sample) {
    assert_eq!(
        (a.denom_bytes, a.limb_denom),
        (b.denom_bytes, b.limb_denom),
        "determinism: {op} x {family}: two in-process measurements disagree on denominators"
    );
    for ((currency, first), (_, second)) in a.readings.each().into_iter().zip(b.readings.each()) {
        assert_eq!(
            first,
            second,
            "determinism: {op} x {family}: two in-process measurements disagree on the {} \
             counter",
            currency.label()
        );
    }
}

/// One judged column's derived scores: the fitted exponent and the larger
/// scale's per-unit constant (`None` where the counter is off).
#[derive(Clone, Copy)]
struct Score {
    exp: Option<f64>,
    per_unit: Option<f64>,
}

/// One evaluated cell: both samples, per-currency scores, and the verdict.
struct CellResult {
    op: &'static str,
    family: &'static str,
    s1: Sample,
    s2: Sample,
    scores: ByCurrency<Score>,
    /// The meters over their bounds; empty means green.
    red: Vec<&'static str>,
}

/// Score a cell's two samples against the exponent bound, the ceilings,
/// and the liveness floors, per currency.
///
/// Every exponent — the limb column's included — is judged against the
/// denominator bytes (packed input, or `n_io` on the I/O-denominated
/// cells), never against `R`: `R` is the schoolbook cost law, so a limb
/// exponent against it reads a flat ~1 on exactly the quadratic converters
/// the bound exists to catch. Constants are judged per denominator byte,
/// except segments (an absolute count: the target is walks that never grow
/// the stack) and the text rows' limb constant, which is per `R` unit
/// under the κ ceiling. The loops run over the currency axis itself
/// ([`ByCurrency::each`]), so a currency added to the axis is judged on
/// every cell or the destructuring fails to compile.
fn evaluate(op: &'static str, family: &'static str, s1: Sample, s2: Sample) -> CellResult {
    let score = |c: Currency| -> Score {
        let (Some(m1), Some(m2)) = (*s1.readings.get(c), *s2.readings.get(c)) else {
            return Score {
                exp: None,
                per_unit: None,
            };
        };
        let exp = exponent(m1, m2, s1.denom_bytes, s2.denom_bytes);
        let per_unit = match c {
            Currency::Heap => {
                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
            }
            Currency::Segments => m2 as f64,
            Currency::Limb => m2 as f64 / s2.limb_denom as f64,
            Currency::Scan => m2 as f64 / s2.denom_bytes as f64,
        };
        Score {
            exp: Some(exp),
            per_unit: Some(per_unit),
        }
    };
    let scores = ByCurrency {
        heap: score(Currency::Heap),
        segments: score(Currency::Segments),
        limb: score(Currency::Limb),
        scan: score(Currency::Scan),
    };

    let mut red = Vec::new();
    for (c, s) in scores.each() {
        let (ceiling, exp_label, const_label) = match c {
            Currency::Heap => (
                MAX_HEAP_BYTES_PER_INPUT_BYTE,
                "heap exponent",
                "heap constant",
            ),
            Currency::Segments => (
                MAX_GROWN_STACK_SEGMENTS as f64,
                "segments exponent",
                "segments count",
            ),
            Currency::Limb => (
                if s2.text_row {
                    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT
                } else {
                    MAX_LIMB_OPS_PER_INPUT_BYTE
                },
                "limb exponent",
                "limb constant",
            ),
            Currency::Scan => (
                MAX_SCAN_BITS_PER_INPUT_BYTE,
                "scan exponent",
                "scan constant",
            ),
        };
        if s.exp.is_some_and(|e| e > MAX_SCALING_EXPONENT) {
            red.push(exp_label);
        }
        if s.per_unit.is_some_and(|v| v > ceiling) {
            red.push(const_label);
        }
    }
    // The liveness floors bind in this same pass, at both scales: a counter
    // reading below the least a watching meter could honestly read means the
    // meter is not watching the work the ceilings claim to bound.
    for (c, _) in scores.each() {
        let trip = match c {
            Currency::Heap => HEAP_FLOOR_TRIP,
            Currency::Segments => SEG_FLOOR_TRIP,
            Currency::Limb => LIMB_FLOOR_TRIP,
            Currency::Scan => SCAN_FLOOR_TRIP,
        };
        if [&s1, &s2].iter().any(|s| {
            s.readings
                .get(c)
                .is_some_and(|r| below_floor(*s.floors.get(c), r))
        }) {
            red.push(trip);
        }
    }

    CellResult {
        op,
        family,
        s1,
        s2,
        scores,
        red,
    }
}

// ─── rendering ──────────────────────────────────────────────────────────────

/// Render one liveness declaration's floor value: the committed minimum, or
/// `-` for a not-applicable column.
fn floor_value(liveness: Liveness) -> String {
    match liveness {
        Liveness::Floor { min, .. } => min.to_string(),
        Liveness::NotApplicable { .. } => "-".to_string(),
    }
}

/// Render one result row.
///
/// The byte range is the cell's denominator (packed input, or `n_io` on the
/// I/O-denominated cells); a text row's limb constant reads `/R` (its
/// exponent, like every exponent, is against the denominator bytes),
/// everything else `/B`. The `flr` column shows the larger scale's committed
/// liveness floors per judged column (`-` where not applicable; derivations
/// in the legend above the matrix).
fn row(out: &mut dyn Write, r: &CellResult) -> io::Result<()> {
    let verdict = if r.red.is_empty() { "GREEN" } else { "RED" };
    let limb = match (r.scores.limb.exp, r.scores.limb.per_unit) {
        (Some(e), Some(c)) => {
            let unit = if r.s2.text_row { "/R" } else { "/B" };
            format!("limb[e{e:5.2} {c:>10.1}{unit}]")
        }
        _ => "limb[      off      ]".to_string(),
    };
    let scan = match (r.scores.scan.exp, r.scores.scan.per_unit) {
        (Some(e), Some(c)) => format!("scan[e{e:5.2} {c:>10.1}/B]"),
        _ => "scan[      off      ]".to_string(),
    };
    let reasons = if r.red.is_empty() {
        String::new()
    } else {
        format!("  <- {}", r.red.join(", "))
    };
    writeln!(
        out,
        "{verdict:<5} {op:<24} {family:<12} {n1:>8}->{n2:<8} B  \
         heap[e{he:5.2} {hc:>10.1}/B]  seg[e{se:5.2} {sc:>4}]  {limb}  {scan}  \
         flr[h {fh:>6} l {fl:>6} s {fs:>6}]{reasons}",
        op = r.op,
        family = r.family,
        n1 = r.s1.denom_bytes,
        n2 = r.s2.denom_bytes,
        he = r.scores.heap.exp.unwrap_or(0.0),
        hc = r.scores.heap.per_unit.unwrap_or(0.0),
        se = r.scores.segments.exp.unwrap_or(0.0),
        sc = r.s2.readings.segments.unwrap_or(0),
        fh = floor_value(r.s2.floors.heap),
        fl = floor_value(r.s2.floors.limb),
        fs = floor_value(r.s2.floors.scan),
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
            let s1 = measure(heap, op.name, c1);
            let s2 = measure(heap, op.name, c2);
            // The runner self-verifies: every cell is measured twice in
            // process and every counter reading and denominator must
            // agree exactly — the board's judged quantities are
            // deterministic domain counters, so any disagreement is a
            // nondeterminism bug in a meter or a body, stopped here
            // rather than laundered into a verdict.
            for (level, first) in [(small, &s1), (large, &s2)] {
                let again = (op.prepare)(level)
                    .expect("a cell's applicability depends on the family, never the size");
                assert_deterministic(op.name, small.name, first, &measure(heap, op.name, again));
            }
            results.push(evaluate(op.name, small.name, s1, s2));
        }
    }

    writeln!(
        out,
        "amplification board: transient cost vs denominator bytes (packed input; total I/O on \
         the text and cross cells), each cell at two scales"
    )?;
    writeln!(
        out,
        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
         heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \
         segments <= {MAX_GROWN_STACK_SEGMENTS}, \
         limb <= {MAX_LIMB_OPS_PER_INPUT_BYTE} ops/B \
         (text rows: <= {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT} ops/R), \
         scan <= {MAX_SCAN_BITS_PER_INPUT_BYTE} bits/B; \
         and every committed liveness floor met (flr[...]: a counter below its floor is red: \
         the meter is not watching that work; segments is ceiling-only by policy, its honest \
         floor is zero). every judged quantity is a deterministic counter: the time-exponent \
         leg lives in the bench judge (just bench-judge)"
    )?;
    writeln!(out)?;
    writeln!(out, "liveness declarations on this board:")?;
    let mut legend = std::collections::BTreeSet::new();
    for r in &results {
        for (currency, liveness) in r.s2.floors.each() {
            legend.insert(match liveness {
                Liveness::Floor { why, .. } => format!("  {} floor: {why}", currency.label()),
                Liveness::NotApplicable { reason } => {
                    format!("  {} n/a: {reason}", currency.label())
                }
            });
        }
    }
    for line in &legend {
        writeln!(out, "{line}")?;
    }
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

// ─── the bench export ───────────────────────────────────────────────────────

/// One board cell exposed for wall-clock benchmarking.
///
/// The bench suite (`benches/board.rs`) is the board's wall-time shadow: its
/// criterion group and function IDs are exactly [`BenchCell::op`] and
/// [`BenchCell::family`], so a board cell names the bench that times it and
/// a criterion filter selects a cell. The bench judge (`tools/benchjudge`)
/// reads those same IDs out of criterion's saved estimates to run the time
/// leg the module doc describes.
pub struct BenchCell {
    /// The board row's operation name: the bench group ID.
    pub op: &'static str,
    /// The input family's name: the bench function ID within the group.
    pub family: &'static str,
    /// The family operands the row's prepare reads, shared across cells.
    data: Rc<FamilyData>,
    /// The board row's prepare, re-run per measured body.
    prepare: fn(&FamilyData) -> Option<Cell>,
}

impl BenchCell {
    /// Build one fresh run of the cell's measured body.
    ///
    /// Operands are decoded anew on every call — the board's prepare
    /// discipline — so a bench harness rebuilds destructive operands in its
    /// untimed setup and times the returned closure alone. The closure is
    /// exactly what the board meters: same operands, same operation, same
    /// kept-alive result.
    pub fn body(&self) -> Box<dyn FnOnce() -> Box<dyn Any>> {
        (self.prepare)(&self.data)
            .expect("cell applicability was settled at construction")
            .body
    }

    /// The cell's denominator bytes at its scale: packed input, or total
    /// I/O on the I/O-denominated rows.
    ///
    /// Runs one untimed body to read the output side back from the actual
    /// result, exactly as the board's measurement does (a prediction never
    /// substitutes for the result, and a text output is checked against the
    /// honesty ceiling on the way). The bench judge denominates its fitted
    /// time exponents against these bytes — the board's own convention — so
    /// a family whose packed bytes grow faster than the scale knob (the
    /// cliff comb's value content is quadratic in its parameter) is judged
    /// against what the operation actually reads and writes, never against
    /// the knob.
    pub fn denominator_bytes(&self) -> usize {
        let cell =
            (self.prepare)(&self.data).expect("cell applicability was settled at construction");
        let result = (cell.body)();
        match cell.denom {
            Denom::Input => cell.input_bytes,
            Denom::Io(spec) => {
                let output_bytes = (spec.output_bytes)(result.as_ref());
                if let Some(text) = spec.text {
                    if text.output_is_text {
                        assert_honest_text(self.op, output_bytes, text.radix_units);
                    }
                }
                cell.input_bytes + output_bytes
            }
        }
    }
}

/// Which slice of the shape × operation product a bench run times.
///
/// The deterministic board always runs the whole product (its cells are
/// cheap counters); wall-clock benching is not, so the mirror has two
/// modes, both derived from the same axis declarations — the subset is a
/// rule over the product, never a second hand-maintained cell list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchMode {
    /// The pinned rule-derived subset: every operation on the benign
    /// control, each shape's designed-stress pairings (declared per
    /// shape on the shape axis), and the board-red riders
    /// ([`BOARD_RED_BENCH_RIDERS`]).
    Pinned,
    /// The whole product: the mode for final verdicts.
    Full,
}

/// Board-red cells outside the designed pairings that the pinned bench
/// subset must still time: the deterministic board's standing reds each
/// keep a time leg.
///
/// Membership is by `(operation, family)` cell name, expectations live in
/// the judge's roster as ever; a red cured on the board leaves this list
/// in the same change that cures it.
pub const BOARD_RED_BENCH_RIDERS: &[(&str, &str)] = &[];

/// Every board cell of the chosen [`BenchMode`] at `scale`, in board row
/// order.
///
/// `scale` multiplies the family base sizes exactly as [`run`]'s does; the
/// cells are op × family pairings applicable at that scale, at one
/// measurement level (a bench varies repetition, not size).
///
/// # Panics
///
/// Panics if `scale` is not a strictly positive finite number.
pub fn bench_cells(scale: f64, mode: BenchMode) -> Vec<BenchCell> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "bench cells: scale must be a positive finite number"
    );
    let families: Vec<Rc<FamilyData>> = FAMILIES
        .iter()
        .map(|&kind| Rc::new(FamilyData::build(kind, scale, 0)))
        .collect();
    let mut cells = Vec::new();
    for op in ops() {
        for family in &families {
            let include = match mode {
                BenchMode::Full => true,
                BenchMode::Pinned => {
                    designed(family.kind, op.group)
                        || BOARD_RED_BENCH_RIDERS.contains(&(op.name, family.name))
                }
            };
            if include && (op.prepare)(family).is_some() {
                cells.push(BenchCell {
                    op: op.name,
                    family: family.name,
                    data: Rc::clone(family),
                    prepare: op.prepare,
                });
            }
        }
    }
    cells
}
