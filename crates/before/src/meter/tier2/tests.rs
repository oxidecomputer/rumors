//! Hand-computed pins for the Tier 2 size function on small trees, the
//! join/meet 1-Lipschitz coding pin the board's input denomination rests on,
//! and the join/meet subadditivity pins.
//!
//! Each size test states the tree, the hand-derived Tier 2 bit count, and the
//! hand-derived current bit count, so a regression in the walk (path sums,
//! zigzag map, gamma lengths, topology accounting) fails against arithmetic a
//! reader can re-derive in the margin. Gamma lengths used below:
//! `gamma(0) = 1`, `gamma(1) = 3`, `gamma(2) = 3`, `gamma(4) = 5`,
//! `gamma(2^b - 1) = 2b + 1`.

use proptest::prelude::*;

use crate::codec::Base;
use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, hugeleaf, wide_tooth_comb,
};
use crate::testing::bridge::from_oracle_version;
use crate::testing::bridge::{packed_bits_of, to_oracle_version};
use crate::testing::compactness::{arb_comb_params, comb};
use crate::testing::{generators, optrace};
use crate::version::skyline;
use crate::{oracle, Clock, Party, Version};

use super::{tier2_size, Tier2Size};

/// Per-input-leaf slack for the join/meet 1-Lipschitz coding pin \[derived\].
///
/// At each overlay boundary the output's jump is at most the largest input
/// jump there (`max`/`min` are 1-Lipschitz in each argument), and the
/// output's boundaries are a subset of the union of the inputs'. A coded
/// output delta may telescope several union jumps when equal-valued spans
/// collapse, so per input leaf the coding spends at most: the input's own
/// delta code, +1 bit for the zigzag sign convention, +1 bit of gamma
/// subadditivity per merged jump, and ≤ 2 bits of code-length rounding per
/// coded delta — 4 bits covers every term.
const JOIN_MEET_BOUNDARY_SLACK_BITS: u64 = 4;

/// Assert the join/meet 1-Lipschitz coding pin on one operand pair,
/// under every emitter of record.
///
/// The statement the board's input denomination of the packed-output
/// mutators rests on: the output's coded size is at most the inputs' plus
/// O(1) bits per boundary — boundaries (leaves) contained in the union of
/// the inputs', and total Tier 2 bits within
/// [`JOIN_MEET_BOUNDARY_SLACK_BITS`] per input leaf of the inputs' sum.
fn check_join_meet_lipschitz(a: &Version, b: &Version) {
    let sa = tier2_size(&packed_bits_of(&to_oracle_version(a)));
    let sb = tier2_size(&packed_bits_of(&to_oracle_version(b)));
    for (name, emit) in EMITTERS {
        let out = emit(a, b);
        let so = tier2_size(&packed_bits_of(&to_oracle_version(&out)));
        assert!(
            so.leaves < sa.leaves + sb.leaves,
            "{name}: {} output leaves reach the input leaf total {} + {}: \
             an output boundary appeared outside the inputs' boundaries",
            so.leaves,
            sa.leaves,
            sb.leaves,
        );
        let ceiling =
            sa.total_bits + sb.total_bits + JOIN_MEET_BOUNDARY_SLACK_BITS * (sa.leaves + sb.leaves);
        assert!(
            so.total_bits <= ceiling,
            "{name}: 1-Lipschitz coding pin violated: {} output bits > {} + {} inputs + {} \
             slack per input leaf over {} leaves",
            so.total_bits,
            sa.total_bits,
            sb.total_bits,
            JOIN_MEET_BOUNDARY_SLACK_BITS,
            sa.leaves + sb.leaves,
        );
    }
}

/// The empty version is the single leaf 0: one topology bit plus `gamma(0)`,
/// 2 bits in both encodings.
#[test]
fn empty_version_is_two_bits() {
    let v = Version::new();
    let size = tier2_size(&packed_bits_of(&to_oracle_version(&v)));
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 2,
            nodes: 1,
            leaves: 1,
            first_leaf_bits: 1,
            delta_bits: 0,
        }
    );
    assert_eq!(size.total_bits, v.encoded_bits() as u64);
}

/// A single ticked leaf (value 1) is one topology bit plus `gamma(1) = 3`,
/// 4 bits in both codings (stored Tier 2 and the packed spelling alike).
#[test]
fn single_small_leaf_matches_current_size() {
    let mut v = Version::new();
    v.tick(&Party::seed());
    let size = tier2_size(&packed_bits_of(&to_oracle_version(&v)));
    assert_eq!(size.total_bits, 4);
    assert_eq!((size.nodes, size.leaves), (1, 1));
    assert_eq!(size.total_bits, v.encoded_bits() as u64);
}

/// A single huge leaf `2^b - 1` is one topology bit plus `gamma(2^b - 1) =
/// 2b + 1`: Tier 2 equals the min-lifted packed spelling's `2b + 2` bits exactly, at a magnitude
/// wide enough to spill machine-word arithmetic.
#[test]
fn single_big_leaf_matches_current_size() {
    for b in [7, 200] {
        let packed = hugeleaf(b);
        let size = tier2_size(packed.as_bits());
        assert_eq!(size.total_bits as usize, 2 * b + 2);
        assert_eq!((size.nodes, size.leaves), (1, 1));
        assert_eq!(size.first_leaf_bits as usize, 2 * b + 1);
        assert_eq!(size.total_bits, packed.version().encoded_bits() as u64);
    }
}

/// One fork `(1, 0, 2)` sizes to 11 Tier 2 bits, hand-derived.
///
/// Leaves are 1 and 3 absolute: 3 topology bits + `gamma(1) = 3` +
/// `zigzag(+2) = 4 -> gamma(4) = 5`, against the min-lifted packed
/// spelling's 10 (`3 + gamma(1) + gamma(0) + gamma(2) = 3 + 3 + 1 + 3`).
#[test]
fn one_fork_matches_hand_computation() {
    let v = from_oracle_version(&oracle::Version::node(
        1u64,
        oracle::Version::leaf(0u64),
        oracle::Version::leaf(2u64),
    ));
    assert_eq!(packed_bits_of(&to_oracle_version(&v)).len(), 10);
    assert_eq!(v.encoded_bits(), 11, "the stored coding is Tier 2 itself");
    let size = tier2_size(&packed_bits_of(&to_oracle_version(&v)));
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 11,
            nodes: 3,
            leaves: 2,
            first_leaf_bits: 3,
            delta_bits: 5,
        }
    );
}

/// The dense spine `S(2)` has preorder leaves 0, 1, 0: Tier 2 is 5 topology
/// bits + `gamma(0) = 1` + `zigzag(+1) = 2 -> 3` + `zigzag(-1) = 1 -> 3`,
/// 12 bits, exactly the min-lifted packed spelling’s `4d + 4 = 12`.
#[test]
fn dense_spine_matches_hand_computation() {
    let packed = crate::meter::dense(2);
    assert_eq!(packed.bits, 12);
    let size = tier2_size(packed.as_bits());
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 12,
            nodes: 5,
            leaves: 3,
            first_leaf_bits: 1,
            delta_bits: 6,
        }
    );
    assert_eq!(size.total_bits, packed.version().encoded_bits() as u64);
}

/// The boundary comb's Tier 2 size is exactly `10n + 4k + 2` bits against
/// the min-lifted packed spelling's `n(2k + 10) + 2`: Tier 2 wire bits do not bound value content.
///
/// `cliff_comb(k, n)` codes each `±1` leaf delta in 3 bits where the packed spelling's
/// form stores a fresh `gamma(2^k − 1)` per tooth, so the current/Tier 2
/// size ratio grows without bound in `k` — the `≤ 2×` compactness envelope
/// holds in the useless direction while the comb's `2n + 1` leaves carry
/// `Θ(nk)` bits of absolute value content behind `Θ(n + k)` Tier 2 wire
/// bits. The per-part pin: `4n + 1` topology bits, `gamma(2^k − 1) =
/// 2k + 1` first-leaf bits, `3(2n − 1)` oscillation deltas plus the
/// `2k + 3`-bit closing delta to the terminal leaf 0. The exact ratios at
/// `n = k`: 9.837× (n = 64), 146.980× (n = 1024), 585.837× (n = 4096);
/// the floors below sit just under them.
#[test]
fn cliff_comb_tier2_size_is_linear_while_current_is_quadratic() {
    for (k, n) in [(3, 2), (64, 64), (200, 50), (1024, 1024), (4096, 4096)] {
        let packed = cliff_comb(k, n);
        let size = tier2_size(packed.as_bits());
        assert_eq!(
            size,
            Tier2Size {
                total_bits: (10 * n + 4 * k + 2) as u64,
                nodes: (4 * n + 1) as u64,
                leaves: (2 * n + 1) as u64,
                first_leaf_bits: (2 * k + 1) as u64,
                delta_bits: (6 * n + 2 * k) as u64,
            }
        );
        assert_eq!(packed.bits, n * (2 * k + 10) + 2);
    }
    for (n, ratio_floor) in [(64, 9.83), (1024, 146.97), (4096, 585.83)] {
        let current = (n * (2 * n + 10) + 2) as f64;
        let tier2 = (14 * n + 2) as f64;
        assert!(
            current / tier2 >= ratio_floor,
            "current/tier2 ratio at n = k = {n} fell below its pinned floor {ratio_floor}"
        );
    }
}

/// A plain running-value accumulator over the comb's Tier 2 delta stream
/// costs limb work quadratic in the wire bits: per-wire-bit cost roughly
/// doubles when the size doubles.
///
/// This is the executable witness that carry-run amortization does not
/// transfer to the delta coding: each 3-bit `±1` delta lands exactly on
/// the `2^k` carry boundary, so applying it to a plain big-integer
/// accumulator propagates a full `k`-bit carry or borrow — `Θ(k)` limb work
/// bought by `O(1)` wire bits, `Θ(W²)` total in wire bits `W`. Under
/// today's coding the same tree pays `2k + 1` stored bits per crossing
/// (the envelope suite pins those operations linear). Any Tier 2 sweep
/// that must materialize running leaf values — strict decode's
/// nonnegativity validation included, since values are naturals and a
/// plain 2-bit/level topology check cannot see a delta drive one negative
/// — inherits this cost unless it uses a carry-immune accumulator design.
#[cfg(feature = "limb-meter")]
#[test]
fn cliff_comb_plain_delta_sweep_is_quadratic_in_tier2_wire_bits() {
    // Apply the comb's delta stream to a plain accumulator: v1 = 2^k − 1,
    // then the 2n − 1 oscillation deltas (+1, −1, …) and the closing −2^k,
    // exactly the values a Tier 2 leaf sweep must materialize in order.
    let limb_ops_per_wire_bit = |scale: usize| {
        let (k, n) = (scale, scale);
        let one = Base::from(1u8);
        let mut v = (Base::from(1u8) << k as u32) - &one;
        crate::meter::reset_limb_ops();
        for i in 1..(2 * n) {
            v = if i % 2 == 1 { &v + &one } else { v - &one };
        }
        let closing = Base::from(1u8) << k as u32;
        v -= &closing;
        let ops = crate::meter::limb_ops();
        assert_eq!(
            v,
            Base::ZERO,
            "the delta stream telescopes back to the terminal leaf 0"
        );
        ops as f64 / (14 * n + 2) as f64
    };
    let small = limb_ops_per_wire_bit(512);
    let large = limb_ops_per_wire_bit(1024);
    assert!(
        large / small >= 1.8,
        "per-wire-bit limb cost must roughly double per size doubling \
         (measured {small:.2} then {large:.2} limb ops per wire bit): \
         a plain accumulator over the comb's delta stream is quadratic"
    );
}

proptest! {
    /// Join and meet of arbitrary normal-form event trees hold the
    /// 1-Lipschitz coding pin: output boundaries within the union of the
    /// inputs', output coded size within the inputs' plus the per-leaf
    /// slack.
    #[test]
    fn arbitrary_pairs_hold_the_lipschitz_pin(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        check_join_meet_lipschitz(&from_oracle_version(&a), &from_oracle_version(&b));
    }

    /// Join and meet over every version pair produced by an organic
    /// fork/tick/send/sync/join history hold the 1-Lipschitz coding pin.
    #[test]
    fn organic_pairs_hold_the_lipschitz_pin(ops in optrace::world_strategy_up_to(120)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for pair in clocks.windows(2) {
            check_join_meet_lipschitz(pair[0].version(), pair[1].version());
        }
    }

    /// Join and meet of alternating combs — the ratio meter's
    /// tightness family, whose every consecutive-leaf delta is a full
    /// magnitude swing — hold the 1-Lipschitz coding pin.
    #[test]
    fn comb_pairs_hold_the_lipschitz_pin(
        (m1, p1) in arb_comb_params(),
        (m2, p2) in arb_comb_params(),
    ) {
        check_join_meet_lipschitz(&comb(m1, p1), &comb(m2, p2));
    }
}

/// Join and meet across the adversarial event shapes of record (dense
/// spine, bigroot, hugeleaf, boundary comb) hold the 1-Lipschitz coding pin
/// on every cross of the family grid.
#[test]
fn adversarial_crosses_hold_the_lipschitz_pin() {
    let shapes = [
        dense(512).version(),
        bigroot(200, 100).version(),
        hugeleaf(500).version(),
        cliff_comb(64, 64).version(),
    ];
    for a in &shapes {
        for b in &shapes {
            check_join_meet_lipschitz(a, b);
        }
    }
}

// ───────────────────── join/meet subadditivity pins ─────────────────────

/// Guaranteed coding savings of the join/meet subadditivity lemma, in bits
/// \[derived\].
///
/// For canonical `a`, `b` and `c` either their join (pointwise max) or meet
/// (pointwise min), the Tier 2 sizes satisfy
/// `size(c) <= size(a) + size(b) - 2`, term by term: the canonical output
/// topology embeds in the union of the input topologies, which share at
/// least the root (1 topology bit saved); the output's first leaf value is
/// one of the inputs' first leaf values, so the other input's first-leaf
/// code (>= 1 bit) goes unmatched; and every output boundary charges a
/// distinct input boundary at the same point whose delta code covers the
/// output's — output boundaries are contained in the union of the inputs',
/// pointwise max/min is 1-Lipschitz in each argument, and the zigzag-gamma
/// code length depends only on the delta's magnitude (a sign flip is free:
/// `gamma(2m)` and `gamma(2m - 1)` have equal length because `2m + 1` is
/// never a power of two). Equality holds at `a = b = Version::new()`, so
/// this margin is the strongest constant the lemma admits and must never
/// loosen.
const JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS: u64 = 2;

/// The emitters of record, named: join and meet, packed-form and
/// skyline emission kernel.
///
/// The pins are statements about an emitter's actual output, so every
/// check takes the emitter as a parameter and the suites below iterate
/// this table — the skyline kernel re-instantiates each pin the
/// packed-form operators established.
#[allow(clippy::type_complexity)]
const EMITTERS: [(&str, fn(&Version, &Version) -> Version); 4] = [
    ("operator join", operator_join),
    ("operator meet", operator_meet),
    ("kernel join", skyline_join),
    ("kernel meet", skyline_meet),
];

/// The public `|` operator: the subadditivity pins' first emitter of record
/// (the kernel plus its short-circuits).
fn operator_join(a: &Version, b: &Version) -> Version {
    a | b
}

/// The public `&` operator: the subadditivity pins' first emitter of record
/// (the kernel plus its short-circuits).
fn operator_meet(a: &Version, b: &Version) -> Version {
    a & b
}

/// The emission kernel's join, called directly (no short-circuits).
///
/// Also asserts the emitted stream's exact length agreement with
/// [`tier2_size`] on the result: the pins then price the kernel's own
/// output stream, not merely the value it denotes.
fn skyline_join(a: &Version, b: &Version) -> Version {
    let out = skyline::emit::join(&skyline::encode(a), &skyline::encode(b));
    let decoded = skyline::decode(&out).expect("an emitted join is canonical");
    assert_eq!(
        out.len() as u64,
        tier2_size(&packed_bits_of(&to_oracle_version(&decoded))).total_bits,
        "the emitted join stream must be exactly the canonical coded size"
    );
    decoded
}

/// The emission kernel's meet, called directly (no short-circuits).
fn skyline_meet(a: &Version, b: &Version) -> Version {
    let out = skyline::emit::meet(&skyline::encode(a), &skyline::encode(b));
    let decoded = skyline::decode(&out).expect("an emitted meet is canonical");
    assert_eq!(
        out.len() as u64,
        tier2_size(&packed_bits_of(&to_oracle_version(&decoded))).total_bits,
        "the emitted meet stream must be exactly the canonical coded size"
    );
    decoded
}

/// Assert the subadditivity lemma on one operand pair under one emitter.
///
/// The invariant: the emitted output's Tier 2 size stays at least
/// [`JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS`] below the sum of the inputs'
/// Tier 2 sizes.
fn check_subadditive(
    name: &str,
    emit: fn(&Version, &Version) -> Version,
    a: &Version,
    b: &Version,
) {
    let sa = tier2_size(&packed_bits_of(&to_oracle_version(a)));
    let sb = tier2_size(&packed_bits_of(&to_oracle_version(b)));
    let so = tier2_size(&packed_bits_of(&to_oracle_version(&emit(a, b))));
    assert!(
        so.total_bits + JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS <= sa.total_bits + sb.total_bits,
        "{name}: subadditivity violated: {} output bits > {} + {} input bits - {} pinned savings",
        so.total_bits,
        sa.total_bits,
        sb.total_bits,
        JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS,
    );
}

/// Assert the subadditivity lemma for both join and meet on one operand
/// pair, under every emitter of record.
fn check_join_meet_subadditive(a: &Version, b: &Version) {
    for (name, emit) in EMITTERS {
        check_subadditive(name, emit, a, b);
    }
}

/// `2^bits - 1` as a [`Base`]: the all-ones magnitude of a given bit width.
fn all_ones(bits: usize) -> Base {
    (Base::from(1u8) << u32::try_from(bits).expect("magnitude bit count fits u32"))
        - &Base::from(1u8)
}

/// Build the version whose skyline takes `values[i]` on the `i`th cell of a
/// uniform dyadic grid.
///
/// `values.len()` must be a power of two. The oracle's normalizing
/// constructors collapse equal-valued uniform runs, so the result is
/// canonical whatever the values. Recursive over the grid's `O(log)` depth
/// (test-only; the measured paths are iterative).
fn grid_version(values: &[Base]) -> Version {
    fn build(values: &[Base]) -> oracle::Version {
        match values {
            [v] => oracle::Version::leaf(v.clone()),
            _ => {
                let (l, r) = values.split_at(values.len() / 2);
                oracle::Version::node(0u64, build(l), build(r))
            }
        }
    }
    assert!(
        values.len().is_power_of_two(),
        "uniform grid needs a power-of-two cell count: got {}",
        values.len()
    );
    from_oracle_version(&build(values))
}

/// Plateau magnitude bit widths spanning small values, the machine-word
/// boundary, and magnitudes far past one word.
fn magnitude_bits() -> impl Strategy<Value = usize> {
    prop_oneof![1usize..=8, 60usize..=68, 190usize..=200]
}

/// Join and meet of two empty versions sit exactly on the lemma's equality
/// case under every emitter of record: 2 output bits against 2 + 2 input
/// bits, so the pinned savings margin is the strongest constant the lemma
/// admits.
#[test]
fn empty_pair_is_the_subadditivity_equality_case() {
    let (a, b) = (Version::new(), Version::new());
    for (name, emit) in EMITTERS {
        let so = tier2_size(&packed_bits_of(&to_oracle_version(&emit(&a, &b))));
        assert_eq!(
            so.total_bits + JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS,
            tier2_size(&packed_bits_of(&to_oracle_version(&a))).total_bits
                + tier2_size(&packed_bits_of(&to_oracle_version(&b))).total_bits,
            "{name} of two empty versions must realize the savings margin exactly",
        );
    }
}

/// Join and meet across the full adversarial event-shape grid hold the
/// subadditivity lemma on every cross of the family grid.
///
/// The grid: dense spine, bigroot, hugeleaf, boundary comb, wide-tooth
/// comb, cliff fan, cancelling chain, alternating spine.
#[test]
fn adversarial_crosses_hold_subadditivity() {
    let shapes = [
        dense(256).version(),
        bigroot(128, 64).version(),
        hugeleaf(300).version(),
        cliff_comb(48, 48).version(),
        wide_tooth_comb(96, 32, 32).version(),
        cliff_fan(48, 32).version(),
        cancelling_chain(48, 32).version(),
        alt_spine(128).version(),
    ];
    for a in &shapes {
        for b in &shapes {
            check_join_meet_subadditive(a, b);
        }
    }
}

/// A single huge step against a flat half-height plateau holds the
/// subadditivity lemma.
///
/// This is the shape where a crossing switch emits a boundary delta
/// neither input's *local* codes appear to pay for; the step's own
/// full-height delta code at the same boundary covers the output's
/// smaller jump there.
#[test]
fn hugeleaf_vs_step_holds_subadditivity() {
    for b in [8usize, 64, 200, 1000] {
        let step = from_oracle_version(&oracle::Version::node(
            0u64,
            oracle::Version::leaf(all_ones(b)),
            oracle::Version::leaf(0u64),
        ));
        let flat = hugeleaf(b - 1).version();
        check_join_meet_subadditive(&step, &flat);
        check_join_meet_subadditive(&flat, &step);
    }
}

/// The boundary comb against flat plateaus at its valleys' height, its
/// cliff, above all teeth, and at zero holds the subadditivity lemma.
///
/// Clipping teeth or erasing valleys only removes boundaries, and every
/// surviving boundary keeps a covering input code.
#[test]
fn comb_vs_flat_holds_subadditivity() {
    for (k, n) in [(3usize, 2usize), (48, 48)] {
        let comb_version = cliff_comb(k, n).version();
        let cliff = Base::from(1u8) << u32::try_from(k).expect("cliff bit count fits u32");
        let heights = [
            all_ones(k),     // the valleys' height: join is the comb, meet is flat
            cliff.clone(),   // the teeth's height: meet is the comb, join is flat
            &cliff + &cliff, // above every tooth: join is flat, meet is the comb
            Base::ZERO,      // below everything: the empty version
        ];
        for height in heights {
            let flat = from_oracle_version(&oracle::Version::leaf(height));
            check_join_meet_subadditive(&comb_version, &flat);
            check_join_meet_subadditive(&flat, &comb_version);
        }
    }
}

proptest! {
    /// Join and meet of arbitrary normal-form event trees hold the
    /// subadditivity lemma.
    #[test]
    fn arbitrary_pairs_hold_subadditivity(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        check_join_meet_subadditive(&from_oracle_version(&a), &from_oracle_version(&b));
    }

    /// Join and meet over every version pair produced by an organic
    /// fork/tick/send/sync/join history hold the subadditivity lemma.
    #[test]
    fn organic_pairs_hold_subadditivity(ops in optrace::world_strategy_up_to(120)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for pair in clocks.windows(2) {
            check_join_meet_subadditive(pair[0].version(), pair[1].version());
        }
    }

    /// Join and meet of alternating combs — every consecutive-leaf delta a
    /// full magnitude swing — hold the subadditivity lemma.
    #[test]
    fn comb_pairs_hold_subadditivity(
        (m1, p1) in arb_comb_params(),
        (m2, p2) in arb_comb_params(),
    ) {
        check_join_meet_subadditive(&comb(m1, p1), &comb(m2, p2));
    }

    /// Join and meet of the deep unbalanced shape-grid trees (left spine,
    /// right spine, zigzag, bushy) hold the subadditivity lemma across
    /// every shape cross and scale pairing.
    #[test]
    fn deep_shape_pairs_hold_subadditivity(
        shape_a in generators::arb_shape(),
        shape_b in generators::arb_shape(),
        scale_a in 1usize..=48,
        scale_b in 1usize..=48,
    ) {
        check_join_meet_subadditive(
            &generators::shape_version(shape_a, scale_a),
            &generators::shape_version(shape_b, scale_b),
        );
    }

    /// Join and meet of interleaved plateau grids — high/low runs of
    /// independent periods, phases, and magnitudes on a shared 32-cell
    /// grid — hold the subadditivity lemma.
    #[test]
    fn interleaved_plateau_pairs_hold_subadditivity(
        ma in magnitude_bits(),
        mb in magnitude_bits(),
        pa in prop_oneof![Just(1usize), Just(2), Just(4)],
        pb in prop_oneof![Just(1usize), Just(2), Just(4)],
        phase in 0usize..=3,
    ) {
        const CELLS: usize = 32;
        let high_a = all_ones(ma);
        let high_b = all_ones(mb);
        let a: Vec<Base> = (0..CELLS)
            .map(|i| if (i / pa) % 2 == 0 { high_a.clone() } else { Base::ZERO })
            .collect();
        let b: Vec<Base> = (0..CELLS)
            .map(|i| if ((i + phase) / pb) % 2 == 0 { Base::ZERO } else { high_b.clone() })
            .collect();
        check_join_meet_subadditive(&grid_version(&a), &grid_version(&b));
    }

    /// Join and meet of plateau grids at staggered widths — one operand's
    /// plateaus 16 cells wide, the other's oscillating cell by cell, so no
    /// boundary structure aligns — hold the subadditivity lemma.
    #[test]
    fn staggered_width_pairs_hold_subadditivity(
        ma in magnitude_bits(),
        mb in magnitude_bits(),
        phase in 0usize..=1,
    ) {
        let high_a = all_ones(ma);
        let high_b = all_ones(mb);
        let a: Vec<Base> = (0..4)
            .map(|i| if i % 2 == 0 { Base::ZERO } else { high_a.clone() })
            .collect();
        let b: Vec<Base> = (0..64)
            .map(|i| if (i + phase) % 2 == 0 { high_b.clone() } else { Base::ZERO })
            .collect();
        check_join_meet_subadditive(&grid_version(&a), &grid_version(&b));
    }

    /// Join and meet of two staircases stepping cell-by-cell across the
    /// same power-of-two cliff — shifted copies or opposed directions —
    /// hold the subadditivity lemma at every crossing pattern.
    #[test]
    fn cliff_staircase_pairs_hold_subadditivity(
        k in prop_oneof![8usize..=10, 62usize..=66, 190usize..=194],
        shift in 1usize..=4,
        descending in any::<bool>(),
    ) {
        const CELLS: usize = 16;
        // The staircase floor 2^k - CELLS/2, so the ascent crosses the
        // cliff at the grid's midpoint.
        let floor = (Base::from(1u8) << u32::try_from(k).expect("cliff bit count fits u32"))
            - &Base::from((CELLS / 2) as u8);
        let a: Vec<Base> = (0..CELLS).map(|i| &floor + &Base::from(i as u64)).collect();
        let b: Vec<Base> = (0..CELLS)
            .map(|i| {
                let step = if descending { CELLS - 1 - i } else { i + shift };
                &floor + &Base::from(step as u64)
            })
            .collect();
        check_join_meet_subadditive(&grid_version(&a), &grid_version(&b));
    }
}

/// Equal leaf values meeting across a subtree boundary make Tier 2 strictly
/// smaller: `(0, (0, 0, 1), 1)` is 10 bits against today's 14.
///
/// Preorder leaves are 0, 1, 1, so the second delta is zero — a 1-bit code
/// where today stores `gamma(1)` twice. Pins the "sometimes smaller" claim.
#[test]
fn cross_boundary_equal_leaves_are_smaller_in_tier2() {
    let v = from_oracle_version(&oracle::Version::node(
        0u64,
        oracle::Version::node(
            0u64,
            oracle::Version::leaf(0u64),
            oracle::Version::leaf(1u64),
        ),
        oracle::Version::leaf(1u64),
    ));
    assert_eq!(packed_bits_of(&to_oracle_version(&v)).len(), 14);
    let size = tier2_size(&packed_bits_of(&to_oracle_version(&v)));
    assert_eq!(
        size,
        Tier2Size {
            total_bits: 10,
            nodes: 5,
            leaves: 3,
            first_leaf_bits: 1,
            delta_bits: 4,
        }
    );
}
