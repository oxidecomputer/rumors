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
//! memo block by width or by nesting, and several such pre-scans in one walk.
//! Each case supplies three versions for its party to tick
//! ([`TickCase::versions`]): the drawn version, its fill-fixed successor, and a
//! late perturbation of that successor. So the groups' tick laws meet ticks
//! that fill and ticks that grow, where the walk's state is richest.
//!
//! Each law's property runs the configured case count divided by its
//! strategy's divisor, which reflects what one case costs; the co-generator's
//! census floors each strategy's regime at that count.
//!
//! The expansion is partial by design: a group over any other signature
//! expands to nothing here, and [`super::deep`] drives every group.

use super::{party, ver};
use crate::testing::generators::co_generated::TickCase;
use crate::{Party, Version};

/// Whether `law` holds on each of `case`'s three versions, ticked by the
/// case's party.
fn holds_on_case(law: fn(&Version, &Party) -> bool, case: &TickCase) -> bool {
    let p = party(&case.party);
    case.versions().iter().all(|version| law(&ver(version), &p))
}

/// Expands the law-group roster into one module per group over one version
/// and one party, named by the roster's driver column inside this module,
/// holding one module of per-law proptests for each co-generated regime.
macro_rules! co_generated_group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( co_generated_group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version, party)) => {
        mod $driver {
            use proptest::prelude::*;

            use super::holds_on_case;
            use crate::testing::generators::co_generated::{
                arb_deep_spine_case, arb_multi_scan_case, arb_spine_case, arb_wide_case,
                DEEP_SPINE_CASE_DIVISOR, MULTI_SCAN_CASE_DIVISOR, WIDE_CASE_DIVISOR,
            };

            law_proptests! {
                /// Every law in the group holds on co-generated spines, whose
                /// lookahead sites nest along the spine and share ranges with
                /// sites in small side regions.
                mod spines = $driver($group);
                divisor 1;
                params (case in arb_spine_case());
                check |law| holds_on_case(law, &case);
            }

            law_proptests! {
                /// Every law in the group holds on co-generated spines a few
                /// hundred levels deep, whose outermost pre-scan usually spans
                /// more than one memo block through nested sites alone.
                mod deep_spines = $driver($group);
                divisor DEEP_SPINE_CASE_DIVISOR;
                params (case in arb_deep_spine_case());
                check |law| holds_on_case(law, &case);
            }

            law_proptests! {
                /// Every law in the group holds on co-generated cases whose one
                /// pre-scan reserves more memo slots than one block holds.
                mod wide_pre_scans = $driver($group);
                divisor WIDE_CASE_DIVISOR;
                params (case in arb_wide_case());
                check |law| holds_on_case(law, &case);
            }

            law_proptests! {
                /// Every law in the group holds on co-generated cases whose
                /// walk runs several pre-scans, reusing the memo's blocks.
                mod multiple_pre_scans = $driver($group);
                divisor MULTI_SCAN_CASE_DIVISOR;
                params (case in arb_multi_scan_case());
                check |law| holds_on_case(law, &case);
            }
        }
    };
    (@one $group:ident, $driver:ident, $other:tt) => {};
}

crate::for_each_law_group!(co_generated_group_drivers);
