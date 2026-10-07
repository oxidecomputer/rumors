//! Resource checks for identity fast paths.
//!
//! The at-rest form's refcounted backing store makes clone identity
//! observable (`ptr_eq`), and the identity-law fast paths dispatch on it:
//! clone-then-op answers without a walk, each shortcut citing its law in
//! `before::testing::laws` at the code site. These pins hold the fast paths LIVE —
//! every clone-operand cell must read zero walk work, and every cell rides
//! beside a walking leg on the same operands' values in distinct buffers,
//! so a dead meter (which also reads zero) cannot green the section and
//! the walked path stays covered.

use before::testing::meter;
use before::testing::meter::registry::Shape;
use before::{Clock, Version};

/// Peak heap allowed for `join_all`'s fixed-size fold bookkeeping.
const EQUAL_JOIN_PEAK_HEAP: usize = 440;

/// Scan bits of one closure run, on a fresh counter.
fn scanned(f: impl FnOnce()) -> u64 {
    meter::reset_scan_bits();
    f();
    meter::scan_bits()
}

/// The fixture: one multi-party snapshot `v` with real structure, a
/// buffer-distinct byte-equal re-decode `v'`, and a concurrent
/// divergence `w` for the walking legs.
fn fixture() -> (Version, Version, Version) {
    let mut main = Clock::seed();
    let mut others: Vec<Clock> = (0..6).map(|_| main.fork()).collect();
    let mut rounds = |main: &mut Clock, n: usize| {
        let k = others.len();
        for i in 0..n {
            main.tick();
            let msg = others[i % k].send().clone();
            main.recv(&msg);
        }
    };
    rounds(&mut main, 24);
    let mut diverged = main.fork();
    rounds(&mut main, 24);
    let v = main.version().clone();
    for _ in 0..24 {
        diverged.tick();
    }
    let w = diverged.version().clone();
    assert!(v.concurrent(&w), "the walking legs need a real walk");
    let redecoded = Version::decode(&v.encode()[..]).expect("a stored stream re-decodes");
    (v, redecoded, w)
}

/// Joining byte-equal versions returns shared storage without copying either
/// operand.
///
/// The operands are decoded independently, so pointer identity cannot take
/// the shortcut. Growing both streams fourfold must leave the fold's peak heap
/// unchanged: only fixed-size bookkeeping may be allocated before the
/// byte-equality check returns a clone.
#[test]
fn equal_join_all_has_fixed_heap_cost() {
    let run = |depth: usize| {
        let encoded = Shape::Dense.build1(depth);
        let a = super::version_of(&encoded);
        let b = super::version_of(&encoded);
        let (peak, out) = super::peak_heap(|| a.join_all([&b]));
        assert_eq!(out, a, "joining equal versions preserves their value");
        assert!(
            peak <= EQUAL_JOIN_PEAK_HEAP,
            "join_all over equal versions used {peak} transient bytes; the fixed-size \
             bookkeeping ceiling is {EQUAL_JOIN_PEAK_HEAP} bytes"
        );
        peak
    };

    let small = run(1_000);
    let large = run(4_000);
    assert_eq!(
        small, large,
        "growing equal operands must not increase join_all's transient heap"
    );
}

/// Clone operands answer every identity-law fast path without a
/// walk.
///
/// Comparison (`order_reflexive`), join and meet idempotence
/// (`merge_idempotent`/`meet_idempotent`), the coincident hull
/// (`span_with_self_is_coincident`), and the n-ary folds' adjacent
/// clone collapse — all zero scanned bits over operands that share
/// one buffer, while the same operations walk (nonzero) on a
/// concurrent pair, so the zeros are fast paths, not a dead meter.
#[test]
fn clone_operands_answer_without_a_walk() {
    let (v, _, w) = fixture();
    let c = v.clone();

    let cells: &[(&str, &dyn Fn())] = &[
        ("cmp", &|| assert!(v.partial_cmp(&c).is_some())),
        ("join", &|| assert_eq!(&(&v | &c), &v)),
        ("meet", &|| assert_eq!(&(&v & &c), &v)),
        ("span", &|| assert_eq!(v.span(&c).lo(), &v)),
        ("join_all", &|| {
            assert_eq!(v.join_all([&c, &v]), v);
        }),
        ("meet_all", &|| {
            assert_eq!(v.meet_all([&c, &v]), v);
        }),
        ("span_all", &|| {
            assert_eq!(v.span_all([&c, &v]).hi(), &v);
        }),
    ];
    for (name, cell) in cells {
        let read = scanned(cell);
        assert_eq!(
            read, 0,
            "{name} over clone operands must answer by clone identity, \
             not a walk ({read} bits scanned)"
        );
    }

    // The walking legs: the same operations on a concurrent pair
    // read nonzero, so the zeros above are fast paths firing, not a
    // dead scan meter.
    let walking: &[(&str, &dyn Fn())] = &[
        ("cmp", &|| assert!(v.partial_cmp(&w).is_none())),
        ("join", &|| assert!((&v | &w) >= v)),
        ("span", &|| assert!(v.span(&w).hi() >= v)),
    ];
    for (name, cell) in walking {
        let read = scanned(cell);
        assert!(
            read > 0,
            "{name} over a concurrent pair must walk: a zero here is a \
             dead scan meter"
        );
    }
}

/// Byte-equal operands in distinct buffers keep the walked paths
/// covered.
///
/// Comparison takes the full sweep (the clone-identity rung must
/// not fire across buffers), while the byte-compare rung answers
/// join/meet/span/distance/lag with no bit-stream walk — and every
/// verdict equals the clone-operand fast path's.
#[test]
fn distinct_buffers_keep_the_walked_paths_covered() {
    let (v, redecoded, _) = fixture();

    // The comparison sweep runs whole: equal streams survive both
    // directions to exhaustion, so the read is both streams' bits.
    let cmp = scanned(|| assert_eq!(v.partial_cmp(&redecoded), Some(core::cmp::Ordering::Equal)));
    assert!(
        cmp > 0,
        "cmp over byte-equal distinct buffers must take the sweep: \
         clone identity must not fire across buffers"
    );

    // The byte-compare rung (canonical_eq) answers the lattice ops:
    // no bit-stream walk, verdicts identical to the clone legs'.
    let byte_rung: &[(&str, &dyn Fn())] = &[
        ("join", &|| assert_eq!(&(&v | &redecoded), &v)),
        ("meet", &|| assert_eq!(&(&v & &redecoded), &v)),
        ("span", &|| assert_eq!(v.span(&redecoded).lo(), &v)),
    ];
    for (name, cell) in byte_rung {
        let read = scanned(cell);
        assert_eq!(
            read, 0,
            "{name} over byte-equal operands must answer by the byte \
             compare, not a walk ({read} bits scanned)"
        );
    }
}

/// The metric fast paths skip the fold at arithmetic width.
///
/// `distance` and `lag` over clone or byte-equal operands fold
/// nothing through the accumulator
/// (`distance_to_self_is_zero`/`lag_to_self_is_zero`), where the
/// same pair walked whole at the parent — and a real pair still
/// folds, so the zeros are the equality rung, not a dead touch
/// meter.
#[cfg(feature = "touch-meter")]
#[test]
fn metric_fast_paths_skip_the_fold() {
    let (v, redecoded, w) = fixture();
    let c = v.clone();
    let touches = |f: &dyn Fn()| {
        suanpan::touch_meter::reset();
        f();
        suanpan::touch_meter::touches()
    };

    let equal_cells: &[(&str, &dyn Fn())] = &[
        ("distance_clone", &|| {
            assert_eq!(v.distance(&c), before::Rank::ZERO)
        }),
        ("distance_redecoded", &|| {
            assert_eq!(v.distance(&redecoded), before::Rank::ZERO)
        }),
        ("lag_clone", &|| assert_eq!(v.lag(&c), before::Rank::ZERO)),
        ("lag_redecoded", &|| {
            assert_eq!(v.lag(&redecoded), before::Rank::ZERO)
        }),
    ];
    for (name, cell) in equal_cells {
        let read = touches(cell);
        assert_eq!(
            read, 0,
            "{name}: equal operands must skip the fold entirely \
             ({read} digit touches)"
        );
    }
    assert!(
        touches(&|| assert!(v.distance(&w) > before::Rank::ZERO)) > 0,
        "distance over a real pair must fold: a zero here is a dead \
         touch meter"
    );
}

/// An empty operand answers every lattice identity/absorption rung
/// without a walk.
///
/// The identity ladder's empty rungs — `v ∨ 0 = v` (no-op), `0 ∨ v = v` (adopt
/// the incoming stream wholesale, an `O(1)` refcount clone), `0 ∧ v = 0` / `v ∧
/// 0 = 0` (absorption), and the span forms — must all settle with zero scanned
/// bits, in both orders at every entry point: the operators and `|=`/`&=`
/// assigns (`0 |= v` is the seed pattern the fold accumulators hit on their
/// first join), the span entry points, and two-element folds (whose single
/// combine reads two untouched inputs). The walking control below proves the
/// zeros are fast paths firing, not a dead meter: without these rungs the
/// general emission walk produces byte-identical values (every value law stays
/// green), so only this scan pin witnesses the rungs' existence.
#[test]
fn empty_operands_answer_without_a_walk() {
    let (v, _, _) = fixture();
    let empty = Version::new();

    let cells: &[(&str, &dyn Fn())] = &[
        ("join_adopt", &|| assert_eq!(&(&empty | &v), &v)),
        ("join_noop", &|| assert_eq!(&(&v | &empty), &v)),
        ("meet_absorb_l", &|| assert_eq!(&(&empty & &v), &empty)),
        ("meet_absorb_r", &|| assert_eq!(&(&v & &empty), &empty)),
        ("span_l", &|| assert_eq!(empty.span(&v).hi(), &v)),
        ("span_r", &|| assert_eq!(v.span(&empty).hi(), &v)),
        ("join_view_seed", &|| {
            let mut acc = Version::new();
            acc |= &v;
            assert_eq!(&acc, &v);
        }),
        ("join_view_noop", &|| {
            let mut acc = v.clone();
            acc |= &empty;
            assert_eq!(&acc, &v);
        }),
        ("meet_view_absorb", &|| {
            let mut acc = v.clone();
            acc &= &empty;
            assert_eq!(&acc, &empty);
        }),
        ("meet_view_absorb_l", &|| {
            let mut acc = Version::new();
            acc &= &v;
            assert_eq!(&acc, &empty);
        }),
        // The fold entry points: a two-element fold's only combine reads
        // two untouched inputs rather than updating an owned result.
        ("join_fold_noop", &|| assert_eq!(&v.join_all([&empty]), &v)),
        ("join_fold_adopt", &|| assert_eq!(&empty.join_all([&v]), &v)),
        ("meet_fold_absorb_r", &|| {
            assert_eq!(&v.meet_all([&empty]), &empty)
        }),
        ("meet_fold_absorb_l", &|| {
            assert_eq!(&empty.meet_all([&v]), &empty)
        }),
    ];
    for (name, cell) in cells {
        let read = scanned(cell);
        assert_eq!(
            read, 0,
            "{name} over an empty operand must answer by the empty \
             rung, not a walk ({read} bits scanned)"
        );
    }

    // The walking control: join over a concurrent pair still walks,
    // so the zeros above are the rungs firing, not a dead meter.
    let (a, _, w) = fixture();
    assert!(
        scanned(|| assert!((&a | &w) >= a)) > 0,
        "join over a concurrent pair must walk: a zero here is a dead \
         scan meter"
    );
}
