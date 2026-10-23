//! The law drivers over co-generated tick cases: every group over one version
//! and one party, fed tick operands built together, region by region.
//!
//! The arbitrary and deep drivers draw a tick's version and party
//! independently, and organic traces stay a few levels deep, so a party branch
//! seldom sits over a version branch several levels running, and the tick
//! walk's pre-scan state stays simple. The strategies in
//! [`crate::testing::generators::co_generated`] build the two operands together
//! and reach the intricate state instead: lookahead sites nested many levels
//! deep, ranges holding several sites, pre-scans that reserve more than one
//! memo block, and several such pre-scans in one walk. Each case supplies three
//! versions for its party to tick ([`TickCase::versions`]): the drawn version,
//! its fill-fixed successor, and a late perturbation of that successor. So the
//! groups' tick laws meet ticks that fill and ticks that grow, where the walk's
//! state is richest.
//!
//! Each property runs the configured case count divided by its strategy's
//! divisor, which reflects what one case costs; the co-generator's census
//! floors each strategy's regime at that count.
//!
//! The expansion is partial by design: a group over any other signature
//! expands to nothing here, and [`super::deep`] drives every group.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use super::{party, ver};
use crate::testing::generators::co_generated::TickCase;
use crate::testing::laws::Law;
use crate::{Party, Version};

/// Checks every law in `group` on each of `case`'s three versions, ticked by
/// the case's party, naming the law and the version's role on failure.
// The group's type is its law registry's slice type, spelled as the registry
// spells it, so a name for it would only add indirection.
#[allow(clippy::type_complexity)]
fn assert_case_laws(
    group: &[Law<fn(&Version, &Party) -> bool>],
    case: &TickCase,
) -> Result<(), TestCaseError> {
    let p = party(&case.party);
    let [drawn, fill_fixed, late] = case.versions();
    for (role, version) in [
        ("drawn", drawn),
        ("fill-fixed", fill_fixed),
        ("late-perturbed", late),
    ] {
        let v = ver(&version);
        for (name, law) in group {
            prop_assert!(
                law(&v, &p),
                "law violated: {} on the {} version",
                name,
                role
            );
        }
    }
    Ok(())
}

/// Expands the law-group roster into one module of co-generated proptests per
/// group over one version and one party, named by the roster's driver column
/// inside this module.
macro_rules! co_generated_group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( co_generated_group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        mod $driver {
            use proptest::prelude::*;

            use super::assert_case_laws;
            use crate::testing::generators::co_generated::{
                arb_multi_scan_case, arb_spine_case, arb_wide_case, MULTI_SCAN_CASE_DIVISOR,
                WIDE_CASE_DIVISOR,
            };
            use crate::testing::generators::divided_config;
            use crate::testing::laws;

            proptest! {
                /// Every law in the group holds on co-generated spines, whose
                /// lookahead sites nest along the spine and share ranges with
                /// sites in small side regions.
                #[test]
                fn spines(case in arb_spine_case()) {
                    assert_case_laws(laws::$group, &case)?;
                }
            }

            proptest! {
                #![proptest_config(divided_config(WIDE_CASE_DIVISOR))]
                /// Every law in the group holds on co-generated cases whose one
                /// pre-scan reserves more memo slots than one block holds.
                #[test]
                fn wide_pre_scans(case in arb_wide_case()) {
                    assert_case_laws(laws::$group, &case)?;
                }
            }

            proptest! {
                #![proptest_config(divided_config(MULTI_SCAN_CASE_DIVISOR))]
                /// Every law in the group holds on co-generated cases whose
                /// walk runs several pre-scans, reusing the memo's blocks.
                #[test]
                fn multiple_pre_scans(case in arb_multi_scan_case()) {
                    assert_case_laws(laws::$group, &case)?;
                }
            }
        }
    };
    (@one $group:ident, $driver:ident, $other:tt) => {};
}

crate::for_each_law_group!(co_generated_group_drivers);
