//! Deterministic resource checks across public operations and adversarial input
//! families.
//!
//! Each family supplies a related set of operands. Every operation that can use
//! those operands becomes a board cell, so adding a family or operation expands
//! coverage without hand-writing individual pairings. The coverage table
//! requires every rostered public method and grouped trait family to be either
//! measured or explicitly inapplicable.
//!
//! # What a cell measures
//!
//! A cell runs at increasing input scales and records:
//!
//! - peak transient heap bytes;
//! - stack segments allocated by recursive work;
//! - encoded bits scanned or written; and
//! - accumulator digits touched.
//!
//! The last two counters are feature-gated because they instrument hot
//! primitives. They cover work that allocation and stack measurements cannot
//! see: streaming traversal and arithmetic over wide running values. Wasmtime
//! fuel separately checks total execution cost without depending on internal
//! counters or wall-clock timing.
//!
//! For each quantity, the board fits a log-log scaling trend across the measured
//! sizes and checks the largest per-byte cost. A cell is green only when every
//! trend and constant stays within its ceiling. The acceptance run uses the
//! complete measurement ladder; a single-scale run is diagnostic only.
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
//! Most cells divide cost by encoded input bytes. An operation whose required
//! output can dominate its input instead uses total encoded input and output;
//! the measurement reads the actual result size.
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
//! Board heap readings share a process and are therefore indicative. The
//! process-isolated envelopes in `tests/meter.rs` are the exact records. Board
//! acceptance uses release builds so debug assertions do not add instrumented
//! work to production measurements.

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
    HEAP_FLAT_ALLOWANCE_BYTES, LADDER_TOP_SCALE, MACHINE_WORD_MAGNITUDE_BITS,
    MAX_GROWN_STACK_SEGMENTS, MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
    SCAN_FLOOR_BITS_PER_INPUT_BYTE, SCAN_TOUCH_FLOOR_BITS, TICKS_BOARD_COUNT,
};
pub use coverage::{BOARD_NOT_APPLICABLE, BOARD_PRICED};
pub use currency::{ByCurrency, Currency, Floors, Liveness};
pub use family::study_family_versions;
pub use measure::HeapMeter;
pub use render::Summary;
pub use shard::{
    check_worst_map, emit_shard, max_useful_shards, run, run_acceptance, worst_map, ShardSpawner,
};
pub use worst::{NEAR_TIE_RATIO, WORST_MAP_SCALES};
