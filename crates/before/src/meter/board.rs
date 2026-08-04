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
//! - **Shapes** (the family list; the `family` module): each shape builds
//!   an *operand bundle* — a version, a disjoint party pair, a designated
//!   event × id cross, a fold population — and a uniform post-pass derives
//!   the rest (a cross shape's version is its event side; its id side
//!   becomes a party pair through the disjoint-mount adapter; every
//!   version gains its rank pair, and its ticked comparison counterpart
//!   wherever the shape did not build its own pairing). A shape reaches
//!   every operation its bundle supplies: adding a shape grows the
//!   product without naming any operation.
//! - **Operations** (the row table; the `ops` module): each row declares
//!   the bundle slots its signature consumes and prepares its cell from
//!   them alone — never from the shape's identity — so adding an
//!   operation grows the product across every shape that supplies its
//!   operands.
//! - **Currencies** ([`ByCurrency`]; the `currency` module): every judged
//!   quantity — the declarations, the readings, the scores — carries one
//!   field per metering currency, and the judgment iterates the axis
//!   itself. Adding a currency is a compile error at every declaration
//!   site until each operation answers its floor-or-NA question
//!   ([`Floors`]), so a meter can never be half-wired onto the board.
//!
//! The deterministic board always runs the whole product (the smoke test
//! pins the count); the wall-clock mirror selects with [`BenchMode`].
//!
//! # The module map
//!
//! One submodule per responsibility: `ceilings` (the pinned judgment
//! constants and the declared per-cell models), `currency` (the currency
//! axis), `family` (the shape axis: size knobs, operand bundles, mount
//! adapters), `ops` (the operation axis: the row table), `operand` (the
//! content walks the floors and denominators are stated in), `floors`
//! (the liveness vocabulary), `defect` (the rejection rows' placed
//! defects), `cell` (one prepared cell and its denomination rule),
//! `measure` (the metering engine), `judge` (scoring and verdicts),
//! `render` (the per-cell measurement discipline and the printed
//! matrix), `worst` (the worst-case map: the argmax fold over the same
//! judged cells, and its committed ranking pin), `shard` (the sweep
//! itself: the operation × family grid split across child processes,
//! each owning its own allocator, merged back in board row order),
//! `export` (the bench mirror), and `coverage` (the tiling table and the
//! red-triage buffer).
//!
//! # The criterion
//!
//! Each cell runs its operation at two input scales (the second twice the
//! first) and reads five deterministic meters over the body alone:
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
//!   this is the one column that sees it;
//! - **accumulator digit touches**
//!   ([`suanpan::touch_meter`]), only when
//!   the `limb-meter` feature compiles the counter into the accumulator:
//!   digit-state cost is work done *wider*, not more often — a walk that
//!   re-reads a wide running value per step allocates nothing extra,
//!   recurses nothing, does O(1) `Base` ops, and scans no extra bits, so
//!   this is the one deterministic column that sees the genre (the tick
//!   walk's width-circulation finding lived entirely in this currency).
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
//! [`MAX_SCAN_BITS_PER_INPUT_BYTE`], [`MAX_TOUCHES_PER_INPUT_BYTE`] — or,
//! on the text rows' limb column, [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`]), and
//! every committed liveness floor is met; **RED** otherwise, with the
//! offending meters named.
//!
//! # Liveness floors
//!
//! A ceiling over a counter proves the *instrumented* work is small, not
//! that the operation is cheap: four of the five judged columns are sensors
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
//! matrix. The per-currency derivation conventions — which floor each
//! row genre takes and the honest not-applicable genres, the rejection
//! rows' floors included — live in the `floors` module, beside the
//! constructors that commit them, along with the disclosure of the four
//! cells no deterministic leg watches.
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
//! # Determinism and the time leg
//!
//! Every quantity the board judges or renders is a deterministic counter,
//! so two board runs at the same scale are byte-identical under any
//! machine load: the board reads no clock, conditions nothing on timing,
//! and comparing runs needs no exclusion rules. Time still has its own
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
//! # Acceptance scales and the profile of record
//!
//! Every cell runs at a size scale; the inner loop uses the default
//! (scale 1, seconds of runtime). Acceptance is [`ACCEPTANCE_SCALE`]'s rule —
//! that constant owns the ×4 calibration argument and the both-scales
//! requirement — plus the bench judge green across the same two scales;
//! the enforced per-operation record remains the process-isolated
//! envelope suite in `tests/meter.rs` throughout.
//!
//! Readings of record come from the **release profile** (the `amp-board*`
//! recipes): debug assertions perform metered work — `Base` comparisons
//! through the limb shim, probe cursors that record scan bits — so a dev
//! board measures the algorithm plus its verification scaffolding, while
//! release measures the production work alone, the honest denominator. A
//! dev run remains a legitimate debugging view (the assertions themselves
//! are live there); its readings must never be pinned *on this board*.
//! (The envelope suite pins dev-profile numbers deliberately — the
//! stricter reading for its scenarios; `tests/meter.rs` argues the
//! choice. The two profiles of record differ because the two instruments
//! ask different questions.)
//!
//! # Denomination
//!
//! Most cells charge cost against **packed input bytes** alone. Two cell
//! classes have *mandatory* output asymptotically larger than any constant
//! times their input, so an input-only ceiling on them is unsatisfiable by
//! construction — and an unsatisfiable criterion degenerates into exemption
//! holes. Those cells are denominated against **total I/O bytes** `n_io`,
//! with the output side read back from the operation's actual result, never
//! assumed from its inputs. The re-denominated classes are the text rows
//! (limb constant per radix-work unit `R`), the output-dominated
//! projection crosses (packed I/O), and — exponents alone — the
//! flat-denominator comb-scatter shape, fitted against value content;
//! each class's derivation, the do-not-re-denominate list, the
//! output-honesty ceiling, and the rank rows' value-content denominator
//! live in the `cell` module, on the denomination rule itself.
//!
//! # Declared per-cell models
//!
//! Some cells are judged against a **declared model** — a ratified
//! cost law derived at the cell, with a dated owner rationale committed
//! at the declaring constant — in place of one global ceiling, because
//! the global form is unsatisfiable on work their contracts mandate (the
//! same reasoning that re-denominates the I/O cells). A modeled cell
//! reads green because its behavior is *intended and modeled*; red is
//! reserved for untriaged contradictions (the red-triage buffer,
//! [`BOARD_EXPECTED_REDS`], is empty on the settled tree). Each model is
//! disclosed on its row face (`decl[...]`), derived at its constant's
//! definition site (the `ceilings` module's declared-models section),
//! and held honest on the under side — banded floors where the model
//! predicts a quantity, committed liveness pins where it declares a
//! class — so an improved kernel forces a deliberate re-declaration,
//! and tripwired in the test suite by a wrong artifact reading red. The
//! ratified models — the fold rows' `O(D log k)` reduction, the
//! capacity-chain heap band, the family-stated heap ceilings, and the
//! mirror-wide render limb model — are each derived and priced in the
//! `ceilings` module, at their declaring constants.
//!
//! # The rejection surface
//!
//! Cost claims are total: rejecting an input is an outcome with a cost,
//! bounded like any other, whether or not the caller honored the usage
//! invariants. The rejection rows price the fallible surface — overlap,
//! the empty difference, strict decode, and text parse — with the defect
//! **maximally deferred** in every shape: an early-exit-only measurement
//! would be the cheapest artifact that passes. The `defect` module builds
//! each placed defect and derives its placement; rejections produce no
//! output, so every rejection row is denominated against the fed stream
//! alone, and the `coverage` module records the fallible surface's
//! bounded-or-delegated remainder.
//!
//! # Reading the numbers
//!
//! Each child sweeps single-threaded — one cell at a time, the peak-heap
//! counter reset between cells, its whole grid slice sharing one
//! process's allocator — so a cell's heap number can include allocator
//! noise from the harness itself: the board's numbers are *indicative*.
//! The enforced
//! record is the meter test binary (`tests/meter.rs`), whose scenarios run
//! one per process under nextest and pin exact envelopes. Zero-measurement
//! cells score exponent 0; a meter that moves from 0 to a nonzero count is
//! clamped through `max(m, 1)` before the ratio, so the exponent stays
//! finite.
//!
//! # Families
//!
//! Every family reaches every operation its operand bundle supplies (the
//! product section above). The roster of record is the registry's
//! [`FamilyId`](crate::meter::registry::FamilyId) — the board's axis is
//! `FamilyId::board()`, the roster filtered on each variant's committed
//! coverage answer — with the per-family genre notes on the variants and
//! the carrier classes and operand bundles in the `family` module. The
//! board's columns are deliberately narrower than the registry: a family
//! earns a column only as a whole-surface adversary, while kernel-seam
//! probes live in the envelope suite alone, each with its dated
//! envelope-only ruling on its registry row.
//!
//! # Coverage: the board tiling
//!
//! Every public operation — every row of `before::surface`'s method and
//! family rosters — either is priced by at least one board row (its
//! claim in the complexity-claims roster cites the rows by name) or
//! appears in [`BOARD_NOT_APPLICABLE`] with the mechanism-based reason
//! it has no meaningful adversarial operand of its own. The two sides
//! are disjoint and jointly total, enforced by the tiling test in the
//! complexity-claims suite (`board_coverage_tiles_the_public_surface`),
//! so a new public operation cannot land unpriced and unexcused.
//!
//! The wall-time mirror rides the same axes: the bench suite's criterion
//! IDs are exactly the board's op × family cell names ([`bench_cells`] is
//! the board's own table), so board coverage is bench coverage cell for
//! cell, with no second enumeration (the `export` module derives the
//! judged subset). Which surface rows ride which delegated mechanisms,
//! and the error-path dispositions the table cannot carry, are recorded
//! in the `coverage` module beside the table itself.

mod ceilings;
mod cell;
mod coverage;
mod currency;
mod defect;
mod export;
mod family;
mod floors;
mod judge;
mod measure;
mod operand;
mod ops;
mod render;
mod shard;
#[cfg(test)]
mod tests;
mod worst;

pub use ceilings::{
    ACCEPTANCE_SCALE, ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE,
    ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE, CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR,
    FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, HEAP_FLAT_ALLOWANCE_BYTES, INDEX_PROBE_SCAN_BITS,
    MACHINE_WORD_MAGNITUDE_BITS, MAX_GROWN_STACK_SEGMENTS, MAX_HEAP_BYTES_PER_INPUT_BYTE,
    MAX_LIMB_OPS_PER_INPUT_BYTE, MAX_SCALING_EXPONENT, MAX_SCAN_BITS_PER_INPUT_BYTE,
    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
    MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING, MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT,
    SCAN_FLOOR_BITS_PER_INPUT_BYTE, SCAN_TOUCH_FLOOR_BITS, TEXT_BYTES_PER_RADIX_UNIT,
    TEXT_PIPELINE_LIMB_OPS_PER_VALUE, TICKS_BOARD_COUNT,
};
pub use coverage::{ExpectedRed, BOARD_EXPECTED_REDS, BOARD_NOT_APPLICABLE};
pub use currency::{ByCurrency, Currency, Floors, Liveness};
pub use export::{bench_cells, BenchCell, BenchMode, BOARD_DECLARED_BENCH_RIDERS};
pub use family::study_family_versions;
pub use measure::HeapMeter;
pub use render::Summary;
pub use shard::{check_worst_map, emit_shard, max_useful_shards, run, worst_map, ShardSpawner};
pub use worst::{NEAR_TIE_RATIO, WORST_MAP_SCALES};
