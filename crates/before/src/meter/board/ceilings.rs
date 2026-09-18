//! Constants used by the amplification-board judgment.
//!
//! Global ceilings apply to the default linear cost model. A cell with a more
//! precise bound supplies its own units through the general model mechanism;
//! constants here calibrate the proportional checks that differ from the
//! global defaults. Model selection and units live beside the operation, where
//! their derivation can be reviewed.

// ─── the pinned ceilings ────────────────────────────────────────────────────
//
// Several ceilings below argue their calibration from the worst honest reader
// at the release profile of record. The readings themselves live in the pin
// commits (`git log -S` the constant), never in this prose: a quoted reading
// would keep asserting itself as present-tense fact while headroom absorbed
// the drift. A change that moves a ceiling's worst honest reader is a
// deliberate event: re-measure, re-derive, and re-pin the constant — the
// event re-words derivations and re-pins numbers, never prose readings.

/// Green requires every meter's scaling exponent at or below this.
///
/// The contract is amortized-linear; 1.15 leaves room for measurement noise
/// (allocator rounding, `Vec` doubling) without admitting a real log factor at
/// these input sizes.
pub const MAX_SCALING_EXPONENT: f64 = 1.15;

/// Green requires peak transient heap at most this many bytes per encoded input
/// byte, over the flat allowance.
///
/// Pinned at the worst honest cell without a declared model, with 25% headroom
/// and rounded up. The exponent leg separately rejects superlinear growth.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 24.0;

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
/// Calibrated against the benign control and the green adversarial families: a
/// single walk over encoded operands scans ~8 bits per byte, while multi-walk
/// operations such as `distance` scan a small multiple. The committed families
/// all read under this ceiling with
/// room to spare, so only a walk that re-scans state growing with the input
/// — the fold genre — goes red on this column.
pub const MAX_SCAN_BITS_PER_INPUT_BYTE: f64 = 96.0;

/// Green requires at most this many accumulator digit touches per denominator
/// byte (asserted only when the `touch-meter` feature is lit).
///
/// Calibrated against the worst honest reader at the release profile of
/// record: the delta-folding kernels (validate, sweep, emit, the query folds,
/// the query folds) touch a handful of digits per delta code — single digits
/// per encoded byte on organic shapes — and the heaviest honest readers are
/// the cells where per-site resolution or the balanced reduction's per-level
/// re-touching legitimately stacks; the board's worst-case map at both
/// sampling scales identifies them, and the fold rows' touch constant is
/// judged at this same ceiling — only their exponent leg rides the fold
/// model. The ceiling is the worst honest reading ×1.25, rounded up
/// (owner-ratified: the cell-specific ceilings' margin convention; the
/// reading lives in the pin commit), so a kernel that re-reads digit state
/// growing with the input — the width-circulation genre — goes red on this
/// column's constant instead of hiding in headroom, and an honest family
/// that reads past the worst witness is a deliberate re-measure event:
/// re-derive the worst honest reader from the board's worst-case map and
/// re-pin the constant, never absorbed slack.
pub const MAX_TOUCHES_PER_INPUT_BYTE: f64 = 22.0;

/// Scan liveness floor: an operation that must examine its encoded operands
/// scans at least this many bits per encoded input byte.
///
/// One bit per byte is an eighth of the stored bits: far below any honest full
/// walk (measured ~8 bits per byte across the board), and far above a counter
/// that has stopped watching (which reads ~0).
pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

/// Scan liveness floor for legitimate early-exit operations: even an immediate
/// divergence answer reads the operands' root codes.
pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;

/// Scan liveness floor for the tick-cross rows: all 8 bits of every input byte.
///
/// The paired fill walk examines every topology bit and payload code of both
/// operands at least once; `WHY_SCAN_TICK_WALK` carries the row-face wording.
pub(super) const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

/// Magnitudes at most this wide may legitimately be handled in machine words;
/// wider values enter the accumulator word by word, so touch floors bind only
/// on them.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// The fixed count the `version_ticks` cell registers per measurement.
///
/// Fixed so the cell's judged axis is the encoded input alone: the count's whole
/// contribution is the boundary codes' gamma width (the flatness rows of
/// `tests/meter.rs` pin that axis point to point), and 512 sits far enough past
/// the single tick that an implementation iterating even a fraction of the
/// count would blow the scaling ceiling rather than hide in a constant.
pub const TICKS_BOARD_COUNT: u64 = 512;

/// Exponent legs are fitted only where the denominator pair grows at least this
/// much between the cell's two probes.
///
/// The fit divides by `log(denominator growth)`: the families' probe pairs
/// double their scaled dimension by construction, so a pair growing less than
/// this says the operand does not scale with the knob at all (the benign rank
/// pair moves 6 -> 7 bytes) and the division manufactures exponents out of
/// word-scale reading noise (log 2 / log 7/6 amplifies x4.5). An unjudged
/// exponent renders `-.--` and the cell rides its constants and floors, which
/// bound single-size cost regardless \[derived; the sub-scaling tripwire in the
/// test suite pins both directions\].
pub const MIN_EXPONENT_DENOM_GROWTH: f64 = 1.5;

/// Maximum scan bits per input byte and balanced-reduction level.
///
/// For `k` operands, the board allows this coefficient times `log2(2k)`.
/// The coefficient is the largest release-profile measurement across the fold
/// rows, with 25% headroom and rounding. The board fails if a fold exceeds it.
pub const FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL: f64 = 12.0;

/// Heap ceiling for materializing the output-dominated comb-scatter
/// projection, in bytes per total-I/O byte.
///
/// The direct payload writer and split skyline builder hold a constant number
/// of stream-sized buffers. Both projection spellings measure a flat 2.0 B/B
/// at the board's largest committed scale; the ceiling applies the standard
/// 25% margin and rounds up. Their exponent remains judged independently.
pub const COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE: f64 = 3.0;

/// The base sampling scale of the measurement ladder, and the size
/// multiplier of a bare single-scale board run.
///
/// Each cell's ladder is the two sizes measured at this scale and the two at
/// [`LADDER_TOP_SCALE`]: four points feeding one exponent trend, each also
/// carrying its own constant and floor checks.
pub const DEFAULT_SCALE: f64 = 1.0;

/// The top sampling scale of the measurement ladder.
///
/// The base-scale sizes under-detect segment amplifiers: stacker grows a
/// segment only past ~1 MiB of frames, so a recursion-frame amplifier whose
/// onset sits above the base depths reads a false green there. ×4 is the
/// witnessed calibration floor — the smallest sampling scale at which every
/// segment-onset amplifier the suite has caught reads red — so the ladder
/// tops out there. **Campaign acceptance is every cell green across the whole
/// ladder, one acceptance invocation measuring all of it under the
/// determinism tripwire**, with each exponent judged as one trend over the
/// four measured points; an acceptance run is acceptance-time only (the
/// inner loop stays at the base scale, and the enforced per-operation record
/// remains the envelope suite in `tests/meter.rs` regardless of board
/// onset).
pub const LADDER_TOP_SCALE: f64 = 4.0;
