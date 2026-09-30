//! Envelope types and operation-specific resource pins.

// ─── pinned envelopes ───────────────────────────────────────────────────────

/// One scenario's pins: a peak-heap ceiling and one [`Bound`] per counter
/// column, per the file doc's convention.
#[derive(Clone, Copy)]
pub struct Envelope {
    /// Peak heap delta over the scenario body, in bytes.
    pub peak_heap: usize,
    /// Accumulator digit touches.
    pub touch: Bound,
    /// encoded bits scanned.
    pub scan: Bound,
}

/// One counter column's pin: its ceiling and the floor under it.
#[derive(Clone, Copy)]
pub struct Bound {
    /// The measured reading ×1.25, rounded up; only ever tightened.
    pub ceiling: u64,
    /// The floor under the reading.
    pub floor: Floor,
}

/// A lower bound that detects either counter failure or unexpected cost drift.
#[derive(Clone, Copy)]
pub enum Floor {
    /// The improvement tripwire: the measured reading ×0.75, rounded down
    /// (zero where the reading is zero, under which it asserts nothing). A
    /// trip is a drop of more than 25% from the pinned reading: attribute
    /// it.
    Tripwire(u64),
    /// The liveness floor of a walk that must read every live input bit,
    /// less a stated tail.
    ///
    /// The floor is `8 × input_bytes` minus, per operand stream, at most
    /// one byte of padding and `tail_bits` the walk may leave unread once
    /// its verdict is decided. A trip means the work left the metered
    /// primitives. The scan counter counts the builder's writes and splices
    /// as well as reads, so on a row whose walk also writes (`ID_JOIN`,
    /// `ID_WITHOUT`) the floor attests total bypass, not that the reads
    /// stayed metered; on a read-only row (the validators, the decoders,
    /// `ID_COVERS`, `ID_DISJOINT`) it attests the reads. Only for a row
    /// whose `input_bytes` is the byte length of exactly the streams the
    /// walk reads, and whose walk must read them by contract (a strict
    /// validator or decoder: `tail_bits` 0), by construction (an id walk
    /// whose output depends on every tag: `tail_bits` 0), or to a decision
    /// point (the diverted id pair, decided at the last unary node's tag
    /// pair, leaving exactly each stream's 2-bit terminal below it:
    /// `tail_bits` 2, so an early exit at the decision is an improvement
    /// the floor admits, while a walk that skipped the deciding tag pair
    /// is not).
    LiveBits {
        /// The operand streams `input_bytes` counts.
        streams: u64,
        /// Bits per stream the walk may leave unread.
        tail_bits: u64,
    },
}

/// A column pinned at `ceiling` with its improvement tripwire at `floor`.
pub const fn band(ceiling: u64, floor: u64) -> Bound {
    Bound {
        ceiling,
        floor: Floor::Tripwire(floor),
    }
}

/// A scan column pinned at `ceiling` over the liveness floor of a walk that
/// reads `streams` operand streams whole.
pub const fn whole_input(ceiling: u64, streams: u64) -> Bound {
    Bound {
        ceiling,
        floor: Floor::LiveBits {
            streams,
            tail_bits: 0,
        },
    }
}

/// A scan column pinned at `ceiling` over the liveness floor of a walk that
/// reads `streams` operand streams to a decision point, leaving at most
/// `tail_bits` of each unread.
pub const fn to_decision(ceiling: u64, streams: u64, tail_bits: u64) -> Bound {
    Bound {
        ceiling,
        floor: Floor::LiveBits { streams, tail_bits },
    }
}

/// Build an [`Envelope`] from a row's columns.
pub const fn envelope(peak_heap: usize, touch: Bound, scan: Bound) -> Envelope {
    Envelope {
        peak_heap,
        touch,
        scan,
    }
}

// Pins per the file doc's convention; each row's trailing comment states
// the mechanism that prices it.
#[rustfmt::skip]
pub mod envelope {
    use super::{band, envelope, to_decision, whole_input, Envelope};
    pub const DECODE_DENSE: Envelope                = envelope(120_035,            band(4, 2),      whole_input(468_758, 1)); // wire decode is validate + wrap; the whole-input scan floor proves the validator runs
    pub const CMP_DENSE: Envelope                   = envelope( 30_720, band(156_254, 93_752),       band(468_760, 281_256)); // the iterative sweep over the Bytes-backed form, with touch and scan tripwires
    pub const CMP_DENSE_SELF: Envelope              = envelope( 51_200, band(156_257, 93_753),       band(937_515, 562_509)); // aligned ties in lockstep to full depth: both streams' bits scanned whole
    pub const JOIN_DENSE: Envelope                  = envelope(130_277, band(156_255, 93_753),       band(625_018, 375_010)); // the emit kernel's peak alone; the lhs clone is a refcount bump
    pub const DECODE_BIGROOT: Envelope              = envelope( 60_090,    band(2_348, 1_408),      whole_input(137_512, 1)); // wire decode is validate + wrap; the wide root folds once
    pub const CMP_BIGROOT: Envelope                 = envelope( 39_540,   band(14_849, 8_909),        band(137_514, 82_508)); // the iterative sweep over the Bytes-backed form
    pub const JOIN_BIGROOT: Envelope                = envelope( 85_060,   band(14_850, 8_910),       band(275_028, 165_016)); // the emit kernel's peak alone; both roots are folded
    pub const DECODE_HUGELEAF: Envelope             = envelope(122_504,    band(7_327, 4_395),      whole_input(312_503, 1)); // the validating decode holds one wide running height
    pub const JOIN_HUGELEAF: Envelope               = envelope(185_494,    band(7_329, 4_397),       band(625_010, 375_006)); // the emit kernel holds both payload buffers, and the lhs clone is a refcount bump, so the public join's peak is the emit kernel's alone
    pub const JOIN_ABSORB: Envelope                 = envelope(270_798, band(163_580, 98_148),     band(2_968_773, 750_007)); // the first wide collapse separates topology from payload once; later collapses truncate topology, and the result is interleaved once at the end
    pub const ID_JOIN: Envelope                     = envelope(279_132,            band(0, 0),    whole_input(3_125_023, 2)); // iterative id walks: frame bits on the heap
    pub const ID_COVERS: Envelope                   = envelope(     10,            band(0, 0), to_decision(1_250_005, 2, 2)); // iterative id walks; the diverted pair is decided at the last unary node's tag pair, so the scan floor leaves each stream's terminal unread
    pub const ID_DISJOINT: Envelope                 = envelope(     10,            band(0, 0), to_decision(1_250_005, 2, 2)); // iterative id walks; the diverted pair is decided at the last unary node's tag pair, so the scan floor leaves each stream's terminal unread
    pub const ID_WITHOUT: Envelope                  = envelope(521_110,            band(0, 0),    whole_input(2_500_005, 1)); // iterative complement over the Bytes-backed at-rest form; `input_bytes` counts the subtrahend alone, not the seed's two-bit stream, so the floor errs on the low side; dev builds run no shadow re-parse of the diff emission (the differential suites carry the normal-form check)
    pub const DECODE_CLIFF: Envelope                = envelope(  4_052,    band(4_003, 2_401),       whole_input(17_923, 1)); // wire decode is validate + wrap; the accumulator crosses each cliff cheaply
    pub const CMP_CLIFF: Envelope                   = envelope(  1_330,    band(5_284, 3_170),         band(17_925, 10_755)); // the cliff-free sweep (two accumulators, opened once) over the Bytes-backed at-rest form
    pub const JOIN_CLIFF: Envelope                  = envelope(  5_362,    band(5_289, 3_173),         band(35_848, 21_508)); // the emit kernel's peak alone; each tooth funds its own work
    pub const MEET_CLIFF: Envelope                  = envelope(  4_422,    band(5_289, 3_173),         band(23_055, 13_833)); // the pointwise minimum clamps every tooth to the flat operand's height while every delta still crosses the carry boundary in the accumulator
    pub const DECODE_WIDE_TOOTH: Envelope           = envelope(125_100,   band(14_218, 8_530),    whole_input(1_000_480, 1)); // wire decode is validate + wrap; each wide delta funds its fold
    // CMP_WIDE_TOOTH's deliberately thin heap margin is a change-detector
    // on the backend's and the accumulator's allocation policies: the
    // measurement depends on the big-integer backend's allocation policy at
    // the locked version, so a dependency bump is a deliberate re-measure
    // event, not noise.
    pub const CMP_WIDE_TOOTH: Envelope              = envelope(  1_250,   band(15_499, 9_299),     band(1_000_483, 600_289)); // each wide delta funds its fold; heap stays at the stacks and accumulator
    pub const JOIN_WIDE_TOOTH: Envelope             = envelope(128_312,   band(15_504, 9_302),   band(2_000_963, 1_200_577)); // each wide delta re-coded into the output, paid by its own zigzag code
    pub const MEET_WIDE_TOOTH: Envelope             = envelope(127_087,   band(15_504, 9_302),     band(1_005_613, 603_367)); // wide deltas folded but never re-emitted: the collapse discipline at spilled operand widths
    pub const DECODE_ALT_SPINE: Envelope            = envelope(120_035,            band(4, 2),      whole_input(468_758, 1)); // wire decode is validate + wrap; per-level state stays two bits however the descent direction flips
    // Version validator rows: the validator's transient is the
    // open-ancestor bit stack plus reallocation growth, bits per level,
    // not frames. Its work is cursor reads end to end (it allocates
    // near-nothing and, off the wide families, does little arithmetic),
    // so scan is the column that sees a re-read the others cannot, and
    // the whole-input floor under it is what a validator that stops
    // reading fails. Decode is validate plus the wrap, so each shape's
    // scan reading equals its validate row's.
    pub const SKYLINE_VALIDATE_DENSE: Envelope      = envelope( 61_440,            band(4, 2),      whole_input(468_758, 1)); // the open-ancestor bit stack
    pub const SKYLINE_VALIDATE_CLIFF: Envelope      = envelope(  1_770,    band(4_003, 2_401),       whole_input(17_923, 1)); // the cliff-free accumulator: amortized O(1) per delta
    pub const SKYLINE_VALIDATE_WIDE_TOOTH: Envelope = envelope(  1_520,   band(14_218, 8_530),    whole_input(1_000_480, 1)); // each wide delta funds its fold; heap stays at the bit stack
    pub const SKYLINE_VALIDATE_HUGELEAF: Envelope   = envelope( 80_980,    band(7_327, 4_395),      whole_input(312_503, 1)); // one wide decode and one wide accumulator load, both linear in the code's width
    pub const SKYLINE_VALIDATE_ALT_SPINE: Envelope  = envelope( 61_440,            band(4, 2),      whole_input(468_758, 1)); // per-level state stays two bits however the descent direction flips
}
