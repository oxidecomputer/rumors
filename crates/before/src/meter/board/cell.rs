//! One prepared measurement: its body, cost model, and liveness declarations.
//!
//! # Denomination
//!
//! Encoded input bytes are the default cost axis. When producing the result is
//! itself required work and its encoding can dominate the input, the cell uses
//! actual input-plus-output bytes instead. In-memory numeric operands use their
//! value width, which their public construction paths bound by encoded input.
//!
//! A shape whose encoding has a large fixed intercept may provide a separate
//! content axis for the growth fit. Constants remain charged to encoded bytes,
//! so this prevents a misleading exponent without relaxing the byte ceiling.
//!
//! # Resource models
//!
//! Every currency has two checks: growth against `trend` units and the largest
//! reading against `constant` units. The default units implement the board's
//! linear contract. A cell may instead state the units of a more precise bound,
//! such as `D log k`, and may replace that currency's proportional ceiling.
//! The judge treats all such models alike and always retains the global growth
//! ceiling. The rendered row discloses every override.

use std::any::Any;

use super::currency::{ByCurrency, Currency, Floors};

/// How one side of a resource model derives its units.
#[derive(Clone, Copy)]
pub(super) enum Units {
    /// Use the board's ordinary units for this side of the model.
    Default,
    /// Multiply the ordinary units by this factor.
    Scale(f64),
    /// Use an operation-derived number of units directly.
    Explicit(usize),
}

/// A cell's expected bound for one measured resource.
///
/// `trend` is the axis against which growth is fitted. `constant` is the axis
/// against which the largest reading is normalized. Keeping them separate
/// expresses, for example, a logarithmic marginal factor without weakening a
/// tighter proportional constant. An absent `ceiling` retains the currency's
/// global ceiling.
#[derive(Clone, Copy)]
pub(super) struct ModelSpec {
    /// How this cell derives units for the growth fit.
    pub(super) trend: Units,
    /// How this cell derives units for the proportional check.
    pub(super) constant: Units,
    /// A cell-specific proportional ceiling, if the global one does not apply.
    pub(super) ceiling: Option<f64>,
}

impl ModelSpec {
    /// Use ordinary units and replace only the proportional ceiling.
    pub(super) fn ceiling(ceiling: f64) -> Self {
        assert!(
            ceiling.is_finite() && ceiling > 0.0,
            "a model ceiling is positive"
        );
        Self {
            trend: Units::Default,
            constant: Units::Default,
            ceiling: Some(ceiling),
        }
    }

    /// Judge both growth and proportional cost against `units`.
    pub(super) fn work(units: usize) -> Self {
        assert!(units > 0, "resource-model work units are positive");
        Self {
            trend: Units::Explicit(units),
            constant: Units::Explicit(units),
            ceiling: None,
        }
    }

    /// Scale the growth axis while retaining the ordinary constant axis.
    pub(super) fn scaled_trend(factor: f64) -> Self {
        assert!(
            factor.is_finite() && factor > 0.0,
            "a model scale is positive"
        );
        Self {
            trend: Units::Scale(factor),
            constant: Units::Default,
            ceiling: None,
        }
    }

    /// Scale both axes and replace the proportional ceiling.
    pub(super) fn scaled(factor: f64, ceiling: f64) -> Self {
        assert!(
            factor.is_finite() && factor > 0.0,
            "a model scale is positive"
        );
        assert!(
            ceiling.is_finite() && ceiling > 0.0,
            "a model ceiling is positive"
        );
        Self {
            trend: Units::Scale(factor),
            constant: Units::Scale(factor),
            ceiling: Some(ceiling),
        }
    }
}

/// One prepared cell run: the operand bytes it charges against, the
/// denomination rule, and the body to measure.
///
/// `prepare` builds (and decodes) operands outside measurement; the body's
/// result is boxed and kept alive until the meters are read, so peak heap
/// includes the fully materialized output.
pub(super) struct Cell {
    /// The operand bytes.
    pub(super) input_bytes: usize,
    /// How the meters are denominated (the board module doc's criterion).
    pub(super) denom: Denom,
    /// The cell's liveness declarations, one per floored column.
    pub(super) floors: Floors,
    /// Resource models that differ from the board's global linear defaults.
    pub(super) models: ByCurrency<Option<ModelSpec>>,
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
            models: ByCurrency {
                heap: None,
                segments: None,
                scan: None,
                touch: None,
            },
            body: Box::new(move || Box::new(body())),
        }
    }

    /// Replace one currency's global linear model for this cell.
    pub(super) fn with_model(mut self, currency: Currency, model: ModelSpec) -> Cell {
        *self.models.get_mut(currency) = Some(model);
        self
    }

    /// Package an I/O-denominated encoded-output body: the output side of `n_io`
    /// is read back from the actual result.
    pub(super) fn io<R: Any>(
        input_bytes: usize,
        floors: Floors,
        output_bytes: fn(&dyn Any) -> usize,
        body: impl FnOnce() -> R + 'static,
    ) -> Cell {
        Cell {
            input_bytes,
            denom: Denom::Io(IoSpec { output_bytes }),
            floors,
            models: ByCurrency {
                heap: None,
                segments: None,
                scan: None,
                touch: None,
            },
            body: Box::new(move || Box::new(body())),
        }
    }
}
