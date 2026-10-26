//! The law drivers over multi-scale pairs: the law groups over one version,
//! over two, and over ranks, fed pairs that reach the rank integral's
//! deferral and exact rank ties at freezing scale.
//!
//! [`arb_multiscale_pair`] draws bushy regions, a quarter of the time beside
//! a deep same-side spine, heights moved by steps from a few bits to 2,200,
//! and a second version derived from the first. Its pairs freeze and defer
//! parked heights in the integral, in single folds and in co-sweeps, and tie
//! exactly in rank while differing as versions, regimes no arbitrary or
//! organic pair reaches. The groups driven here hold the order, encoding,
//! valuation, and measure laws those regimes test. The measure tests' census
//! floors each regime at this module's case count.
//!
//! The expansion is partial by design. The groups over three versions and
//! over version lists, and every group over parties or clocks, expand to
//! nothing here: at full width, a multi-scale triple or list can hold a
//! hundred kilobytes or more of encoding, and the triple and list laws spend
//! seconds per case on such inputs in a debug build. [`super::deep`] drives
//! every group, at heights within each size's budget.

use proptest::prelude::*;

use super::ver;
use crate::testing::generators::leaf_list::{arb_multiscale_pair, TWO_VERSION_DIVISOR};

/// Expands the law-group roster into one module of multi-scale proptests per
/// group over versions or ranks alone, one per law, named by the roster's
/// driver column inside this module.
macro_rules! multiscale_group_drivers {
    (args: (); $(($group:ident, $driver:ident, $shape:tt)),* $(,)?) => {
        $( multiscale_group_drivers!(@one $group, $driver, $shape); )*
    };
    (@one $group:ident, $driver:ident, (version)) => {
        law_proptests! {
            /// Every law in the group holds on both versions of a multi-scale
            /// pair.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_multiscale_pair());
            check |law| law(&ver(&a.tree())) && law(&ver(&b.tree()));
        }
    };
    (@one $group:ident, $driver:ident, (version, version)) => {
        law_proptests! {
            /// Every law in the group holds on multi-scale pairs, in both
            /// operand orders.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_multiscale_pair());
            check |law| {
                let (a, b) = (ver(&a.tree()), ver(&b.tree()));
                law(&a, &b) && law(&b, &a)
            };
        }
    };
    (@one $group:ident, $driver:ident, (rank, rank, rank)) => {
        law_proptests! {
            /// Every law in the group holds on the ranks of a multi-scale pair
            /// and the distance between them: exact fractions at freezing
            /// scale, and equal ranks of distinct versions.
            mod $driver = $driver($group);
            divisor TWO_VERSION_DIVISOR;
            params ((a, b) in arb_multiscale_pair());
            check |law| {
                let (a, b) = (ver(&a.tree()), ver(&b.tree()));
                law(&a.rank(), &b.rank(), &a.distance(&b))
            };
        }
    };
    (@one $group:ident, $driver:ident, $other:tt) => {};
}

crate::for_each_law_group!(multiscale_group_drivers);
