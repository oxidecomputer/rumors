//! One prepared cell: the measured body, the operand bytes it charges
//! against, its denomination rule, and its committed liveness
//! declarations.

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
    /// How the meters are denominated (the module doc's criterion).
    pub(super) denom: Denom,
    /// The cell's liveness declarations, one per floored column.
    pub(super) floors: Floors,
    /// The fold rows' operand count at this scale: `Some` on the two
    /// n-ary fold rows only, where it drives the declared `FoldLog`
    /// model (the declared-models section above).
    pub(super) fold_arity: Option<u64>,
    /// The party fold's declared search allowance at this scale, in
    /// scan bits ([`INDEX_PROBE_SCAN_BITS`](super::ceilings::INDEX_PROBE_SCAN_BITS)'s derivation).
    ///
    /// Added to the declared scan ceiling; zero on the version fold (no
    /// overlap test) and wherever the operands carry no both-present
    /// structure.
    pub(super) fold_search_bits: u64,
    /// Whether the heap column is judged against the ratified
    /// capacity-chain model ([`capacity_chain_peak`](super::ceilings::capacity_chain_peak)) instead of the
    /// flat ceiling: the output-dominated projection on the
    /// comb-scatter cross only.
    pub(super) capacity_model: bool,
    /// A family-stated flat heap ceiling in bytes per denominator byte,
    /// judged in place of [`MAX_HEAP_BYTES_PER_INPUT_BYTE`](super::ceilings::MAX_HEAP_BYTES_PER_INPUT_BYTE)'s.
    ///
    /// The declared-models mechanism at a flat constant, for the cell
    /// classes whose honest constant a ratified derivation puts over
    /// the global allowance (each declaring constant —
    /// [`TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE`](super::ceilings::TOOTH_TAIL_PARSE_HEAP_BYTES_PER_TEXT_BYTE),
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

/// A cell's denomination rule (see the module doc's list of which rows get
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
    /// model at operand count `arity` (the declared-models section).
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
    /// capacity-chain model (the declared-models section).
    pub(super) fn with_capacity_model(mut self) -> Cell {
        self.capacity_model = true;
        self
    }

    /// Declare this cell's heap constant judged against a family-stated
    /// flat ceiling (the declared-models section); the exponent leg
    /// stays at the global bound.
    pub(super) fn with_declared_heap(mut self, bytes_per_denom_byte: f64) -> Cell {
        self.declared_heap = Some(bytes_per_denom_byte);
        self
    }

    /// Declare this cell's limb column judged against a family-stated
    /// model (the declared-models section): `exponent` replaces the
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
