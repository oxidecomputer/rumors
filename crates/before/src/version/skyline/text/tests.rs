//! Differential pins for the skyline text kernels: byte-identical text
//! against the production renderer, byte-identical skyline streams against
//! the transcoder, and reject parity against the production parser.

use proptest::prelude::*;

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
/// The kernel renders byte-identical text to the production `Display`,
/// and parsing that text back yields the byte-identical skyline stream
/// the transcoder produces.
fn assert_text_kernels_agree(v: &Version) {
    let enc = super::super::encode(v);
    let text = render(&enc);
    assert_eq!(
        text,
        v.to_string(),
        "the skyline renderer must produce the production Display's bytes"
    );
    let parsed = parse(&text).expect("canonical text parses");
    assert_eq!(
        parsed, enc,
        "the skyline parser must produce the transcoder's stream byte for byte"
    );
}

/// Every adversarial generator family renders and parses byte-identically
/// against the production text path, across a deterministic size grid.
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
/// renders and parses byte-identically against the production text path.
#[test]
fn exhaustive_small_scope_renders_and_parses_identically() {
    for t in all_normal_events(EV_SMALL_DEPTH) {
        assert_text_kernels_agree(&from_oracle_version(&t));
    }
}

/// The kernels reproduce the production parser's grammar decisions on a
/// deterministic accept/reject corpus: the same values accepted (with the
/// same skyline stream) and the same errors rejected.
#[test]
fn parse_reject_parity_with_the_production_parser() {
    // Accepted: value-preserving leading zeros and whitespace leniency.
    for text in ["007", " ( 1 , 0 , 2 ) ", "(1, 2, (0, (1, 0, 2), 0))"] {
        let v: Version = text.parse().expect("the production parser accepts");
        assert_eq!(
            parse(text).expect("the kernel accepts what production accepts"),
            super::super::encode(&v),
            "an accepted parse yields the value's canonical skyline stream"
        );
    }
    // Rejected: the kernel returns the production parser's exact error.
    for text in [
        "",                  // no node at all
        "(1, 2",             // unbalanced
        "(1, 0)",            // an event node has three parts
        "1 2",               // trailing junk after the leaf `1`
        "(, 0, 1)",          // a base is a nonempty digit run
        "(café, 0)",         // non-ASCII byte
        "(5, 3, 3)",         // equal sibling leaves
        "(1, 2, 3)",         // no zero-base child
        "(5, 3, 3) x",       // trailing junk outranks canonicality
        "(0, 5, 5)",         // equal sibling leaves: the builder's absorb shape
        "(0, (1, 2, 2), 0)", // nested equal sibling leaves
    ] {
        let expected = text.parse::<Version>().expect_err("production rejects");
        let got = parse(text).expect_err("the kernel rejects what production rejects");
        assert_eq!(
            got, expected,
            "reject parity on {text:?}: the kernel must return the production error"
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

/// A deterministic byte-mutation sweep holds accept/reject parity between
/// the kernel and the production parser.
///
/// Each case deletes, inserts, replaces, or truncates one point of a
/// rendered generator-family text; on every mutant the two parsers must
/// agree — the same accepts (with the kernel yielding the transcoder's
/// stream) and the same error kind on rejects. The hand-picked corpus
/// above pins known grammar decisions; this sweep is its generated
/// regression sibling over [`REJECT_PARITY_FUZZ_CASES`] mutants at the
/// fixed [`REJECT_PARITY_FUZZ_SEED`].
#[test]
fn mutated_texts_hold_reject_parity_with_the_production_parser() {
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
                "an accepted mutant {mutated:?} must yield the transcoder's stream"
            ),
            (Err(production), Err(kernel)) => assert_eq!(
                kernel, production,
                "reject parity on {mutated:?}: the kernel must return the production error"
            ),
            (production, kernel) => panic!(
                "accept/reject parity broke on {mutated:?}: production accepted={}, \
                 kernel accepted={}",
                production.is_ok(),
                kernel.is_ok()
            ),
        }
    }
}

proptest! {
    /// Arbitrary normal-form trees (magnitudes past `u64::MAX` included)
    /// render and parse byte-identically against the production text path.
    #[test]
    fn arbitrary_trees_render_and_parse_identically(t in generators::arb_oracle_version()) {
        assert_text_kernels_agree(&from_oracle_version(&t));
    }

    /// Every version produced by an organic fork/tick/send/sync/join
    /// history renders and parses byte-identically against the production
    /// text path.
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
