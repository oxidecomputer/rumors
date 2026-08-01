//! The currency axis: the metering currencies the board judges, and the
//! per-currency container every judged quantity lives in.
//!
//! The board is a product over three declarative axes — shapes ×
//! operations × currencies — and this module is the third axis's
//! definition. A *currency* is one deterministic meter the criterion
//! judges (peak heap bytes, grown stack segments, big-integer limb
//! operations, packed-stream scan bits, accumulator digit touches);
//! [`ByCurrency`] is the container
//! with **one field per currency**, and every per-currency quantity on the
//! board — a cell's liveness declarations, a sample's counter readings, a
//! result's scores — is a `ByCurrency<T>`.
//!
//! # Totality by construction
//!
//! The field-per-currency shape is the axis's totality mechanism: adding a
//! currency means adding a field here, and every construction site in the
//! operation table then fails to compile until it declares, for the new
//! currency, either a derived floor or an explicit not-applicable with its
//! rationale ([`Liveness`]). There is no default, no `..` construction,
//! and no wildcard to hide behind — a currency cannot be half-wired, which
//! is the genre of gap (work migrating into an unjudged currency on rows
//! that never carried it) this axis exists to make inexpressible.
//! [`ByCurrency::each`] destructures exhaustively for the same reason: the
//! judgment and render loops iterate the axis itself, so a new currency is
//! judged and rendered everywhere or the destructuring fails to compile.

/// One metering currency: a deterministic counter column of the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    /// Peak transient heap bytes (the caller-installed counting allocator).
    Heap,
    /// Grown stacker segments (recursion-driven stack cost).
    Segments,
    /// Big-integer limb operations (arithmetic-width cost; `limb-meter`).
    Limb,
    /// Packed-stream scan bits (traversal cost; `scan-meter`).
    Scan,
    /// Accumulator digit touches (digit-state cost; `limb-meter`).
    Touch,
}

impl Currency {
    /// The currency's rendered column name.
    pub fn label(self) -> &'static str {
        match self {
            Currency::Heap => "heap",
            Currency::Segments => "segments",
            Currency::Limb => "limb",
            Currency::Scan => "scan",
            Currency::Touch => "touch",
        }
    }
}

/// One value per metering currency: the container the currency axis
/// distributes over the board's cells.
///
/// Constructed only by exhaustive struct literal (no `Default`, no `..`):
/// see the module doc's totality argument.
#[derive(Debug, Clone, Copy)]
pub struct ByCurrency<T> {
    /// The peak-heap column's value.
    pub heap: T,
    /// The grown-segments column's value.
    pub segments: T,
    /// The limb column's value.
    pub limb: T,
    /// The scan column's value.
    pub scan: T,
    /// The touch column's value.
    pub touch: T,
}

impl<T> ByCurrency<T> {
    /// Every currency's value, in the axis's fixed judgment and render
    /// order.
    ///
    /// The exhaustive destructure is deliberate: a currency added to the
    /// axis fails to compile here until it joins the iteration, so no
    /// judgment or render loop can silently skip it.
    pub fn each(&self) -> [(Currency, &T); 5] {
        let ByCurrency {
            heap,
            segments,
            limb,
            scan,
            touch,
        } = self;
        [
            (Currency::Heap, heap),
            (Currency::Segments, segments),
            (Currency::Limb, limb),
            (Currency::Scan, scan),
            (Currency::Touch, touch),
        ]
    }

    /// The one field the given currency names.
    pub fn get(&self, currency: Currency) -> &T {
        match currency {
            Currency::Heap => &self.heap,
            Currency::Segments => &self.segments,
            Currency::Limb => &self.limb,
            Currency::Scan => &self.scan,
            Currency::Touch => &self.touch,
        }
    }
}

/// One judged column's liveness declaration for one cell: the least the
/// counter must read if the meter is watching the work, or the reason no
/// floor can bind.
///
/// Every cell carries one per currency (see [`Floors`]); the derivation
/// conventions live with the floor constructors, in the board's `floors`
/// module.
#[derive(Clone, Copy, Debug)]
pub enum Liveness {
    /// The counter must read at least `min`; `why` is the semantic
    /// derivation (or the documented deterministic-liveness rationale).
    Floor {
        /// The least count a watching meter can honestly read.
        min: u64,
        /// The derivation, rendered in the board's legend.
        why: &'static str,
    },
    /// No floor can bind on this cell; the reason renders in the legend.
    NotApplicable {
        /// Why the column cannot be floored here.
        reason: &'static str,
    },
}

/// A cell's floor-or-NA declarations, one per currency.
///
/// Constructing a board cell requires answering the floor question for
/// every currency on the axis — a cell cannot enter the board without the
/// answers, and a currency cannot enter the axis without every cell's
/// answer (the module doc's totality argument). Segments' declaration is
/// the ceiling-only policy NA on every cell today: the target is walks
/// that never grow the stack, so its honest floor is zero, and a zero
/// floor asserts nothing.
pub type Floors = ByCurrency<Liveness>;
