//! The coincident-span clone-identity rungs, held live by scan parity.
//!
//! `Span::place` and `Span::dominance` on a clone-coincident span must
//! read exactly the scan bits of the collapsed form they document
//! (one pairwise comparison / one containment), while the same verdicts
//! over coincident endpoints in *distinct* buffers take the fused
//! three-stream walk and read strictly more — so a lost rung and a dead
//! scan meter both read red.

#![cfg(feature = "scan-meter")]

use before::causally::{known_at, Dominance, Endpoint, Placement, Span};
use before::{meter, Clock, Version};

/// Scan bits of one closure run, on a fresh counter.
fn scanned(f: impl FnOnce()) -> u64 {
    meter::reset_scan_bits();
    f();
    meter::scan_bits()
}

/// The fixture: a multi-party version `v`, a probe `w` concurrent to it,
/// and a buffer-distinct byte-equal re-decode of `v` for the walking legs.
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
    (v, w, redecoded)
}

/// `Span::place` on a clone-coincident span reads exactly the collapsed
/// pairwise comparison's scan; coincident endpoints in distinct buffers
/// take the fused three-stream walk and read strictly more.
#[test]
fn coincident_place_collapses_to_the_pair_sweep() {
    let (v, w, redecoded) = fixture();
    // The hull of a version with its own clone stores one buffer twice.
    let coincident = v.span(&v.clone());
    // Byte-equal endpoints in distinct buffers: the rung must not fire.
    let distinct = Span::new(&v, &redecoded).expect("equal endpoints are a valid span");

    let collapsed = scanned(|| {
        assert_eq!(w.partial_cmp(&v), None);
    });
    assert!(
        collapsed > 0,
        "the pair sweep reads bits: a zero is a dead meter"
    );

    let fast = scanned(|| {
        assert_eq!(coincident.place(&w), Placement::Concurrent(Endpoint::Both));
    });
    assert_eq!(
        fast, collapsed,
        "place over a clone-coincident span must collapse to one pairwise \
         comparison ({fast} vs {collapsed} scanned bits)"
    );

    let walked = scanned(|| {
        assert_eq!(distinct.place(&w), Placement::Concurrent(Endpoint::Both));
    });
    assert!(
        walked > collapsed,
        "coincident endpoints in distinct buffers must take the fused \
         three-stream walk ({walked} vs {collapsed} scanned bits): the \
         clone rung must not fire across buffers"
    );
}

/// `Span::dominance` on a clone-coincident span reads exactly the
/// collapsed containment's scan; coincident endpoints in distinct
/// buffers take the fused walk and read strictly more.
#[test]
fn coincident_dominance_collapses_to_one_containment() {
    let (v, w, redecoded) = fixture();
    let coincident = v.span(&v.clone());
    let distinct = Span::new(&v, &redecoded).expect("equal endpoints are a valid span");

    let collapsed = scanned(|| {
        assert!(!known_at(&w).contains(&v));
    });
    assert!(
        collapsed > 0,
        "the containment reads bits: a zero is a dead meter"
    );

    let fast = scanned(|| {
        assert_eq!(coincident.dominance(&w), Dominance::Before);
    });
    assert_eq!(
        fast, collapsed,
        "dominance over a clone-coincident span must collapse to one \
         containment ({fast} vs {collapsed} scanned bits)"
    );

    let walked = scanned(|| {
        assert_eq!(distinct.dominance(&w), Dominance::Before);
    });
    // The fused walk's dominance early-exit can read *fewer* bits than
    // the collapsed containment on a refuting probe, so the walking leg
    // pins divergence, not direction: distinct buffers must not read
    // scan-identical to the collapsed form.
    assert_ne!(
        walked, collapsed,
        "coincident endpoints in distinct buffers must take the fused walk \
         ({walked} vs {collapsed} scanned bits): the clone rung must not \
         fire across buffers"
    );
    // The verdicts agree across the rung boundary.
    assert_eq!(coincident.dominance(&w), distinct.dominance(&w));
}
