//! Relational scan checks for fused span construction.
//!
//! The fused hull kernel's resource identities, stated relationally against
//! the single-op emissions and the pair comparison on the same operands so
//! there is no constant to re-pin: one fused sweep feeds both endpoints, so
//! the pair's streams are decoded once where the composed emitters decode
//! them twice. Two regimes, denominated separately: the *pair* regime
//! (binary `span`, and `span_all`'s leaf combines) shares its operands
//! between the meet and join legs — decode-halving; the *interior* regime
//! (`span_all`'s combines over already-merged hulls) reads a different
//! operand pair per leg — consolidation into one fold, no shared walk.

use super::power_of_two;
use before::testing::meter;
use before::{Clock, Version};

/// Scan bits of one closure run, on a fresh counter.
fn scanned(f: impl FnOnce()) -> u64 {
    meter::reset_scan_bits();
    f();
    meter::scan_bits()
}

/// The span fixture: two comparable snapshots `s < v` of one
/// multi-party history, a divergent line `div` concurrent to `v`,
/// and a population of intermediate snapshots for the n-ary
/// regimes.
///
/// Received sends give every stream real multi-party structure.
fn fixture() -> (Version, Version, Version, Vec<Version>) {
    let mut main = Clock::seed();
    let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
    let mut population = Vec::new();
    let mut rounds = |main: &mut Clock, population: &mut Vec<Version>, n: usize| {
        let k = others.len();
        for i in 0..n {
            main.tick();
            let msg = others[i % k].send().clone();
            main.recv(&msg);
            if i % 7 == 0 {
                population.push(main.version().clone());
            }
        }
    };
    rounds(&mut main, &mut population, 24);
    let s = main.version().clone();
    let mut diverged = main.fork();
    // The plateau above the word range makes the fixture exercise wide
    // gamma decoding.
    main.ticks(power_of_two(80));
    rounds(&mut main, &mut population, 24);
    let v = main.version().clone();
    for _ in 0..24 {
        diverged.tick();
    }
    let div = diverged.version().clone();
    assert!(s < v, "the snapshot chain is strict");
    assert!(v.concurrent(&div), "the lines diverge");
    (s, v, div, population)
}

/// Comparable and concurrent span construction satisfy one scan
/// identity each.
///
/// A *comparable* pair's hull is the pair handed back
/// (`span_is_the_pair_hull`): the span costs exactly one comparison
/// sweep — `span == cmp(a, b)` — with zero emission, so the pin is
/// scan identity with the pair sweep itself. A *concurrent* pair is
/// the only emitting case: the fused hull decodes the pair once
/// (scan counts both stream reads and builder writes, and the fused
/// sweep's writes are the two single-op outputs exactly), after
/// paying the ladder's classifying comparison — its early-exiting
/// concurrent prefix — up front:
/// `span + decode(a) + decode(b) == meet + join + cmp(a, b)`. Each
/// operand's decode is priced as half its self-comparison against a
/// buffer-distinct re-decode (`cmp(x, x')` reads `x` twice and
/// writes nothing; a shared buffer would answer by clone identity
/// without a walk). Stated relationally, so no measured constant
/// can rot.
#[test]
fn span_fuses_the_pair_walk() {
    let (s, v, div, _) = fixture();

    // The comparable regime: hand-back at the cost of the pair sweep.
    let fused = scanned(|| {
        let _ = s.span(&v);
    });
    let cmp_sv = scanned(|| assert!(s.partial_cmp(&v).is_some()));
    eprintln!("MEASURED span_pair_comparable: fused={fused} cmp={cmp_sv}");
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert_eq!(
        fused, cmp_sv,
        "comparable: the hull is the pair handed back at exactly one \
         comparison sweep, zero emission"
    );

    // The concurrent regime: the one emitting case, the fused hull's
    // decode saving intact, the classifying comparison accounted.
    let (a, b) = (&v, &div);
    let fused = scanned(|| {
        let _ = a.span(b);
    });
    let met = scanned(|| {
        let _ = a & b;
    });
    let joined = scanned(|| {
        let _ = a | b;
    });
    let cmp_ab = scanned(|| assert!(a.partial_cmp(b).is_none()));
    let redecode = |x: &Version| Version::decode(&x.encode()[..]).expect("re-decodes");
    let (a2, b2) = (redecode(a), redecode(b));
    let decode_a = scanned(|| assert!(a.partial_cmp(&a2).is_some())) / 2;
    let decode_b = scanned(|| assert!(b.partial_cmp(&b2).is_some())) / 2;
    eprintln!(
        "MEASURED span_pair_concurrent: fused={fused} meet={met} join={joined} \
         cmp={cmp_ab} pair_decode={}",
        decode_a + decode_b,
    );
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert_eq!(
        fused + decode_a + decode_b,
        met + joined + cmp_ab,
        "concurrent: the fused hull must cost the composed emissions minus \
         one decode of the pair, plus the ladder's classifying comparison"
    );
}

/// `span_all`'s leaf combines use the fused pair walk:
/// at one item the multi-input operation scans exactly as the binary span.
#[test]
fn span_all_leaf_combine_is_the_fused_pair_walk() {
    let (s, v, _, _) = fixture();
    let fused = scanned(|| {
        let _ = s.span(&v);
    });
    let unary = scanned(|| {
        let _ = s.span_all([&v]);
    });
    eprintln!("MEASURED span_all_unary: span={fused} span_all={unary}");
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert_eq!(
        unary, fused,
        "span_all at one item must scan exactly as the binary span"
    );
}

/// The multi-input hull costs less than the two composed folds, and
/// the saving is the leaf level's — the interior regime
/// consolidates without a shared walk, so the fold stays strictly
/// above half the composition.
///
/// `span_all` fuses exactly the leaf combines (two raw inputs, one
/// shared pair walk); interior combines read a different operand
/// pair per leg (`lo₁ ∧ lo₂`, `hi₁ ∨ hi₂`), costing what the two
/// composed folds' interior levels cost. The composition is
/// measured over the identical population, receiver included.
#[test]
fn span_all_fuses_the_leaf_level() {
    let (s, _, _, population) = fixture();
    assert!(
        population.len() >= 4,
        "the population exercises interior combines"
    );
    let fused = scanned(|| {
        let _ = s.span_all(&population);
    });
    let composed = scanned(|| {
        let _ = s.meet_all(&population);
        let _ = s.join_all(&population);
    });
    eprintln!("MEASURED span_all_population: fused={fused} composed={composed}");
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert!(
        fused < composed,
        "the leaf-level fusion must undercut the composed folds \
         ({fused} vs {composed})"
    );
    assert!(
        fused * 2 > composed,
        "interior combines have no shared pair walk: the fold must stay \
         strictly above half the composition ({fused} vs {composed})"
    );
}

/// The fused hull folds each crossing into one shared running difference.
///
/// Separate scan tests cover decode sharing. This test covers the arithmetic
/// half: maintaining one difference must cost fewer digit touches than
/// maintaining the meet and join differences separately.
#[cfg(feature = "touch-meter")]
#[test]
fn span_shares_the_crossing_folds() {
    // The concurrent pair: the ladder's only emitting case, so this
    // is the pair that still reaches the fused emission walk these
    // pins are about (a comparable pair hands its operands back at
    // one comparison sweep — `span_fuses_the_pair_walk`'s regime).
    let (_, v, div, _) = fixture();
    let touches = |f: &dyn Fn()| {
        suanpan::touch_meter::reset();
        f();
        suanpan::touch_meter::touches()
    };
    let fused = touches(&|| {
        let _ = v.span(&div);
    });
    let met = touches(&|| {
        let _ = &v & &div;
    });
    let joined = touches(&|| {
        let _ = &v | &div;
    });
    let cmp = touches(&|| assert!(v.partial_cmp(&div).is_none()));
    eprintln!("MEASURED span_pair_touches: fused={fused} meet={met} join={joined} cmp={cmp}");
    assert!(fused > 0, "a live touch meter reads nonzero on a real walk");
    // The fused walk folds every crossing once into one difference. Subtract
    // the separately measured comparison prefix before comparing it with
    // the two independent folds.
    assert!(
        fused - cmp < met + joined,
        "the fused hull's own folds must undercut the composed \
         emissions' two accumulators ({} vs {} composed touches)",
        fused - cmp,
        met + joined
    );
}
