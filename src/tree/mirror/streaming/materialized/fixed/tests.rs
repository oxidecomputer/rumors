//! The fixed allowance's figure, and how the minimum budget and the
//! ledger's budget divide a memory budget around it.

use proptest::prelude::*;

use super::fixed_allowance;
use crate::tree::mirror::streaming::{
    Backend, Local,
    materialized::{admission_budget, floor_allowance, largest_grant, min_memory_budget},
};

/// The in-memory backend's floor allowance.
const FLOOR: usize = floor_allowance::<<Local as Backend>::Erased, <Local as Backend>::Error>();

/// The in-memory backend's largest grant.
const LARGEST: usize = largest_grant::<<Local as Backend>::Erased, <Local as Backend>::Error>();

/// The fixed allowance on the in-memory backend is the "about 7 MiB" the
/// memory budget's documentation sets aside out of every budget, on the
/// 64-bit targets the documentation's figures are measured on.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_fixed_allowance_is_the_documented_figure() {
    const MIB: usize = 1 << 20;
    let allowance = fixed_allowance::<Local>();
    assert!(
        (6 * MIB + MIB / 2..7 * MIB + MIB / 2).contains(&allowance),
        "the fixed allowance is {allowance} B, not about 7 MiB",
    );
}

/// Budgets around the floor and fixed allowances, where the ledger's budget
/// leaves zero, and across the whole range.
fn budgets() -> impl Strategy<Value = usize> {
    let set_aside = FLOOR + fixed_allowance::<Local>();
    prop_oneof![
        0..=2 * set_aside,
        set_aside - 1..=set_aside + 1,
        any::<usize>(),
    ]
}

proptest! {
    /// At every message-size target, a memory budget at the minimum for
    /// that target leaves its ledger exactly the largest grant.
    ///
    /// So the ledger can always admit any one unit beside the floor slots.
    /// And every byte of budget above the floor and fixed allowances goes to
    /// the ledger, while a budget at or below them leaves it nothing.
    #[test]
    fn the_ledger_admits_against_what_the_allowances_leave(
        target in any::<usize>(),
        budget in budgets(),
    ) {
        let set_aside = FLOOR + fixed_allowance::<Local>();
        prop_assert_eq!(admission_budget::<Local>(min_memory_budget::<Local>(target)), LARGEST);
        prop_assert_eq!(admission_budget::<Local>(budget), budget.saturating_sub(set_aside));
    }
}
