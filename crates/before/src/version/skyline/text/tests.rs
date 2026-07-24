//! Differential pins for the skyline text kernels: byte-identical text
//! against the production renderer, byte-identical skyline streams against
//! the transcoder, and reject parity against the production parser.

use proptest::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, hugeleaf, wide_tooth_comb,
    Packed,
};
use crate::testing::bridge::from_oracle_version;
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::{generators, optrace};
use crate::{Clock, Version};

use super::{parse, render};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    Version::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
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
