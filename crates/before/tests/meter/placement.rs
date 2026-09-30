//! Relational scan checks for causal filtering and placement.
//!
//! The `causally` filter and placement walks' resource identities, stated
//! relationally against the pair comparison sweep so there is no constant to
//! re-pin: each fused walk must cost exactly its composition minus the saved
//! probe scans on full sweeps, degenerate to the pair sweep byte-for-byte on
//! one exhaustion-confirmed bound, and add verdict-driven exits the
//! composition never had.

use super::power_of_two;
use std::cmp::Ordering;

use before::causally;
use before::testing::meter;
use before::{Clock, Dominance, Endpoint, Placement, Precedence, Span, Version};

/// Scan bits of one closure run, on a fresh counter.
fn scanned(f: impl FnOnce()) -> u64 {
    meter::reset_scan_bits();
    f();
    meter::scan_bits()
}

/// The placement fixture: one clock's comparable snapshot chain
/// `s < v < e` (multi-party skylines via received sends, so the
/// streams have real structure), plus a divergent line for the
/// concurrent cases.
///
/// Returns `(s, v, e, div)` with `s < v < e`, `s <= div`, and
/// `div` concurrent to both `v` and `e`.
fn fixture() -> (Version, Version, Version, Version) {
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
    let s = main.version().clone();
    let mut diverged = main.fork();
    // The plateau above the word range makes the fixture exercise wide
    // gamma decoding.
    main.ticks(power_of_two(80));
    rounds(&mut main, 24);
    let v = main.version().clone();
    rounds(&mut main, 24);
    let e = main.version().clone();
    for _ in 0..24 {
        diverged.tick();
    }
    let div = diverged.version().clone();
    assert!(s < v && v < e, "the snapshot chain is strict");
    assert!(s <= div, "the divergent line extends the fork point");
    assert!(
        v.concurrent(&div) && e.concurrent(&div),
        "the lines diverge"
    );
    (s, v, e, div)
}

/// On a full sweep where no demand settles before
/// exhaustion), the fused membership walk scans exactly the
/// two-walk composition minus one probe scan — each stream decoded
/// once.
///
/// The probe sits below the hole and the ceiling, so the hole's
/// subtraction and the ceiling's containment both confirm only at
/// exhaustion. Stated relationally against the pair sweep on the
/// same operands (`cmp(p, p')` prices one probe scan as half its
/// reading, `p'` a buffer-distinct re-decode of `p`: a shared
/// buffer would answer by clone identity without a walk), so the
/// identity self-normalizes and no measured constant can rot.
#[test]
fn query_fused_walk_scans_each_stream_once() {
    let (s, v, e, _) = fixture();
    let query = causally::since(&v) & causally::before(&e);

    let fused = scanned(|| {
        assert!(!query.contains(&s));
    });
    let s_redecoded = Version::decode(&s.encode()[..]).expect("a stored stream re-decodes");
    let cmp_sv = scanned(|| assert!(s.partial_cmp(&v).is_some()));
    let cmp_se = scanned(|| assert!(s.partial_cmp(&e).is_some()));
    let cmp_ss = scanned(|| assert!(s.partial_cmp(&s_redecoded).is_some()));
    eprintln!(
        "MEASURED query_one_pass: fused={fused} composed={} probe_scan={} \
         encoded_bits: s={} v={} e={}",
        cmp_sv + cmp_se,
        cmp_ss / 2,
        s.encoded_bits(),
        v.encoded_bits(),
        e.encoded_bits(),
    );
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert_eq!(
        fused + cmp_ss / 2,
        cmp_sv + cmp_se,
        "the fused walk must cost the composition minus exactly one probe scan"
    );
}

/// Over a single stored bound, the membership walk is
/// the pair sweep.
///
/// A bound whose verdict confirms only at exhaustion reads scan
/// bits byte-identical to `partial_cmp` on the same operands,
/// ceiling demand and hole alike, while a bound whose verdict
/// refutes mid-walk answers at the first refuted direction, at or
/// strictly under the raw sweep (which must refute both
/// directions, or confirm one to exhaustion).
///
/// This is the identity the classifier conversions in `rumors` rest
/// on: a single-bound `contains` costs at most what the raw
/// comparison it replaces cost.
#[test]
fn query_single_bound_matches_the_pair_sweep() {
    let (_, v, e, div) = fixture();
    // Exhaustion-confirmed verdicts: the ceiling admits the probe,
    // the hole holds it — byte-identical to the sweep.
    let raw = scanned(|| {
        let _ = v.partial_cmp(&e);
    });
    let ceiling = scanned(|| {
        assert!(causally::Query::from(causally::before(&e)).contains(&v));
    });
    let hole = scanned(|| {
        assert!(!causally::since(&e).contains(&v));
    });
    eprintln!("MEASURED query_single_bound/exhaustion: raw={raw} ceiling={ceiling} hole={hole}");
    assert!(raw > 0, "a live scan meter reads nonzero on a real walk");
    assert_eq!(
        ceiling, raw,
        "an exhaustion-confirmed ceiling must scan exactly as the pair sweep"
    );
    assert_eq!(
        hole, raw,
        "an exhaustion-confirmed hole must scan exactly as the pair sweep"
    );

    // Refuted verdicts: the bail acts at the first refuted
    // direction, strictly under a raw sweep still confirming its
    // other direction.
    let raw = scanned(|| {
        let _ = e.partial_cmp(&v);
    });
    let ceiling_refuted = scanned(|| {
        assert!(!causally::Query::from(causally::before(&v)).contains(&e));
    });
    let hole_satisfied = scanned(|| {
        assert!(causally::since(&v).contains(&e));
    });
    eprintln!(
        "MEASURED query_single_bound/refuted: raw={raw} \
         ceiling_refuted={ceiling_refuted} hole_satisfied={hole_satisfied}"
    );
    assert!(
        ceiling_refuted < raw,
        "a refuted ceiling must bail before the raw sweep's domination confirm"
    );
    assert!(
        hole_satisfied < raw,
        "a satisfied hole must drop before the raw sweep's domination confirm"
    );

    // A concurrent bound refutes the watched direction no later
    // than the raw sweep refutes both.
    let raw = scanned(|| {
        let _ = v.partial_cmp(&div);
    });
    let concurrent = scanned(|| {
        assert!(!causally::Query::from(causally::before(&div)).contains(&v));
    });
    eprintln!("MEASURED query_single_bound/concurrent: raw={raw} ceiling={concurrent}");
    assert!(
        concurrent <= raw,
        "a concurrent ceiling must answer no later than the raw sweep"
    );
}

/// The composition's early exits survive the fusion, and
/// the fused walk stays strictly under the composition on both
/// concurrent cases.
///
/// Concurrent to the ceiling: the refuted containment answers at
/// the deciding interval. Concurrent to the hole: the hole is
/// satisfied and its stream dropped at the deciding interval while
/// the ceiling sweeps on — the two-walk composition's bail, minus
/// its second probe scan.
#[test]
fn query_early_exits_survive_the_fusion() {
    let (s, v, e, div) = fixture();

    // Concurrent to the ceiling.
    let query = causally::since(&s) & causally::before(&div);
    let fused = scanned(|| {
        assert!(!query.contains(&v));
    });
    let composed = scanned(|| {
        let _ = v.partial_cmp(&s);
        let _ = v.partial_cmp(&div);
    });
    eprintln!("MEASURED query_concurrent_ceiling: fused={fused} composed={composed}");
    assert!(
        fused < composed,
        "concurrent-to-ceiling: the fused walk ({fused}) must undercut the \
         composition ({composed})"
    );

    // Concurrent to the hole, within the ceiling.
    let top = &e | &div;
    let query = causally::since(&div) & causally::before(&top);
    let fused = scanned(|| {
        assert!(query.contains(&v));
    });
    let composed = scanned(|| {
        let _ = v.partial_cmp(&div);
        let _ = v.partial_cmp(&top);
    });
    eprintln!("MEASURED query_concurrent_hole: fused={fused} composed={composed}");
    assert!(
        fused < composed,
        "concurrent-to-hole: the dropped hole stream must keep the fused \
         walk ({fused}) under the composition ({composed})"
    );
}

/// One fused span pass decodes each stream once.
///
/// On a full sweep (every relation comparable) the fused span
/// placement scans exactly the two-comparison composition minus one
/// probe scan. The dominance coarsening never costs more: on a
/// probe dominating the whole span nothing refutes and the walk
/// is the placement walk to the bit, while on a merely contained
/// probe the end stream's refuted domination drops that cursor and
/// the coarser verdict reads strictly cheaper.
///
/// Stated relationally like the range walk's one-pass pin
/// (`cmp(v, v')` prices one probe scan as half its reading, `v'` a
/// buffer-distinct re-decode of `v`: a shared buffer would answer
/// by clone identity without a walk), so no measured constant can
/// rot.
#[test]
fn span_place_scans_each_stream_once() {
    let (s, v, e, _) = fixture();
    let span = Span::new(&s, &e).unwrap();

    let fused = scanned(|| {
        assert_eq!(span.place(&v), Placement::Between);
    });
    let dominance = scanned(|| {
        assert_eq!(span.dominance(&v), Dominance::Between);
    });
    let v_redecoded = Version::decode(&v.encode()[..]).expect("a stored stream re-decodes");
    let cmp_vs = scanned(|| assert!(v.partial_cmp(&s).is_some()));
    let cmp_ve = scanned(|| assert!(v.partial_cmp(&e).is_some()));
    let cmp_vv = scanned(|| assert!(v.partial_cmp(&v_redecoded).is_some()));
    eprintln!(
        "MEASURED span_one_pass: fused={fused} dominance={dominance} composed={} \
         probe_scan={}",
        cmp_vs + cmp_ve,
        cmp_vv / 2,
    );
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert_eq!(
        fused + cmp_vv / 2,
        cmp_vs + cmp_ve,
        "the fused span walk must cost the composition minus exactly one probe scan"
    );
    assert!(
        dominance < fused,
        "on a contained probe the end stream's refuted domination must drop \
         that cursor: dominance ({dominance}) under full resolution ({fused})"
    );

    // The mirrored coarsening on the same contained probe: the
    // start stream's refuted precedence drops that cursor, while
    // the membership walk — both required directions confirming
    // only at exhaustion — is the placement walk to the bit.
    let precedence = scanned(|| {
        assert_eq!(span.precedence(&v), Precedence::Between);
    });
    let contains = scanned(|| {
        assert!(span.contains(&v));
    });
    eprintln!("MEASURED span_one_pass_mirror: precedence={precedence} contains={contains}");
    assert!(
        precedence < fused,
        "on a contained probe the start stream's refuted precedence must drop \
         that cursor: precedence ({precedence}) under full resolution ({fused})"
    );
    assert_eq!(
        contains, fused,
        "with both membership directions confirming only at exhaustion, the \
         membership walk is the placement walk to the bit"
    );

    // A probe dominating the whole span refutes nothing on
    // either side: the dominance walk is the placement walk to the
    // bit.
    let whole = Span::new(&s, &v).unwrap();
    let place_whole = scanned(|| {
        assert_eq!(whole.place(&e), Placement::After);
    });
    let dominance_whole = scanned(|| {
        assert_eq!(whole.dominance(&e), Dominance::After);
    });
    eprintln!("MEASURED span_whole_sweep: place={place_whole} dominance={dominance_whole}");
    assert_eq!(
        dominance_whole, place_whole,
        "with nothing refuted, the dominance walk is the placement walk to the bit"
    );

    // Dually, a probe preceding the whole span refutes nothing on
    // either side: the precedence walk is the placement walk to
    // the bit.
    let ahead = Span::new(&v, &e).unwrap();
    let place_ahead = scanned(|| {
        assert_eq!(ahead.place(&s), Placement::Before);
    });
    let precedence_ahead = scanned(|| {
        assert_eq!(ahead.precedence(&s), Precedence::Before);
    });
    eprintln!("MEASURED span_whole_precede: place={place_ahead} precedence={precedence_ahead}");
    assert_eq!(
        precedence_ahead, place_ahead,
        "with nothing refuted, the precedence walk is the placement walk to the bit"
    );
}

/// The span walk's concurrency exits fire per
/// endpoint, and every concurrent case stays strictly under the
/// two-comparison composition.
///
/// Concurrent to both endpoints: the walk returns at the second
/// deciding interval. Concurrent to one endpoint: that endpoint's
/// cursor is dropped at its deciding interval (its stream is never
/// scanned further) while the other relation sweeps on.
#[test]
fn span_concurrent_exits_survive_the_fusion() {
    let (s, v, e, div) = fixture();
    let top = &e | &div;

    for (lo, hi, probe, verdict, case) in [
        // div is concurrent to both v and e: the early return.
        (&v, &e, &div, Placement::Concurrent(Endpoint::Both), "both"),
        // v is past s but concurrent to div: the hi-drop path.
        (&s, &div, &v, Placement::Concurrent(Endpoint::End), "end"),
        // v is concurrent to div but under div|e: the lo-drop path.
        (
            &div,
            &top,
            &v,
            Placement::Concurrent(Endpoint::Start),
            "start",
        ),
    ] {
        let span = Span::new(lo, hi).unwrap();
        let fused = scanned(|| {
            assert_eq!(span.place(probe), verdict);
        });
        // The two-comparison composition on the span's own
        // endpoints: the operands the fused walk actually replaces.
        let composed = scanned(|| {
            let _ = probe.partial_cmp(lo);
            let _ = probe.partial_cmp(hi);
        });
        eprintln!("MEASURED span_concurrent_{case}: fused={fused} composed={composed}");
        assert!(
            fused < composed,
            "concurrent-to-{case}: the fused span walk ({fused}) must undercut \
             the composition ({composed})"
        );
    }
}

/// The dominance query stops when the probe fails to dominate the start,
/// to dominate, on both failure classes.
///
/// A *concurrent* start refutes `lo <= probe` at its first opposing
/// interval — one interval before the pair sweep's two-flag
/// concurrency exit — so the walk returns strictly before full
/// resolution and strictly under the floor-first two-check shape it
/// replaces; against the old *first check alone* the earlier bail
/// buys back only part of the fused walk's end-stream prefix, so
/// that reading is printed, not bounded. A *dominating* start
/// (`probe < lo`, comparable) is where the bail changes class: the
/// old floor-first check could confirm `Greater` only at
/// exhaustion, while the single-flag refutation lands at the first
/// excess interval — strictly under even the first check.
#[test]
fn dominance_bails_at_the_refuted_start() {
    let (s, v, e, div) = fixture();

    // Case 1: the start is concurrent to the probe.
    let top = &e | &div;
    let span = Span::new(&div, &top).unwrap();
    let fused = scanned(|| {
        assert_eq!(span.dominance(&v), Dominance::Before);
    });
    let place = scanned(|| {
        assert_eq!(span.place(&v), Placement::Concurrent(Endpoint::Start));
    });
    // The two-check shape the dominance face replaces: compare the
    // start version against the probe (the floor-first check,
    // which exits at the concurrency), then check the end
    // version's containment in the probe's past (the second probe
    // decode the fusion ends).
    let first_check = scanned(|| {
        assert!(div.partial_cmp(&v).is_none());
    });
    let two_check = first_check
        + scanned(|| {
            assert!(!causally::before(&v).contains(&top));
        });
    eprintln!(
        "MEASURED dominance_bail_concurrent: fused={fused} place={place} \
         first_check={first_check} two_check={two_check}"
    );
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert!(
        fused < place,
        "the dominance bail ({fused}) must return before full resolution ({place})"
    );
    assert!(
        fused < two_check,
        "the dominance bail ({fused}) must undercut the two-check shape ({two_check})"
    );

    // Case 2: the start strictly dominates the probe.
    let span = Span::new(&v, &e).unwrap();
    let fused = scanned(|| {
        assert_eq!(span.dominance(&s), Dominance::Before);
    });
    let first_check = scanned(|| {
        assert_eq!(v.partial_cmp(&s), Some(Ordering::Greater));
    });
    let two_check = first_check
        + scanned(|| {
            assert!(!causally::before(&s).contains(&e));
        });
    eprintln!(
        "MEASURED dominance_bail_dominating: fused={fused} \
         first_check={first_check} two_check={two_check}"
    );
    assert!(
        fused < first_check,
        "on a dominating start the single-flag bail ({fused}) must undercut \
         even the floor-first check ({first_check}), which confirms Greater \
         only at exhaustion"
    );
    assert!(
        fused < two_check,
        "the dominance bail ({fused}) must undercut the two-check shape ({two_check})"
    );
}

/// The precedence query stops when the probe fails to precede the end,
/// to precede — the dominance bail, mirrored — on both failure
/// error classes.
///
/// A *concurrent* end refutes `probe <= hi` at its first opposing
/// interval — one interval before the pair sweep's two-flag
/// concurrency exit — so the walk returns strictly before full
/// resolution and strictly under the ceiling-first two-check shape
/// it replaces. A *preceded* end (`hi < probe`, comparable) is
/// where the bail changes class: the ceiling-first check could
/// confirm `Less` only at exhaustion, while the single-flag
/// refutation lands at the first excess interval — strictly under
/// even the first check.
#[test]
fn precedence_bails_at_the_refuted_end() {
    let (s, v, e, div) = fixture();

    // Case 1: the end is concurrent to the probe.
    let span = Span::new(&s, &div).unwrap();
    let fused = scanned(|| {
        assert_eq!(span.precedence(&v), Precedence::After);
    });
    let place = scanned(|| {
        assert_eq!(span.place(&v), Placement::Concurrent(Endpoint::End));
    });
    // The two-check shape the precedence face replaces: compare the
    // end version against the probe (the ceiling-first check, which
    // exits at the concurrency), then check the start version's
    // containment in the probe's causal future (the second probe
    // decode the fusion ends).
    let first_check = scanned(|| {
        assert!(div.partial_cmp(&v).is_none());
    });
    let two_check = first_check
        + scanned(|| {
            assert!(!causally::after(&v).contains(&s));
        });
    eprintln!(
        "MEASURED precedence_bail_concurrent: fused={fused} place={place} \
         first_check={first_check} two_check={two_check}"
    );
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert!(
        fused < place,
        "the precedence bail ({fused}) must return before full resolution ({place})"
    );
    assert!(
        fused < two_check,
        "the precedence bail ({fused}) must undercut the two-check shape ({two_check})"
    );

    // Case 2: the end strictly precedes the probe.
    let span = Span::new(&s, &v).unwrap();
    let fused = scanned(|| {
        assert_eq!(span.precedence(&e), Precedence::After);
    });
    let first_check = scanned(|| {
        assert_eq!(v.partial_cmp(&e), Some(Ordering::Less));
    });
    let two_check = first_check
        + scanned(|| {
            assert!(!causally::after(&e).contains(&s));
        });
    eprintln!(
        "MEASURED precedence_bail_preceded: fused={fused} \
         first_check={first_check} two_check={two_check}"
    );
    assert!(
        fused < first_check,
        "on a preceded end the single-flag bail ({fused}) must undercut \
         even the ceiling-first check ({first_check}), which confirms Less \
         only at exhaustion"
    );
    assert!(
        fused < two_check,
        "the precedence bail ({fused}) must undercut the two-check shape ({two_check})"
    );
}

/// The membership query stops at the first refuted
/// required direction, on either side.
///
/// A probe above the end refutes `probe <= hi` at its first excess
/// interval; a probe below the start refutes `lo <= probe` the
/// same way. Either bail answers strictly before the
/// full-resolution placement walk, which confirms its
/// `After`/`Before` verdict only at exhaustion.
#[test]
fn contains_bails_at_either_refuted_side() {
    let (s, v, e, _) = fixture();

    // Above the end: `probe <= hi` refuted mid-walk.
    let span = Span::new(&s, &v).unwrap();
    let fused = scanned(|| assert!(!span.contains(&e)));
    let place = scanned(|| {
        assert_eq!(span.place(&e), Placement::After);
    });
    eprintln!("MEASURED contains_bail_above: fused={fused} place={place}");
    assert!(fused > 0, "a live scan meter reads nonzero on a real walk");
    assert!(
        fused < place,
        "the membership bail ({fused}) must return before full resolution ({place})"
    );

    // Below the start: `lo <= probe` refuted mid-walk.
    let span = Span::new(&v, &e).unwrap();
    let fused = scanned(|| assert!(!span.contains(&s)));
    let place = scanned(|| {
        assert_eq!(span.place(&s), Placement::Before);
    });
    eprintln!("MEASURED contains_bail_below: fused={fused} place={place}");
    assert!(
        fused < place,
        "the membership bail ({fused}) must return before full resolution ({place})"
    );
}
