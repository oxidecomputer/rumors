//! Differential pins for the skyline text kernels.
//!
//! The public entries route here (`Display` to [`render`], `FromStr` to
//! [`parse`]), so asserts against them pin entry agreement and determinism, not
//! an independent value. The independent legs: parsing rendered text must land
//! on the *transcoder's* stream byte for byte (the construction-language
//! transcoder shares nothing with either kernel), a hand-stated accept/reject
//! corpus plus a deterministic mutation sweep pin the grammar's decisions by
//! expected error variant, and the schoolbook parse twin ([`parse_schoolbook`])
//! runs differentially against the production kernel over the mutant corpora —
//! the twin keeps its own canonical flags and a trailing validator gate, so a
//! production accept of the wrong *value* (a canonical stream that is not the
//! text's value, which entry agreement and a validator gate both miss) breaks
//! the byte comparison.

use core::cmp::Ordering;

use proptest::prelude::*;
use suanpan::Accumulator;

use crate::codec::text::{parse_base, Cur};
use crate::codec::{Base, BitsBuf};
use crate::error::Parse;
use crate::meter::registry::Shape;
use crate::meter::Packed;
use crate::testing::bridge::from_oracle_version;
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::{generators, optrace};
use crate::version::skyline::build::SkylineBuilder;
use crate::version::skyline::signed::{gamma_code, zigzag_signed, Sign};
use crate::version::skyline::validate_bits;
use crate::{Clock, Version};

use super::{parse, render};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// The full differential pin on one version, over transcoded operands.
///
/// The independent leg: parsing the rendered text must land on the stored
/// stream the transcoder built, byte for byte, so render and parse are exact
/// inverses anchored to the construction language. The `Display` comparison
/// pins entry agreement (the public entry routes to the kernel).
fn assert_text_kernels_agree(v: &Version) {
    let enc = super::super::encode(v);
    let text = render(crate::codec::built_view(&enc));
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
        Shape::Dense.packed1(1),
        Shape::Dense.packed1(2),
        Shape::Dense.packed1(64),
        Shape::Dense.packed1(1_000),
        Shape::Bigroot.packed2(7, 3),
        Shape::Bigroot.packed2(200, 50),
        Shape::Bigroot.packed2(1_000, 200),
        Shape::Hugeleaf.packed1(1),
        Shape::Hugeleaf.packed1(64),
        Shape::Hugeleaf.packed1(5_000),
        Shape::CliffComb.packed2(3, 2),
        Shape::CliffComb.packed2(64, 64),
        Shape::CliffComb.packed2(512, 512),
        Shape::WideToothComb.packed3(64, 8, 16),
        Shape::WideToothComb.packed3(512, 192, 64),
        Shape::CliffFan.packed2(64, 64),
        Shape::CliffFan.packed2(512, 128),
        Shape::CancellingChain.packed2(64, 64),
        Shape::CancellingChain.packed2(512, 128),
        Shape::JumpComb.packed2(1, 2),
        Shape::JumpComb.packed2(64, 64),
        Shape::JumpComb.packed2(512, 128),
        Shape::Harmonic.packed1(1),
        Shape::Harmonic.packed1(64),
        Shape::Harmonic.packed1(1_000),
        Shape::AltSpine.packed1(1),
        Shape::AltSpine.packed1(2),
        Shape::AltSpine.packed1(3),
        Shape::AltSpine.packed1(64),
        Shape::AltSpine.packed1(1_001),
        Shape::WideArming.packed2(10, 1),
        Shape::WideArming.packed2(16, 8),
        Shape::WideArming.packed2(64, 64),
        Shape::WideArming.packed2(256, 256),
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

/// The kernel's grammar decisions are pinned on a deterministic accept/reject
/// corpus.
///
/// Each accepted text yields the value's canonical skyline stream — asserted
/// against the validator directly, not inferred from the value agreement —
/// and each rejected text yields the *stated* error variant; the expectation
/// lives in the table, not in another run of the same kernel.
#[test]
fn parse_corpus_pins_the_grammar_decisions() {
    // Accepted: value-preserving leading zeros and whitespace leniency.
    for text in ["007", " ( 1 , 0 , 2 ) ", "(1, 2, (0, (1, 0, 2), 0))"] {
        let v: Version = text.parse().expect("the public entry accepts");
        let enc = parse(text).expect("the kernel accepts the corpus's accepted texts");
        assert!(
            super::super::validate_bits(crate::codec::built_view(&enc)).is_ok(),
            "an accepted parse must build a canonical skyline stream: {text:?}"
        );
        assert_eq!(
            enc,
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

/// Drive `judge` over the deterministic mutant corpus.
///
/// [`REJECT_PARITY_FUZZ_CASES`] single-point mutations (delete, insert,
/// replace, truncate) of the generator-family seed texts, replayed at the
/// fixed [`REJECT_PARITY_FUZZ_SEED`] so every consumer judges the same texts.
fn for_each_seeded_mutant(mut judge: impl FnMut(&str)) {
    let seeds: Vec<String> = [
        Shape::Dense.packed1(3),
        Shape::Bigroot.packed2(7, 3),
        Shape::Hugeleaf.packed1(64),
        Shape::CliffComb.packed2(3, 2),
        Shape::JumpComb.packed2(1, 2),
        Shape::Harmonic.packed1(2),
        Shape::AltSpine.packed1(2),
        Shape::WideArming.packed2(10, 2),
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
        judge(&mutated);
    }
}

/// A deterministic byte-mutation sweep holds the kernel and its public entry in
/// lockstep on every mutant.
///
/// Each case deletes, inserts, replaces, or truncates one point of a rendered
/// generator-family text. The public entry routes to the kernel, so the
/// agreement legs pin entry plumbing and determinism — never panicking, and
/// deciding every mutant one way — and every accepted mutant's stream is
/// asserted canonical against the validator directly. That leg is what covers
/// the acceptance frontier the mutants explore: the builder collapses equal
/// sibling leaves unconditionally, and the render↔parse inverse pair and the
/// transcoder differential pin canonicality, but only over rendered texts of
/// canonical values. The hand-stated corpus above pins known grammar decisions
/// by variant; this sweep is its generated regression sibling over
/// [`REJECT_PARITY_FUZZ_CASES`] mutants at the fixed
/// [`REJECT_PARITY_FUZZ_SEED`]; the schoolbook twin's differential below judges
/// the same corpus with an independent kernel.
#[test]
fn mutated_texts_hold_reject_parity_through_the_public_entry() {
    for_each_seeded_mutant(
        |mutated| match (mutated.parse::<Version>(), parse(mutated)) {
            (Ok(v), Ok(enc)) => {
                assert!(
                    validate_bits(crate::codec::built_view(&enc)).is_ok(),
                    "an accepted mutant {mutated:?} must build a canonical skyline stream"
                );
                assert_eq!(
                    enc,
                    super::super::encode(&v),
                    "an accepted mutant {mutated:?} must land both entries on one stored stream"
                );
            }
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
        },
    );
}

/// Assert the production kernel and the schoolbook twin decide one text
/// identically: the same error variant on rejects, byte-equal streams on
/// accepts.
///
/// The comparison the entry-agreement legs cannot make: the twin extracts its
/// deltas under a different accumulator discipline, keeps its own canonical
/// flags, and gates its output through the validator, so a production kernel
/// that accepted a mutant text as the wrong value — or built a non-canonical
/// stream from it — disagrees here.
fn assert_twin_parity(text: &str) {
    match (parse(text), parse_schoolbook(text)) {
        (Ok(prod), Ok(twin)) => assert_eq!(
            prod, twin,
            "both kernels accept {text:?} but build different streams"
        ),
        (Err(prod), Err(twin)) => assert_eq!(
            prod, twin,
            "the kernels reject {text:?} with different variants"
        ),
        (prod, twin) => panic!(
            "accept/reject split on {text:?}: production accepted={}, schoolbook accepted={}",
            prod.is_ok(),
            twin.is_ok()
        ),
    }
}

/// The schoolbook parse twin agrees with the production kernel on every mutant
/// of the deterministic corpus: equal verdicts, byte-equal streams on accepts.
///
/// Judges exactly the texts
/// [`mutated_texts_hold_reject_parity_through_the_public_entry`] judges, with
/// an independent kernel in the second seat ([`assert_twin_parity`]'s
/// contract), so the acceptance frontier the mutants explore is held by a
/// cross-kernel comparison rather than one kernel checking itself.
#[test]
fn schoolbook_twin_agrees_on_the_mutant_corpus() {
    for_each_seeded_mutant(assert_twin_parity);
}

/// The schoolbook delta-extraction kernel: [`parse`] with the
/// compensating-subtraction re-zero in place of the reset.
///
/// Value-exact, with its own canonical flags and a trailing validator gate —
/// which is what seats it as the independent second kernel of the twin-parity
/// differentials ([`assert_twin_parity`]). It is also deliberately kept
/// failing the parse flatness criterion (the `limb-meter` contrast below):
/// after each leaf's extraction it re-zeroes the accumulator by subtracting
/// the extracted magnitude back, which zeroes the *value* but not the digit
/// buffer's top — the high-water walk the wide-arming family prices.
fn parse_schoolbook(s: &str) -> Result<BitsBuf, Parse> {
    /// What a parsed subtree contributes to its parent's
    /// normal-form check.
    struct Child {
        base: Base,
        is_leaf: bool,
    }
    /// While parsing an open node's text, what the stream still
    /// owes it.
    enum EvFrame {
        /// Consumed `(`, the base, and the first separator.
        NeedLeft { base: Base },
        /// Consumed the left child and the second separator.
        NeedRight { base: Base, left: Child },
    }

    let mut cur = Cur::new(s);
    let mut builder = SkylineBuilder::with_capacity(s.len() as u64);
    let mut frames: Vec<EvFrame> = Vec::new();
    let mut delta = Accumulator::new();
    let mut emitted_first = false;
    let mut canonical = true;

    'nodes: loop {
        match cur.peek() {
            Some(b'(') => {
                cur.bump();
                let base = parse_base(&mut cur)?;
                if cur.bump() != Some(b',') {
                    return Err(Parse::Syntax);
                }
                delta.add_magnitude(&base);
                frames.push(EvFrame::NeedLeft { base });
                continue 'nodes;
            }
            Some(c) if c.is_ascii_digit() => {}
            _ => return Err(Parse::Syntax),
        }
        let base = parse_base(&mut cur)?;
        delta.add_magnitude(&base);

        // The discipline under contrast: extract, then re-zero by
        // compensating subtraction — value-zero, top unmoved.
        let (sign, magnitude) = delta.sign_magnitude();
        let code = if emitted_first {
            gamma_code(&zigzag_signed(
                Sign::from_is_negative(sign == Ordering::Less),
                Base::from(magnitude.clone()),
            ))
        } else {
            emitted_first = true;
            gamma_code(&Base::from(magnitude.clone()))
        };
        match sign {
            Ordering::Greater => delta.sub_wide(&magnitude),
            Ordering::Less => delta.add_wide(&magnitude),
            Ordering::Equal => {}
        }
        builder.leaf(frames.len(), code);
        delta.sub_magnitude(&base);

        let mut summary = Child {
            base,
            is_leaf: true,
        };
        loop {
            match frames.pop() {
                None => break 'nodes,
                Some(EvFrame::NeedLeft { base }) => {
                    if cur.bump() != Some(b',') {
                        return Err(Parse::Syntax);
                    }
                    frames.push(EvFrame::NeedRight {
                        base,
                        left: summary,
                    });
                    continue 'nodes;
                }
                Some(EvFrame::NeedRight { base, left }) => {
                    if cur.bump() != Some(b')') {
                        return Err(Parse::Syntax);
                    }
                    if left.base != Base::ZERO && summary.base != Base::ZERO {
                        canonical = false;
                    }
                    if left.is_leaf && summary.is_leaf && left.base == summary.base {
                        canonical = false;
                    }
                    delta.sub_magnitude(&base);
                    summary = Child {
                        base,
                        is_leaf: false,
                    };
                }
            }
        }
    }
    if cur.peek().is_some() {
        return Err(Parse::Syntax);
    }
    if !canonical {
        return Err(Parse::NotCanonical);
    }
    let bits = builder.finish();
    validate_bits(crate::codec::built_view(&bits))
        .expect("a canonical text parse builds a canonical skyline stream");
    Ok(bits)
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

    /// Proptest-generated single-point mutants of rendered arbitrary trees
    /// hold twin parity: the production kernel and the schoolbook twin return
    /// equal verdicts, and byte-equal streams on accepts.
    ///
    /// The generated sibling of
    /// [`schoolbook_twin_agrees_on_the_mutant_corpus`], reaching texts
    /// (magnitudes past `u64::MAX` included) and mutation sites the
    /// deterministic corpus's eight seed shapes cannot.
    #[test]
    fn mutated_renders_hold_twin_parity(
        t in generators::arb_oracle_version(),
        op in 0usize..4,
        at_seed in any::<prop::sample::Index>(),
        byte_seed in any::<prop::sample::Index>(),
    ) {
        let mut bytes = from_oracle_version(&t).to_string().into_bytes();
        let b = MUTATION_ALPHABET[byte_seed.index(MUTATION_ALPHABET.len())];
        match op {
            0 => {
                bytes.remove(at_seed.index(bytes.len()));
            }
            1 => bytes.insert(at_seed.index(bytes.len() + 1), b),
            2 => {
                let at = at_seed.index(bytes.len());
                bytes[at] = b;
            }
            _ => bytes.truncate(at_seed.index(bytes.len())),
        }
        let mutated = String::from_utf8(bytes).expect("ASCII mutations of ASCII text stay UTF-8");
        assert_twin_parity(&mutated);
    }
}

/// The parse-side delta-extraction discipline, held by committed contrast on
/// the wide-arming family.
///
/// [`parse`] extracts one signed magnitude per leaf and *resets* the
/// accumulator, so the digit buffer's top settles to zero and every extraction
/// pays the span written since the previous leaf. The schoolbook kernel
/// ([`parse_schoolbook`], the twin-parity differentials' second seat) is the
/// same parse with the one other discipline a value-equality suite cannot
/// distinguish: it re-zeroes by *subtracting the extracted magnitude back* —
/// value-exact, but the subtraction of a normalized magnitude from a redundant
/// spelling leaves nonzero digits cancelling across the whole span, so the top
/// stays parked at the widest swing and every later extraction re-walks its
/// dead digits. `Shape::WideArming.packed2(w, w)` separates the two: after its
/// single `2^(32w)` swing, the `Θ(w)` trailing zero-delta leaves cost the
/// schoolbook kernel `Θ(w²)` touches on `Θ(w)` text (the exact-`top` genre at
/// the text seam) while the shipped reset discipline stays flat per byte. Both
/// leg's runs are value-pinned to the stored stream, so the contrast is an
/// adequacy witness, not a broken kernel: the wide-arming parse flatness band
/// in `tests/meter.rs` (`parse_wide_arming_touch_cost_is_flat_per_unit`, which
/// the board's wide-arming column follows) is never decoration while this
/// kernel keeps failing its criterion.
#[cfg(feature = "limb-meter")]
mod schoolbook_contrast {
    use suanpan::touch_meter;

    use crate::codec::BitsBuf;
    use crate::error::Parse;
    use crate::meter::registry::Shape;
    use crate::version::skyline::encode;

    use super::{parse, parse_schoolbook, render};

    /// One kernel run over `Shape::WideArming.packed2(s, s)`'s rendered text: text
    /// bytes and accumulator touches over the parse body alone,
    /// value-pinned against the stored stream.
    fn run(s: usize, kernel: fn(&str) -> Result<BitsBuf, Parse>) -> (u64, u64) {
        let v = Shape::WideArming.packed2(s, s).version();
        let enc = encode(&v);
        let text = render(crate::codec::built_view(&enc));
        let bytes = text.len() as u64;
        touch_meter::reset();
        let parsed = kernel(&text).expect("rendered text parses");
        let touches = touch_meter::touches();
        assert_eq!(
            parsed, enc,
            "the kernel must land on the stored stream byte for byte"
        );
        (bytes, touches)
    }

    /// Digit width (and gap count) of the contrast's small run; the
    /// large run doubles both.
    const WIDE_ARMING_SMALL: usize = 256;

    /// The schoolbook read stays superlinear on the wide-arming family (≥ ×1.5
    /// per byte across the doubling) while the shipped reset discipline stays
    /// flat (≤ ×1.25), both value-exact in one run.
    ///
    /// The committed contrast: a criterion the schoolbook kernel passed would
    /// be decoration, and a shipped parse that reads the schoolbook signature
    /// here fails before the envelope suite's absolute ceilings move.
    #[test]
    fn schoolbook_parse_reads_superlinear_on_wide_arming() {
        let (small_bytes, small_touches) = run(WIDE_ARMING_SMALL, parse_schoolbook);
        let (large_bytes, large_touches) = run(2 * WIDE_ARMING_SMALL, parse_schoolbook);
        eprintln!(
            "MEASURED schoolbook_parse_wide_arming: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 150,
            "the schoolbook kernel reads flat on the wide-arming family \
             ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the adequacy witness went green — the kernel no longer demonstrates \
             the high-water walk, so the flatness band it anchors is decoration \
             until a red demonstrator replaces it"
        );
        let (small_bytes, small_touches) = run(WIDE_ARMING_SMALL, parse);
        let (large_bytes, large_touches) = run(2 * WIDE_ARMING_SMALL, parse);
        eprintln!(
            "MEASURED shipped_parse_wide_arming: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 4
                <= u128::from(small_touches) * u128::from(large_bytes) * 5,
            "the shipped parse grew more than x1.25 per byte across the \
             wide-arming doubling ({small_touches}/{small_bytes}B -> \
             {large_touches}/{large_bytes}B): the extraction is paying a stale \
             high-water span again (the envelope suite's wide-arming parse band \
             carries the absolute ceilings)"
        );
    }
}
