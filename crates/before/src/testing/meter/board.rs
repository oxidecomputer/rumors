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
//! A cell runs at increasing input scales and records its peak additional
//! live heap bytes, including retained results. The fuel board (below)
//! measures the same cells' execution in Wasmtime fuel.
//!
//! For each quantity, the board fits a log-log scaling trend across the measured
//! sizes and checks the largest per-byte cost. A cell is green only when every
//! trend and constant stays within its ceiling. The acceptance judgment uses
//! the complete measurement ladder; a single-scale run is diagnostic only.
//! Two more pairs of samples check ceilings and liveness without entering the
//! growth fit: small inputs, where minimum allocation sizes dominate, and
//! inputs whose size knobs sit just past a power of two, where a buffer that
//! grows by doubling holds its peak ([`Sampling`]). Every sample checks its full heap
//! peak against `I + C * D`: `I` is [`HEAP_INTERCEPT_BYTES`], `C` defaults to
//! [`MAX_HEAP_BYTES_PER_INPUT_BYTE`], and `D` is the row's declared constant
//! units. Query evaluation has a separate coefficient
//! ([`QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE`]) because it keeps one cursor
//! and comparison state per bound. No bytes are subtracted from reported
//! readings. Beneath that bound, each cell's heap also faces a ceiling of its
//! own ([`CellCeilings`]), 10% above its worst calibrated reading per unit;
//! its other samples are held to that same per-unit ceiling, so they may grow
//! by more than 10% before the cell turns red.
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
//! where a second currency, *fuel*, counts every wasm instruction the cell
//! body executes: element moves inside a `Vec`, arithmetic inside
//! `num-bigint`, and buffer fills included. Fuel counts the instructions of
//! the production code the body calls, a few of the board's own around it,
//! and the allocator's work for the body's own allocations. That last share
//! depends on the body alone: the guest's allocator gives each body a fresh
//! heap, so nothing the cell's preparation left in the heap reaches the
//! reading ([`FuelMeter`] describes the hooks). A fuel run judges fuel alone;
//! the native board stays the measure of record for heap.
//!
//! Fuel is judged by the same exponent bound and declared models as every
//! other currency. Its proportional ceilings are calibrated per cell
//! ([`CellCeilings`]), because fuel per byte differs between cells by orders
//! of magnitude. State outside the heap, such as a lazily initialized static,
//! can carry from one cell to the next within an instance, so a fuel run
//! measures each cell in a fresh guest instance; the `shard` module's docs
//! give the deal.
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
//! The heap counter could read zero because the implementation bypassed the
//! counting allocator. Each cell therefore declares either a justified minimum
//! heap reading or why no nonzero minimum exists; fuel, which no
//! implementation can bypass, carries one floor for every cell. Falling below that floor is a failure, just like
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
//! heap ceilings and its worst-case pin are taken and checked on 64-bit hosts; fuel is taken and checked in a wasm32 guest, whose readings
//! depend on the guest's bytes and not on the host that runs it. A native
//! reading differs between 64-bit hosts only when the measured operation
//! reaches a target-conditional code path that changes allocation.
//! Architecture-specific code that only selects
//! instructions, such as an intrinsic or an assembly routine, changes neither.
//! For example, `num-bigint` chooses the divisor of its decimal conversion by
//! architecture, so formatting a [`Count`](crate::Count) allocates differently
//! on x86_64 than on aarch64.
//!
//! The ceilings judge such a reading on every host; the per-cell heap ceiling
//! is calibrated from the larger of a cell's aarch64 and x86_64 readings for
//! that reason. Where the path also changes which family reads worst, the
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
mod sampling;
mod shard;
#[cfg(test)]
mod tests;
mod worst;

pub use ceilings::{
    COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE, DEFAULT_SCALE,
    DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, FUEL_INTERCEPT, FUEL_TREND_RESOLUTION,
    HEAP_INTERCEPT_BYTES, HEAP_TREND_RESOLUTION_BYTES, LADDER_TOP_SCALE,
    MACHINE_WORD_MAGNITUDE_BITS, MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MIN_EXPONENT_DENOM_GROWTH, QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE,
    RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE,
    SMALL_INPUT_SCALE, TICKS_BOARD_COUNT,
};
pub use cell_ceilings::{
    cell_ceilings, recalibrate, write_recalibrations, CellCeilings, Recalibration,
    CEILING_RESOLUTION, CELL_CEILING_HEADROOM, FUEL_ARCHS, HEAP_ARCHS,
};
pub use compare::{capture, capture_file, compare, source_tree, SOURCE_TREE_ENV};
pub use coverage::{BOARD_NOT_APPLICABLE, BOARD_PRICED};
pub use currency::{ByCurrency, Currency, Floors, Liveness, FUEL_FLOOR};
#[cfg(test)]
pub(crate) use family::WIDE_STEP_PAIRS_STEP_BITS;
pub use ladder::{Ladder, Verdict};
pub use measure::{FuelMeter, HeapMeter, Meters};
pub use render::Summary;
pub use sampling::{Sampling, PAST_POWER_OFFSET};
pub use shard::{cell_shard, emit_shard, max_useful_shards, run, worst_map, ShardSpawner};
pub use worst::{NEAR_TIE_RATIO, WORST_MAP_SCALES};
