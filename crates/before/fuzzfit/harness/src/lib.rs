//! Fuel-metered resource checks over generated `before` programs.
//!
//! The resource board measures chosen input families with per-currency
//! counters. This harness adds instruction-cost observations and randomized
//! shapes. It runs the selected operations in [`ops`] as a release Wasm guest
//! under Wasmtime fuel metering. Fuel is deterministic for fixed guest bytes,
//! call sequences, and payloads; it measures Wasm work, not native elapsed time.
//!
//! Calibration fits fuel against operand size over a deterministic corpus
//! and commits the results in [`bands`]. Enforcement compares new samples with
//! those pins instead of recalibrating them. Pointwise bounds catch extra work
//! or implausibly low readings; [`curve`] checks within-program trends that a
//! wide pointwise band could hide. A deterministic prefix must exercise and
//! refit every main band, and fixed deep replays cover large operands. The
//! separate bootstrap corpus must price every step through a main or small
//! band and exercise every small-operand pin.
//!
//! Programs execute natively and in the guest. The native mirror supplies
//! operand sizes and expected return values; final live-register encodings
//! must agree byte for byte. Equality shortcuts receive this differential
//! check but are excluded from the fuel fits. Samples below a main band's
//! floor receive a pointwise check only where a small band applies.
//!
//! The vocabulary is a subset of the public API, bounded by the generated
//! construction budgets described in [`strategies`]. The numeric suite
//! separately calls the same guest over geometric width and arity sweeps:
//! Rank parsing and formatting, Ticks arithmetic and rendering, and both
//! size dimensions of shape combination. Its wide inputs exceed the program
//! budgets, and its fits check growth without recalibrating the program pins.

pub mod bands;
pub mod curve;
pub mod drive;
pub mod fit;
pub mod ops;
pub mod strategies;
pub mod wasm;
