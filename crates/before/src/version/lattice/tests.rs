//! Differential pins for Version merging against three witnesses.
//!
//! The recursive oracle's join and meet (through the bridge) are the byte-level
//! value witness, a three-cursor overlay walk re-derives every output plateau
//! pointwise, and the lattice laws are asserted on the emitted streams
//! themselves. The fused hull sweep rides every oracle comparison: on each
//! witnessed pair it must reproduce both single-op outputs byte for byte from
//! its one walk.
//!
//! Canonical uniqueness is what makes the oracle differential total: the
//! emitted stream must equal the oracle's encoded result *byte for byte*, so a
//! silent side-switch misread cannot hide behind an equivalent-but-different
//! spelling. The pointwise walk is the second independent witness, sharing
//! nothing with the oracle's recursion either: it re-materializes absolute
//! heights (test-only) and checks `max`/`min` on every elementary interval of
//! the three-stream overlay.

use proptest::prelude::*;
use rayon::prelude::*;
use suanpan::Accumulator;

use crate::accumulator;
use crate::testing::bridge::{from_oracle_version, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::meter::registry::Shape;
use crate::testing::meter::Encoding;
use crate::testing::{generators, optrace};
use crate::version::instrument::validate;
use crate::version::io::regions::{HeightChange, RegionReader, VersionRegionReader};
use crate::{Clock, Version};
use num_bigint::BigUint;

/// Decode a meter-generated encoded shape as a [`Version`].
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// Assert both emitters against the recursive oracle on one pair, in both
/// operand orders, and run the pointwise overlay witness on each emitted
/// stream.
///
/// The fused hull sweep rides the same comparison: it must reproduce both
/// single-op outputs byte for byte from its one walk, and its carried relation
/// must match the oracle's lattice reading of the pair ([`oracle_relation`]).
fn assert_emits(a: &Version, b: &Version) {
    let (ta, tb) = (to_oracle_version(a), to_oracle_version(b));
    let joined = from_oracle_version(&(ta.clone() | tb.clone()));
    let met = from_oracle_version(&(ta & tb));
    for (x, y) in [(a, b), (b, a)] {
        let out = x.join(y);
        assert_eq!(out, joined, "join must match the oracle: {a:?} vs {b:?}");
        validate(&out).expect("an emitted join is canonical");
        assert_pointwise(x, y, &out, false);
        let out = x.meet(y);
        assert_eq!(out, met, "meet must match the oracle: {a:?} vs {b:?}");
        validate(&out).expect("an emitted meet is canonical");
        assert_pointwise(x, y, &out, true);
        let hulled = x.hull_bits(y);
        assert_eq!(
            hulled.relation,
            oracle_relation(&met, x, y),
            "the fused verdict must match the oracle's lattice reading: {a:?} vs {b:?}"
        );
        assert_eq!(
            hulled.lo, met,
            "the fused hull's meet must match: {a:?} vs {b:?}"
        );
        assert_eq!(
            hulled.hi, joined,
            "the fused hull's join must match: {a:?} vs {b:?}"
        );
    }
}

/// The pair's causal order, read off the oracle's meet by the lattice laws
/// alone: `x <= y` iff `x ∧ y = x`, and canonical uniqueness makes that one
/// byte comparison per direction.
///
/// Independent of the sweep under test on both faces — the meet comes from the
/// recursive oracle, the reading from the order-theoretic definition — so the
/// fused verdict differential shares nothing with the fold it checks.
fn oracle_relation(met: &Version, x: &Version, y: &Version) -> Option<core::cmp::Ordering> {
    match (met == x, met == y) {
        (true, true) => Some(core::cmp::Ordering::Equal),
        (true, false) => Some(core::cmp::Ordering::Less),
        (false, true) => Some(core::cmp::Ordering::Greater),
        (false, false) => None,
    }
}

/// Walk the three-stream overlay and check the output height is the pointwise
/// `max` (or `min` when `meet`) of the input heights on every elementary
/// interval.
///
/// Materializes absolute running heights (test-only; the emitter never does)
/// with one signed accumulator per stream pair, advancing whichever cursors'
/// plateaus end first — the deepest cursors step, per the nesting rule the
/// sweeps rest on.
fn assert_pointwise(a: &Version, b: &Version, out: &Version, meet: bool) {
    let (mut ca, ha) = VersionRegionReader::open(a);
    let (mut cb, hb) = VersionRegionReader::open(b);
    let (mut co, ho) = VersionRegionReader::open(out);
    // BigInt differences out − a and out − b: the pointwise claim reads off
    // their signs without materializing any height.
    let mut oa = Accumulator::new();
    accumulator::fold(&mut oa, &ho, 0, false);
    accumulator::fold(&mut oa, &ha, 0, true);
    let mut ob = Accumulator::new();
    accumulator::fold(&mut ob, &ho, 0, false);
    accumulator::fold(&mut ob, &hb, 0, true);
    let mut intervals = 0u64;
    loop {
        intervals += 1;
        let (against_a, against_b) = (oa.sign(), ob.sign());
        if meet {
            assert!(
                against_a <= core::cmp::Ordering::Equal && against_b <= core::cmp::Ordering::Equal,
                "interval {intervals}: a meet plateau above an input"
            );
            assert!(
                against_a == core::cmp::Ordering::Equal || against_b == core::cmp::Ordering::Equal,
                "interval {intervals}: a meet plateau below both inputs"
            );
        } else {
            assert!(
                against_a >= core::cmp::Ordering::Equal && against_b >= core::cmp::Ordering::Equal,
                "interval {intervals}: a join plateau below an input"
            );
            assert!(
                against_a == core::cmp::Ordering::Equal || against_b == core::cmp::Ordering::Equal,
                "interval {intervals}: a join plateau above both inputs"
            );
        }
        if ca.done() && cb.done() && co.done() {
            return;
        }
        // Advance every cursor whose plateau ends at this boundary: first the
        // deepest — their plateaus end first, since overlapping dyadic
        // intervals nest — then any shallower cursor the deepest side's flip
        // level ties (the two-cursor sweeps' rule, which extends to three
        // streams unchanged).
        let depth = ca.depth().max(cb.depth()).max(co.depth());
        let mut flip = u64::MAX;
        let (mut so, mut sa, mut sb) = (None, None, None);
        if co.depth() == depth {
            let (f, step) = co.step();
            flip = flip.min(f);
            so = Some(step);
        }
        if ca.depth() == depth {
            let (f, step) = ca.step();
            flip = flip.min(f);
            sa = Some(step);
        }
        if cb.depth() == depth {
            let (f, step) = cb.step();
            flip = flip.min(f);
            sb = Some(step);
        }
        if so.is_none() && !co.done() && flip <= co.depth() {
            so = Some(co.step().1);
        }
        if sa.is_none() && !ca.done() && flip <= ca.depth() {
            sa = Some(ca.step().1);
        }
        if sb.is_none() && !cb.done() && flip <= cb.depth() {
            sb = Some(cb.step().1);
        }
        // Fold the boundary's deltas: the output's raises both differences, an
        // input's lowers its own.
        if let Some(step) = &so {
            fold_step(&mut oa, false, step);
            fold_step(&mut ob, false, step);
        }
        if let Some(step) = &sa {
            fold_step(&mut oa, true, step);
        }
        if let Some(step) = &sb {
            fold_step(&mut ob, true, step);
        }
    }
}

/// Fold one raw step delta into a signed difference, subtracting when the
/// stream sits on the difference's negative side.
fn fold_step(diff: &mut Accumulator, subtract: bool, step: &HeightChange) {
    if subtract {
        accumulator::subtract_signed(diff, step);
    } else {
        accumulator::fold_signed(diff, step);
    }
}

/// The registered input families used by deterministic grids.
fn family_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&Shape::Dense.build1(1)),
        version_of(&Shape::Dense.build1(2)),
        version_of(&Shape::Dense.build1(64)),
        version_of(&Shape::Bigroot.build2(7, 3)),
        version_of(&Shape::Bigroot.build2(64, 16)),
        version_of(&Shape::Hugeleaf.build1(1)),
        version_of(&Shape::Hugeleaf.build1(64)),
        version_of(&Shape::CliffComb.build2(3, 2)),
        version_of(&Shape::CliffComb.build2(16, 16)),
        version_of(&Shape::WideToothComb.build3(16, 8, 8)),
        version_of(&Shape::CliffFan.build2(16, 8)),
        version_of(&Shape::CancellingChain.build2(16, 8)),
        version_of(&Shape::AltSpine.build1(3)),
        version_of(&Shape::AltSpine.build1(64)),
        version_of(&Shape::Harmonic.build1(16)),
    ]
}

/// Every ordered pair drawn from the registered families emits
/// byte-identically to the recursive oracle, validates as canonical, and
/// re-derives pointwise.
///
/// Each operand is also paired against the pair's join and meet — the shapes
/// where long shared plateaus force ties and total collapses.
#[test]
fn family_pairs_emit_identically() {
    let pool = family_pool();
    pool.par_iter().for_each(|a| {
        for b in &pool {
            assert_emits(a, b);
            let joined = a | b;
            assert_emits(a, &joined);
            let met = a & b;
            assert_emits(&met, b);
        }
    });
}

/// A flat operand above a deep one collapses the whole output to one leaf
/// through the absorb cascade, byte-identically to the recursive oracle.
#[test]
fn flat_over_deep_collapses_totally() {
    let deep = version_of(&Shape::Dense.build1(512));
    let flat = version_of(&Shape::Hugeleaf.build1(600));
    assert_emits(&deep, &flat);
    let joined = deep.join(&flat);
    assert_eq!(joined, flat, "a dominating flat operand is the join");
}

/// A left spine whose right child at every level is the leaf pair `(0, 1)`.
///
/// Joining a sufficiently high flat version with this shape emits one leaf,
/// recovering the same wide left payload once at every level.
#[cfg(feature = "scan-meter")]
fn reanchor_spine(depth: usize) -> Version {
    let pair = crate::testing::oracles::tree::Version::node(
        BigUint::ZERO,
        crate::testing::oracles::tree::Version::leaf(BigUint::ZERO),
        crate::testing::oracles::tree::Version::leaf(BigUint::from(1u8)),
    );
    let mut tree = crate::testing::oracles::tree::Version::leaf(BigUint::ZERO);
    for _ in 0..depth {
        tree = crate::testing::oracles::tree::Version::node(BigUint::ZERO, tree, pair.clone());
    }
    from_oracle_version(&tree)
}

/// Measure the more expensive operand order for a wide-leaf/spine join.
#[cfg(feature = "scan-meter")]
fn reanchor_join_scan(depth: usize) -> (u64, u64) {
    let flat = version_of(&Shape::Hugeleaf.build1(10 * depth));
    let spine = reanchor_spine(depth);
    let input_bits = flat.encoded_bits() + spine.encoded_bits();
    crate::testing::meter::reset_scan_bits();
    let joined = std::hint::black_box(&flat | &spine);
    let forward = crate::testing::meter::scan_bits();
    assert_eq!(joined, flat, "the dominating flat version is the join");
    crate::testing::meter::reset_scan_bits();
    let joined = std::hint::black_box(&spine | &flat);
    let reverse = crate::testing::meter::scan_bits();
    assert_eq!(joined, flat, "the dominating flat version is the join");
    let scanned = forward.max(reverse);
    (scanned, input_bits)
}

/// Join work remains linear when one wide payload survives a collapse at every
/// level. When width and depth double together, scanned bits per input bit stay
/// within a factor of 1.25.
#[cfg(feature = "scan-meter")]
#[test]
fn reanchor_join_scan_is_linear_per_input_bit() {
    let (small_scan, small_bits) = reanchor_join_scan(32);
    let (large_scan, large_bits) = reanchor_join_scan(64);
    assert!(
        4 * large_scan * small_bits <= 5 * small_scan * large_bits,
        "scan work per input bit grew too quickly: {small_scan}/{small_bits} -> \
         {large_scan}/{large_bits}"
    );
}

/// Exhaustive small scope: every ordered pair of normal-form event trees to the
/// small-scope depth emits join and meet byte-identically to the recursive
/// oracle.
///
/// Brute force reaches every boundary case — aligned ties, flush-right ties,
/// plateau consumption, switches at and across zero deltas, collapse cascades —
/// deterministically rather than by sampling.
#[test]
fn exhaustive_small_scope_emits_identically() {
    let pool: Vec<(crate::testing::oracles::tree::Version, Version)> =
        all_normal_events(EV_SMALL_DEPTH)
            .iter()
            .map(|t| {
                let v = from_oracle_version(t);
                (t.clone(), v)
            })
            .collect();
    pool.par_iter().for_each(|(ta, va)| {
        for (tb, vb) in &pool {
            let joined = from_oracle_version(&(ta.clone() | tb.clone()));
            let met = from_oracle_version(&(ta.clone() & tb.clone()));
            assert_eq!(
                va.join(vb),
                joined,
                "join must match the oracle: {va:?} vs {vb:?}"
            );
            assert_eq!(
                va.meet(vb),
                met,
                "meet must match the oracle: {va:?} vs {vb:?}"
            );
            let hulled = va.hull_bits(vb);
            assert_eq!(
                hulled.relation,
                oracle_relation(&met, va, vb),
                "the fused verdict must match the oracle's lattice reading: {va:?} vs {vb:?}"
            );
            assert_eq!(
                (hulled.lo, hulled.hi),
                (met, joined),
                "the fused hull must match both single-op outputs: {va:?} vs {vb:?}"
            );
        }
    });
}

/// The lattice laws hold on the emitted streams themselves over the family
/// pool: commutativity, idempotence, and absorption for both operators, as byte
/// equality of canonical streams.
#[test]
fn family_lattice_laws_hold_on_the_kernel() {
    let pool = family_pool();
    for a in &pool {
        assert_eq!(a.join(a), *a, "join is idempotent");
        assert_eq!(a.meet(a), *a, "meet is idempotent");
        for b in &pool {
            let joined = a.join(b);
            let met = a.meet(b);
            assert_eq!(joined, b.join(a), "join commutes");
            assert_eq!(met, b.meet(a), "meet commutes");
            assert_eq!(a.join(&met), *a, "join absorbs the meet");
            assert_eq!(a.meet(&joined), *a, "meet absorbs the join");
        }
    }
}

/// Associativity holds on the emitted streams over every family triple.
#[test]
fn family_associativity_holds_on_the_kernel() {
    let pool = family_pool();
    pool.par_iter().for_each(|a| {
        for b in &pool {
            for c in &pool {
                assert_eq!(a.join(b).join(c), a.join(&b.join(c)), "join associates");
                assert_eq!(a.meet(b).meet(c), a.meet(&b.meet(c)), "meet associates");
            }
        }
    });
}

proptest! {
    /// Arbitrary normal-form pairs (magnitudes past `u64::MAX` included) emit
    /// byte-identically to the recursive oracle, validate, and re-derive
    /// pointwise.
    ///
    /// The pair's join and meet supply the dominated shapes arbitrary pairs
    /// alone under-hit.
    #[test]
    fn arbitrary_pairs_emit_identically(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        let (va, vb) = (from_oracle_version(&a), from_oracle_version(&b));
        assert_emits(&va, &vb);
        let joined = &va | &vb;
        assert_emits(&va, &joined);
        let met = &va & &vb;
        assert_emits(&met, &vb);
    }

    /// Arbitrary triples satisfy associativity and the absorption pair on the
    /// emitted streams.
    #[test]
    fn arbitrary_triples_hold_the_lattice_laws(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
        c in generators::arb_oracle_version(),
    ) {
        let a = from_oracle_version(&a);
        let b = from_oracle_version(&b);
        let c = from_oracle_version(&c);
        prop_assert_eq!(a.join(&b).join(&c), a.join(&b.join(&c)));
        prop_assert_eq!(a.meet(&b).meet(&c), a.meet(&b.meet(&c)));
        prop_assert_eq!(a.join(&a.meet(&b)), a.clone());
        prop_assert_eq!(a.meet(&a.join(&b)), a);
    }

    /// Every pair of versions produced by one organic fork/tick/send/sync/join
    /// history emits byte-identically to the recursive oracle.
    #[test]
    fn organic_histories_emit_identically(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        let pool: Vec<(crate::testing::oracles::tree::Version, &Version)> = clocks
            .iter()
            .map(|c| (to_oracle_version(c.version()), c.version()))
            .collect();
        for (ta, va) in &pool {
            for (tb, vb) in &pool {
                let joined = from_oracle_version(&(ta.clone() | tb.clone()));
                let met = from_oracle_version(&(ta.clone() & tb.clone()));
                prop_assert_eq!(
                    va.join(vb),
                    joined.clone(),
                    "join must match the oracle: {:?} vs {:?}", va, vb
                );
                prop_assert_eq!(
                    va.meet(vb),
                    met.clone(),
                    "meet must match the oracle: {:?} vs {:?}", va, vb
                );
                let hulled = va.hull_bits(vb);
                prop_assert_eq!(
                    hulled.relation,
                    oracle_relation(&met, va, vb),
                    "the fused verdict must match the oracle's lattice reading: {:?} vs {:?}", va, vb
                );
                prop_assert_eq!(
                    (hulled.lo, hulled.hi),
                    (met, joined),
                    "the fused hull must match both single-op outputs: {:?} vs {:?}", va, vb
                );
            }
        }
    }

    /// Interval-grid pairs whose plateaus swing across the machine-word
    /// boundary emit byte-identically, validate, and re-derive pointwise: the
    /// switch-delta arithmetic is exercised at spilled widths in both
    /// directions.
    ///
    /// The width bands cover values within one word, across a word boundary,
    /// and well beyond it, so coder and arithmetic errors surface through the
    /// pointwise oracle.
    #[test]
    fn wide_grid_pairs_emit_identically(
        ma in prop_oneof![1usize..=8, 29usize..=34, 60usize..=68, 190usize..=200],
        mb in prop_oneof![1usize..=8, 29usize..=34, 60usize..=68, 190usize..=200],
        pa in prop_oneof![Just(1usize), Just(2), Just(4)],
        pb in prop_oneof![Just(1usize), Just(2), Just(4)],
        phase in 0usize..=3,
    ) {
        const CELLS: usize = 16;
        let high = |bits: usize| (BigUint::from(1u8) << u32::try_from(bits).expect("width fits")) - &BigUint::from(1u8);
        let (high_a, high_b) = (high(ma), high(mb));
        let a: Vec<BigUint> = (0..CELLS)
            .map(|i| if (i / pa) % 2 == 0 { high_a.clone() } else { BigUint::ZERO })
            .collect();
        let b: Vec<BigUint> = (0..CELLS)
            .map(|i| if ((i + phase) / pb) % 2 == 0 { BigUint::ZERO } else { high_b.clone() })
            .collect();
        assert_emits(&grid_version(&a), &grid_version(&b));
    }
}

/// Build the version whose height function takes `values[i]` on the `i`th cell of a
/// uniform dyadic grid (test-only; recursive over the grid's `O(log)` depth).
fn grid_version(values: &[BigUint]) -> Version {
    fn build(values: &[BigUint]) -> crate::testing::oracles::tree::Version {
        match values {
            [v] => crate::testing::oracles::tree::Version::leaf(v.clone()),
            _ => {
                let (l, r) = values.split_at(values.len() / 2);
                crate::testing::oracles::tree::Version::node(0u64, build(l), build(r))
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
