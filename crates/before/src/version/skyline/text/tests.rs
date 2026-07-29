//! Differential pins for the skyline text kernels.
//!
//! The public entries route here (`Display` to [`render`], `FromStr` to
//! [`parse`]), so asserts against them pin entry agreement and
//! determinism, not an independent value. The independent legs: parsing
//! rendered text must land on the *transcoder's* stream byte for byte
//! (the construction-language transcoder shares nothing with either
//! kernel), and a hand-stated accept/reject corpus plus a deterministic
//! mutation sweep pin the grammar's decisions by expected error variant.

use proptest::prelude::*;

use crate::error::Parse;
use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    jump_comb, wide_tooth_comb, Packed,
};
use crate::testing::bridge::from_oracle_version;
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::{generators, optrace};
use crate::{Clock, Version};

use super::{parse, render};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// The full differential pin on one version, over transcoded operands.
///
/// The independent leg: parsing the rendered text must land on the
/// stored stream the transcoder built, byte for byte, so render and
/// parse are exact inverses anchored to the construction language. The
/// `Display` comparison pins entry agreement (the public entry routes to
/// the kernel).
fn assert_text_kernels_agree(v: &Version) {
    let enc = super::super::encode(v);
    let text = render(&enc);
    assert_eq!(
        text,
        v.to_string(),
        "the public Display entry must route to the renderer unchanged"
    );
    let parsed = parse(&text).expect("canonical text parses");
    assert_eq!(
        parsed, enc,
        "parsing rendered text must land on the transcoder's stream byte for byte"
    );
}

/// Every adversarial generator family round-trips render→parse onto the
/// transcoder's stream, across a deterministic size grid.
#[test]
fn generator_families_render_and_parse_identically() {
    let shapes: Vec<Packed> = vec![
        dense(1),
        dense(2),
        dense(64),
        dense(1_000),
        bigroot(7, 3),
        bigroot(200, 50),
        bigroot(1_000, 200),
        hugeleaf(1),
        hugeleaf(64),
        hugeleaf(5_000),
        cliff_comb(3, 2),
        cliff_comb(64, 64),
        cliff_comb(512, 512),
        wide_tooth_comb(64, 8, 16),
        wide_tooth_comb(512, 192, 64),
        cliff_fan(64, 64),
        cliff_fan(512, 128),
        cancelling_chain(64, 64),
        cancelling_chain(512, 128),
        jump_comb(1, 2),
        jump_comb(64, 64),
        jump_comb(512, 128),
        harmonic(1),
        harmonic(64),
        harmonic(1_000),
        alt_spine(1),
        alt_spine(2),
        alt_spine(3),
        alt_spine(64),
        alt_spine(1_001),
    ];
    for p in &shapes {
        assert_text_kernels_agree(&version_of(p));
    }
}

/// Exhaustive small scope: every normal-form tree to the small depth
/// round-trips render→parse onto the transcoder's stream.
#[test]
fn exhaustive_small_scope_renders_and_parses_identically() {
    for t in all_normal_events(EV_SMALL_DEPTH) {
        assert_text_kernels_agree(&from_oracle_version(&t));
    }
}

/// The kernel's grammar decisions are pinned on a deterministic
/// accept/reject corpus.
///
/// Each accepted text yields the value's canonical skyline stream, and
/// each rejected text yields the *stated* error variant — the
/// expectation lives in the table, not in another run of the same
/// kernel.
#[test]
fn parse_corpus_pins_the_grammar_decisions() {
    // Accepted: value-preserving leading zeros and whitespace leniency.
    for text in ["007", " ( 1 , 0 , 2 ) ", "(1, 2, (0, (1, 0, 2), 0))"] {
        let v: Version = text.parse().expect("the public entry accepts");
        assert_eq!(
            parse(text).expect("the kernel accepts the corpus's accepted texts"),
            super::super::encode(&v),
            "an accepted parse yields the value's canonical skyline stream"
        );
    }
    // Rejected: the kernel returns the corpus's stated error variant, and
    // the public entry agrees.
    for (text, want) in [
        ("", Parse::Syntax),                        // no node at all
        ("(1, 2", Parse::Syntax),                   // unbalanced
        ("(1, 0)", Parse::Syntax),                  // an event node has three parts
        ("1 2", Parse::Syntax),                     // trailing junk after the leaf `1`
        ("(, 0, 1)", Parse::Syntax),                // a base is a nonempty digit run
        ("(café, 0)", Parse::Syntax),               // non-ASCII byte
        ("(5, 3, 3)", Parse::NotCanonical),         // equal sibling leaves
        ("(1, 2, 3)", Parse::NotCanonical),         // no zero-base child
        ("(5, 3, 3) x", Parse::Syntax),             // trailing junk outranks canonicality
        ("(0, 5, 5)", Parse::NotCanonical),         // equal siblings: the absorb shape
        ("(0, (1, 2, 2), 0)", Parse::NotCanonical), // nested equal sibling leaves
    ] {
        let got = parse(text).expect_err("the kernel rejects the corpus's rejected texts");
        assert_eq!(got, want, "reject variant on {text:?}");
        let public = text
            .parse::<Version>()
            .expect_err("the public entry rejects");
        assert_eq!(
            public, want,
            "the public entry must agree with the corpus on {text:?}"
        );
    }
}

/// How many mutated texts the generated reject-parity sweep judges.
const REJECT_PARITY_FUZZ_CASES: usize = 512;

/// The sweep's fixed PRNG seed: every run replays the same corpus.
const REJECT_PARITY_FUZZ_SEED: u64 = 0x5EED_CA5E_0B57_AC1E;

/// Replacement and insertion bytes for the mutation sweep: the grammar's
/// whole alphabet plus one byte outside it.
const MUTATION_ALPHABET: &[u8] = b"0123456789(), x";

/// A deterministic byte-mutation sweep holds the kernel and its public
/// entry in lockstep on every mutant.
///
/// Each case deletes, inserts, replaces, or truncates one point of a
/// rendered generator-family text. The public entry routes to the
/// kernel, so the agreement legs pin entry plumbing and determinism —
/// never panicking, and deciding every mutant one way — while the
/// kernel's internal validator gate makes every accepted mutant's stream
/// canonical. The hand-stated corpus above pins known grammar decisions
/// by variant; this sweep is its generated regression sibling over
/// [`REJECT_PARITY_FUZZ_CASES`] mutants at the fixed
/// [`REJECT_PARITY_FUZZ_SEED`].
#[test]
fn mutated_texts_hold_reject_parity_through_the_public_entry() {
    let seeds: Vec<String> = [
        dense(3),
        bigroot(7, 3),
        hugeleaf(64),
        cliff_comb(3, 2),
        jump_comb(1, 2),
        harmonic(2),
        alt_spine(2),
    ]
    .iter()
    .map(|p| version_of(p).to_string())
    .collect();
    let mut next = crate::testing::rng::word_stream(REJECT_PARITY_FUZZ_SEED);
    let mut pick = move |n: usize| (next() % n as u64) as usize;
    for _ in 0..REJECT_PARITY_FUZZ_CASES {
        let mut bytes = seeds[pick(seeds.len())].clone().into_bytes();
        match pick(4) {
            0 => {
                bytes.remove(pick(bytes.len()));
            }
            1 => {
                let b = MUTATION_ALPHABET[pick(MUTATION_ALPHABET.len())];
                bytes.insert(pick(bytes.len() + 1), b);
            }
            2 => {
                let b = MUTATION_ALPHABET[pick(MUTATION_ALPHABET.len())];
                let at = pick(bytes.len());
                bytes[at] = b;
            }
            _ => bytes.truncate(pick(bytes.len())),
        }
        let mutated = String::from_utf8(bytes).expect("ASCII mutations of ASCII text stay UTF-8");
        match (mutated.parse::<Version>(), parse(&mutated)) {
            (Ok(v), Ok(enc)) => assert_eq!(
                enc,
                super::super::encode(&v),
                "an accepted mutant {mutated:?} must land both entries on one stored stream"
            ),
            (Err(public), Err(kernel)) => assert_eq!(
                kernel, public,
                "reject parity on {mutated:?}: the public entry must relay the kernel's error"
            ),
            (public, kernel) => panic!(
                "accept/reject parity broke on {mutated:?}: public entry accepted={}, \
                 kernel accepted={}",
                public.is_ok(),
                kernel.is_ok()
            ),
        }
    }
}

proptest! {
    /// Arbitrary normal-form trees (magnitudes past `u64::MAX` included)
    /// round-trip render→parse onto the transcoder's stream.
    #[test]
    fn arbitrary_trees_render_and_parse_identically(t in generators::arb_oracle_version()) {
        assert_text_kernels_agree(&from_oracle_version(&t));
    }

    /// Every version produced by an organic fork/tick/send/sync/join
    /// history round-trips render→parse onto its stored stream.
    #[test]
    fn organic_histories_render_and_parse_identically(ops in optrace::world_strategy_up_to(120)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for clock in &clocks {
            assert_text_kernels_agree(clock.version());
        }
    }
}

/// The parse-side delta accumulator's standing superlinearity, pinned
/// red on the wide-arming family.
///
/// [`parse`] extracts one signed magnitude per leaf from a plain
/// running accumulator, and that extraction currently pays the
/// accumulator's *high-water* span, not its settled value: after
/// `wide_arming(w, w)`'s one `2^(32w)` swing, every one of the `Θ(w)`
/// trailing zero-delta leaves re-walks the swing's `w` dead digits —
/// `Θ(w²)` touches on `Θ(w)` text, the exact-`top` genre
/// (`tooth_tail`'s rustdoc carries the mechanism) at the text seam.
/// This pin holds the mechanism's measured signature in both
/// directions: per-byte touch growth at least ×1.5 across the doubling
/// (a parse that pays the settled top reads ~×1.0 and must retire this
/// pin deliberately — the cure's commit replaces it with a flatness
/// band and promotes the wide-arming board column, which is withheld
/// on exactly this finding), and absolute ceilings at the measured
/// record ×1.25 (a still-worse regression trips them). The value leg
/// (the parse lands on the stored stream) anchors both runs.
#[cfg(feature = "limb-meter")]
mod parse_accumulator_superlinearity {
    use suanpan::touch_meter;

    use crate::meter::wide_arming;
    use crate::version::skyline::encode;

    use super::{parse, render};

    /// One shipped-parse run over `wide_arming(s, s)`'s rendered text:
    /// text bytes and accumulator touches over the parse body alone,
    /// value-pinned against the stored stream.
    fn run(s: usize) -> (u64, u64) {
        let v = wide_arming(s, s).version();
        let enc = encode(&v);
        let text = render(&enc);
        let bytes = text.len() as u64;
        touch_meter::reset();
        let parsed = parse(&text).expect("rendered text parses");
        let touches = touch_meter::touches();
        assert_eq!(
            parsed, enc,
            "the parse must land on the stored stream byte for byte"
        );
        (bytes, touches)
    }

    /// Digit width (and gap count) of the pin's small run; the large
    /// run doubles both.
    const WIDE_ARMING_SMALL: usize = 256;

    /// Absolute two-scale touch ceilings: the measured record ×1.25.
    ///
    /// [measured 2026-07-29, dev profile, exact counters: touches
    /// 2,109,502 → 8,413,246 across WA(256, 256) → WA(512, 512) on
    /// 70,169 B → 140,219 B of rendered text — 30.1 → 60.0 per byte,
    /// ×2.00 per byte across the doubling: the high-water walk's
    /// quadratic signature. The release-profile board reads the same
    /// mechanism through the public from_str and parse entries at
    /// touch exponent 2.00 when the wide-arming family is given board
    /// rows, at both acceptance scales.]
    const PARSE_WIDE_ARMING_TOUCH_CEILINGS: (u64, u64) = (2_636_877, 10_516_557);

    /// `WA(w, w)` catches the parse accumulator's high-water walk red:
    /// per-byte touch cost grows across the doubling, under absolute
    /// pinned ceilings.
    #[test]
    fn parse_delta_accumulator_reads_superlinear_on_wide_arming() {
        let (small_bytes, small_touches) = run(WIDE_ARMING_SMALL);
        let (large_bytes, large_touches) = run(2 * WIDE_ARMING_SMALL);
        eprintln!(
            "MEASURED parse_wide_arming: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 150,
            "the parse's delta accumulator reads flat on the wide-arming family \
             ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the high-water walk is cured — retire this pin into a flatness band \
             and promote the wide-arming board column (the board-parity roster's \
             wide-arming entry carries the plan)"
        );
        assert!(
            small_touches <= PARSE_WIDE_ARMING_TOUCH_CEILINGS.0
                && large_touches <= PARSE_WIDE_ARMING_TOUCH_CEILINGS.1,
            "the parse's touch cost regressed past the pinned record on the \
             wide-arming family ({small_touches} / {large_touches} against \
             {} / {})",
            PARSE_WIDE_ARMING_TOUCH_CEILINGS.0,
            PARSE_WIDE_ARMING_TOUCH_CEILINGS.1,
        );
    }
}
