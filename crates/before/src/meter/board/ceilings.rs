//! Acceptance limits for the amplification board.
//!
//! These are the default limits. An operation with a different valid bound
//! declares its units and proportional limit beside its board row. Every row
//! remains subject to the shared exponent check.

/// Green requires every meter's scaling exponent at or below this.
///
/// The margin above 1 accommodates allocator size classes and geometric buffer
/// growth across the board's finite ladder. This empirical check catches clear
/// scaling regressions; the implementation and its independent verification
/// remain responsible for each operation's asymptotic contract.
pub const MAX_SCALING_EXPONENT: f64 = 1.15;

/// Proportional term in the heap ceiling `HEAP_INTERCEPT_BYTES + C * D`.
///
/// Most operations use their input size; operations whose required output can
/// be larger use total input and output instead. Numeric rows use value width.
/// Query evaluation has a separate limit because it keeps one cursor and
/// comparison state per bound. These coefficients are measured regression
/// limits, not proofs over unmeasured inputs.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 20.0;

/// Additive term in every sample's heap ceiling, including small-input checks.
///
/// Minimum container and numeric allocations can exceed their proportional
/// budgets at single-digit input sizes. This allowance bounds that excess; it
/// does not identify a fixed allocation common to every operation. Peaks and
/// normalized readings are always reported in full.
pub const HEAP_INTERCEPT_BYTES: usize = 1_024;

/// Resolution of the empirical heap growth fit, independent of its ceiling.
///
/// Inline storage, buffer doubling, and numeric formatting algorithms produce
/// allocation thresholds on the finite ladder. Clamping the fit at 4 KiB
/// accommodates these transitions. Growth below this resolution is not judged
/// by the fit; every raw peak still faces the tighter affine ceiling above.
pub const HEAP_TREND_RESOLUTION_BYTES: usize = 4_096;

/// Extra small-input scale for heap ceilings and counter liveness.
///
/// These samples are outside the four-point growth fit: minimum allocation
/// sizes dominate them, but the affine bound must still hold.
pub const SMALL_INPUT_SCALE: f64 = 0.01;

/// Green requires at most this many bits scanned per denominator
/// byte (asserted only when the `scan-meter` feature is lit).
///
/// One complete walk reads about eight bits per input byte. The higher limit
/// admits operations that make several bounded passes while rejecting repeated
/// scans whose count grows with the input.
pub const MAX_SCAN_BITS_PER_INPUT_BYTE: f64 = 96.0;

/// Green requires at most this many accumulator digit touches per denominator
/// byte (asserted only when the `touch-meter` feature is lit).
///
/// Validation, comparison, emission, and query folds normally touch a small
/// number of digits per input delta. Balanced reductions may revisit digits
/// at each reduction level, so their rows use level-adjusted units while
/// retaining this proportional limit. The value is the largest release-board
/// measurement plus 25%, rounded up. Crossing it requires rechecking the
/// implementation and the limit against the board's worst-case map.
pub const MAX_TOUCHES_PER_INPUT_BYTE: f64 = 22.0;

/// Scan liveness floor: an operation that must examine its operands scans at
/// least this many bits per input byte.
///
/// One bit per byte is well below a complete walk's eight bits per byte, but
/// above the zero reading produced by a disconnected counter.
pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

/// Scan liveness floor for legitimate early-exit operations: even an immediate
/// divergence answer reads the operands' root codes.
pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;

/// Scan liveness floor for the tick-cross rows: all 8 bits of every input byte.
///
/// The paired tick walk examines every topology bit and payload code of both
/// operands at least once. `WHY_SCAN_TICK_WALK` explains this floor in board
/// output.
pub(super) const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

/// Magnitudes at most this wide may legitimately be handled in machine words;
/// wider values enter the accumulator word by word, so touch floors bind only
/// on them.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// The fixed count the `version_ticks` cell registers per measurement.
///
/// Holding the count fixed makes input size the cell's only scaling
/// variable. A count of 512 is large enough that an implementation which loops
/// over ticks cannot hide that work in a fixed cost.
pub const TICKS_BOARD_COUNT: u64 = 512;

/// Exponent legs are fitted only where the denominator pair grows at least this
/// much between the cell's two probes.
///
/// The fit divides by `log(denominator growth)`. A nearly fixed denominator
/// magnifies word-scale measurement changes into meaningless exponents; for
/// example, growth from 6 to 7 bytes amplifies a twofold reading by about 4.5.
/// Below this threshold the board omits the exponent and still applies its
/// proportional limits and liveness floors.
pub const MIN_EXPONENT_DENOM_GROWTH: f64 = 1.5;

/// Maximum scan bits per input byte and balanced-reduction level.
///
/// For `k` operands, the board allows this coefficient times `log2(2k)`.
/// The coefficient is the largest release-profile fold measurement at any
/// judged size plus 25%, rounded up.
pub const FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL: f64 = 17.0;

/// Heap ceiling for materializing the output-dominated comb-scatter
/// projection, in bytes per total-I/O byte.
///
/// The direct payload writer and split skyline builder hold a constant number
/// of stream-sized buffers. Both projection spellings measure a flat 2.0 B/B
/// at the board's largest committed scale; the ceiling applies the standard
/// 25% margin and rounds up. Their exponent remains judged independently.
pub const COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE: f64 = 3.0;

/// Heap ceiling for deserializing an owned serde buffer or a borsh stream, in
/// bytes per input byte.
///
/// Serde transfers its input allocation into the decoded value. Borsh grows
/// one output buffer while reading. Both validate with compact parser state, so
/// their remaining heap stays a small multiple of the encoding. The ceiling is
/// the largest release-profile reading with 25% headroom, rounded up.
pub const DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 4.0;

/// Heap ceiling for deserializing a rank, in bytes per input byte.
///
/// The decoded arbitrary-width numerator cannot adopt the encoded input's byte
/// buffer. The ceiling is the largest release-profile reading with 25%
/// headroom, rounded up.
pub const RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 6.0;

/// Heap ceiling for deserializing and validating a ranked version, in bytes per
/// input byte.
///
/// Validation retains the consumed rank prefix while materializing and ranking
/// the version. The ceiling is the largest release-profile reading with 25%
/// headroom, rounded up.
pub const RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 9.0;

/// Heap ceiling for evaluating a query, in bytes per operand byte.
///
/// Evaluation keeps one cursor and comparison state per bound. Small bounds
/// may therefore have much greater overhead than other `before` values. This
/// operation-specific limit excludes evaluation from the general heap ceiling
/// while retaining the board's linear-growth check. It is the largest
/// release-profile reading with 25% headroom, rounded up.
pub const QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE: f64 = 82.0;

/// Base scale and size multiplier for a single-scale board run.
///
/// Each cell's ladder is the two sizes measured at this scale and the two at
/// [`LADDER_TOP_SCALE`]: four points feeding one exponent trend, each also
/// carrying its own constant and floor checks.
pub const DEFAULT_SCALE: f64 = 1.0;

/// The top sampling scale of the measurement ladder.
///
/// Along with the base scale, this supplies four sizes spanning an eightfold
/// range for the growth fit. Ordinary development runs use the base scale.
pub const LADDER_TOP_SCALE: f64 = 4.0;
