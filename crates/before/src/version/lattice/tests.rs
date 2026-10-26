//! Independent checks for version join and meet.
//!
//! Each generated pair is checked three ways. A recursive tree model supplies
//! the expected whole value. A separate interval walk checks that every part of
//! the output is the pointwise maximum or minimum of the inputs. Lattice-law
//! tests then check the relationships among operations without relying on
//! either model. The combined hull operation must produce the same join and
//! meet as the individual operations.
//!
//! Equality of canonical versions is byte equality, so matching the recursive
//! model also checks the exact emitted representation. Every output must
//! validate and fit within the inputs' combined bit length.

use proptest::prelude::*;
use rayon::prelude::*;
use suanpan::Accumulator;

use crate::testing::bridge::{from_oracle_version, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::generators::leaf_list::{arb_deep_or_correlated_pair, TWO_VERSION_DIVISOR};
use crate::testing::meter::registry::Shape;
use crate::testing::meter::Encoding;
use crate::testing::oracles::flat::{self, check_version};
use crate::testing::property::divided_proptest;
use crate::testing::{generators, optrace};
use crate::version::instrument::validate;
use crate::version::io::regions::{HeightChange, RegionReader, VersionRegionReader};
use crate::{Clock, Span, Version};
use num_bigint::BigUint;
use proptest::sample::Index;

/// Decode a meter-generated encoded shape as a [`Version`].
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// Check join, meet, and hull against both independent models in both operand
/// orders.
fn assert_emits(a: &Version, b: &Version) {
    let (ta, tb) = (to_oracle_version(a), to_oracle_version(b));
    let joined = from_oracle_version(&(ta.clone() | tb.clone()));
    let met = from_oracle_version(&(ta & tb));
    for (x, y) in [(a, b), (b, a)] {
        let out = x.join(y);
        assert_eq!(out, joined, "join must match the oracle: {a:?} vs {b:?}");
        validate(&out).expect("an emitted join is canonical");
        assert_output_fits_inputs("join", x, y, &out);
        assert_pointwise(x, y, &out, false);
        let out = x.meet(y);
        assert_eq!(out, met, "meet must match the oracle: {a:?} vs {b:?}");
        validate(&out).expect("an emitted meet is canonical");
        assert_output_fits_inputs("meet", x, y, &out);
        assert_pointwise(x, y, &out, true);
        let hulled = x.hull_bits(y);
        assert_eq!(
            hulled.relation,
            oracle_relation(&met, x, y),
            "the hull relation must match the oracle: {a:?} vs {b:?}"
        );
        assert_eq!(
            hulled.lo, met,
            "the hull's lower endpoint must be the meet: {a:?} vs {b:?}"
        );
        assert_eq!(
            hulled.hi, joined,
            "the hull's upper endpoint must be the join: {a:?} vs {b:?}"
        );
    }
}

/// Assert the encoding-size bound required by balanced joins and meets.
///
/// Checking live bits is stronger than comparing padded byte lengths. Running
/// this inside the main differential applies the bound to every adversarial
/// family that also checks the operation's value and canonical encoding.
fn assert_output_fits_inputs(name: &str, a: &Version, b: &Version, out: &Version) {
    let input_bits = a.encoded_bits() + b.encoded_bits();
    assert!(
        out.encoded_bits() <= input_bits,
        "{name} output uses {} bits, exceeding its inputs' {input_bits} bits",
        out.encoded_bits(),
    );
}

/// Derive the pair's causal order from the oracle meet.
///
/// By the lattice law, `x <= y` exactly when `x & y == x`. This determines the
/// expected hull relation without using the production comparison algorithm.
fn oracle_relation(met: &Version, x: &Version, y: &Version) -> Option<core::cmp::Ordering> {
    match (met == x, met == y) {
        (true, true) => Some(core::cmp::Ordering::Equal),
        (true, false) => Some(core::cmp::Ordering::Less),
        (false, true) => Some(core::cmp::Ordering::Greater),
        (false, false) => None,
    }
}

/// Check every interval where any input or output changes.
///
/// On each interval, the output must equal the inputs' pointwise maximum for a
/// join or minimum for a meet. Signed differences let the test compare those
/// heights without sharing the production emitter's selection logic.
fn assert_pointwise(a: &Version, b: &Version, out: &Version, meet: bool) {
    let (mut ca, ha) = VersionRegionReader::open(a);
    let (mut cb, hb) = VersionRegionReader::open(b);
    let (mut co, ho) = VersionRegionReader::open(out);
    // BigInt differences out − a and out − b: the pointwise claim reads off
    // their signs without materializing any height.
    let mut oa = Accumulator::new();
    oa.add_shifted_limbs(0, ho.iter_u64_digits());
    oa.sub_shifted_limbs(0, ha.iter_u64_digits());
    let mut ob = Accumulator::new();
    ob.add_shifted_limbs(0, ho.iter_u64_digits());
    ob.sub_shifted_limbs(0, hb.iter_u64_digits());
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
        *diff -= step;
    } else {
        *diff += step;
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
        version_of(&Shape::Staircase.build1(16)),
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

/// Exhaustive small scope: every ordered pair of normal-form event trees to the
/// small-scope depth emits join and meet byte-identically to the recursive
/// oracle.
///
/// Brute force reaches exact equalities, changes at subtree boundaries, sign
/// changes, and canonical collapses deterministically rather than by sampling.
#[test]
fn exhaustive_small_scope_emits_identically() {
    let pool: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    pool.par_iter().for_each(|a| {
        for b in &pool {
            assert_emits(a, b);
        }
    });
}

/// The lattice laws hold on the emitted streams themselves over the family
/// pool: commutativity, idempotence, and absorption for both operators, as byte
/// equality of canonical streams.
#[test]
fn family_lattice_laws_hold() {
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

    /// Every pair of versions produced by one organic op-trace history
    /// emits byte-identically to the recursive oracle.
    #[test]
    fn organic_histories_emit_identically(ops in optrace::production_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for a in &clocks {
            for b in &clocks {
                assert_emits(a.version(), b.version());
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
        pa in prop_oneof![Just(1usize), Just(2), Just(4), Just(8), Just(16)],
        pb in prop_oneof![Just(1usize), Just(2), Just(4), Just(8), Just(16)],
        phase in 0usize..=3,
    ) {
        const CELLS: usize = 32;
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

    /// Staircases crossing a power-of-two boundary exercise joins and meets
    /// where nearly every adjacent delta changes encoded width.
    #[test]
    fn cliff_staircase_pairs_emit_identically(
        exponent in prop_oneof![8usize..=10, 62usize..=66, 190usize..=194],
        shift in 1usize..=4,
        descending in any::<bool>(),
    ) {
        const CELLS: usize = 16;
        let floor = (BigUint::from(1u8)
            << u32::try_from(exponent).expect("test exponent fits u32"))
            - BigUint::from((CELLS / 2) as u8);
        let a: Vec<BigUint> = (0..CELLS)
            .map(|i| &floor + BigUint::from(i as u64))
            .collect();
        let b: Vec<BigUint> = (0..CELLS)
            .map(|i| {
                let step = if descending { CELLS - 1 - i } else { i + shift };
                &floor + BigUint::from(step as u64)
            })
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

// ───────────────────────── against the flat model ─────────────────────────

/// A production version decoded again from its own bytes: equal to it, in
/// storage of its own.
fn redecode(version: &Version) -> Version {
    Version::decode(version.as_bytes()).expect("a version's bytes decode")
}

/// Checks every join and meet spelling of `a` and `b` against the flat
/// model's join and meet of `ma` and `mb`.
///
/// The spellings include both endpoints of both hull spellings, and the
/// stored lengths of an operand and both results are checked against the
/// model's.
fn check_lattice_spellings(
    a: &Version,
    b: &Version,
    ma: &flat::Version,
    mb: &flat::Version,
) -> Result<(), TestCaseError> {
    let (join, meet) = (ma.join(mb), ma.meet(mb));
    let (mut join_borrowed, mut join_owned) = (a.clone(), a.clone());
    join_borrowed |= b;
    join_owned |= b.clone();
    let (mut meet_borrowed, mut meet_owned) = (a.clone(), a.clone());
    meet_borrowed &= b;
    meet_owned &= b.clone();
    let hull = a.span(b);
    let xor = a ^ b;
    let joins = [
        ("a.join(b)", a.join(b)),
        ("&a | &b", a | b),
        ("a | &b", a.clone() | b),
        ("&a | b", a | b.clone()),
        ("a | b", a.clone() | b.clone()),
        ("a |= &b", join_borrowed),
        ("a |= b", join_owned),
        ("a.span(b).hi()", hull.hi().clone()),
        ("(&a ^ &b).hi()", xor.hi().clone()),
    ];
    let meets = [
        ("a.meet(b)", a.meet(b)),
        ("&a & &b", a & b),
        ("a & &b", a.clone() & b),
        ("&a & b", a & b.clone()),
        ("a & b", a.clone() & b.clone()),
        ("a &= &b", meet_borrowed),
        ("a &= b", meet_owned),
        ("a.span(b).lo()", hull.lo().clone()),
        ("(&a ^ &b).lo()", xor.lo().clone()),
    ];
    for (spelling, got) in &joins {
        check_version(got, &join, spelling)?;
    }
    for (spelling, got) in &meets {
        check_version(got, &meet, spelling)?;
    }
    prop_assert_eq!(
        a.is_empty(),
        ma.is_empty(),
        "whether the first operand is empty"
    );
    prop_assert_eq!(
        a.encoded_bits(),
        ma.bits(),
        "the first operand's stored length"
    );
    prop_assert_eq!(
        joins[0].1.encoded_bits(),
        join.bits(),
        "the join's stored length"
    );
    prop_assert_eq!(
        meets[0].1.encoded_bits(),
        meet.bits(),
        "the meet's stored length"
    );
    Ok(())
}

/// How a fold item is taken from its pool member.
#[derive(Clone, Copy, Debug)]
enum Take {
    /// A clone, sharing the member's storage.
    Shared,
    /// A separate decode of the member's bytes, in storage of its own.
    Separate,
    /// A clone of the previous item, or a shared clone of the member when
    /// there is none.
    Repeat,
}

divided_proptest! {
    #![divisor(TWO_VERSION_DIVISOR)]

    /// Every join and meet spelling, and both endpoints of both hull
    /// spellings, equal the flat model's join and meet byte for byte, and the
    /// stored lengths equal the model's.
    ///
    /// The operand pairs are a deep pair in both orders, a version with its
    /// clone and with a separately decoded copy, and the two endpoints of a
    /// decoded span, which are slices of one buffer. The deep pairs are
    /// related by construction, and half are built through the model so that
    /// their joins and meets collapse in cascades; every operand reaches
    /// production through the independent encoder, so production's writer
    /// is judged against the model's canonical form, never against itself.
    #[test]
    fn lattice_matches_the_flat_model_on_deep_pairs((a, b) in arb_deep_or_correlated_pair()) {
        let (ma, mb) = (flat::Version::from_rows(a.leaves()), flat::Version::from_rows(b.leaves()));
        let (va, vb) = (ma.production(), mb.production());
        check_lattice_spellings(&va, &vb, &ma, &mb)?;
        check_lattice_spellings(&vb, &va, &mb, &ma)?;
        check_lattice_spellings(&va, &va.clone(), &ma, &ma)?;
        check_lattice_spellings(&va, &redecode(&va), &ma, &ma)?;
        let wire = Span::decode(&va.span(&vb).encode()[..]).expect("a span's bytes decode");
        check_lattice_spellings(wire.lo(), wire.hi(), &ma.meet(&mb), &ma.join(&mb))?;
    }

    /// `join_all`, `meet_all`, and `span_all`, over owned and borrowed
    /// items, and `Sum` and `collect` over both, equal the flat model's left
    /// folds byte for byte, at arities from 0 to 40.
    ///
    /// The items come from a pool of a deep pair, its join and meet, and the
    /// empty version, each a shared clone, a separate decode, or a repeat of
    /// the item before; repeats share storage, which the folds' run
    /// deduplication skips. Arities to 40 reach every carry of the balanced
    /// fold's binary counter through 32 inputs.
    #[test]
    fn folds_match_the_flat_model_on_deep_pools(
        (a, b) in arb_deep_or_correlated_pair(),
        receiver in any::<Index>(),
        picks in proptest::collection::vec(
            (
                any::<Index>(),
                prop_oneof![Just(Take::Shared), Just(Take::Separate), Just(Take::Repeat)],
            ),
            0..=40,
        ),
    ) {
        let (ma, mb) = (flat::Version::from_rows(a.leaves()), flat::Version::from_rows(b.leaves()));
        let models = [ma.join(&mb), ma.meet(&mb), ma, mb, flat::Version::empty()];
        let pool: Vec<Version> = models.iter().map(flat::Version::production).collect();
        let receiver = receiver.index(pool.len());
        let mut items: Vec<Version> = Vec::with_capacity(picks.len());
        let mut chosen: Vec<&flat::Version> = Vec::with_capacity(picks.len());
        for (pick, take) in &picks {
            let (item, model) = match (take, items.last()) {
                (Take::Repeat, Some(last)) => (last.clone(), chosen[chosen.len() - 1]),
                (Take::Separate, _) => {
                    let i = pick.index(pool.len());
                    (redecode(&pool[i]), &models[i])
                }
                _ => {
                    let i = pick.index(pool.len());
                    (pool[i].clone(), &models[i])
                }
            };
            items.push(item);
            chosen.push(model);
        }
        let join = models[receiver].join_all(chosen.iter().copied());
        let meet = models[receiver].meet_all(chosen.iter().copied());
        let sum = flat::Version::empty().join_all(chosen.iter().copied());
        let recv = &pool[receiver];
        check_version(&recv.join_all(&items), &join, "join_all over borrowed items")?;
        check_version(&recv.join_all(items.clone()), &join, "join_all over owned items")?;
        check_version(&recv.meet_all(&items), &meet, "meet_all over borrowed items")?;
        check_version(&recv.meet_all(items.clone()), &meet, "meet_all over owned items")?;
        let span = recv.span_all(&items);
        check_version(span.lo(), &meet, "span_all over borrowed items, lo")?;
        check_version(span.hi(), &join, "span_all over borrowed items, hi")?;
        let span = recv.span_all(items.clone());
        check_version(span.lo(), &meet, "span_all over owned items, lo")?;
        check_version(span.hi(), &join, "span_all over owned items, hi")?;
        check_version(&items.iter().sum::<Version>(), &sum, "Sum over borrowed items")?;
        check_version(&items.clone().into_iter().sum::<Version>(), &sum, "Sum over owned items")?;
        check_version(&items.iter().collect::<Version>(), &sum, "collect over borrowed items")?;
        check_version(&items.into_iter().collect::<Version>(), &sum, "collect over owned items")?;
    }
}
