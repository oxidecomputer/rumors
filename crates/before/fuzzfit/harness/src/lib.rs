//! The fuzz-fit asymptotics harness: fuel-metered fits of `before`'s public
//! operations over fuzzed shapes.
//!
//! The crate's asymptotic claims (every public operation amortized linear in
//! its denominated size) are guarded elsewhere by chosen adversarial families
//! and per-currency meters. This instrument closes the two structural blind
//! spots of chosen families: shapes nobody chose, and work that escapes the
//! metered currencies. The kernels run compiled to wasm32-unknown-unknown
//! under wasmtime *fuel* metering — fuel decrements per executed wasm
//! instruction, so a reading is deterministic, host-independent, and
//! byte-reproducible under any machine load. Wall time and hardware counters
//! are never read. Constants differ from native codegen; *slopes* are what
//! the instrument fits and enforces.
//!
//! Two legs, sharing one program vocabulary ([`ops`]):
//!
//! - **Calibration** (`bin/calibrate`): size-stratified random sampling per
//!   public operation, log-log regression of fuel against the operation's
//!   denominated size, producing pinned bands (slope, intercept, residual
//!   width) committed in [`bands`], one per band key — kernel × outcome,
//!   so an operation's rejection arm is priced separately from its success
//!   path. The fit is never recomputed inside the enforcement leg:
//!   refitting on every run would mask drift.
//! - **Enforcement** (`tests/enforce.rs`): a proptest family draws random
//!   programs from [`strategies`], executes them step-by-step in the guest,
//!   and judges on two legs with different failure modes — every measured
//!   step's fuel must land inside the pinned band for its key at its size
//!   (the point leg: above-band is a regression flag; below-band is a
//!   liveness flag, since a band a dead measurement passes is decoration),
//!   and every key's within-case bucket-median trend must not out-climb
//!   its pinned slope ([`curve`], the shape leg: a mechanism that tilts
//!   into a wide band keeps its point residuals small, and only the trend
//!   sees it). The same judgment also runs over the whole deterministic
//!   calibration-stream prefix, program by program, every run: the
//!   random draws probe shapes nobody chose, and the prefix leg makes
//!   every kernel × size-decade region the corpus reaches a total,
//!   deterministic verdict instead of a sampled one. A staleness
//!   cross-check refits the same prefix and compares lines against the
//!   pin on every covered key, so calibration drift fails loud instead
//!   of silently widening the gap between pin and reality. Fuel
//!   determinism makes replay exact, so a failure shrinks to a minimal
//!   out-of-band shape and rides along as a committed proptest seed.
//!
//! Every program executes twice: natively (the mirror, which computes each
//! step's denominator from real operand sizes and the expected result bytes)
//! and in the guest (which supplies fuel). The mirror doubles as a
//! wasm-vs-native differential oracle: result encodings must byte-match.

pub mod bands;
pub mod curve;
pub mod drive;
pub mod fit;
pub mod ops;
pub mod strategies;
pub mod wasm;
