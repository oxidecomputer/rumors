//! The measurement engine: run one prepared cell under every meter and capture
//! its counter readings and settled denominators as a [`Sample`].

use super::cell::{Cell, Denom, ModelSpec, Units};
use super::currency::{ByCurrency, Liveness};

/// The peak-heap meter the board reads, supplied by the binary that runs it.
///
/// A counting global allocator is per-binary state the library cannot own, so
/// the runner (the `amp_board` example, the smoke test) installs one and passes
/// readers in. The board calls `reset_peak` before each cell's body and reads
/// `peak` once the body has produced its result, so a cell's heap reading is
/// the most its body raised live bytes above their level at the reset.
///
/// The readers need only agree with each other: a runner may count
/// process-wide, provided no other thread allocates while a cell is measured,
/// or count the measuring thread alone.
pub struct HeapMeter {
    /// Begin a fresh peak reading at the current live level.
    pub reset_peak: fn(),
    /// Return the most that live bytes have risen above their level at the
    /// last `reset_peak`, or zero if they never rose above it.
    pub peak: fn() -> usize,
}

/// The fuel meter the board reads, supplied by a wasm guest that runs it.
///
/// Fuel is the Wasmtime host's count of the instructions a guest instance has
/// executed, so only a guest can read it, through an import its host
/// provides. The board calls `enter` immediately before each cell's body and
/// `exit` once the body has produced its result, while the result is still
/// alive. Each returns the fuel consumed so far; the difference is the cell's
/// fuel reading, and the span between the two reads is the cell's *measured
/// window*.
///
/// Besides reading fuel, the hooks open and close the window. A runner uses
/// them to keep the reading a function of the body alone: what an allocator
/// call costs depends on the heap it runs against, and the cell's preparation
/// allocates in the same instance just before the window. The fuel guest's
/// `enter` gives the body a fresh heap, primed so that its one-time setup is
/// already done, and its `exit` retires it, so the allocator's share of a
/// reading is its work for the body's own allocations.
///
/// Because each hook reads fuel itself, the board makes one call at each end
/// of the window, so a runner whose `enter` reads fuel as its last act, and
/// whose `exit` reads it as its first, runs nothing between the two reads
/// that a bare fuel reader would not.
///
/// State outside the heap can carry between cells too, such as a lazily
/// initialized static, so a runner also measures each cell in a fresh
/// instance (the `shard` module's one-cell deal).
pub struct FuelMeter {
    /// Open the measured window, then return the fuel this instance has
    /// consumed so far.
    pub enter: fn() -> u64,
    /// Return the fuel this instance has consumed so far, then close the
    /// measured window.
    pub exit: fn() -> u64,
}

/// The meters a runner supplies to the board.
///
/// Heap and fuel are read through state only the running binary owns. A
/// currency whose meter is absent reads `None` in every sample, and the
/// board neither judges nor renders it.
pub struct Meters {
    /// The peak-heap meter, or `None` where the runner counts no allocations.
    pub heap: Option<HeapMeter>,
    /// The fuel meter, or `None` outside a wasm guest.
    pub fuel: Option<FuelMeter>,
}

/// A resource model resolved to this sample's concrete units.
///
/// Units count work, not memory, so they are `u64` on every target: a model
/// such as `n²` exceeds a 32-bit `usize` at inputs a wasm32 guest measures.
#[derive(Clone, Copy)]
pub(super) struct Model {
    /// Units against which the growth trend is fitted.
    pub(super) trend_units: u64,
    /// Units against which the proportional constant is checked.
    pub(super) constant_units: u64,
    /// Optional replacement for the currency's global proportional ceiling.
    pub(super) ceiling: Option<f64>,
}

/// One measured run of a cell body: every meter and its denominators.
#[derive(Clone)]
pub(super) struct Sample {
    /// The default proportional units: input bytes, or `n_io` for an
    /// I/O-denominated cell.
    pub(super) denom_bytes: usize,
    /// The default growth units.
    ///
    /// `denom_bytes` everywhere except the flat-denominator shape's
    /// input-denominated cells, where it is the bundle's value content: the
    /// byte denominator is intercept-dominated there, and a two-point
    /// power-law fit against an intercept-dominated denominator manufactures
    /// exponents out of exactly linear marginal work. A resource model may
    /// replace this axis for one currency.
    pub(super) exp_denom_bytes: usize,
    /// The cell's liveness declarations; each sample carries its own since
    /// floors scale with the sample's operands.
    pub(super) floors: ByCurrency<Liveness>,
    /// Resource models that differ from the global linear defaults.
    pub(super) models: ByCurrency<Option<Model>>,
    /// Every currency's counter reading over the body; `None` where the
    /// runner supplies no meter for it (an absent [`Meters`] field), which
    /// exempts it from judgment.
    pub(super) readings: ByCurrency<Option<u64>>,
}

/// Run one prepared cell under all meters.
///
/// The denominators are settled after the meters are read and before the result
/// is dropped: an I/O-denominated cell's output side comes from the actual
/// result rather than a prediction. The peak includes result allocations as
/// well as scratch; releasing an input during the operation can reduce the
/// extra live heap relative to its level at the reset.
///
/// Fuel is read last before the body and first after it, so the heap meter's
/// own work stays out of the fuel reading.
pub(super) fn measure(meters: &Meters, mut cell: Cell, content: Option<usize>) -> Sample {
    if let Some(heap) = &meters.heap {
        (heap.reset_peak)();
    }
    let fuel_origin = meters.fuel.as_ref().map(|fuel| (fuel.enter)());
    let mut observation = None;
    (cell.body)(&mut |result| {
        let fuel = meters.fuel.as_ref().zip(fuel_origin).map(|(fuel, origin)| {
            (fuel.exit)()
                .checked_sub(origin)
                .expect("an instance's consumed fuel never decreases")
        });
        let readings = ByCurrency {
            heap: meters.heap.as_ref().map(|heap| (heap.peak)() as u64),
            fuel,
        };
        let (denom_bytes, exp_denom_bytes) = match &cell.denom {
            // The flat-denominator shape's content denominator carries the exponent
            // legs of its input-denominated cells alone: an I/O-denominated cell's
            // output side already scales.
            Denom::Input => {
                let exp = content.unwrap_or(cell.input_bytes);
                (cell.input_bytes, exp)
            }
            Denom::Io(spec) => {
                let output_bytes = (spec.output_bytes)(result);
                let n_io = cell.input_bytes + output_bytes;
                (n_io, n_io)
            }
        };
        observation = Some((readings, denom_bytes, exp_denom_bytes));
    });
    let (readings, denom_bytes, exp_denom_bytes) =
        observation.expect("a cell observes its result once");
    let resolve = |spec: Option<ModelSpec>| -> Option<Model> {
        let spec = spec?;
        let ordinary_constant = denom_bytes;
        let units = |units: Units, ordinary: usize| -> u64 {
            let ordinary = ordinary as u64;
            let resolved = match units {
                Units::Default => ordinary,
                Units::Scale(factor) => {
                    let scaled = (ordinary as f64 * factor).ceil();
                    // A float-to-integer cast saturates silently, so the
                    // range is checked before it.
                    assert!(
                        scaled < u64::MAX as f64,
                        "resource-model units fit in a u64"
                    );
                    scaled as u64
                }
                Units::Explicit(units) => units,
            };
            assert!(resolved > 0, "resource-model units are positive");
            resolved
        };
        Some(Model {
            trend_units: units(spec.trend, exp_denom_bytes),
            constant_units: units(spec.constant, ordinary_constant),
            ceiling: spec.ceiling,
        })
    };
    let models = ByCurrency::from_fn(|currency| resolve(*cell.models.get(currency)));
    Sample {
        denom_bytes,
        exp_denom_bytes,
        floors: cell.floors.with_fuel(),
        models,
        readings,
    }
}
