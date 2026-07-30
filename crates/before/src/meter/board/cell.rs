//! One prepared cell: the measured body, the operand bytes it charges
//! against, its denomination rule, and its committed liveness
//! declarations.
//!
//! # Denomination
//!
//! Most cells charge cost against packed input bytes alone; the board
//! module doc's Denomination section states the default and the rule
//! that a mandatory-output cell is judged against total I/O bytes
//! `n_io`, its output side read back from the actual result. The
//! re-denominated classes and their derivations:
//!
//! - **Text rows** (`Display` and `FromStr` for all three types): `n_io` is
//!   packed input + text output (`Display`) or text input + packed output
//!   (`FromStr`). Their heap and segment columns are judged per `n_io` byte
//!   at the unchanged ceilings. Their **limb column** is judged on two legs
//!   that exclude different converters. The *exponent* leg — against `n_io`,
//!   like every exponent — is what excludes quadratic conversion: any
//!   schoolbook converter's limb work is `Θ(digits × limbs)`, quadratic in
//!   the value bits however wide its chunks, and reads ~2 there \[measured\].
//!   The *constant* leg — against the radix-work denominator
//!   `R = n_io + Σᵢ (digitsᵢ × limbsᵢ +
//!   TEXT_PIPELINE_LIMB_OPS_PER_VALUE)` over the event values the text
//!   spells (the honest text cost law: schoolbook conversion plus the
//!   delta⇄absolute pipeline's measured per-value arithmetic), at the
//!   ceiling [`MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT`](super::ceilings::MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT) — is what excludes a
//!   wasteful constant: a digit-by-digit schoolbook probe scores ~1 limb
//!   per `R` unit, over κ, while the honest kernels' worst family reads
//!   0.59 \[measured — the test suite's tripwires\]. Only a converter
//!   whose recorded limb work is near-linear in `n_io` with a
//!   pipeline-class constant reads green.
//! - **Flat-denominator exponents** (the comb-scatter shape): the shape
//!   deliberately scales tooth *count* at a fixed 1000-bit tooth
//!   magnitude, so its packed bytes are intercept-dominated — the one
//!   wide leading code plus unit delta codes per tooth — and grow only
//!   ~x1.2 while every slot's value content (and every operation's honest
//!   per-tooth work) doubles per level. A two-point power-law fit against
//!   an intercept-dominated denominator manufactures exponents out of
//!   exactly linear marginal work (log 2 / log 1.2 = 4), so the shape's
//!   input-denominated cells fit their *exponents* against the bundle's
//!   value content (the event side's summed leaf-height bits plus the id
//!   side's packed bytes — the honest scaling axis),
//!   disclosed per row as `expd[content ...]`. Constants and floors stay
//!   per packed byte, the harder reading; I/O-denominated cells keep
//!   `n_io`, whose output side already scales. The tripwire pair below
//!   pins both directions: the packed fit reads a manufactured exponent
//!   on measured flat per-tooth work, and a genuinely quadratic-in-teeth
//!   probe still reads red against the content denominator.
//! - **Output-dominated projection** (`own_version_to_version` and
//!   `clock_own_version_to_version` on the comb × scattered-party cross and the
//!   plateau-comb crosses — reveal-comb, reveal-hifloor, pure-comb):
//!   `n_io` is packed input + packed output. These crosses exist because
//!   the id keeps a wide magnitude per owned site — the scattered party a
//!   wide magnitude per kept tooth (`Θ(e·k)` mandatory output bits), the
//!   plateau ids a re-materialized `2^b`-scale code per kept site
//!   (`Θ(k·b)` output on a `Θ(k + b)` input) — and a packed output cannot
//!   be padded, so `n_io` is the honest denominator on all columns at the
//!   unchanged ceilings, with the projection sweep measured
//!   O(`n_io`)-tight on every one (exponents ≈ 1.0 against `n_io`, scan
//!   at the walk's usual 8 bits per `n_io` byte).
//!
//! The **output-honesty assertion** closes the pad-the-output door on the
//! text side: any text stream entering a denominator must satisfy
//! `text_bytes ≤` [`TEXT_BYTES_PER_RADIX_UNIT`] `× radix units` of the
//! values it spells, checked against the actual bytes.
//!
//! **Do not re-denominate** (these stay input-denominated): both binary
//! codec directions (the coding is canonical 1:1, so input bytes are the
//! honest bound); every scalar, comparison, and query row (word-sized or
//! borrowed results); and the packed-output mutator rows (`join`, `meet`,
//! `tick`, `fork`, `recv`, `sync`, `without`, and every
//! projection cell outside the output-domination cross) — their input denomination rests on output
//! coding ≤ inputs + O(1) per overlay boundary, which is pinned for
//! join/meet as the 1-Lipschitz proptest in
//! [`tier2`](crate::meter::tier2)'s test suite rather than assumed.
//!
//! **Rank operands** (`rank_pair_ops`, `rank_sum`, and `rank_encode`'s
//! input side) are in-memory values with no packed operand form; their
//! denominator of record is the operands' **value content**
//! `bits(num) + exp` in bytes. That content is wire-bounded: every
//! public construction path (the `rank`/`distance`/`lag` folds) emits a
//! rank whose numerator width and exponent are each linear in the
//! packed bits the fold read, so a ceiling per content byte is a
//! ceiling per wire byte up to the fold's own constant. `rank_encode`
//! is I/O-denominated (content in plus the actual canonical bytes out,
//! read back from the result), with the emission's honesty asserted at
//! prepare — the canonical form is at most `9⁄8 · ‖r‖ + O(log ‖r‖)`
//! bits, so a padded
//! output cannot inflate the denominator; `rank_decode`'s operand *is*
//! the canonical bytes, input-denominated like every codec row; and
//! `ranked_encode` stays input-denominated because its output is
//! provenance-bounded within the packed input (asserted at prepare), so
//! input bytes are the honest, harder denominator and the
//! flat-denominator shape's content exponent governs it exactly as it
//! governs `version_rank`.

use std::any::Any;

use super::ceilings::TEXT_BYTES_PER_RADIX_UNIT;
use super::currency::Floors;

/// One prepared cell run: the operand bytes it charges against, the
/// denomination rule, and the body to measure.
///
/// `prepare` builds (and decodes) operands outside measurement; the body's
/// result is boxed and kept alive until the meters are read, so peak heap
/// includes the fully materialized output.
pub(super) struct Cell {
    /// The packed (or, on `FromStr` rows, text) operand bytes.
    pub(super) input_bytes: usize,
    /// How the meters are denominated (the board module doc's criterion).
    pub(super) denom: Denom,
    /// The cell's liveness declarations, one per floored column.
    pub(super) floors: Floors,
    /// The fold rows' operand count at this scale: `Some` on the two
    /// n-ary fold rows only, where it drives the declared `FoldLog`
    /// model (the `ceilings` module's declared-models section).
    pub(super) fold_arity: Option<u64>,
    /// The party fold's declared search allowance at this scale, in
    /// scan bits ([`INDEX_PROBE_SCAN_BITS`](super::ceilings::INDEX_PROBE_SCAN_BITS)'s derivation).
    ///
    /// Added to the declared scan ceiling; zero on the version fold (no
    /// overlap test) and wherever the operands carry no both-present
    /// structure.
    pub(super) fold_search_bits: u64,
    /// Whether the heap column is judged against the ratified
    /// capacity-chain model instead of the flat ceiling.
    ///
    /// The output-dominated projection on the comb-scatter cross only;
    /// [`capacity_chain_peak`](super::ceilings::capacity_chain_peak)
    /// carries the model.
    pub(super) capacity_model: bool,
    /// A family-stated flat heap ceiling in bytes per denominator byte,
    /// judged in place of [`MAX_HEAP_BYTES_PER_INPUT_BYTE`](super::ceilings::MAX_HEAP_BYTES_PER_INPUT_BYTE)'s.
    ///
    /// The declared-models mechanism at a flat constant, for the cell
    /// classes whose honest constant a ratified derivation puts over
    /// the global allowance (each declaring constant —
    /// [`ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE`](super::ceilings::ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE),
    /// [`ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE`](super::ceilings::ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE) — carries
    /// its derivation). The exponent leg is untouched: the declaration
    /// buys a constant, never growth.
    pub(super) declared_heap: Option<f64>,
    /// A family-stated limb model `(exponent ceiling, per-radix-unit
    /// constant ceiling)`, judged in place of the global limb legs.
    ///
    /// The declared-models mechanism for a documented superlinear time
    /// class: the display pair's mirror-wide cells only
    /// ([`MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING`](super::ceilings::MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING) and
    /// [`MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT`](super::ceilings::MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT) carry the
    /// derivation). Meaningful only on text rows, whose limb constant
    /// is denominated per `R` unit.
    pub(super) declared_limb: Option<(f64, f64)>,
    /// The measured body; its result stays alive until the meters are read.
    #[allow(clippy::type_complexity)]
    pub(super) body: Box<dyn FnOnce() -> Box<dyn Any>>,
}

/// A cell's denomination rule (the module doc above lists which rows get
/// which).
pub(super) enum Denom {
    /// Input bytes alone: the default, and the only rule most rows may use.
    Input,
    /// Total I/O bytes: input plus the actual output, read back from the
    /// measured result after the meters are captured.
    Io(IoSpec),
}

/// The I/O-denomination data for a mandatory-output cell.
pub(super) struct IoSpec {
    /// Read the actual output's byte size from the boxed result.
    pub(super) output_bytes: fn(&dyn Any) -> usize,
    /// The text rows' extra terms; `None` for packed-output cells.
    pub(super) text: Option<TextSpec>,
}

/// The text rows' radix-work term and output-honesty data.
pub(super) struct TextSpec {
    /// `Σ digitsᵢ × limbsᵢ` over the values the text spells.
    ///
    /// The limb column is judged against `R = n_io +` this `+` the
    /// pipeline term below, at the κ ceiling; the output-honesty ceiling
    /// is asserted against these units alone (the pipeline term must not
    /// loosen it).
    pub(super) radix_units: u64,
    /// The spelled event values, each granting
    /// [`TEXT_PIPELINE_LIMB_OPS_PER_VALUE`](super::ceilings::TEXT_PIPELINE_LIMB_OPS_PER_VALUE) radix units in `R`.
    ///
    /// Zero on id-only text (boolean tokens force no arithmetic), and
    /// the version side's node count on clock rows for the same reason.
    pub(super) spelled_values: u64,
    /// Whether the measured *output* is the text side (`Display`); the
    /// honesty assertion then runs against the actual output bytes.
    /// `FromStr`'s text is input and is asserted at prepare.
    pub(super) output_is_text: bool,
}

impl Cell {
    /// Package an input-denominated body with its operand byte count and its
    /// liveness declarations.
    pub(super) fn new<R: Any>(
        input_bytes: usize,
        floors: Floors,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Input,
            floors,
            fold_arity: None,
            fold_search_bits: 0,
            capacity_model: false,
            declared_heap: None,
            declared_limb: None,
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Declare this cell's readings judged under the fold rows' `FoldLog`
    /// model at operand count `arity` (the `ceilings` module's
    /// declared-models section).
    pub(super) fn with_fold_arity(mut self, arity: u64) -> Cell {
        self.fold_arity = Some(arity);
        self
    }

    /// Declare the party fold's search allowance in scan bits
    /// ([`INDEX_PROBE_SCAN_BITS`](super::ceilings::INDEX_PROBE_SCAN_BITS)'s derivation).
    pub(super) fn with_fold_search(mut self, bits: u64) -> Cell {
        self.fold_search_bits = bits;
        self
    }

    /// Declare this cell's heap judged against the ratified
    /// capacity-chain model (the `ceilings` module's declared-models
    /// section).
    pub(super) fn with_capacity_model(mut self) -> Cell {
        self.capacity_model = true;
        self
    }

    /// Declare this cell's heap constant judged against a family-stated
    /// flat ceiling (the `ceilings` module's declared-models section); the
    /// exponent leg
    /// stays at the global bound.
    pub(super) fn with_declared_heap(mut self, bytes_per_denom_byte: f64) -> Cell {
        self.declared_heap = Some(bytes_per_denom_byte);
        self
    }

    /// Declare this cell's limb column judged against a family-stated
    /// model (the `ceilings` module's declared-models section):
    /// `exponent` replaces the
    /// global exponent bound, `per_radix_unit` the text ceiling κ.
    pub(super) fn with_declared_limb(mut self, exponent: f64, per_radix_unit: f64) -> Cell {
        self.declared_limb = Some((exponent, per_radix_unit));
        self
    }

    /// Package an I/O-denominated packed-output body: the output side of
    /// `n_io` is read back from the actual result.
    pub(super) fn io<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec {
                output_bytes,
                text: None,
            }),
            floors,
            fold_arity: None,
            fold_search_bits: 0,
            capacity_model: false,
            declared_heap: None,
            declared_limb: None,
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Package a text-row body: I/O-denominated, with the limb column judged
    /// against the radix-work denominator.
    pub(super) fn text<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        spec: TextSpec,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec {
                output_bytes,
                text: Some(spec),
            }),
            floors,
            fold_arity: None,
            fold_search_bits: 0,
            capacity_model: false,
            declared_heap: None,
            declared_limb: None,
            body: Box::new(move || Box::new(body())),
        }
    }
}

/// Assert the output-honesty ceiling on one text stream.
///
/// Every text stream entering a denominator passes through here: text bytes
/// at most [`TEXT_BYTES_PER_RADIX_UNIT`] per radix unit of the values the
/// text spells, so padding the text side of `n_io` trips the run instead
/// of greening a cell.
pub(super) fn assert_honest_text(what: &'static str, text_bytes: usize, radix_units: u64) {
    assert!(
        text_bytes as f64 <= TEXT_BYTES_PER_RADIX_UNIT * radix_units as f64,
        "output honesty: {what}: {text_bytes} text bytes exceed \
         {TEXT_BYTES_PER_RADIX_UNIT} per radix unit over {radix_units} units"
    );
}
