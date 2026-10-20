//! Deterministic resource checks across public operations and adversarial input
//! families.
//!
//! Each family supplies a related set of operands. Every operation that can use
//! those operands becomes a board cell, so adding a family or operation expands
//! coverage without hand-writing individual pairings. The compiler-derived
//! surface check requires every public function to be either measured or
//! explicitly inapplicable. It also pins every public trait implementation;
//! the coverage table groups those implementations into reviewable families.
//!
//! # What a cell measures
//!
//! A cell runs at increasing input scales and records:
//!
//! - peak additional live heap bytes, including retained results;
//! - bits scanned or written; and
//! - accumulator digits touched.
//!
//! The last two counters are feature-gated because they instrument hot
//! primitives. They cover work that allocation measurements cannot
//! see: streaming traversal and arithmetic over wide running values.
//!
//! For each quantity, the board fits a log-log scaling trend across the measured
//! sizes and checks the largest per-byte cost. A cell is green only when every
//! trend and constant stays within its ceiling. The acceptance judgment uses
//! the complete measurement ladder; a single-scale run is diagnostic only.
//! Additional small-input samples check ceilings and liveness without fitting
//! growth through minimum allocation sizes. Every sample checks its full heap
//! peak against `I + C * D`: `I` is [`HEAP_INTERCEPT_BYTES`], `C` defaults to
//! [`MAX_HEAP_BYTES_PER_INPUT_BYTE`], and `D` is the row's declared constant
//! units. Query evaluation has a separate coefficient
//! ([`QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE`]) because it keeps one cursor
//! and comparison state per bound. No bytes are subtracted from reported
//! readings.
//!
//! The heap growth fit has a resolution of [`HEAP_TREND_RESOLUTION_BYTES`] to
//! accommodate allocation thresholds on the finite ladder. It cannot detect growth entirely below
//! that level. These empirical checks are regression evidence over the sampled
//! inputs, not proofs of asymptotic or universal bounds. Deep-tree tests check
//! stack safety directly.
//!
//! Operand preparation and the board's result containers precede the heap
//! baseline. Measuring the maximum live heap above that baseline includes
//! operation scratch and newly allocated results; it does not separate them.
//!
//! # The fuel board
//!
//! The same cells also run inside a wasm guest under Wasmtime fuel metering
//! (the tool-side `board-fuel` crates under this one; `just amp-board-fuel`),
//! where a fourth currency, *fuel*, counts every wasm instruction the cell
//! body executes. Heap, scan, and touch see only the work that passes through
//! their probes; fuel also sees element moves inside a `Vec`, arithmetic
//! inside `num-bigint`, and buffer fills, which none of them counts. The guest
//! builds without the scan and touch probes and installs no counting
//! allocator, so fuel prices the production code alone, and a fuel run judges
//! fuel alone. The native board stays the measure of record for heap, scan,
//! and touch.
//!
//! Fuel is judged by the same exponent bound and declared models as every
//! other currency. Its proportional ceilings are calibrated per cell
//! ([`CellCeilings`]), because fuel per byte differs between cells by orders
//! of magnitude. A fuel reading also depends on the allocator state that
//! earlier cells leave in the same instance, so a fuel run measures each cell
//! in a fresh guest instance; the `shard` module's docs give the deal.
//!
//! # The verdict of record
//!
//! One [`Ladder`] sweep measures every cell at each sampling scale, and
//! everything the board decides about a tree is read from it:
//! [`Ladder::judge`] renders the acceptance matrices, then entry-compares the
//! worst-case ranking pin, and returns both results as one [`Verdict`];
//! [`capture`] writes the same sweep's exact readings for a later
//! [`compare()`]. The readings are deterministic counters, so sweeping again
//! for each consumer would only repeat the same measurements.
//!
//! A [`Ladder`] can only be swept, never read from a file, so every verdict
//! judges the tree that ran it. A capture is an input to [`compare()`] alone,
//! which reports differences and judges nothing.
//!
//! # Counter liveness
//!
//! An internal counter could read zero because the implementation bypassed its
//! probes. Each cell therefore declares either a justified minimum reading or
//! why no nonzero minimum exists. Falling below that floor is a failure, just like
//! exceeding a ceiling. Floors detect a completely bypassed counter; they are
//! not proofs that every relevant instruction was counted.
//!
//! # Cost models
//!
//! Most cells divide cost by input bytes. An operation whose required output
//! can dominate its input instead uses total input and output bytes; the
//! measurement reads the actual result size.
//!
//! Each measured quantity defaults to a linear model in those bytes. A cell may
//! state a more precise expected bound by supplying the units used for its
//! growth trend and proportional constant. For example, a balanced reduction
//! uses input bytes times its logarithmic depth. The same judge handles every
//! model: measured work must remain linear in the stated units, within the
//! currency's ceiling. An optional cell-specific ceiling changes only the
//! proportional check. Every override is visible in the rendered row and needs
//! a derivation beside the operation that supplies it.
//!
//! Rejection paths are measured too, with malformed data placed as late as the
//! generator can arrange so an early exit cannot make the result misleading.
//!
//! # Interpreting results
//!
//! Heap accounting is process-global, so each shard measures one cell at a
//! time. Acceptance uses release builds so debug assertions do not add
//! instrumented work to production measurements.
//!
//! # Target dependence
//!
//! Every reading is deterministic for a given build target. The native board's
//! heap, scan, and touch ceilings and its worst-case pin are taken and checked
//! on 64-bit hosts; fuel is taken and checked in a wasm32 guest, whose readings
//! depend on the guest's bytes and not on the host that runs it. A native
//! reading differs between 64-bit hosts only when the measured operation
//! reaches a target-conditional code path that changes allocation or what the
//! scan and touch meters count. Architecture-specific code that only selects
//! instructions, such as an intrinsic or an assembly routine, changes neither.
//! For example, `num-bigint` chooses the divisor of its decimal conversion by
//! architecture, so formatting a [`Count`](crate::Count) allocates differently
//! on x86_64 than on aarch64.
//!
//! The ceilings judge such a reading on every host. Where the path also
//! changes which family reads worst, the
//! worst-case pin declares the cell target-dependent, checks only that some
//! family drives it, and names the path in a comment at the entry. The pin's
//! table (`WORST_RANKINGS`) is therefore the record of which rankings depend on
//! the target, and why.
//!
//! A 32-bit target changes heap readings broadly: `num-bigint` stores 32-bit
//! digits there, and every pointer-sized field halves.

mod ceilings;
mod cell;
mod cell_ceilings;
mod compare;
mod coverage;
mod currency;
mod defect;
mod family;
mod files;
mod floors;
mod judge;
mod ladder;
mod measure;
mod operand;
mod ops;
mod render;
mod shard;
#[cfg(test)]
mod tests;
mod worst;

pub use ceilings::{
    COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE, DEFAULT_SCALE,
    DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, FUEL_INTERCEPT,
    FUEL_TREND_RESOLUTION, HEAP_INTERCEPT_BYTES, HEAP_TREND_RESOLUTION_BYTES, LADDER_TOP_SCALE,
    MACHINE_WORD_MAGNITUDE_BITS, MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
    QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE, RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE,
    RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, SCAN_FLOOR_BITS_PER_INPUT_BYTE,
    SCAN_TOUCH_FLOOR_BITS, SMALL_INPUT_SCALE, TICKS_BOARD_COUNT,
};
pub use cell_ceilings::{
    cell_ceilings, recalibrate, write_recalibrations, CellCeilings, Recalibration,
    CEILING_RESOLUTION, CELL_CEILING_HEADROOM, FUEL_ARCHS,
};
pub use compare::{capture, capture_file, compare, source_tree, SOURCE_TREE_ENV};
pub use coverage::{BOARD_NOT_APPLICABLE, BOARD_PRICED};
pub use currency::{ByCurrency, Currency, Floors, Liveness, FUEL_FLOOR};
#[cfg(test)]
pub(crate) use family::WIDE_STEP_PAIRS_STEP_BITS;
pub use ladder::{Ladder, Verdict};
pub use measure::{FuelMeter, HeapMeter, Meters};
pub use operand::stored_nonzero_deltas;
pub use render::Summary;
pub use shard::{emit_shard, max_useful_shards, run, worst_map, ShardSpawner};
pub use worst::{NEAR_TIE_RATIO, WORST_MAP_SCALES};
