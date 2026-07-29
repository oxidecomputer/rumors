//! The judgment constants: the pinned global ceilings, the liveness-floor
//! parameters, and the declared per-cell models that replace a global leg
//! where a ratified derivation prices work an operation's own contract
//! mandates.

use crate::Party;

// ─── the pinned ceilings ────────────────────────────────────────────────────
//
// Several ceilings below argue their calibration from a measured worst
// honest reader and name it. Those witnesses are load-bearing prose, and
// nothing mechanical guards them: a new family (or kernel change) that
// reads heavier than a named witness falsifies the calibration argument
// while its cell still reads green, since every ceiling sits well above
// its witness. Whoever lands such a change re-measures and re-words the
// constants whose witness moved.

/// Green requires every meter's scaling exponent at or below this.
///
/// The contract is amortized-linear; 1.15 leaves room for measurement noise
/// (allocator rounding, `Vec` doubling) without admitting a real log factor
/// at these input sizes.
pub const MAX_SCALING_EXPONENT: f64 = 1.15;

/// Green requires peak transient heap at most this many bytes per packed
/// input byte, over the flat allowance.
pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 16.0;

/// Heap bytes ignored before the per-byte constant is computed: fixed-size
/// scaffolding (format machinery, hasher state, container headers) that does
/// not scale with the input.
pub const HEAP_FLAT_ALLOWANCE_BYTES: usize = 8_192;

/// Green requires at most this many grown stack segments, as an absolute
/// count: the target is walks that never grow the stack, so the ceiling is
/// flat, not per-byte.
pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

/// Green requires at most this many big-integer limb operations per packed
/// input byte (asserted only when the `limb-meter` feature is lit).
///
/// Calibrated against the benign control: an amortized-linear walk records a
/// handful of unit-limb operations per node (tens per packed byte at ~2 bits
/// per node, over a hundred for multi-walk operations like `distance`), and
/// that per-node arithmetic is exactly the contract's linear regime. The
/// ceiling sits above it; width blowups are caught by the exponent bound
/// long before the constant.
pub const MAX_LIMB_OPS_PER_INPUT_BYTE: f64 = 128.0;

/// Green requires at most this many packed-stream scan bits per denominator
/// byte (asserted only when the `scan-meter` feature is lit).
///
/// Calibrated against the benign control and the green adversarial
/// families: a single walk over packed operands scans ~8 bits per byte,
/// multi-walk operations (`distance` runs join, meet, and two rank folds;
/// `sync` joins in both directions) scan a small multiple, and the text
/// parsers re-scan their packed output through the strict validator. The
/// worst honest reader measured is well under 64; the ceiling sits at 96 so
/// only a walk that re-scans state growing with the input — the fold genre —
/// goes red on this column.
pub const MAX_SCAN_BITS_PER_INPUT_BYTE: f64 = 96.0;

/// Green requires at most this many accumulator digit touches per
/// denominator byte (asserted only when the `limb-meter` feature is lit).
///
/// Calibrated against the benign control and the green adversarial
/// families at the release profile of record: the delta-folding kernels
/// (validate, sweep, emit, the query folds, the text parse) touch a
/// handful of digits per delta code — single digits per packed byte on
/// organic shapes — and the heaviest honest reader measured is the
/// mirror-narrow tick cross (the memo machinery's per-site resolution) at
/// 30.8 touches per input byte at the default scale, 24.3 at the record
/// scale \[measured 2026-07-26, release, both scales\]. The ceiling sits
/// at 96, scan's own margin convention, so only a walk that re-reads
/// digit state growing with the input — the width-circulation genre —
/// goes red on this column's constant.
pub const MAX_TOUCHES_PER_INPUT_BYTE: f64 = 96.0;

/// Scan liveness floor: an operation that must examine its packed operands
/// scans at least this many bits per packed input byte.
///
/// One bit per byte is an eighth of the stored bits: far below any honest
/// full walk (measured ~8 bits per byte across the board), and far above a
/// counter that has stopped watching (which reads ~0).
pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

/// Scan liveness floor for legitimate early-exit operations: even an
/// immediate divergence answer reads the operands' root codes.
pub const SCAN_TOUCH_FLOOR_BITS: u64 = 2;

/// Scan liveness floor for the tick-cross rows: all 8 bits of every
/// input byte.
///
/// The paired fill walk examines every topology bit and payload code of
/// both operands at least once; `WHY_SCAN_TICK_WALK` carries the
/// row-face wording.
pub(super) const TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE: u64 = 8;

/// Magnitudes at most this wide may legitimately be handled in machine
/// words; wider values force big-integer arithmetic, so limb floors bind
/// only on them.
pub const MACHINE_WORD_MAGNITUDE_BITS: u64 = 128;

/// The fixed count the `version_ticks` cell registers per measurement.
///
/// Fixed so the cell's judged axis is the packed input alone: the
/// count's whole contribution is the boundary codes' gamma width (the
/// flatness rows of `tests/meter.rs` pin that axis point to point), and
/// 512 sits far enough past the single tick that an implementation
/// iterating even a fraction of the count would blow the scaling
/// ceiling rather than hide in a constant.
pub const TICKS_BOARD_COUNT: u64 = 512;

/// Text rows only: green requires at most this many limb operations per
/// radix-work unit
/// `R = n_io + Σᵢ (digitsᵢ × limbsᵢ + TEXT_PIPELINE_LIMB_OPS_PER_VALUE)`
/// over the spelled event values (κ).
///
/// κ carries the text limb column's *constant* leg only; the *exponent* leg
/// is judged against `n_io`, never against `R` — `R` is the honest text
/// cost law itself (schoolbook conversion plus the per-value pipeline
/// term), so an exponent against it reads a flat ~1 on exactly the
/// quadratic converters the bound exists to catch. The legs exclude
/// different converters, and a constant ceiling cannot enforce a complexity
/// class: a `u32`-chunked schoolbook converter scores ~0.11 limb per `R`
/// unit — under κ, and wider chunks only lower it — while its limb work
/// stays quadratic in the value bits and reads exponent ~2 against `n_io`
/// \[measured — the chunked tripwire in the test suite\]; the exponent leg
/// is what excludes it. What κ excludes is a wasteful constant. It is
/// pinned from the production kernels' observed meter at the acceptance scale
/// (release, the profile of record): the honest cells read at most 0.59
/// limb per `R` unit (the staircase pipeline, both directions), so κ
/// leaves the worst honest family ~27% headroom while a digit-by-digit
/// schoolbook probe's measured ~1 limb per `R` unit still exceeds it, and
/// the production parser — radix conversion delegated to the backend's
/// divide-and-conquer parser, one width-proportional limb record per
/// materialized value — reads orders under it on the conversion-dominated
/// families \[measured — the schoolbook and delegating-parser pins in the
/// test suite\]. The test suite pins three legs — the schoolbook probe
/// exceeds κ; the delegating parser stays under κ over a liveness floor;
/// the chunked probe slips under κ and trips the exponent leg — so none
/// can silently soften.
pub const MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT: f64 = 0.75;

/// Radix units each spelled event value contributes to the text limb
/// denominator beyond its conversion term: the delta⇄absolute pipeline's
/// per-value arithmetic allowance.
///
/// The text kernels do mandatory per-value big-integer work that is not
/// radix conversion — the render derives each printed base from
/// delta-sized relative summaries, the parse re-derives delta codes from
/// spelled bases — and `Σ digits × limbs` under-weights it to nothing on
/// small-value trees (a one-digit value is one radix unit; the pipeline
/// around it is not free). The allowance is pinned just above the
/// production kernels' measured honest range \[measured, release, record
/// scale: 5–9 limb ops per spelled value across the small-value families,
/// both directions; the ceiling κ then leaves the worst family ~27%
/// headroom\]. Id tokens contribute nothing: an id tree spells booleans
/// and forces no arithmetic.
pub const TEXT_PIPELINE_LIMB_OPS_PER_VALUE: u64 = 10;

/// Any text stream entering a denominator must hold at most this many bytes
/// per radix unit of the values it spells (the output-honesty ceiling).
///
/// Denominating against I/O bytes opens a door: pad the output, inflate the
/// denominator, read green. The ceiling closes it \[derived\], and its
/// basis is the radix-unit sum (`Σ digits × limbs`, computed from the
/// values outside the render) rather than wire bits, because the skyline
/// wire coding spends O(1) bits on a leaf whose spelled value is wide —
/// no constant per wire bit bounds honest text. Per rendered value the
/// grammar spends its exact decimal digits (`digits ≤ digits × limbs`, one
/// radix unit minimum per value) plus at most 6 syntax bytes (`(`, `)`,
/// and two `, ` separators), and every value contributes at least one
/// radix unit — so honest text stays under 7 bytes per radix unit and
/// padding trips the assertion.
pub const TEXT_BYTES_PER_RADIX_UNIT: f64 = 8.0;

/// Exponent legs are fitted only where the denominator pair grows at
/// least this much between the cell's two probes.
///
/// The fit divides by `log(denominator growth)`: the families' probe
/// pairs double their scaled dimension by construction, so a pair growing
/// less than this says the operand does not scale with the knob at all
/// (the benign rank pair moves 6 -> 7 bytes) and the division manufactures
/// exponents out of word-scale reading noise (log 2 / log 7/6 amplifies
/// x4.5). An unjudged exponent renders `-.--` and the cell rides its
/// constants and floors, which bound single-size cost regardless
/// \[derived; the sub-scaling tripwire in the test suite pins both
/// directions\].
pub const MIN_EXPONENT_DENOM_GROWTH: f64 = 1.5;

// ─── declared per-cell models ────────────────────────────────────────────────
//
// Some cells carry a *declared model* in place of one global ceiling:
// a ratified cost law, derived and priced at the cell with a dated owner
// rationale, that the readings must match — the global ceiling would
// otherwise be unsatisfiable by construction on work the operation's own
// contract mandates. A declared model is disclosed on the row face
// (`decl[...]`) and replaces only the legs it names; its under side is
// held honest by a banded floor where the model predicts a quantity, or
// by a committed liveness pin where it declares a class, so a reading
// under the model means it has gone stale against an improved kernel and
// must be re-declared in a diff that shows the new derivation (the same
// ratchet as a liveness floor).

/// The fold rows' declared scan model: at most this many scan bits per
/// input byte per balanced-reduction level, `log2(2k)` levels over `k`
/// fold operands.
///
/// The fold rows run the balanced binary-counter reduction, whose
/// documented class is `O(D log k)` (`FoldLog` in the claims roster):
/// every input passes through `O(log k)` joins, and each join level
/// re-scans the operands it merges, so scan work per input byte grows by
/// a constant per level — never flat, at any implementation of the
/// balanced reduction. Derivation of the constant: the fold cells read
/// 9.1–10.0 scan bits per byte per level across both scales and both
/// committed populations \[measured 2026-07-28, release: the benign
/// control at k = 512 reads 100.1 bits/B over 10 levels, at k = 2048
/// reads 116.9 over 12\]; 12 leaves the worst honest reading ~20%
/// headroom while a fold whose per-level constant regresses by a third
/// still reads red.
pub const FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL: f64 = 12.0;

/// The scan bits one metered `IdIndex` table probe records: one `u32`
/// table word per probe.
///
/// The party fold's declared search allowance is `32·⌈log2(t+1)⌉`
/// probes' worth per both-present node of each tested input, `t` the
/// accumulator's table size.
///
/// Derivation: `Party::join_all` overlap-tests every input against the
/// fixed accumulator through a per-call table of the accumulator's
/// both-present nodes, and each both-present node the test visits runs
/// one binary search over at most `t` entries — at most `⌈log2(t+1)⌉`
/// probes of one table word each. The allowance is that bound summed
/// over the inputs' both-present nodes, computed from the operands at
/// prepare; it is tight-ish where the searches dominate \[measured
/// 2026-07-28, release, the weave family: readings sit within ~10% of
/// the fold model plus this allowance at both scales\], zero on
/// populations with no both-present structure (scatter's single-leaf
/// operands), and absent from the version fold, which runs no overlap
/// test. The index stays, its searches priced, over a per-input cursor
/// walk: the committed overlap instruments pin the index's asymptotic
/// win (a cursor discipline reads quadratic on the overlap rows and
/// trips the flatness pin), and the index ties or wins wall time on
/// every committed fold population.
pub const INDEX_PROBE_SCAN_BITS: u64 = 32;

/// A packed id operand's both-present node count: the size of the
/// `IdIndex` table a fold builds over it, and the per-input factor of
/// the declared search allowance. One 2-bit presence tag per node.
pub(super) fn both_present_nodes(p: &Party) -> u64 {
    let bits = p.as_bits();
    let mut count = 0u64;
    let mut i = 0;
    while i + 1 < bits.len() {
        count += u64::from(bits[i] && bits[i + 1]);
        i += 2;
    }
    count
}

/// The declared-model band: a modeled reading must sit within
/// `[CAPACITY_MODEL_FLOOR, CAPACITY_MODEL_CEILING] × model`.
///
/// The capacity-chain model fits the committed probe points within 2%
/// \[measured 2026-07-28, release: measured/model 1.005–1.017 across
/// teeth 128–1024\], so ±10% absorbs the walk's small non-chain
/// allocations while a regressed builder — an unanchored doubling chain,
/// an extra buffer copy — overshoots the ceiling, and an improved
/// builder undershoots the floor and forces a deliberate re-declaration.
pub const CAPACITY_MODEL_CEILING: f64 = 1.10;
/// The declared-model band's lower edge; see [`CAPACITY_MODEL_CEILING`].
pub const CAPACITY_MODEL_FLOOR: f64 = 0.90;

/// The ratified capacity-chain peak-heap model for the output-dominated
/// projection's builder: `3·(n+m)·2^(k−1)` bytes.
///
/// `k = ⌈log2(output/(n+m))⌉`, clamped to at least 1 — the committed
/// shapes sit at output ≥ 32× input, so the clamp never binds and
/// exists only to keep the formula total.
///
/// Derivation: the projection's output is not size-derivable from its
/// operands (mandatory `Θ(|v|·|p|)` output on `Θ(|v|+|p|)` input), so no
/// reserve-once bound exists; the output builder anchors its buffer at
/// the operand-size reserve `n+m` and doubles `k` times to reach the
/// output, and peak heap is the last realloc's old+new coexistence:
/// `(n+m)·2^(k−1) + (n+m)·2^k = 3·(n+m)·2^(k−1)`. The board's exponent
/// fit is honestly unjudgeable across this chain — a probe pair
/// straddling a `k` step manufactures an exponent out of the quantized
/// capacity, one inside a step reads sublinear — which is exactly why
/// these cells are judged against the model instead (owner ratification
/// 2026-07-27: the doubling-chain band is the accepted stated-band
/// residual — no pre-walk, no segmented output).
pub(super) fn capacity_chain_peak(input_bytes: usize, output_bytes: usize) -> f64 {
    let anchor = input_bytes as f64;
    let k = (output_bytes as f64 / anchor).log2().ceil().max(1.0);
    3.0 * anchor * (k - 1.0).exp2()
}

/// The fold rows' declared exponent ceiling over the fold currencies
/// (limb, scan, touch): the `FoldLog` model's own predicted exponent plus
/// the global noise slack.
///
/// Work `c·D·log2(2k)` fitted across the cell's two probes
/// (`D₁, k₁) → (D₂, k₂`) reads exponent
/// `1 + log2(log2(2k₂)/log2(2k₁)) / log2(D₂/D₁)` — the log factor's
/// marginal, ~1.14–1.17 at the committed populations — so the ceiling is
/// that prediction plus the same slack [`MAX_SCALING_EXPONENT`] grants
/// linear cells (0.15). A quadratic fold reads ~2 against any committed
/// arity pair and stays red; the model's own liveness is the
/// `fold_log_factor_is_alive` pin, which reads red the day the reduction
/// stops paying its log factor.
pub(super) fn fold_exponent_ceiling(k1: u64, k2: u64, n1: usize, n2: usize) -> f64 {
    let levels1 = (2.0 * k1 as f64).log2();
    let levels2 = (2.0 * k2 as f64).log2();
    let denom_growth = (n2 as f64 / n1 as f64).log2();
    1.0 + (levels2 / levels1).log2() / denom_growth + (MAX_SCALING_EXPONENT - 1.0)
}

/// The tooth-tail parse rows' family-stated heap ceiling, in bytes per
/// text byte.
///
/// `version_parse_noncanon` on the tooth-tail column is judged at this
/// flat constant in place of [`MAX_HEAP_BYTES_PER_INPUT_BYTE`] (the
/// declared-models section; the exponent leg stays at the global
/// bound).
///
/// Derivation: the tooth-tail pair is the board's densest
/// node-per-text-byte family — its text spells thousands of
/// single-digit unit leaves at ~5 text bytes per tree node where every
/// other committed family's text carries multi-digit values — so the
/// parser's materialized tree plus its transient scaffolding
/// legitimately exceeds the global 16 B/B flat allowance at a flat
/// exponent, and parse paths are not constant-optimized by owner
/// policy. Measured 2026-07-28 (release, both acceptance scales):
/// 20.7 → 20.8 B per text byte at heap exponent 1.00 — a constant, not
/// a class; the ceiling is the worst reading ×1.25 (owner ratification
/// 2026-07-28, conditional on exactly this flat-constant profile). A
/// reading over it is a genuine parse-heap regression on the densest
/// committed stream.
pub const TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE: f64 = 26.0;

/// The ascending-cliff tick trio's family-stated heap ceiling, in bytes
/// per packed input byte.
///
/// `version_tick`, `version_ticks`, and `clock_tick` on the ascend-cliff
/// cross are judged at this flat constant in place of
/// [`MAX_HEAP_BYTES_PER_INPUT_BYTE`] (the declared-models section; the
/// exponent leg stays at the global bound).
///
/// Derivation: the ascending cliff is the one committed shape that
/// defeats certificate consumption — the accumulator's zero-run ledger
/// certificates on a monotone climb occupy memory until consumed,
/// bounded at one entry per jump-write (at most half the held digit
/// positions; the bound verified against the ledger code) — so the tick
/// walk's live certificate state is honest `Θ(input)` work-state with a
/// large constant, intended and modeled, not amplification. Measured
/// 2026-07-28 (release, both acceptance scales): 123.8 → 125.9 B per
/// input byte at heap exponent 1.00 — a constant, not a class; the
/// ceiling is the worst reading ×1.25, rounded up (owner ratification
/// 2026-07-28, conditional on exactly this flat-constant profile). A
/// reading over it is a genuine certificate-memory regression on the
/// one shape that defeats consumption.
pub const ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE: f64 = 158.0;

/// The ascending-cliff `version_min_ticks` cell's family-stated heap
/// ceiling, in bytes per packed input byte (judged in place of
/// [`MAX_HEAP_BYTES_PER_INPUT_BYTE`]; the exponent leg stays at the
/// global bound).
///
/// Derivation: the exact fold's anchor web holds one live reign record
/// per simultaneously-open minimum, and the ascending cliff is the one
/// committed shape that defeats batching — `Θ(k)` minima stay open at
/// once, so the fold legitimately holds `Θ(k)` live reign records
/// (~119 B each, the state that keeps the fold's *exponent* linear) at
/// a flat per-byte constant: intended and modeled, not amplification.
/// Measured 2026-07-28 (release, both acceptance scales):
/// 138.7 → 141.3 B per input byte at heap exponent 1.00 — a constant,
/// not a class; the ceiling is the worst reading ×1.25, rounded up
/// (owner ratification 2026-07-28, conditional on exactly this
/// flat-constant profile). A reading over it is a genuine reign-state
/// regression on the one shape that defeats batching.
pub const ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE: f64 = 177.0;

/// The mirror-wide display pair's declared render model: the limb
/// *exponent* ceiling, judged in place of [`MAX_SCALING_EXPONENT`].
///
/// Declared on exactly the `version_display` and `clock_display`
/// mirror-wide cells (the declared-models section; every other column
/// stays at the global bounds).
///
/// Derivation: the render's summary merge on a deep tree of wide
/// interior values is the documented `SuperlinearTime` class (the
/// display impls' `# Complexity` sections; judge-rostered red on the
/// wall leg), so on the mirror-wide cross the limb column honestly
/// reads a superlinear exponent against `n_io` — intended and modeled,
/// not a regression. Measured 2026-07-28 (release, the two acceptance
/// scales): fitted limb exponents 1.55 → 1.81 (`version_display`) and
/// 1.56 → 1.81 (`clock_display`). The ceiling is the worst measured
/// exponent plus the linear cells' slack (1.81 + 0.15, the
/// [`MAX_SCALING_EXPONENT`] margin), so a genuinely quadratic
/// conversion (~2.0) still reads red. The model's under-side is not
/// banded here: the class's liveness floor is the committed
/// `render_merge_superlinearity_is_alive` pin, which reads red the day
/// a render-merge cure lands and forces this declaration's
/// re-derivation in the same change (owner ratification 2026-07-28:
/// the display pair's superlinearity is the documented, judge-rostered
/// class).
pub const MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING: f64 = 1.96;

/// The mirror-wide display pair's declared render model: the limb
/// *constant* ceiling per radix unit, judged in place of
/// [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`] on the same two cells.
///
/// κ is calibrated on conversion-honest cells; the mirror-wide render
/// merge re-folds wide summaries beyond conversion, so its per-`R`
/// constant honestly exceeds κ at the acceptance scales — the same
/// mechanism as the exponent ceiling above, priced on the constant leg.
/// Measured 2026-07-28 (release, both acceptance scales): 0.7 → 1.4
/// (`version_display`) and 0.6 → 1.3 (`clock_display`) limb per `R`
/// unit; the ceiling is the worst reading ×1.25 (owner ratification
/// 2026-07-28, conditional on the render-merge mechanism the liveness
/// pin holds).
pub const MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT: f64 = 1.75;

/// The acceptance scale: the size multiplier of the acceptance-mode
/// board run (`just amp-board-acceptance`).
///
/// The default-scale board under-detects segment amplifiers: stacker grows
/// a segment only past ~1 MiB of frames, so a recursion-frame amplifier
/// whose onset sits above the default depths reads a false green there.
/// ×4 is the witnessed calibration floor — the scale at which every known
/// segment-onset amplifier read red under pre-fix code — so acceptance runs
/// pin it. **Campaign acceptance is all cells green at BOTH the default
/// scale and this one, one run each under the determinism tripwire**; an
/// acceptance-scale run is
/// acceptance-time only (the inner loop stays at the default scale, and the
/// enforced per-operation record remains the envelope suite in
/// `tests/meter.rs` regardless of board onset).
pub const ACCEPTANCE_SCALE: f64 = 4.0;
