//! The judgment constants: the pinned global ceilings, the liveness-floor
//! parameters, and the declared per-cell models that replace a global leg where
//! a ratified derivation prices work an operation's own contract mandates.
//!
//! The declared models, disclosed on their row faces (`decl[...]`) and judged
//! in place of the named global legs (the board module doc's declared-models
//! section carries the criterion and the honesty ratchet):
//!
//! - **The fold rows** (`version_join_all`, `version_meet_all`,
//!   `party_join_all`): the
//!   balanced reduction's documented `O(D log k)` puts a `log2(2k)`
//!   factor in the deterministic counters that no flat ceiling admits at
//!   scale. The scan/touch exponent ceilings become the model's own
//!   predicted exponent plus the linear cells' slack, and the scan
//!   constant [`FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL`] per reduction
//!   level; a quadratic left fold still reads ~2 and stays red, and the
//!   log factor's own liveness is held per public entry point by the claims
//!   suite's `*_log_factor_is_alive` pins.
//! - **Family-stated heap ceilings** (the ascend-cliff tick trio and
//!   the ascend-cliff `version_min_ticks` cell, and the output-dominated
//!   comb-scatter projections): a tighter or larger flat heap bound derived
//!   for that operation and shape. Each constant carries its derivation. The
//!   exponent leg stays at the global bound, so a constant declaration cannot
//!   hide superlinear growth.

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
/// (owner-ratified: the family-stated ceilings' margin convention; the
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

// ─── declared per-cell models ────────────────────────────────────────────────
//
// Some cells carry a *declared model* in place of one global ceiling: a
// ratified upper bound derived for work the operation's contract requires,
// where the global ceiling would otherwise reject the intended algorithm. A
// declared model is disclosed on the row face (`decl[...]`) and replaces only
// the legs it names. Liveness floors independently ensure that the relevant
// counters still observe the work they claim to bound.

/// Maximum scan bits per input byte and balanced-reduction level.
///
/// For `k` operands, the board allows this coefficient times `log2(2k)`.
/// The coefficient is the largest release-profile measurement across the fold
/// rows, with 25% headroom and rounding. The board fails if a fold exceeds it.
pub const FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL: f64 = 12.0;

/// The fold rows' declared exponent ceiling over scan and touch: the fold scan
/// model's own predicted exponent plus the global
/// noise slack.
///
/// Work `c·D·log2(2k)` fitted across the cell's two probes (`D₁, k₁) → (D₂,
/// k₂`) reads exponent `1 + log2(log2(2k₂)/log2(2k₁)) / log2(D₂/D₁)` — the log
/// factor's marginal, ~1.14–1.17 at the committed populations — so the ceiling
/// is that prediction plus the same slack [`MAX_SCALING_EXPONENT`] grants
/// linear cells (0.15). A quadratic fold reads ~2 against any committed arity
/// pair and stays red; the model's own liveness is held per public entry point
/// — one `*_log_factor_is_alive` pin in the asymptotics suite for each of
/// `Version::join_all`, `Version::meet_all`, `Version::span_all`,
/// `Party::join_all`, and `Clock::join_all`, each with its own measured floor —
/// so a entry point whose wiring stops paying the reduction's log factor reads
/// red at that entry point even while the shared core still pays it elsewhere.
pub(super) fn fold_exponent_ceiling(k1: u64, k2: u64, n1: usize, n2: usize) -> f64 {
    let levels1 = (2.0 * k1 as f64).log2();
    let levels2 = (2.0 * k2 as f64).log2();
    let denom_growth = (n2 as f64 / n1 as f64).log2();
    1.0 + (levels2 / levels1).log2() / denom_growth + (MAX_SCALING_EXPONENT - 1.0)
}

/// Heap ceiling for materializing the output-dominated comb-scatter
/// projection, in bytes per total-I/O byte.
///
/// The direct payload writer and split skyline builder hold a constant number
/// of stream-sized buffers. Both projection spellings measure a flat 2.0 B/B
/// at the board's largest committed scale; the ceiling applies the standard
/// 25% margin and rounds up. Their exponent remains judged independently.
pub const COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE: f64 = 3.0;

/// The ascending-cliff tick trio's family-stated heap ceiling, in bytes per
/// encoded input byte.
///
/// `version_tick`, `version_ticks`, and `clock_tick` on the ascend-cliff cross
/// are judged at this flat constant in place of
/// [`MAX_HEAP_BYTES_PER_INPUT_BYTE`] (the declared-models section; the exponent
/// leg stays at the global bound).
///
/// Derivation: the ascending cliff is the one committed shape that defeats
/// certificate consumption — the accumulator's zero-run ledger certificates on
/// a monotone climb occupy memory until consumed, bounded at one entry per
/// jump-write (at most half the held digit positions; the bound verified
/// against the ledger code) — so the tick walk's live certificate state is
/// honest `Θ(input)` work-state with a large constant, intended and modeled,
/// not amplification. The per-entry footprint prices the accumulator's
/// word-backed quick register alongside its digit state — the trade that
/// buys the tick walk's word-scale fast path — with the certificate
/// buffers' capacity rounded at powers of two, sampled at the position
/// inside that period the family base fixes for every ladder point (the
/// base's own doc carries the choice). The profile at the release profile
/// of record is a flat per-byte constant, not a class — the exponent leg
/// stays at the global bound, so growth cannot hide under the stated
/// constant — and the ceiling is the worst reading across the ladder's two
/// sampling scales ×1.25, rounded up (owner-ratified, conditional on
/// exactly this flat-constant profile; the readings live in the pin
/// commit). A reading over it is a genuine certificate-memory regression
/// on the one shape that defeats consumption.
pub const ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE: f64 = 227.0;

/// The ascending-cliff `version_min_ticks` cell's family-stated heap ceiling,
/// in bytes per encoded input byte (judged in place of
/// [`MAX_HEAP_BYTES_PER_INPUT_BYTE`]; the exponent leg stays at the global
/// bound).
///
/// Derivation: the exact fold's anchor web holds one live reign record per
/// simultaneously-open minimum, and the ascending cliff is the one committed
/// shape that defeats batching — `Θ(k)` minima stay open at once, so the fold
/// legitimately holds `Θ(k)` live reign records (the state that keeps the
/// fold's *exponent* linear, each pricing its accumulators' word-backed
/// quick registers alongside their digit state) at a flat per-byte constant:
/// intended and modeled, not amplification. The profile at the release
/// profile of record is a flat per-byte constant, not a class — the
/// exponent leg stays at the global bound — and the ceiling is the worst
/// reading across the ladder's two sampling scales ×1.25, rounded up
/// (owner-ratified, conditional on exactly this flat-constant profile; the
/// readings live in the pin commit). A reading over it is a genuine
/// reign-state regression on the one shape that defeats batching.
pub const ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE: f64 = 247.0;

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
