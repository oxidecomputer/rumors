//! Acceptance limits for the amplification board.
//!
//! These are the default limits. An operation with a different valid bound
//! declares its units and proportional limit beside its board row. Every row
//! remains subject to the shared exponent check.
//!
//! # Measured ceilings
//!
//! A *measured ceiling* is a proportional limit derived from the board's own
//! readings rather than argued from the algorithm. Every proportional ceiling
//! in this module but [`DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE`], which its doc
//! derives from the decoder's mechanism, is measured, and so are the ceilings
//! that board rows declare beside themselves. All the measured ones follow one
//! rule.
//!
//! The rule inverts the judge's test. The board finds a sample within a
//! ceiling `C` when the sample's reading is at most the ceiling's intercept
//! plus `C` times the sample's denominator. The intercept is
//! [`HEAP_INTERCEPT_BYTES`] for heap ceilings. The denominator is the quantity the ceiling is stated per, which
//! its doc names; a fixed bound's denominator is one. Each sample therefore
//! admits every `C` of at least `(reading - intercept) / denominator`. The
//! rule takes the largest such value over every sample the release-profile
//! acceptance board judges, at every sampling of its ladder, adds 25%
//! headroom, and rounds up to a whole number. That is,
//! `C = ceil(1.25 * max((reading - intercept) / denominator))`.
//!
//! Subtracting the intercept lets one rule serve every ceiling. Fixed
//! allocations dominate a small sample's reading, and the intercept already
//! covers them, so the proportional term need not. A small sample whose
//! allocations grow with its input still enters the derivation in full.
//!
//! The board-wide ceilings here are outer bounds. Beneath them, every cell
//! also carries ceilings derived by the same rule from its own readings alone,
//! with 10% headroom (the `cell_ceilings` module).

/// Green requires every meter's scaling exponent at or below this.
///
/// The margin above 1 accommodates allocator size classes and geometric buffer
/// growth across the board's finite ladder. This empirical check catches clear
/// scaling regressions; the implementation and its independent verification
/// remain responsible for each operation's asymptotic contract.
pub const MAX_SCALING_EXPONENT: f64 = 1.15;

/// Proportional term `C` in the heap ceiling `I + C * D`, where `I` is
/// [`HEAP_INTERCEPT_BYTES`].
///
/// Most operations use their input size; operations whose required output can
/// be larger use total input and output instead. Numeric rows use value width.
/// Query evaluation has a separate limit because it keeps one cursor and
/// comparison state per bound. The coefficient is a regression limit over the
/// inputs the board measures, not a proof over unmeasured ones.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 15.0;

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
/// allocation thresholds on the finite ladder. Clamping the fit at this
/// resolution accommodates these transitions. Growth below this resolution is not judged
/// by the fit; every raw peak still faces the tighter affine ceiling above.
pub const HEAP_TREND_RESOLUTION_BYTES: usize = 4_096;

/// Additive term in every fuel ceiling: the instructions a sample may execute
/// beyond its cell's per-unit fuel ceiling.
///
/// A cell's smallest samples carry work that does not scale with their few
/// input bytes (allocator requests, and setup of the operation's cursors and
/// accumulators): about a thousand instructions at the median cell, measured
/// by extrapolating the line through each cell's two smallest samples to an
/// empty input. Without an allowance, that work divided by a handful of bytes
/// would set the cell's per-unit ceiling, which then admits many times the
/// fuel its large samples read.
///
/// The value trades two kinds of looseness. Lowering it lets fixed costs set
/// more cells' ceilings: with no allowance, the per-cell rule's ceiling admits
/// about 1.3 times the top sample's fuel at the median cell and about 30 times
/// at the 90th percentile, against about 1.1 and 3 times at this value. (These
/// ratios measure units as input bytes, which is exact for every cell with no
/// declared model.) Raising it leaves more constant-cost cells, whose every
/// reading stays under the allowance, bounded by the allowance alone.
pub const FUEL_INTERCEPT: u64 = 1_000;

/// Resolution of the fuel growth fit, in instructions.
///
/// A cell that does almost nothing can step between fixed costs as its input
/// crosses a machine-word width, such as a comparison leaving a word-sized
/// fast path, and a step of a few hundred instructions across a few bytes of
/// input reads as a steep exponent. Clamping the fit at this resolution, as the
/// heap fit is clamped at [`HEAP_TREND_RESOLUTION_BYTES`], leaves such steps
/// unjudged by the fit; every reading still faces the cell's fuel ceiling.
pub const FUEL_TREND_RESOLUTION: u64 = 1_000;

/// Extra small-input scale for heap ceilings and counter liveness.
///
/// These samples are outside the four-point growth fit: minimum allocation
/// sizes dominate them, but the affine bound must still hold.
pub const SMALL_INPUT_SCALE: f64 = 0.01;

/// Magnitudes at most this wide may legitimately be held in machine words, so
/// a result this narrow may use inline storage and has no heap floor.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// The fixed count the `version_ticks` cell registers per measurement.
///
/// Holding the count fixed makes input size the cell's only scaling
/// variable. The count is large enough that an implementation which loops
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

/// Heap ceiling for materializing the output-dominated comb-scatter
/// projection, in bytes per total-I/O byte.
///
/// The version writer holds a constant number of stream-sized buffers: its
/// output, and, once a wide payload first collapses, the separate topology
/// and payload streams that it interleaves once at the end. Each buffer grows
/// by doubling, and sealing reallocates it to its exact size, which the heap
/// meter counts beside the old buffer. Those buffers alone would bound the
/// heap at five times the output, but the writer also keeps per-level
/// stacks of branch directions and payload lengths, which that account does
/// not bound per output byte, so the ceiling is measured by the module's
/// rule rather than derived; the comb-scatter projection with its knob just
/// past a power of two reads highest. The exponent remains judged
/// independently.
pub const COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE: f64 = 7.0;

/// Heap ceiling for deserializing an owned serde buffer or a borsh stream, in
/// bytes per input byte.
///
/// Unlike this module's other ceilings, this one is derived from the
/// decoder's mechanism rather than measured. For `n` input bytes, of which
/// `p` are payload codes:
///
/// - **The stream buffer.** A borsh decoder reads each part's stream into a
///   byte vector of its own, which grows by doubling and which the decoded
///   part seals to its length at the end. Its capacity stays below twice
///   the bytes read, and a growth step or the final seal holds the old block
///   and the new one together: at most `3n` during a reallocation, and `2n`
///   between them. Serde takes the input allocation instead: a party or
///   version keeps it, sealed, and a clock or span copies its second part's
///   bytes into a vector of their own and seals the first part to its
///   length, at most `n` bytes while no parser state is live, which costs
///   less.
/// - **The numeric state.** A payload's gamma code spends two input bits per
///   value bit, so the codes hold at most `p/2` bytes of value. The decoded
///   code holds at most `p/2` bytes, and the running height stores each
///   32-bit digit in 8 bytes, so it holds at most `p` bytes; its capacity
///   stays below `2p`, and a growth step holds both blocks: `3p`.
/// - **The two never reallocate together.** The buffer grows only while a
///   byte is read and the height only after a code is complete, so the peak
///   is at most `max(3n + p/2 + 2p, 2n + p/2 + 3p)`, which is `5.5n` when
///   every byte is payload, `p = n`.
///
/// Topology costs the parser two bits of stack per open ancestor: four with
/// the stack's spare capacity, and six while it reallocates, more per flag
/// bit than the payload's state per code bit. But every open ancestor also
/// forces a later right child, a leaf flag and a code of at least one bit,
/// so each spends at least three input bits, and the stack costs at most two
/// bits per input bit, within the 2.5 the bound leaves beside a reallocating
/// buffer; so topology never raises the bound. A composite's first part (a
/// clock's party, a span's lower endpoint) is sealed to its own length before
/// the second part streams, so it costs one byte per byte in place of its
/// buffer's three. The
/// bound's constant terms (minimum capacities, a carry digit) lie within
/// [`HEAP_INTERCEPT_BYTES`]. A one-leaf version whose stream and digit count
/// both sit just past a power of two reaches the bound: its stream buffer at
/// twice the stream, its code at half, and its height's old and new blocks at
/// three times.
pub const DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 5.5;

/// Heap ceiling for deserializing a rank, in bytes per input byte.
///
/// The decoded arbitrary-width numerator cannot adopt the encoded input's byte
/// buffer.
pub const RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 5.0;

/// Heap ceiling for deserializing and validating a ranked version, in bytes per
/// input byte.
///
/// Validation retains the consumed rank prefix while materializing and ranking
/// the version.
pub const RANKED_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE: f64 = 9.0;

/// Heap ceiling for evaluating a query, in bytes per operand byte.
///
/// Evaluation keeps one cursor and comparison state per bound. Small bounds
/// may therefore have much greater overhead than other `before` values. This
/// operation-specific limit excludes evaluation from the general heap ceiling
/// while retaining the board's linear-growth check.
///
/// A measured ceiling; the module doc states the rule.
pub const QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE: f64 = 128.0;

/// Base scale and size multiplier for a single-scale board run.
///
/// Each cell's ladder is the two sizes measured at this scale and the two at
/// [`LADDER_TOP_SCALE`]: four points feeding one exponent trend, each also
/// carrying its own constant and floor checks.
pub const DEFAULT_SCALE: f64 = 1.0;

/// The top sampling scale of the measurement ladder.
///
/// Along with the base scale, this supplies four sizes for the growth fit, from
/// the base scale's smaller size to this scale's larger one. Ordinary development runs use the base scale.
pub const LADDER_TOP_SCALE: f64 = 4.0;
