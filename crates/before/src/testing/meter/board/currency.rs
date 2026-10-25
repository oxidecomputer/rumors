//! Resource quantities judged by the amplification board.
//!
//! [`ByCurrency`] has one required field for each quantity. Adding a quantity
//! therefore makes every reading, ceiling, floor, and rendering site fail to
//! compile until it handles the new measurement.

/// One deterministic resource measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Currency {
    /// Peak additional live heap bytes, including retained results.
    Heap,
    /// Wasm instructions executed, as Wasmtime fuel (execution cost; read
    /// only when the board runs inside a wasm guest that supplies a
    /// [`FuelMeter`](super::FuelMeter)).
    Fuel,
}

impl Currency {
    /// The currency's rendered column name.
    pub fn label(self) -> &'static str {
        match self {
            Currency::Heap => "heap",
            Currency::Fuel => "fuel",
        }
    }
}

/// One value for every resource measurement.
///
/// This type deliberately has no default: every construction site must handle
/// every field.
#[derive(Debug, Clone, Copy)]
pub struct ByCurrency<T> {
    /// The peak-heap column's value.
    pub heap: T,
    /// The fuel column's value.
    pub fuel: T,
}

impl<T> ByCurrency<T> {
    /// Builds a value for every currency from `value`, called once per
    /// currency in display order.
    pub fn from_fn(mut value: impl FnMut(Currency) -> T) -> Self {
        ByCurrency {
            heap: value(Currency::Heap),
            fuel: value(Currency::Fuel),
        }
    }

    /// Every measurement and its value, in display order.
    pub fn each(&self) -> [(Currency, &T); 2] {
        let ByCurrency { heap, fuel } = self;
        [(Currency::Heap, heap), (Currency::Fuel, fuel)]
    }

    /// Return the value for `currency`.
    pub fn get(&self, currency: Currency) -> &T {
        match currency {
            Currency::Heap => &self.heap,
            Currency::Fuel => &self.fuel,
        }
    }

    /// Return the mutable value for `currency`.
    pub fn get_mut(&mut self, currency: Currency) -> &mut T {
        match currency {
            Currency::Heap => &mut self.heap,
            Currency::Fuel => &mut self.fuel,
        }
    }
}

/// The minimum live reading for one cell, or why no nonzero floor exists.
#[derive(Clone, Copy, Debug)]
pub enum Liveness {
    /// The counter must read at least `min`; `why` is the semantic derivation
    /// (or the documented deterministic-liveness rationale).
    Floor {
        /// The least count a functioning meter can read.
        min: u64,
        /// Why every valid implementation must produce at least `min`.
        why: &'static str,
    },
    /// No floor can bind on this cell; the reason renders in the legend.
    NotApplicable {
        /// Why no nonzero floor can be justified.
        reason: &'static str,
    },
}

/// A cell's liveness declarations, one for each measurement whose counter
/// the measured implementation could bypass.
///
/// Heap counts only the allocations that pass through the counting
/// allocator, so each cell argues its own floor. Fuel counts every
/// instruction the guest executes, so no implementation can bypass it, and
/// one floor serves every cell ([`FUEL_FLOOR`]); [`Floors::with_fuel`] adds
/// it.
#[derive(Clone, Copy, Debug)]
pub struct Floors {
    /// The peak-heap column's declaration.
    pub heap: Liveness,
}

/// Fuel's liveness floor, shared by every cell.
///
/// A cell body runs between the two fuel readings, and running anything
/// executes at least one instruction, so a reading of zero means the fuel
/// reader is not counting.
pub const FUEL_FLOOR: Liveness = Liveness::Floor {
    min: 1,
    why: "running a cell body executes at least one instruction",
};

impl Floors {
    /// Returns every currency's declaration: these, and [`FUEL_FLOOR`].
    pub fn with_fuel(self) -> ByCurrency<Liveness> {
        let Floors { heap } = self;
        ByCurrency {
            heap,
            fuel: FUEL_FLOOR,
        }
    }
}
