use std::cmp::Ordering;

use proptest::prelude::*;

use super::*;
use crate::testing::bridge::{from_oracle_version, to_oracle_version};
use crate::testing::generators::arb_oracle_version;
use crate::Clock;

/// `a <= b` under the impl causal order (`None` means concurrent, so not
/// ordered).
fn le(a: &Version, b: &Version) -> bool {
    matches!(a.partial_cmp(b), Some(Ordering::Less | Ordering::Equal))
}
/// The span witness fixture: an alice chain `a[0] < ... < a[4]` and `b1`
/// concurrent to every version of it.
fn span_fixtures() -> ([Version; 5], Version) {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let chain = [(); 5].map(|()| alice.tick().clone());
    let b1 = bob.tick().clone();
    for v in &chain {
        assert!(v.concurrent(&b1), "the lines diverge");
    }
    (chain, b1)
}

/// Every one of the nine [`Placement`] verdicts on a constructed witness.
///
/// The five chain regions land on `[a2, a4]`, the coincident `At(Both)` on
/// `[a2, a2]`, and all three `Concurrent` payloads on spans whose endpoints
/// straddle the divergent line.
#[test]
fn span_place_places_every_witness() {
    let ([a1, a2, a3, a4, a5], b1) = span_fixtures();
    let genesis = Version::new();

    // The chain verdicts, on a proper span.
    let span = Span::new(&a2, &a4).unwrap();
    assert_eq!(span.place(&genesis), Placement::Before);
    assert_eq!(span.place(&a1), Placement::Before);
    assert_eq!(span.place(&a2), Placement::At(Endpoint::Start));
    assert_eq!(span.place(&a3), Placement::Between);
    assert_eq!(span.place(&a4), Placement::At(Endpoint::End));
    assert_eq!(span.place(&a5), Placement::After);
    // Concurrent to both endpoints of the same span.
    assert_eq!(span.place(&b1), Placement::Concurrent(Endpoint::Both));

    // Equality to one endpoint of a coincident span is equality to both: always
    // `At(Both)`, never `At(Start)` or `At(End)`.
    let coincident = Span::new(&a2, &a2).unwrap();
    assert_eq!(coincident.place(&a2), Placement::At(Endpoint::Both));

    // Concurrent to the start only: `hi = a2 | b1` dominates both lines, so `b1
    // ∥ a2` while `b1 < hi`.
    let top = &a2 | &b1;
    let straddling = Span::new(&a2, &top).unwrap();
    assert_eq!(
        straddling.place(&b1),
        Placement::Concurrent(Endpoint::Start)
    );

    // Concurrent to the end only: `a2 > a1` while `a2 ∥ a1 | b1`.
    let side_top = &a1 | &b1;
    let sideways = Span::new(&a1, &side_top).unwrap();
    assert_eq!(sideways.place(&a2), Placement::Concurrent(Endpoint::End));
}

/// Every [`Dominance`] verdict on the nine placement witnesses: the
/// coarsening's three fibers, each exercised through all its members.
#[test]
fn span_dominance_coarsens_every_witness() {
    let ([a1, a2, a3, a4, a5], b1) = span_fixtures();

    let span = Span::new(&a2, &a4).unwrap();
    // After: At(End), Placement::After, and the coincident At(Both).
    assert_eq!(span.dominance(&a4), Dominance::After);
    assert_eq!(span.dominance(&a5), Dominance::After);
    let coincident = Span::new(&a2, &a2).unwrap();
    assert_eq!(coincident.dominance(&a2), Dominance::After);
    // Between: At(Start), Placement::Between, Concurrent(End).
    assert_eq!(span.dominance(&a2), Dominance::Between);
    assert_eq!(span.dominance(&a3), Dominance::Between);
    let side_top = &a1 | &b1;
    let sideways = Span::new(&a1, &side_top).unwrap();
    assert_eq!(sideways.dominance(&a2), Dominance::Between);
    // Before: Placement::Before, Concurrent(Start), Concurrent(Both).
    assert_eq!(span.dominance(&a1), Dominance::Before);
    let top = &a2 | &b1;
    let straddling = Span::new(&a2, &top).unwrap();
    assert_eq!(straddling.dominance(&b1), Dominance::Before);
    assert_eq!(span.dominance(&b1), Dominance::Before);
}

/// Every [`Precedence`] verdict on the nine placement witnesses: the
/// coarsening's three fibers — [`Dominance`]'s, mirrored — each
/// exercised through all its members.
#[test]
fn span_precedence_coarsens_every_witness() {
    let ([a1, a2, a3, a4, a5], b1) = span_fixtures();

    let span = Span::new(&a2, &a4).unwrap();
    // Before: Placement::Before, At(Start), and the coincident At(Both).
    assert_eq!(span.precedence(&a1), Precedence::Before);
    assert_eq!(span.precedence(&a2), Precedence::Before);
    let coincident = Span::new(&a2, &a2).unwrap();
    assert_eq!(coincident.precedence(&a2), Precedence::Before);
    // Between: At(End), Placement::Between, Concurrent(Start).
    assert_eq!(span.precedence(&a4), Precedence::Between);
    assert_eq!(span.precedence(&a3), Precedence::Between);
    let top = &a2 | &b1;
    let straddling = Span::new(&a2, &top).unwrap();
    assert_eq!(straddling.precedence(&b1), Precedence::Between);
    // After: Placement::After, Concurrent(End), Concurrent(Both).
    assert_eq!(span.precedence(&a5), Precedence::After);
    let side_top = &a1 | &b1;
    let sideways = Span::new(&a1, &side_top).unwrap();
    assert_eq!(sideways.precedence(&a2), Precedence::After);
    assert_eq!(span.precedence(&b1), Precedence::After);
}

/// Membership on the nine placement witnesses: exactly the at-endpoint
/// and between placements are contained — every outside and concurrent
/// genre is not, the coincident segment included.
#[test]
fn span_contains_admits_every_witness() {
    let ([a1, a2, a3, a4, a5], b1) = span_fixtures();

    let span = Span::new(&a2, &a4).unwrap();
    // Within: both endpoints and the interior.
    assert!(span.contains(&a2));
    assert!(span.contains(&a3));
    assert!(span.contains(&a4));
    // Outside on the chain: below and above.
    assert!(!span.contains(&a1));
    assert!(!span.contains(&a5));
    // Beside: concurrent to both endpoints.
    assert!(!span.contains(&b1));
    // Beside one endpoint only: concurrent to the start, and to the end.
    let top = &a2 | &b1;
    let straddling = Span::new(&a2, &top).unwrap();
    assert!(!straddling.contains(&b1));
    let side_top = &a1 | &b1;
    let sideways = Span::new(&a1, &side_top).unwrap();
    assert!(!sideways.contains(&a2));
    // The coincident segment is one version: membership is equality.
    let coincident = Span::new(&a2, &a2).unwrap();
    assert!(coincident.contains(&a2));
    assert!(!coincident.contains(&a1));
    assert!(!coincident.contains(&a3));
    assert!(!coincident.contains(&b1));
}

/// The validating door admits exactly the ordered pairs: `lo <= hi` composes
/// (coincident included), while reversed and incomparable pairs are rejected
/// with `Crossed`.
#[test]
fn span_new_rejects_unordered_pairs() {
    let ([_, a2, _, a4, _], b1) = span_fixtures();
    assert!(Span::new(&a2, &a4).is_ok());
    assert!(Span::new(&a2, &a2).is_ok(), "coincident is ordered");
    assert_eq!(Span::new(&a4, &a2), Err(Crossed), "reversed crosses");
    assert_eq!(
        Span::new(&a2, &b1),
        Err(Crossed),
        "an incomparable pair bounds nothing"
    );
    assert_eq!(Span::new(&b1, &a2), Err(Crossed));
}

/// The constructor doors accept any ownership mix per endpoint (owned,
/// borrowed, and one of each) and build the same span from the same values.
#[test]
fn span_doors_accept_owned_and_borrowed_endpoints() {
    let ([_, a2, _, a4, _], _) = span_fixtures();
    let borrowed = Span::new(&a2, &a4).unwrap();
    let owned: Span<'static> = Span::new(a2.clone(), a4.clone()).unwrap();
    let mixed = Span::new(&a2, a4.clone()).unwrap();
    assert_eq!(owned, borrowed);
    assert_eq!(mixed, borrowed);
}

/// The coincident constructors build the span `[v, v]`, equal to the singleton
/// hull `v.span(&v)`.
///
/// `Span::at` on an owned or borrowed version and both `From` spellings agree:
/// both endpoints are the version, and the pair is clone-identity certified
/// coincident.
#[test]
fn at_builds_the_coincident_span() {
    let ([_, a2, ..], _) = span_fixtures();
    let point: Span<'static> = Span::at(a2.clone());
    assert_eq!((point.lo(), point.hi()), (&a2, &a2));
    assert!(point.is_coincident(), "clone identity certifies the point");
    assert_eq!(point, a2.span(&a2), "the singleton hull, exactly");

    let lent = Span::at(&a2);
    assert!(lent.is_coincident(), "a lent pair reads one buffer");
    assert_eq!(lent, point);

    let consumed = Span::from(a2.clone());
    let borrowed = Span::from(&a2);
    assert!(consumed.is_coincident() && borrowed.is_coincident());
    assert_eq!(consumed, point);
    assert_eq!(borrowed, point);
}

/// The deriving doors on every input genre: the receiver keeps the hull total,
/// and every genre yields its tightest containing span.
///
/// An empty iterator's hull is the coincident `[self, self]`; a comparable
/// pair's is its validated span from either operand order (binary and n-ary
/// alike); a concurrent pair's is a hull whose fresh endpoints strictly bracket
/// both inputs; and owned items feed the n-ary door as references do.
#[test]
fn span_derives_the_hull() {
    let ([a1, a2, _, _, _], b1) = span_fixtures();

    // The empty iterator: the receiver alone, the coincident span.
    assert_eq!(
        a1.span_all(Vec::<&Version>::new()),
        Span::new(&a1, &a1).unwrap()
    );
    // Comparable pairs: the hull is the flip repair, both operand orders,
    // binary and n-ary alike.
    let flat = Span::new(&a1, &a2).unwrap();
    assert_eq!(a1.span(&a2), flat);
    assert_eq!(a2.span(&a1), flat);
    assert_eq!(a1.span_all([&a2]), flat);
    assert_eq!(a2.span_all([&a1]), flat);
    // A concurrent pair has no reordering, but it has a hull: both inputs sit
    // strictly inside it.
    let hull = a2.span(&b1);
    assert_eq!(hull.place(&a2), Placement::Between);
    assert_eq!(hull.place(&b1), Placement::Between);
    // Owned items feed the n-ary door (the Borrow calling convention).
    assert_eq!(a1.span_all([a2.clone()]), flat);
}
proptest! {
    /// The span gate's family claim, differentially against `partial_cmp`:
    /// `Span::new` admits exactly the pairs the causal order deems ordered, and
    /// on every admitted pair the trusted door builds the identical span.
    #[test]
    fn span_gate_admits_exactly_the_ordered(
        lo in arb_oracle_version(),
        hi in arb_oracle_version(),
    ) {
        let lo = from_oracle_version(&lo);
        let hi = from_oracle_version(&hi);
        let admitted = Span::new(&lo, &hi);
        prop_assert_eq!(
            admitted.is_ok(),
            le(&lo, &hi),
            "the gate admits exactly the ordered pairs"
        );
    }
}
// ───────────────────────────── the span wire form ─────────────────────────────

/// Committed witnesses, one per rejection genre the span wire decode can reach.
///
/// A strictly crossed pair, a concurrent pair (both orders), a non-canonical
/// component on each side of the seam, truncation at every byte boundary
/// (inside the meet, at the seam with the join missing entirely, inside the
/// join), a trailing byte after the complete composite, and a set padding bit
/// inside each component's final byte.
#[test]
fn span_decode_rejects_each_genre() {
    use crate::error::Decode;
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let older = alice.tick().clone();
    let newer = alice.tick().clone();
    let beside = bob.tick().clone(); // concurrent to alice's whole line
    let bytes = Span::new(&older, &newer).unwrap().encode();
    let lo_len = older.encode().len();

    // The accepted baseline the witnesses below are each one defect away from.
    assert_eq!(
        Span::decode(&bytes[..]).unwrap(),
        Span::new(&older, &newer).unwrap()
    );

    // Strictly crossed: the join strictly below the meet.
    let crossed = [newer.encode(), older.encode()].concat();
    assert!(
        matches!(Span::decode(&crossed[..]), Err(Decode::NotCanonical)),
        "a strictly crossed pair is the canonical spelling of no span"
    );

    // Concurrent: neither component bounds the other, in both orders.
    for (a, b) in [(&older, &beside), (&beside, &older)] {
        let concurrent = [a.encode(), b.encode()].concat();
        assert!(
            matches!(Span::decode(&concurrent[..]), Err(Decode::NotCanonical)),
            "a concurrent pair is the canonical spelling of no span"
        );
    }

    // Truncation at every byte boundary. Every cut lands mid-tree: a
    // component's final byte always carries live bits (encode pads only to the
    // next byte boundary), so no proper byte prefix parses whole.
    assert!(
        matches!(Span::decode(&[][..]), Err(Decode::Truncated)),
        "empty input"
    );
    for cut in 1..bytes.len() {
        let genre = match cut.cmp(&lo_len) {
            Ordering::Less => "inside the meet",
            Ordering::Equal => "at the seam: the join missing entirely",
            Ordering::Greater => "inside the join",
        };
        assert!(
            matches!(Span::decode(&bytes[..cut]), Err(Decode::Truncated)),
            "cut at byte {cut} ({genre})"
        );
    }

    // Live bits past the complete composite.
    let trailing = [bytes.clone(), vec![0]].concat();
    assert!(
        matches!(Span::decode(&trailing[..]), Err(Decode::TrailingBits)),
        "trailing zero byte"
    );

    // A set padding bit inside each component's final byte. Both witnesses must
    // end mid-byte for the padding to exist at all.
    assert_ne!(
        older.encoded_bits() % 8,
        0,
        "the meet witness ends mid-byte"
    );
    assert_ne!(
        newer.encoded_bits() % 8,
        0,
        "the join witness ends mid-byte"
    );
    let mut meet_padding = bytes.clone();
    meet_padding[lo_len - 1] |= 0x01;
    assert!(
        matches!(Span::decode(&meet_padding[..]), Err(Decode::TrailingBits)),
        "set bit in the meet's padding"
    );
    let mut join_padding = bytes.clone();
    *join_padding.last_mut().unwrap() |= 0x01;
    assert!(
        matches!(Span::decode(&join_padding[..]), Err(Decode::TrailingBits)),
        "set bit in the join's padding"
    );

    // A non-canonical component on each side of the seam: an internal node
    // whose two leaf children carry height 0 and delta 0 — the collapsible
    // sibling pair minimal topology forbids. As a *join* it denotes the empty
    // version, so it dominates an empty meet and only canonicality can reject
    // it — which is exactly the check the fused walk must not lose.
    let collapsible: Vec<u8> = vec![0b0111_1000];
    assert!(
        matches!(Version::decode(&collapsible[..]), Err(Decode::NotCanonical)),
        "the component witness is itself non-canonical"
    );
    let empty = Version::new().encode();
    let meet_noncanon = [collapsible.clone(), empty.clone()].concat();
    assert!(
        matches!(Span::decode(&meet_noncanon[..]), Err(Decode::NotCanonical)),
        "non-canonical meet"
    );
    let join_noncanon = [empty, collapsible].concat();
    assert!(
        matches!(Span::decode(&join_noncanon[..]), Err(Decode::NotCanonical)),
        "non-canonical join"
    );
}

/// FUSED-VALIDATE VERDICT IDENTITY, exhaustively at small scope.
///
/// Over every ordered pair of normal-form event trees to the committed depth
/// bound, the fused wire decode of `lo.encode() ++ hi.encode()` agrees with the
/// composed form — decode each component, then validate with `Span::new` —
/// accepting exactly the same composites, producing the same span on every
/// accept, and rejecting every crossed or concurrent pair as `NotCanonical`.
/// The corpus reaches ordered, reversed, coincident, and concurrent pairs by
/// brute force, and the liveness floors prove both verdicts fired at scale.
#[test]
fn span_decode_verdict_matches_the_composed_form_exhaustively() {
    use crate::error::Decode;
    use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
    let corpus: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let encodings: Vec<Vec<u8>> = corpus.iter().map(Version::encode).collect();
    let (mut accepted, mut rejected) = (0u64, 0u64);
    for (lo, lo_bytes) in corpus.iter().zip(&encodings) {
        for (hi, hi_bytes) in corpus.iter().zip(&encodings) {
            let composite = [lo_bytes.as_slice(), hi_bytes.as_slice()].concat();
            let fused = Span::decode(&composite[..]);
            match Span::new(lo, hi) {
                Ok(span) => {
                    accepted += 1;
                    match fused {
                        Ok(decoded) => assert_eq!(
                            decoded, span,
                            "the fused decode's accept is the composed span"
                        ),
                        Err(e) => {
                            panic!("fused decode must accept the ordered pair [{lo}, {hi}]: {e}")
                        }
                    }
                }
                Err(Crossed) => {
                    rejected += 1;
                    assert!(
                        matches!(fused, Err(Decode::NotCanonical)),
                        "fused decode must reject the unordered pair [{lo}, {hi}] as NotCanonical"
                    );
                }
            }
        }
    }
    // Liveness: both verdicts fire, at scale.
    assert!(accepted > 1_000, "acceptance is live: {accepted}");
    assert!(rejected > 1_000, "rejection is live: {rejected}");
}

proptest! {
    /// FUSED-VALIDATE VERDICT IDENTITY over arbitrary pairs.
    ///
    /// The fused wire decode of `a.encode() ++ b.encode()` agrees with the
    /// composed form (decode each component, then `Span::new`) on both
    /// verdicts, and on every accept the two forms produce the same span.
    #[test]
    fn span_decode_verdict_matches_the_composed_form(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        use crate::error::Decode;
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let composite = [a.encode(), b.encode()].concat();
        let fused = Span::decode(&composite[..]);
        match Span::new(&a, &b) {
            Ok(span) => match fused {
                Ok(decoded) => prop_assert_eq!(
                    decoded, span,
                    "the fused decode's accept is the composed span"
                ),
                Err(e) => return Err(TestCaseError::fail(format!(
                    "fused decode must accept the ordered pair [{a}, {b}]: {e}"
                ))),
            },
            Err(Crossed) => prop_assert!(
                matches!(fused, Err(Decode::NotCanonical)),
                "fused decode must reject the unordered pair [{a}, {b}] as NotCanonical"
            ),
        }
    }

    /// The span composite is prefix-free: distinct spans' encodings are never
    /// byte prefixes of one another.
    ///
    /// Pinned directly on the composite (it rides the components' committed
    /// prefix-freedom, but the pin is on the composite itself, never inferred).
    /// Prefix-freedom is what lets one composite self-delimit inside a larger
    /// stream: the borsh leg reads exactly one span and leaves the next field's
    /// bytes unread.
    #[test]
    fn span_encoding_is_prefix_free(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        oc in arb_oracle_version(),
        od in arb_oracle_version(),
    ) {
        let x = from_oracle_version(&oa).span(&from_oracle_version(&ob));
        let y = from_oracle_version(&oc).span(&from_oracle_version(&od));
        if x != y {
            let (ex, ey) = (x.encode(), y.encode());
            prop_assert!(
                !ex.starts_with(&ey) && !ey.starts_with(&ex),
                "prefix-free: {:02x?} vs {:02x?}", ex, ey
            );
        }
    }
}

/// Structural genres outrank the pair verdict on multiply-defective composites,
/// exactly as decoding the components would order them.
///
/// Each witness stacks a second defect on a composite the pair relation already
/// rejects, and the structural genre wins: a set padding bit or a spurious
/// trailing byte after a crossed join is `TrailingBits`, a cut after an early
/// refutation is `Truncated` — never the pair rejection's `NotCanonical`. The
/// negative-height witnesses pin the admission walk's subsumption seam: a whole
/// negative-height join rejects `NotCanonical` (the same genre the standalone
/// validator gives those bytes), and a negative-height join that is *also*
/// truncated rejects `Truncated` — the one deliberate divergence from
/// component-wise decoding, which reports the height dip it meets first; the
/// fused walk carries no height accumulator, so the whole-parse rule decides
/// instead.
#[test]
fn span_decode_structural_genres_outrank_the_pair_verdict() {
    use crate::error::Decode;
    let mut clock = Clock::seed();
    let one = clock.tick().clone();
    let empty = Version::new();

    // The empty version's canonical byte: leaf flag `1`, gamma(0) `1`,
    // the padding marker, five zero padding bits.
    assert_eq!(empty.encode(), vec![0xE0]);
    // A negative-height stream: root internal `0`, left leaf `1` with
    // absolute height gamma(0) `1`, right leaf `1` with delta
    // zigzag(-1) `010`, then the padding marker — 0b0111_0101. Its
    // running height dips to -1, which only canonicality rejects.
    let neg = vec![0x75];
    assert!(matches!(
        Version::decode(&neg[..]),
        Err(Decode::NotCanonical)
    ));

    // A whole negative-height join over the empty meet: the join never
    // dominates (its dip sits below the meet's zero), so the admission verdict
    // subsumes the height check under the same genre.
    let composite = [empty.encode(), neg].concat();
    assert!(
        matches!(Span::decode(&composite[..]), Err(Decode::NotCanonical)),
        "a whole negative-height join rejects as the validator would"
    );

    // The same stream cut before its right subtree: 0b0011_1010 parses root
    // internal, left-inner leaf height 0, then delta zigzag(-1) — the dip — and
    // then runs out of bits. The structural genre wins.
    let truncated_neg = [empty.encode(), vec![0x3A]].concat();
    assert!(
        matches!(Span::decode(&truncated_neg[..]), Err(Decode::Truncated)),
        "truncation outranks the refuted pair verdict"
    );

    // A crossed pair (join strictly below the meet) with a set padding
    // bit after the join's marker: the padding defect wins.
    let crossed_padding = [one.encode(), vec![0xE4]].concat();
    assert!(
        matches!(
            Span::decode(&crossed_padding[..]),
            Err(Decode::TrailingBits)
        ),
        "malformed padding outranks the refuted pair verdict"
    );

    // A crossed pair with a spurious all-zero byte after the join: the
    // composite re-encoding shorter than its input is the same
    // trailing-bits genre.
    let crossed_trailing = [one.encode(), vec![0xE0, 0x00]].concat();
    assert!(
        matches!(
            Span::decode(&crossed_trailing[..]),
            Err(Decode::TrailingBits)
        ),
        "a trailing zero byte outranks the refuted pair verdict"
    );

    // A crossed pair whose join is also cut mid-tree: refutation is decided
    // early, and the walk still parses to the cut.
    let taller = {
        let mut main = Clock::seed();
        let mut other = main.fork();
        other.tick();
        main.recv(other.send());
        main.tick();
        main.version().clone()
    };
    let join = {
        let mut main = Clock::seed();
        let mut other = main.fork();
        other.tick();
        main.recv(other.send());
        main.version().clone()
    };
    let join_bytes = join.encode();
    let crossed_truncated = [taller.encode(), join_bytes[..join_bytes.len() - 1].to_vec()].concat();
    assert!(
        matches!(Span::decode(&crossed_truncated[..]), Err(Decode::Truncated)),
        "truncation outranks the refuted pair verdict"
    );
}

/// Structural genres outrank the coincident-pair verdict: a composite whose
/// join stream byte-equals its meet still rejects by its structural defect,
/// never silently dedups.
///
/// The admission walk's `Equal` verdict is what dispatches the coincident
/// span's storage dedup, and it is pronounced only after the join's padding
/// check — so a coincident composite carrying a set padding bit or a spurious
/// trailing zero byte is `TrailingBits`, and one whose byte-equal join is cut
/// mid-tree is `Truncated`: the same precedence the crossed-pair witnesses pin,
/// exercised through the dedup-dispatching arm.
#[test]
fn span_decode_structural_genres_outrank_the_coincident_verdict() {
    use crate::error::Decode;
    let mut main = Clock::seed();
    let mut other = main.fork();
    other.tick();
    main.recv(other.send());
    main.tick();
    let v = main.version().clone();
    let bytes = v.encode();
    assert!(
        bytes.len() > 1 && !v.encoded_bits().is_multiple_of(8),
        "the witness needs a multi-byte stream with live padding bits"
    );

    // The clean coincident composite accepts (the dedup baseline).
    let coincident = [bytes.clone(), bytes.clone()].concat();
    let span = Span::decode(&coincident[..]).expect("the coincident composite decodes");
    assert!(span.lo().view().ptr_eq(span.hi().view()));

    // A set padding bit in the byte-equal join's final byte: the padding defect
    // wins over the Equal verdict.
    let mut padded = coincident.clone();
    *padded.last_mut().expect("nonempty") |= 0x01;
    assert!(
        matches!(Span::decode(&padded[..]), Err(Decode::TrailingBits)),
        "malformed padding outranks the coincident verdict"
    );

    // A spurious all-zero byte after the byte-equal join: the same trailing
    // genre.
    let trailing = [coincident.clone(), vec![0x00]].concat();
    assert!(
        matches!(Span::decode(&trailing[..]), Err(Decode::TrailingBits)),
        "a trailing zero byte outranks the coincident verdict"
    );

    // The byte-equal join cut mid-tree: truncation wins, though every
    // byte read so far matched the meet exactly.
    let truncated = [bytes.clone(), bytes[..bytes.len() - 1].to_vec()].concat();
    assert!(
        matches!(Span::decode(&truncated[..]), Err(Decode::Truncated)),
        "truncation outranks the coincident verdict"
    );
}

/// FUSED-VALIDATE VERDICT IDENTITY beyond the exhaustive corpus's reach: deep
/// spines, wide fans, and payload magnitudes at and past the machine word, on
/// both verdicts.
///
/// The small-scope sweep is exhaustive to depth 2; these constructed families
/// sample the genres it cannot contain — 300-level spines (deep path stacks,
/// long unary runs), 1024-leaf fans (maximal-width plateaus), absolute heights
/// above `2^64` (payload codes past the decoder's word window), and heights at
/// the 63/64-bit sign edges of the zigzag map — and check the fused decode
/// against the composed form (decode, decode, `Span::new`) on accept, reject,
/// and the decoded span itself, for the pair, its hulls, and the coincident
/// span.
#[test]
fn span_decode_verdict_matches_the_composed_form_off_corpus() {
    use crate::error::Decode;
    use crate::oracle;

    fn composed(bytes: &[u8], seam: usize) -> Result<Span<'static>, Decode> {
        let lo = Version::decode(&bytes[..seam])?;
        let hi = Version::decode(&bytes[seam..])?;
        Span::new(&lo, &hi)
            .map(|s| s.into_owned())
            .map_err(|Crossed| Decode::NotCanonical)
    }

    fn check_identity(lo: &Version, hi: &Version) {
        let lo_bytes = lo.encode();
        let seam = lo_bytes.len();
        let composite = [lo_bytes, hi.encode()].concat();
        let fused = Span::decode(&composite[..]);
        match (fused, composed(&composite, seam)) {
            (Ok(f), Ok(c)) => {
                assert_eq!(f, c, "accept identity for [{lo}, {hi}]");
                assert_eq!(f.encode(), composite, "re-encode identity");
            }
            (Err(ef), Err(ec)) => assert_eq!(
                std::mem::discriminant(&ef),
                std::mem::discriminant(&ec),
                "genre identity for [{lo}, {hi}]: fused {ef:?}, composed {ec:?}"
            ),
            (f, c) => panic!("verdict mismatch for [{lo}, {hi}]: fused {f:?}, composed {c:?}"),
        }
    }

    // A left-descending spine: every level one internal node whose right child
    // is a leaf.
    let spine = |depth: usize, bump: u64| {
        let mut t = oracle::Version::leaf(0u64);
        for i in 0..depth {
            let i = i as u64;
            t = oracle::Version::node(i % 7 + bump, t, oracle::Version::leaf(i % 3));
        }
        t
    };
    // A complete tree: `2^depth` leaves with mixed heights.
    fn fan(depth: usize, salt: u64) -> oracle::Version {
        fn go(d: usize, ix: u64, salt: u64) -> oracle::Version {
            if d == 0 {
                oracle::Version::leaf(ix.wrapping_mul(2654435761).wrapping_add(salt) % 5)
            } else {
                oracle::Version::node(ix % 2, go(d - 1, ix * 2, salt), go(d - 1, ix * 2 + 1, salt))
            }
        }
        go(depth, 1, salt)
    }
    // Nested `u64::MAX` bases: absolute heights above `2^64`, so the payload
    // gamma codes outgrow the decoder's word window.
    let giant = |extra: u64| {
        oracle::Version::node(
            u64::MAX,
            oracle::Version::node(
                u64::MAX,
                oracle::Version::leaf(extra),
                oracle::Version::leaf(0u64),
            ),
            oracle::Version::leaf(1u64),
        )
    };
    // One height at a chosen bit edge beside a zero leaf.
    let bit_edge =
        |h: u64| oracle::Version::node(0u64, oracle::Version::leaf(h), oracle::Version::leaf(0u64));

    let shapes = [
        spine(300, 0),
        spine(300, 1),
        spine(120, 3),
        fan(10, 0),
        fan(10, 9),
        fan(6, 4),
        giant(0),
        giant(5),
        bit_edge((1u64 << 63) - 1),
        bit_edge(1u64 << 63),
        bit_edge((1u64 << 62) + 1),
        bit_edge(u64::MAX),
        oracle::Version::leaf(0u64),
    ];
    let versions: Vec<Version> = shapes.iter().map(from_oracle_version).collect();
    for a in &versions {
        for b in &versions {
            // The raw pair (ordered, crossed, or concurrent), the hull against
            // each operand (always ordered), and the coincident span.
            check_identity(a, b);
            let hull = a.span(b);
            check_identity(a, hull.hi());
            check_identity(hull.lo(), b);
            check_identity(a, a);
        }
    }
}

proptest! {
    /// A single-bit mutation of a valid composite never aliases it.
    ///
    /// The mutated bytes are rejected, or they decode to a *different* span
    /// whose endpoint values, re-derived through the oracle bridge into a
    /// fresh composite, encode exactly the mutated bytes — the span-level face
    /// of the components' mutation sweeps, crossing the seam and both padding
    /// regions that only the composite has. The re-derivation is the accept
    /// side's whole strength: decode adopts accepted bytes and `Eq` is byte
    /// equality, so only an independently rebuilt composite can convict an
    /// admission walk that accepted a non-canonical spelling.
    #[test]
    fn span_single_bit_mutations_never_alias(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        flip_seed in any::<prop::sample::Index>(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let span = a.span(&b);
        let mut bytes = span.encode();
        let flip = flip_seed.index(bytes.len() * 8);
        bytes[flip / 8] ^= 0x80 >> (flip % 8);
        match Span::decode(&bytes[..]) {
            Err(_) => {}
            Ok(mutant) => {
                prop_assert_ne!(
                    &mutant, &span,
                    "a single-bit mutation decoded back to the same span: \
                     two spellings of one value were both accepted"
                );
                // The admitted endpoints dominate (`lo <= hi`), so their hull
                // is exactly `(lo, hi)` rebuilt from scratch.
                let canon = from_oracle_version(&to_oracle_version(mutant.lo()))
                    .span(&from_oracle_version(&to_oracle_version(mutant.hi())));
                prop_assert_eq!(
                    canon.encode(), bytes,
                    "an accepted composite must be the canonical encoding of its span: \
                     the oracle re-derivation encodes it differently"
                );
            }
        }
    }
}

// ─────────────────── the coincident span's storage dedup ───────────────────

/// A wire-loaded coincident span stores one buffer twice.
///
/// The admission walk detects `hi == lo` in the pass that proves dominance, so
/// the decoded endpoints share storage (clone identity holds) exactly as a
/// computed coincident hull's do — and the span still re-encodes
/// byte-identically and equals the computed form.
#[test]
fn decoded_coincident_span_shares_one_buffer() {
    let mut clock = Clock::seed();
    for _ in 0..12 {
        clock.tick();
    }
    let v = clock.version().clone();
    let computed = v.span(&v);
    assert!(
        computed.lo().view().ptr_eq(computed.hi().view()),
        "a computed coincident hull stores one buffer twice"
    );

    let bytes = computed.encode();
    let decoded = Span::decode(&bytes[..]).expect("a canonical composite decodes");
    assert_eq!(decoded, computed, "the wire round-trips the span");
    assert_eq!(decoded.encode(), bytes, "re-encoding is byte-identical");
    assert!(
        decoded.lo().view().ptr_eq(decoded.hi().view()),
        "the decode-fused equality must dedup the coincident span's storage: \
         wire-loaded spans hit the ptr_eq ladder exactly like computed ones"
    );
}

/// A strictly-dominating wire span keeps two distinct endpoint streams: the
/// dedup fires only on coincidence.
///
/// Both endpoints adopt slices of the one read buffer (no per-endpoint copy),
/// observable as the decode reproducing both components byte-for-byte.
#[test]
fn decoded_strict_span_keeps_distinct_endpoints() {
    let mut clock = Clock::seed();
    let lo = clock.tick().clone();
    let hi = clock.tick().clone();
    let span = Span::new(&lo, &hi).expect("ordered");
    let decoded = Span::decode(&span.encode()[..]).expect("a canonical composite decodes");
    assert!(
        !decoded.lo().view().ptr_eq(decoded.hi().view()),
        "distinct endpoints must not read as clones"
    );
    assert_eq!(decoded.lo(), &lo);
    assert_eq!(decoded.hi(), &hi);
}

proptest! {
    /// The coincident span's fast rungs agree with the fused walk across buffer
    /// identity.
    ///
    /// `place`, `dominance`, `precedence`, and `contains` against `[v, v]`
    /// return identical verdicts whether the endpoints share one buffer (the
    /// clone-identity rung: hull doors, wire decode) or sit in distinct
    /// byte-equal buffers (the fused three-stream walk), and place transcribes
    /// `probe.partial_cmp(v)` exactly — the
    /// `degenerate_span_place_is_partial_cmp` law's table.
    #[test]
    fn coincident_span_rungs_agree_across_buffer_identity(
        ov in arb_oracle_version(),
        op in arb_oracle_version(),
    ) {
        let v = from_oracle_version(&ov);
        let probe = from_oracle_version(&op);
        let shared = v.span(&v); // one buffer, two clones
        let distinct_v = from_oracle_version(&ov);
        let distinct = Span::new(&v, &distinct_v).expect("equal versions are ordered");
        prop_assert_eq!(shared.place(&probe), distinct.place(&probe));
        prop_assert_eq!(shared.dominance(&probe), distinct.dominance(&probe));
        prop_assert_eq!(shared.precedence(&probe), distinct.precedence(&probe));
        prop_assert_eq!(shared.contains(&probe), distinct.contains(&probe));
        let expected = match probe.partial_cmp(&v) {
            Some(Ordering::Less) => Placement::Before,
            Some(Ordering::Equal) => Placement::At(Endpoint::Both),
            Some(Ordering::Greater) => Placement::After,
            None => Placement::Concurrent(Endpoint::Both),
        };
        prop_assert_eq!(shared.place(&probe), expected);
    }
}

// ───────────────────────────── the span algebra ─────────────────────────────

/// Every containment verdict on constructed witnesses: union covers,
/// intersection is the overlap, and disjoint segments intersect to `None`.
///
/// The matrix laws quantify the operators across every owned/borrowed cell; the
/// witness pins one readable instance.
#[test]
fn containment_operators_on_chain_witnesses() {
    let ([a1, a2, a3, a4, _], b1) = span_fixtures();

    let head = Span::new(&a1, &a2).unwrap();
    let tail = Span::new(&a2, &a4).unwrap();
    let mid = Span::new(&a2, &a3).unwrap();

    // Union covers both segments end to end.
    assert_eq!(&head + &tail, Span::new(&a1, &a4).unwrap());
    // Overlapping segments intersect at their shared segment.
    assert_eq!(&head * &tail, Some(Span::new(&a2, &a2).unwrap()));
    assert_eq!(&mid * &tail, Some(mid.clone()));
    // Disjoint segments share no version.
    assert_eq!(&head * &Span::new(&a3, &a4).unwrap(), None);
    // A concurrent point beside the chain still has a union (the hull)
    // and an empty intersection.
    let beside = Span::new(&b1, &b1).unwrap();
    let hull = &mid + &beside;
    assert_eq!(*hull.lo(), &a2 & &b1);
    assert_eq!(*hull.hi(), &a3 | &b1);
    assert_eq!(&mid * &beside, None);
}

/// The pointwise operators restrict to the version operators on coincident
/// spans, and their point results stay coincident with one shared buffer.
///
/// The fused point-combine's `O(1)` certificate rides into the output, so a
/// fold over points stays on the fused path.
#[test]
fn pointwise_operators_restrict_to_versions_on_points() {
    let ([a1, _, _, _, _], b1) = span_fixtures();

    let pa = Span::new(&a1, &a1).unwrap();
    let pb = Span::new(&b1, &b1).unwrap();

    let joined = &pa | &pb;
    assert_eq!(*joined.lo(), &a1 | &b1);
    assert!(joined.is_coincident(), "a point join shares one buffer");

    let met = &pa & &pb;
    assert_eq!(*met.lo(), &a1 & &b1);
    assert!(met.is_coincident(), "a point meet shares one buffer");

    // The union of two points is their hull — strictly wider than
    // either on concurrent points.
    let hull = &pa + &pb;
    assert!(!hull.is_coincident());
    assert_eq!(hull, a1.span(&b1));
}

/// The n-ary doors settle the receiver on an empty iterator — owned endpoints,
/// value unchanged — and `intersect_all`'s `None` means an empty intersection,
/// never an empty input.
#[test]
fn nary_doors_settle_the_receiver_on_empty_input() {
    let ([a1, a2, _, _, _], _) = span_fixtures();
    let span = Span::new(&a1, &a2).unwrap();
    let none: [Span; 0] = [];
    assert_eq!(span.union_all(none.iter()), span);
    assert_eq!(span.intersect_all(none.iter()), Some(span.clone()));
    assert_eq!(span.join_all(none.iter()), span);
    assert_eq!(span.meet_all(none.iter()), span);
}

/// One mixed n-ary fold per door — coincident and wide inputs together, so the
/// point and wide combine arms both fire — equals the sequential binary fold.
///
/// The laws quantify this over arities and rotations; the witness pins one
/// readable instance.
#[test]
fn nary_doors_match_sequential_folds_on_a_mixed_family() {
    let ([a1, a2, a3, a4, _], b1) = span_fixtures();
    let seed = Span::new(&a1, &a2).unwrap();
    let family = [
        Span::new(&a3, &a3).unwrap(), // a point
        Span::new(&a2, &a4).unwrap(), // a wide segment
        Span::new(&b1, &b1).unwrap(), // a concurrent point
    ];
    assert_eq!(
        seed.union_all(&family),
        family.iter().fold(seed.clone(), |acc, s| &acc + s),
    );
    // The sequential reference folds *through* `Option` deliberately: the door
    // defers its verdict to the end, so the reference must complete the same
    // fold (`try_fold` would exit at the first `None`).
    #[allow(clippy::manual_try_fold)]
    let sequential_intersect = family
        .iter()
        .fold(Some(seed.clone()), |acc, s| acc.and_then(|a| &a * s));
    assert_eq!(seed.intersect_all(&family), sequential_intersect);
    assert_eq!(
        seed.join_all(&family),
        family.iter().fold(seed.clone(), |acc, s| &acc | s),
    );
    assert_eq!(
        seed.meet_all(&family),
        family.iter().fold(seed.clone(), |acc, s| &acc & s),
    );
}

// ───────────────────────────── the quotient view ─────────────────────────────

/// The quotient view reaches every `Concurrent` placement, matching the eagerly
/// projected span in each corner.
///
/// The `own_span_matches_the_projected_span` law probes with the span's own
/// operands and the projected endpoints, and every such probe dominates the
/// projected start by construction (projection only shrinks a version, so `(a ∧
/// b) / p <= a` always): the law can never present a probe concurrent to the
/// projected start, and the `Concurrent(Start)`/`Concurrent(Both)`
/// transcription arms are its negative space. This witness constructs them —
/// sibling forks inside the masking party's region (concurrent to the start,
/// below the end), a foreign line (concurrent to both), and a start-dominating
/// probe beside only the end — and pins the dominance and precedence
/// coarsenings (and the non-membership) of each.
#[test]
fn own_span_place_reaches_every_concurrent_corner() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let vb = bob.tick().clone();
    // Two sibling lines inside what re-joins into one party region.
    let mut left = alice.fork();
    let vl = left.tick().clone();
    let vr = alice.tick().clone();
    alice.join(left).expect("fork halves re-join");
    let party = alice.party();

    let hi = &vl | &vr;
    let span = Span::new(&vl, &hi).expect("vl bounds its own join");
    let view = &span / party;
    // Both endpoints' supports sit inside the party's region, so the
    // eager projection is the span itself.
    let eager = Span::new(
        (span.lo() / party).to_version(),
        (span.hi() / party).to_version(),
    )
    .expect("projection is monotone");
    assert_eq!(eager, span, "the region covers both endpoints");

    // The sibling: concurrent to the start, strictly below the end.
    assert_eq!(view.place(&vr), Placement::Concurrent(Endpoint::Start));
    assert_eq!(view.dominance(&vr), Dominance::Before);
    assert_eq!(view.precedence(&vr), Precedence::Between);
    // The foreign line: concurrent to both endpoints.
    assert_eq!(view.place(&vb), Placement::Concurrent(Endpoint::Both));
    assert_eq!(view.dominance(&vb), Dominance::Before);
    assert_eq!(view.precedence(&vb), Precedence::After);
    // Above the start, beside the end.
    let probe = &vl | &vb;
    assert_eq!(view.place(&probe), Placement::Concurrent(Endpoint::End));
    assert_eq!(view.dominance(&probe), Dominance::Between);
    assert_eq!(view.precedence(&probe), Precedence::After);
    // Every corner transcribes the eagerly projected span's verdict —
    // and a concurrent probe is beside the segment, never within it.
    for probe in [&vr, &vb, &probe] {
        assert_eq!(view.place(probe), eager.place(probe));
        assert_eq!(view.dominance(probe), eager.dominance(probe));
        assert_eq!(view.precedence(probe), eager.precedence(probe));
        assert!(!view.contains(probe));
    }
}

/// The quotient view's verdicts against a party that owns only part of the
/// span's history match the eagerly projected span, and materialization hands
/// that span back.
///
/// The `own_span_matches_the_projected_span` law quantifies this; the witness
/// pins the masked-view divergence from the unprojected span.
#[test]
fn own_span_projects_both_endpoints() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let a1 = alice.tick().clone();
    let b1 = bob.tick().clone();
    let both = &a1 | &b1;
    let span = a1.span(&both);

    let view = &span / alice.party();
    // Alice's projection drops bob's contribution: the projected join collapses
    // onto her own line, so a1 dominates the whole view…
    assert_eq!(view.dominance(&a1), Dominance::After);
    // …while the unprojected span keeps bob's tick above a1.
    assert_eq!(span.dominance(&a1), Dominance::Between);
    // The mirrored coarsening diverges the same way: b1 precedes the
    // unprojected join but nothing of alice's view…
    assert_eq!(span.precedence(&b1), Precedence::Between);
    assert_eq!(view.precedence(&b1), Precedence::After);
    // …and the unprojected join itself lies beyond the view's segment.
    assert!(span.contains(&both));
    assert!(!view.contains(&both));
    // Materialization is the eagerly projected span (the owned endpoints ride
    // straight into the door: no borrow, no settle).
    let eager = Span::new(
        (&a1 / alice.party()).to_version(),
        (&both / alice.party()).to_version(),
    )
    .unwrap();
    assert_eq!(view.to_span(), eager);
    assert_eq!(Span::from(view), eager);
    // The endpoint views compare as the projections they name.
    assert_eq!(view.lo(), (&a1 / alice.party()).to_version());
    assert_eq!(view.hi(), (&both / alice.party()).to_version());
}
