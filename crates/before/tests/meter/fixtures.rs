//! Shared adversarial-family sizes and constructors.

use super::*;

/// Build a uniform version through the public clock operations.
pub fn uniform_version(ticks: impl Into<Ticks>) -> Version {
    let mut version = Version::new();
    Party::seed().ticks(&mut version, ticks);
    version
}

/// Convert a test oracle's integer without routing through decimal text.
pub fn ticks_from_big(value: &BigUint) -> Ticks {
    value
        .iter_u64_digits()
        .rev()
        .fold(Ticks::ZERO, |mut ticks, word| {
            for _ in 0..u64::BITS {
                ticks += ticks.clone();
            }
            ticks += Ticks::from(word);
            ticks
        })
}

/// Construct `2^exponent` as an unbounded tick count.
pub fn power_of_two(exponent: usize) -> Ticks {
    let mut ticks = Ticks::from(1u8);
    for _ in 0..exponent {
        ticks += ticks.clone();
    }
    ticks
}

// ─── scenario sizes ─────────────────────────────────────────────────────────

/// Depth of the dense event spine `S(d)` scenarios.
pub const DENSE_DEPTH: usize = 125_000;

/// Event count used by the ordinary multi-tick scenarios.
pub const TICKS_POINT_LO: u64 = 512;

/// Root magnitude (bits) of the bigroot scenarios.
pub const BIGROOT_MAGNITUDE_BITS: usize = 40_000;

/// Spine depth of the bigroot scenarios.
pub const BIGROOT_DEPTH: usize = 10_000;

/// Leaf magnitude (bits) of the hugeleaf scenarios: one gamma code as wide
/// as the whole input, the shape where any cost superlinear in a single
/// code's width shows up undiluted.
pub const HUGELEAF_MAGNITUDE_BITS: usize = 125_000;

/// Depth of the id spine `I(d, divert)` pair scenarios.
pub const ID_DEPTH: usize = 250_000;

/// Tooth magnitude (bits) of the boundary comb scenarios; also its tooth
/// count, so crossing work `n·k` grows quadratically while each crossing
/// stays paid for by a `2k + 1`-bit stored code.
pub const CLIFF_SCALE: usize = 1_024;

/// The wide × deep tick crosses' shared scale: magnitude bits and
/// shortcut depth together, deep enough that a per-level re-touch of
/// the wide content would overshoot the envelopes by orders of
/// magnitude.
pub const TICK_CROSS_SCALE: usize = 4_000;

/// Staircase depth of the ownership-hole tick scenario: enough distinct
/// plateaus that the unowned regions' per-leaf work would dominate
/// the envelope if the block scan failed to engage.
pub const HOLE_STAIR_DEPTH: usize = 2_000;

/// Id depth of the ownership-hole tick scenario: a party owning one
/// diverted fragment `2^-8` of the space, leaving the staircase's runs
/// unowned.
pub const HOLE_ID_DEPTH: usize = 8;

/// Site count of the sub-scan hole pairs: enough deep-region crossings
/// that the sub-scans' per-leaf/block routing dominates the envelope,
/// split evenly between the two lead depths (the routing boundary's two
/// sides).
pub const SCAN_HOLE_UNITS: usize = 16;

/// Descending steps per sub-scan hole region: deep enough that rerouting
/// one lead's regions alone from block summaries to per-leaf work
/// moves the pinned columns past their ceilings.
pub const SCAN_HOLE_STEPS: usize = 128;

/// Spine-depth pair of the masked-hole scenarios: the depth band holds
/// the fused comparison's accumulator readings flat across this doubling.
pub const MASK_HOLE_DEPTH_LO: usize = 1_000;

/// The masked-hole depth pair's larger point (the envelope row's scale).
pub const MASK_HOLE_DEPTH_HI: usize = 2_000;

/// Mask depth of the masked-hole triple: the one knob the fused
/// comparison's accumulator readings may be a function of.
pub const MASK_HOLE_MASK_DEPTH: usize = 8;

/// Spine depth of the min_ticks ascending-cliff scenario: enough
/// simultaneously stacked nonzero boundary differences that per-boundary
/// transient memory dominates the envelope.
pub const ASCEND_STACK_DEPTH: usize = 2_000;

/// Leaf magnitude (bits) of the min_ticks ascending-cliff scenario: wide
/// enough that the plateau rides the frozen component, word-scale enough
/// that every stacked boundary difference is a compaction candidate.
pub const ASCEND_STACK_MAGNITUDE_BITS: usize = 64;

/// Owned-fragment count of the alternating-ownership comb scenario,
/// interleaving one owned leaf and one absent gap per level down the
/// alternating spine.
pub const COMB_FRAGMENTS: usize = 2_000;

/// Tooth width (bits) of the wide-tooth comb scenarios: wider than any
/// machine word, so every Version delta is a genuinely wide operand while
/// still oscillating across the `2^k` cliff.
pub const WIDE_TOOTH_WIDTH_BITS: usize = 192;

/// Tooth magnitude (bits) of the two-operand jump-comb scenarios.
///
/// Comfortably over the rank freeze allowance's 256-bit digit bound, so
/// every cheap fold arriving behind a wide switch jump in the meet
/// stream fires the eviction.
pub const JUMP_PAIR_MAGNITUDE_BITS: usize = 512;

/// Comb levels of the two-operand jump-comb query scenario (the
/// superlinearity band's small run; the large run doubles it).
pub const JUMP_PAIR_TEETH: usize = 512;

/// Freeze-position digits of the two-operand jump-comb query scenario
/// (an eighth of the teeth, the board family's proportion; the large
/// run doubles it).
pub const JUMP_PAIR_DIGITS: usize = 64;

/// Forked-party count of the concurrent-pair query scenarios: every one
/// of the `n − 1` overlay boundaries is an emit side switch, in the join
/// and the meet alike.
pub const CONCURRENT_PAIR_LEAVES: usize = 4_096;

/// Tooth magnitude (bits) of the mask-drift families: wide enough that a
/// per-boundary materialization of the walk's integrator reads would be
/// unmistakably superlinear, word-scale enough to keep the scenario in
/// seconds.
pub const MASK_DRIFT_MAGNITUDE_BITS: usize = 512;

/// Tooth count of the mask-drift families' envelope scenarios.
pub const MASK_DRIFT_TEETH: usize = 1_024;

/// Depth of the harmonic spine `H(d)` rank scenario: deep enough that the
/// fold's per-level numerator re-shifts dominate every constant.
pub const RANK_HARMONIC_DEPTH: usize = 65_536;

/// Spine depth behind the max-exponent rank of the pair-mismatch scenario.
pub const RANK_PAIR_DEPTH: usize = 500_000;

/// Consecutive fractional exponents in the mixed-sum scenario.
pub const RANK_SUM_FRACTIONS: usize = 128;

/// Numerator width of the mixed-sum scenario's integer rank.
pub const RANK_SUM_WIDE_BITS: usize = 250_000;
