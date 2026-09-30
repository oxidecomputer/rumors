//! Relational cost checks for fused span decoding.
//!
//! The fused span decode's resource identities, stated relationally against
//! the standalone component decode and the pair comparison on the same
//! operands so there is no constant to rot. `Span::decode` parses the first
//! component exactly as `Version::decode` does, then ONE admission walk
//! parses the second while validating dominance in the same pass; what the
//! fusion avoids is the second component's standalone parse — its stream scan,
//! payload decoding, and validation-height accumulator (dominance over a
//! canonical first component subsumes nonnegativity). Scan and touch tests cover
//! the two savings independently.

use super::power_of_two;
use before::testing::meter;
use before::{Clock, Span, Version};

/// Scan bits of one closure run, on a fresh counter.
fn scanned(f: impl FnOnce()) -> u64 {
    meter::reset_scan_bits();
    f();
    meter::scan_bits()
}

/// The wire fixture: two comparable snapshots `s < v` of one
/// multi-party history, their composite `[s, v]` wire bytes, and
/// the byte boundary where the second component begins.
///
/// Received sends give both streams real multi-party structure, so
/// every leg folds live deltas.
fn fixture() -> (Version, Version, Vec<u8>, usize) {
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
    // The plateau above the word range makes the second component exercise
    // wide gamma decoding.
    main.ticks(power_of_two(80));
    rounds(&mut main, 24);
    let v = main.version().clone();
    assert!(s < v, "the snapshot chain is strict");
    let bytes = Span::new(&s, &v).unwrap().encode();
    let boundary = s.encode().len();
    (s, v, bytes, boundary)
}

/// Fused span decoding scans the second component exactly once.
///
/// The composed decode + decode + compare shape sits strictly
/// above it, by exactly the second component's parse scan.
///
/// The fused reading equals the first decode plus comparison and is less
/// than decoding both components before comparing them.
#[test]
fn span_decode_scans_the_second_component_once() {
    let (s, v, bytes, boundary) = fixture();
    let fused = scanned(|| {
        let _ = Span::decode(&bytes[..]).expect("a canonical composite decodes");
    });
    let decode_lo = scanned(|| {
        let _ = Version::decode(&bytes[..boundary]).expect("the first component decodes");
    });
    let decode_hi = scanned(|| {
        let _ = Version::decode(&bytes[boundary..]).expect("the second component decodes");
    });
    let cmp = scanned(|| assert!(s < v));
    eprintln!(
        "MEASURED span_decode_scan: fused={fused} decode_lo={decode_lo} \
         decode_hi={decode_hi} cmp={cmp}"
    );
    assert!(
        fused > 0 && decode_hi > 0,
        "live scan meters read nonzero on real walks"
    );
    assert_eq!(
        fused,
        decode_lo + cmp,
        "the fused decode must scan exactly the first component's parse \
         plus one comparison sweep"
    );
    assert!(
        fused < decode_lo + decode_hi + cmp,
        "the fused decode must undercut the composed \
         decode + decode + compare shape ({fused} vs {})",
        decode_lo + decode_hi + cmp
    );
}

/// Fused span decoding omits the second validation-height accumulator.
///
/// Accumulator touches equal the first decode plus comparison and remain
/// below the composed sequence, whose second decode performs extra folds.
#[cfg(feature = "touch-meter")]
#[test]
fn span_decode_deletes_the_second_validation_accumulator() {
    let (s, v, bytes, boundary) = fixture();
    let touches = |f: &dyn Fn()| {
        suanpan::touch_meter::reset();
        f();
        suanpan::touch_meter::touches()
    };
    let fused = touches(&|| {
        let _ = Span::decode(&bytes[..]).expect("a canonical composite decodes");
    });
    let decode_lo = touches(&|| {
        let _ = Version::decode(&bytes[..boundary]).expect("the first component decodes");
    });
    let decode_hi = touches(&|| {
        let _ = Version::decode(&bytes[boundary..]).expect("the second component decodes");
    });
    let cmp = touches(&|| assert!(s < v));
    eprintln!(
        "MEASURED span_decode_touches: fused={fused} decode_lo={decode_lo} \
         decode_hi={decode_hi} cmp={cmp}"
    );
    assert!(
        fused > 0 && decode_hi > 0,
        "live touch meters read nonzero on real walks: the undercut margin \
         is the second validation's whole fold traffic"
    );
    assert_eq!(
        fused,
        decode_lo + cmp,
        "the fused decode must fold exactly the first component's \
         validation plus one comparison — no second height accumulator"
    );
    assert!(
        fused < decode_lo + decode_hi + cmp,
        "a parse-then-validate spelling reads the composed sum exactly \
         ({} here); the fusion must undercut it (read {fused})",
        decode_lo + decode_hi + cmp
    );
}
