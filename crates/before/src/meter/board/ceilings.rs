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

/// Green requires peak transient heap, after subtracting the flat allowance,
/// at most this many bytes per denominator byte.
///
/// Most operations use their encoded input size; operations whose required
/// output can be larger use total input and output instead. This limit is 20
/// B/B, rounded up from the largest release-board measurement governed by it.
/// The exponent check separately detects scaling regressions.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 20.0;

/// Heap bytes ignored before the per-byte constant is computed: fixed-size
/// scaffolding (format machinery, hasher state, container headers) that does
/// not scale with the input.
pub const HEAP_FLAT_ALLOWANCE_BYTES: usize = 8_192;

/// Green requires at most this many grown stack segments, as an absolute count:
/// the target is walks that never grow the stack, so the ceiling is flat, not
/// per-byte.
pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

/// Green requires at most this many encoded bits scanned per denominator
/// byte (asserted only when the `scan-meter` feature is lit).
///
/// One complete walk reads about eight bits per encoded byte. The higher limit
/// admits operations that make several bounded passes while rejecting repeated
/// scans whose count grows with the input.
pub const MAX_SCAN_BITS_PER_INPUT_BYTE: f64 = 96.0;

/// Green requires at most this many accumulator digit touches per denominator
/// byte (asserted only when the `touch-meter` feature is lit).
///
/// Validation, comparison, emission, and query folds normally touch a small
/// number of digits per encoded delta. Balanced reductions may revisit digits
/// at each reduction level, so their rows use level-adjusted units while
/// retaining this proportional limit. The value is the largest release-board
/// measurement plus 25%, rounded up. Crossing it requires rechecking the
/// implementation and the limit against the board's worst-case map.
pub const MAX_TOUCHES_PER_INPUT_BYTE: f64 = 22.0;

/// Scan liveness floor: an operation that must examine its encoded operands
/// scans at least this many bits per encoded input byte.
///
/// One bit per byte is well below a complete walk's eight bits per byte, but
/// above the zero reading produced by a disconnected counter.
pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

/// Scan liveness floor for legitimate early-exit operations: even an immediate
/// divergence answer reads the operands' root codes.
pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;

/// Scan liveness floor for the tick-cross rows: all 8 bits of every input byte.
///
/// The paired fill walk examines every topology bit and payload code of both
/// operands at least once. `WHY_SCAN_TICK_WALK` explains this floor in board
/// output.
pub(super) const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

/// Magnitudes at most this wide may legitimately be handled in machine words;
/// wider values enter the accumulator word by word, so touch floors bind only
/// on them.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// The fixed count the `version_ticks` cell registers per measurement.
///
/// Holding the count fixed makes encoded input size the cell's only scaling
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
/// The coefficient is the largest release-profile fold measurement plus 25%,
/// rounded up.
pub const FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL: f64 = 12.0;

/// Heap ceiling for materializing the output-dominated comb-scatter
/// projection, in bytes per total-I/O byte.
///
/// The direct payload writer and split skyline builder hold a constant number
/// of stream-sized buffers. Both projection spellings measure a flat 2.0 B/B
/// at the board's largest committed scale; the ceiling applies the standard
/// 25% margin and rounds up. Their exponent remains judged independently.
pub const COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE: f64 = 3.0;

/// Heap ceiling for deserializing an owned serde buffer or a borsh stream, in
/// bytes per encoded input byte.
///
/// Serde transfers its input allocation into the decoded value. Borsh grows
/// one output buffer while reading. Both validate with compact parser state, so
/// their remaining heap stays a small multiple of the encoding. The ceiling is
/// the largest release-profile reading with 25% headroom, rounded up.
pub const DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 4.0;

/// Base scale and size multiplier for a single-scale board run.
///
/// Each cell's ladder is the two sizes measured at this scale and the two at
/// [`LADDER_TOP_SCALE`]: four points feeding one exponent trend, each also
/// carrying its own constant and floor checks.
pub const DEFAULT_SCALE: f64 = 1.0;

/// The top sampling scale of the measurement ladder.
///
/// Stack growth can begin only after roughly one mebibyte of frames, beyond
/// some base-scale inputs. Scale four is the smallest committed scale that has
/// exposed every such regression represented by the board. An acceptance run
/// measures both scales and fits each exponent over all four points; ordinary
/// development runs remain at the base scale.
pub const LADDER_TOP_SCALE: f64 = 4.0;
