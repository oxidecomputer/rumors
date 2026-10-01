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
//! see: streaming traversal and arithmetic over wide running values. Wasmtime
//! fuel separately checks total execution cost without depending on internal
//! counters or wall-clock timing.
//!
//! For each quantity, the board fits a log-log scaling trend across the measured
//! sizes and checks the largest per-byte cost. A cell is green only when every
//! trend and constant stays within its ceiling. The acceptance run uses the
//! complete measurement ladder; a single-scale run is diagnostic only.
//! Additional small-input samples check ceilings and liveness without fitting
//! growth through minimum allocation sizes. Every sample checks its full heap
//! peak against `1,024 + C * D`: `C` defaults to 20 and `D` is the row's
//! declared constant units. Query evaluation has a separate coefficient because
//! it keeps one cursor and comparison state per bound. No bytes are subtracted
//! from reported readings.
//!
//! The heap growth fit has 4 KiB resolution to accommodate allocation
//! thresholds on the finite ladder. It cannot detect growth entirely below
//! that level. These empirical checks are regression evidence over the sampled
//! inputs, not proofs of asymptotic or universal bounds. Deep-tree tests check
//! stack safety directly.
//!
//! Operand preparation and the board's result containers precede the heap
//! baseline. Measuring the maximum live heap above that baseline includes
//! operation scratch and newly allocated results; it does not separate them.
//!
//! # Counter liveness
//!
//! An internal counter could read zero because the implementation bypassed its
//! probes. Each cell therefore declares either a minimum honest reading or why
//! no nonzero minimum exists. Falling below that floor is a failure, just like
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

mod ceilings;
mod cell;
mod coverage;
mod currency;
mod defect;
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
    COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE, DEFAULT_SCALE,
    DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL,
    HEAP_INTERCEPT_BYTES, HEAP_TREND_RESOLUTION_BYTES, LADDER_TOP_SCALE,
    MACHINE_WORD_MAGNITUDE_BITS, MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
    QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE, RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE,
    RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE, SCAN_FLOOR_BITS_PER_INPUT_BYTE,
    SCAN_TOUCH_FLOOR_BITS, SMALL_INPUT_SCALE, TICKS_BOARD_COUNT,
};
pub use coverage::{BOARD_NOT_APPLICABLE, BOARD_PRICED};
pub use currency::{ByCurrency, Currency, Floors, Liveness};
pub use measure::HeapMeter;
pub use render::Summary;
pub use shard::{
    check_worst_map, emit_shard, max_useful_shards, run, run_acceptance, worst_map, ShardSpawner,
};
pub use worst::{NEAR_TIE_RATIO, WORST_MAP_SCALES};
