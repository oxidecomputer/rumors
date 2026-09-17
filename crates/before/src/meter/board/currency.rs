//! Resource quantities judged by the amplification board.
//!
//! [`ByCurrency`] has one required field for each quantity. Adding a quantity
//! therefore makes every reading, ceiling, floor, and rendering site fail to
//! compile until it handles the new measurement.

/// One deterministic resource measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    /// Peak transient heap bytes (the caller-installed counting allocator).
    Heap,
    /// Grown stacker segments (recursion-driven stack cost).
    Segments,
    /// Encoded scan bits (traversal cost; `scan-meter`).
    Scan,
    /// Accumulator digit touches (digit-state cost; `touch-meter`).
    Touch,
}

impl Currency {
    /// The currency's rendered column name.
    pub fn label(self) -> &'static str {
        match self {
            Currency::Heap => "heap",
            Currency::Segments => "segments",
            Currency::Scan => "scan",
            Currency::Touch => "touch",
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
    /// The grown-segments column's value.
    pub segments: T,
    /// The scan column's value.
    pub scan: T,
    /// The touch column's value.
    pub touch: T,
}

impl<T> ByCurrency<T> {
    /// Every measurement and its value, in display order.
    pub fn each(&self) -> [(Currency, &T); 4] {
        let ByCurrency {
            heap,
            segments,
            scan,
            touch,
        } = self;
        [
            (Currency::Heap, heap),
            (Currency::Segments, segments),
            (Currency::Scan, scan),
            (Currency::Touch, touch),
        ]
    }

    /// Return the value for `currency`.
    pub fn get(&self, currency: Currency) -> &T {
        match currency {
            Currency::Heap => &self.heap,
            Currency::Segments => &self.segments,
            Currency::Scan => &self.scan,
            Currency::Touch => &self.touch,
        }
    }
}

/// The minimum live reading for one cell, or why no nonzero floor exists.
#[derive(Clone, Copy, Debug)]
pub enum Liveness {
    /// The counter must read at least `min`; `why` is the semantic derivation
    /// (or the documented deterministic-liveness rationale).
    Floor {
        /// The least count a watching meter can honestly read.
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

/// A cell's liveness declaration for every measurement.
pub type Floors = ByCurrency<Liveness>;
