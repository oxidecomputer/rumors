//! Contract pins for the masked comparison co-walk.
//!
//! The verdict mass lives elsewhere: the projection laws in [`crate::testing::laws`]
//! pin every entry point against the materialized form over generated,
//! organic, and fuzzed populations (the module doc's testing section). What
//! lives here is the Panics contract's negative space: the silent sweep over
//! the canonicality violations the walk does not structurally notice.

use core::cmp::Ordering;

use num_bigint::BigUint;
use proptest::prelude::*;

use crate::bits::BitsWriter;
use crate::error::Decode;
use crate::testing::generators::leaf_list::{arb_deep_or_correlated_pair, TWO_VERSION_DIVISOR};
use crate::testing::generators::regions::arb_deep_party;
use crate::testing::oracles::flat::{self, check_version};
use crate::testing::property::divided_proptest;
use crate::version::io::validate::whole;
use crate::{OwnVersion, Version};

use super::Comparison;

/// A canonicality violation the walk does not structurally notice sweeps
/// silently.
///
/// A collapsible-sibling-pair stream, which the validator rejects as
/// [`Decode::NotCanonical`], flows through both comparison questions
/// without panicking. The verdict is unspecified by contract, so the pin
/// asserts only that the calls return; what it protects is the Panics
/// sections' split between the structurally-noticed violations (truncation,
/// malformation), which panic, and the rest, which do not.
#[test]
fn collapsible_sibling_pair_sweeps_without_panicking() {
    // (5, 5): internal root, first leaf absolute gamma(5), then the zero
    // right-sibling delta: the collapsible pair.
    let mut bad = BitsWriter::new();
    bad.push(false); // root: internal
    bad.push(true); // left leaf
    bad.write_gamma(&BigUint::from(5u64));
    bad.push(true); // right leaf
    bad.write_gamma(&BigUint::ZERO); // zigzag(0): equal sibling
    assert!(
        matches!(whole(bad.reader()), Err(Decode::NotCanonical)),
        "the witness must sit outside the contract's canonical-operand precondition"
    );
    // The canonical spelling of the same step function: the single leaf 5.
    let mut good = BitsWriter::new();
    good.push(true);
    good.write_gamma(&BigUint::from(5u64));
    whole(good.reader()).expect("the peer operand is canonical");
    let bad = Version::from_test_bits(bad);
    let good = Version::from_test_bits(good);
    // Both entry points, both operand positions: each call must return. The
    // verdicts are unspecified and deliberately unpinned.
    let _ = Comparison::order(&bad, &good);
    let _ = Comparison::order(&good, &bad);
    let _ = Comparison::equal(&bad, &good);
    let _ = Comparison::equal(&good, &bad);
}

/// Checks every comparison spelling of a view against a version, in both
/// directions, against the causal relation `want`.
fn check_view_against_version(
    view: &OwnVersion<'_>,
    version: &Version,
    want: Option<Ordering>,
) -> Result<(), TestCaseError> {
    let le = matches!(want, Some(Ordering::Less | Ordering::Equal));
    let ge = matches!(want, Some(Ordering::Greater | Ordering::Equal));
    let (lt, gt, eq) = (
        want == Some(Ordering::Less),
        want == Some(Ordering::Greater),
        want == Some(Ordering::Equal),
    );
    prop_assert_eq!(view.partial_cmp(version), want, "view.partial_cmp(version)");
    prop_assert_eq!(
        version.partial_cmp(view),
        want.map(Ordering::reverse),
        "version.partial_cmp(view)"
    );
    prop_assert_eq!(
        (
            *view <= *version,
            *view < *version,
            *view >= *version,
            *view > *version,
            *view == *version
        ),
        (le, lt, ge, gt, eq),
        "view against version"
    );
    prop_assert_eq!(
        (
            *version >= *view,
            *version > *view,
            *version <= *view,
            *version < *view,
            *version == *view
        ),
        (le, lt, ge, gt, eq),
        "version against view"
    );
    prop_assert_eq!(
        version.concurrent(view),
        want.is_none(),
        "version.concurrent(view)"
    );
    Ok(())
}

divided_proptest! {
    #![divisor(TWO_VERSION_DIVISOR)]

    /// A deep version's view through a deep party agrees with the flat
    /// model's restriction, as a version and in every comparison.
    ///
    /// Through `/` and through `project`, the view materializes to the model's
    /// restriction byte for byte, and every comparison of the view against a
    /// version and against another view agrees with the model's order of the
    /// restrictions.
    ///
    /// The versions are a deep pair, the second viewed through its own
    /// party, and the parties are deep and fragmented, so a view's owned
    /// cells start and stop at many depths among the version's leaves. Every
    /// operand reaches production through the independent encoder.
    #[test]
    fn projection_matches_the_flat_model_on_deep_operands(
        (a, b) in arb_deep_or_correlated_pair(),
        p in arb_deep_party(),
        q in arb_deep_party(),
    ) {
        let (ma, mb) = (flat::Version::from_rows(a.leaves()), flat::Version::from_rows(b.leaves()));
        let (mp, mq) = (flat::Party::from_oracle(&p), flat::Party::from_oracle(&q));
        let (va, vb) = (ma.production(), mb.production());
        let (pp, pq) = (mp.production(), mq.production());
        let (restricted_a, restricted_b) = (ma.project(&mp), mb.project(&mq));
        let (view_a, view_b) = (&va / &pp, &vb / &pq);
        check_version(&view_a.to_version(), &restricted_a, "(a / p).to_version()")?;
        check_version(&Version::from(va.project(&pp)), &restricted_a, "Version::from(a.project(p))")?;
        check_version(&view_b.to_version(), &restricted_b, "(b / q).to_version()")?;
        check_view_against_version(&view_a, &vb, restricted_a.relation(&mb))?;
        check_view_against_version(&view_a, &va, restricted_a.relation(&ma))?;
        let want = restricted_a.relation(&restricted_b);
        prop_assert_eq!(view_a.partial_cmp(&view_b), want, "view_a.partial_cmp(view_b)");
        prop_assert_eq!(view_b.partial_cmp(&view_a), want.map(Ordering::reverse), "view_b.partial_cmp(view_a)");
        let le = matches!(want, Some(Ordering::Less | Ordering::Equal));
        let ge = matches!(want, Some(Ordering::Greater | Ordering::Equal));
        prop_assert_eq!(
            (view_a <= view_b, view_a < view_b, view_a >= view_b, view_a > view_b, view_a == view_b),
            (le, want == Some(Ordering::Less), ge, want == Some(Ordering::Greater), want == Some(Ordering::Equal)),
            "view_a against view_b"
        );
        let materialized = view_a.to_version();
        prop_assert!(view_a == materialized && materialized == view_a, "a view equals its materialization");
    }
}
