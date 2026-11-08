//! The fixed allowance's figure, how the minimum budget and the ledger's budget
//! divide a memory budget around the session allowance, and the largest target
//! a memory budget affords.

use proptest::prelude::*;

use super::{fixed_allowance, session_allowance};
use crate::tree::mirror::streaming::{
    Backend, Local,
    materialized::{Allowances, affordable_target, floor_allowance, largest_grant},
    remote::wire_pools_bytes,
};

/// The in-memory backend's floor allowance.
const FLOOR: usize = floor_allowance::<<Local as Backend>::Erased, <Local as Backend>::Error>();

/// The in-memory backend's largest grant.
const LARGEST: usize = largest_grant::<<Local as Backend>::Erased, <Local as Backend>::Error>();

/// The fixed allowance on the in-memory backend is the "about 7 MiB" the memory
/// budget's documentation sets aside out of every budget, on the 64-bit targets
/// the documentation's figures are measured on.
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

/// Message-size targets around zero, around the run's frame overhead, and
/// across the whole range.
fn targets() -> impl Strategy<Value = usize> {
    prop_oneof![0usize..1 << 12, 0usize..1 << 24, any::<usize>()]
}

/// Budgets around the floor and session allowances at `target`, where the
/// ledger's budget leaves zero, and across the whole range.
fn budgets(target: usize) -> impl Strategy<Value = usize> {
    let set_aside = FLOOR.saturating_add(session_allowance::<Local>(target));
    prop_oneof![
        0..=set_aside.saturating_mul(2),
        set_aside.saturating_sub(1)..=set_aside.saturating_add(1),
        any::<usize>(),
    ]
}

/// Allowance parts and memory or ledger budgets from the small values real
/// allowances take to `usize::MAX`, the saturated session allowance a `usize`
/// narrower than 64 bits gives at the largest targets included.
fn bytes() -> impl Strategy<Value = usize> {
    prop_oneof![
        0usize..1 << 12,
        0usize..1 << 32,
        usize::MAX - (1 << 12)..=usize::MAX,
        Just(usize::MAX),
        any::<usize>(),
    ]
}

proptest! {
    /// At every message-size target, a ledger gets what its memory budget
    /// leaves beside the floor and session allowances, `B = M − A − X`.
    ///
    /// A memory budget at the minimum for the target leaves its ledger
    /// exactly the largest grant, so the ledger can always admit any one
    /// unit against the budget, and every byte of budget above the
    /// minimum goes to the ledger. The session allowance is the fixed
    /// allowance beside the two wire pools at the target. Where the minimum
    /// saturates, `M − A − X` stands for nothing a `usize` counts, and only
    /// the largest grant is claimed
    /// (`requested_budgets_cover_the_largest_grant`). A test-path ledger
    /// budget sits beside both allowances, and `B = M − A − X` gives it
    /// back wherever its memory budget does not saturate.
    #[test]
    fn the_ledger_admits_against_what_the_allowances_leave(
        (target, budget) in targets().prop_flat_map(|target| (Just(target), budgets(target))),
    ) {
        let allowance = session_allowance::<Local>(target);
        prop_assert_eq!(allowance, fixed_allowance::<Local>().saturating_add(wire_pools_bytes(target)));
        let allowances = Allowances::of::<Local>(target);
        let minimum = allowances.minimum();
        prop_assert_eq!(minimum, (FLOOR + LARGEST).saturating_add(allowance));
        prop_assert_eq!(allowances.requested(minimum).ledger, LARGEST);
        let leaves = |memory: usize| memory.saturating_sub(FLOOR).saturating_sub(allowance);

        let requested = allowances.requested(budget);
        prop_assert_eq!(requested.memory, budget.max(minimum));
        if minimum < usize::MAX {
            prop_assert_eq!(requested.ledger, leaves(requested.memory));
        }

        let exact = allowances.ledger(budget);
        prop_assert_eq!(exact.ledger, budget);
        prop_assert_eq!(exact.memory, FLOOR.saturating_add(allowance).saturating_add(budget));
        if exact.memory < usize::MAX {
            prop_assert_eq!(leaves(exact.memory), budget);
        }
    }

    /// Whatever the allowances, saturated ones included, a requested memory
    /// budget gives its ledger at least the largest grant.
    ///
    /// It holds the session to the larger of the request and the minimum,
    /// and below saturation, that memory budget is exactly the ledger's
    /// beside the floor and session allowances.
    ///
    /// On a 32-bit target the wire pools at a target of about 1 GiB or more
    /// saturate the session allowance and with it the minimum, a regime no
    /// 64-bit target reaches through the types. A request of `usize::MAX`
    /// there must still leave the ledger the largest grant, never the zero
    /// that subtracting a saturated allowance from it leaves.
    #[test]
    fn requested_budgets_cover_the_largest_grant(
        floor in bytes(),
        largest in bytes(),
        session in bytes(),
        memory in bytes(),
    ) {
        let allowances = Allowances::new(floor, largest, session);
        let minimum = allowances.minimum();
        prop_assert_eq!(minimum, floor.saturating_add(session).saturating_add(largest));
        let requested = allowances.requested(memory);
        prop_assert!(requested.ledger >= largest);
        prop_assert_eq!(requested.memory, memory.max(minimum));
        if minimum < usize::MAX {
            prop_assert_eq!(requested.memory, floor + session + requested.ledger);
        }
    }

    /// The minimum memory budget never shrinks as the message-size target
    /// grows, saturated targets included.
    ///
    /// That is the premise on which [`affordable_target`]'s binary search
    /// finds every target within a budget below the one it returns.
    #[test]
    fn the_minimum_grows_with_the_target(a in targets(), b in targets()) {
        let (smaller, larger) = (a.min(b), a.max(b));
        prop_assert!(
            Allowances::of::<Local>(smaller).minimum()
                <= Allowances::of::<Local>(larger).minimum()
        );
    }

    /// The affordable target is the largest target, at most the one asked
    /// for, whose minimum is within the memory budget, or zero when not
    /// even zero's is.
    ///
    /// It never exceeds the target asked for; its minimum is within the
    /// budget unless it is zero; and it is either the target asked for or
    /// one below a target whose minimum exceeds the budget. With the
    /// minimum never shrinking as the target grows
    /// (`the_minimum_grows_with_the_target`), those three single out the
    /// largest such target. Budgets lie on both sides of the minimum at
    /// the target asked for, and at zero.
    #[test]
    fn the_affordable_target_is_the_largest_within_the_budget(
        (target, memory) in targets().prop_flat_map(|target| {
            let at_zero = Allowances::of::<Local>(0).minimum();
            let memory = prop_oneof![
                budgets(target),
                at_zero.saturating_sub(1)..=at_zero.saturating_add(1),
            ];
            (Just(target), memory)
        }),
    ) {
        let fits = |target: usize| Allowances::of::<Local>(target).minimum() <= memory;
        let affordable = affordable_target::<Local>(memory, target);
        prop_assert!(affordable <= target);
        prop_assert!(fits(affordable) || affordable == 0);
        prop_assert!(affordable == target || !fits(affordable + 1));
        if !fits(0) {
            prop_assert_eq!(affordable, 0);
        }
    }
}
