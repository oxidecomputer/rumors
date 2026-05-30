//! Shared input generation for the itc differential benchmarks.
//!
//! Every input is built through the *public* API. Fork a seed into a universe of
//! pairwise-disjoint members, then preserve a random subset and `join` it back into a
//! single tree. The members not preserved are simply dropped, so their regions of the id
//! space become structural `0` holes — the preserved subset determines the shape, so
//! *choosing which members to preserve randomizes the tree*. A two-group partition yields
//! two trees that are disjoint by construction (the operands for `join`/`sync`/compare).
//!
//! Impl and oracle inputs are built from the *same* [`Plan`] (fork schedule + preserve
//! labels + per-member tick counts), and `fork`/`tick`/`join` are deterministic, so the
//! two are structurally identical (the differential suite proves they agree). The
//! timings are therefore a like-for-like comparison on the same trees.
//!
//! All randomness is seeded with a fixed constant, so inputs are reproducible run to run
//! — which is what criterion's regression comparisons require. Generation lives here,
//! outside the timed region; the benches only clone (oracle) or `decode` (impl, which is
//! not `Clone`) a prebuilt template to get a fresh value per iteration.

#![allow(dead_code)] // Each bench target compiles this module but uses only part of it.

use itc::{oracle, Clock, Party, Version};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;

/// Fixed RNG seed: identical, reproducible inputs on every run.
pub const SEED: u64 = 0x1737_C10C_C0DE;

/// Universe sizes (number of forked members) the suite sweeps. The joined trees grow
/// roughly linearly with this, so it doubles as the "tree size" axis.
pub const SIZES: &[usize] = &[8, 32, 128, 512, 2_048, 8_192, 32_768];

/// Preserve-group labels. A member is assigned to a group it will be `join`ed into, or
/// [`DISCARD`]ed (left a structural hole).
pub const GROUP_A: u8 = 0;
pub const GROUP_B: u8 = 1;
pub const DISCARD: u8 = u8::MAX;

/// A reproducible recipe for one randomized input, applied identically to the impl and
/// the oracle.
pub struct Plan {
    /// `schedule[i]` names the member to fork when the universe holds `i + 1` members
    /// (so it is always a valid index). Length `n - 1`, building a universe of `n`.
    pub schedule: Vec<usize>,
    /// `label[m]` assigns universe member `m` to a preserve group or [`DISCARD`].
    pub label: Vec<u8>,
    /// `ticks[m]` is how many events member `m` records before any join — giving the
    /// versions/clocks real history. Unused by the party-only builders.
    pub ticks: Vec<u32>,
}

/// Build a plan for a universe of `n` members partitioned into `groups` preserve groups
/// (1 or 2). Every group is guaranteed at least one member, so each joins into a nonempty
/// tree; the rest are spread across the groups, with about a third discarded to punch
/// holes (structure) into the result.
pub fn plan(rng: &mut StdRng, n: usize, groups: u8) -> Plan {
    assert!((1..=2).contains(&groups), "1 or 2 preserve groups");
    assert!(n >= groups as usize, "need at least one member per group");

    let schedule: Vec<usize> = (0..n - 1).map(|i| rng.gen_range(0..=i)).collect();

    // Hand the first `groups` shuffled members one-each to guarantee non-emptiness, then
    // label the remainder: a third discarded, the rest spread across the groups.
    let mut order: Vec<usize> = (0..n).collect();
    order.shuffle(rng);
    let mut label = vec![DISCARD; n];
    for (g, &m) in order.iter().take(groups as usize).enumerate() {
        label[m] = g as u8;
    }
    for &m in &order[groups as usize..] {
        label[m] = if rng.gen_bool(0.33) {
            DISCARD
        } else {
            rng.gen_range(0..groups)
        };
    }

    let ticks: Vec<u32> = (0..n).map(|_| rng.gen_range(0..4)).collect();
    Plan {
        schedule,
        label,
        ticks,
    }
}

// ───────────────────────────── party universes ─────────────────────────────

/// The `groups` preserved [`Party`] trees for `plan`, built through the public API.
pub fn impl_parties(plan: &Plan, groups: u8) -> Vec<Party> {
    let mut universe = vec![Party::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    let mut slots: Vec<Option<Party>> = universe.into_iter().map(Some).collect();
    (0..groups)
        .map(|g| {
            let members = take_group(&mut slots, &plan.label, g);
            members
                .reduce(|mut acc, p| {
                    acc.join(p).expect("universe members are pairwise disjoint");
                    acc
                })
                .expect("every group has at least one member")
        })
        .collect()
}

/// The oracle counterpart of [`impl_parties`] — same plan, structurally identical trees.
pub fn oracle_parties(plan: &Plan, groups: u8) -> Vec<oracle::Party> {
    let mut universe = vec![oracle::Party::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    let mut slots: Vec<Option<oracle::Party>> = universe.into_iter().map(Some).collect();
    (0..groups)
        .map(|g| {
            let members = take_group(&mut slots, &plan.label, g);
            members
                .reduce(|mut acc, p| {
                    acc.join(p).expect("universe members are pairwise disjoint");
                    acc
                })
                .expect("every group has at least one member")
        })
        .collect()
}

// ───────────────────────────── clock universes ─────────────────────────────

/// The `groups` preserved [`Clock`] trees for `plan`: fork a seed clock into a universe,
/// tick each member `plan.ticks[m]` times to give it history, then join each group.
pub fn impl_clocks(plan: &Plan, groups: u8) -> Vec<Clock> {
    let mut universe = vec![Clock::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    for (m, c) in universe.iter_mut().enumerate() {
        for _ in 0..plan.ticks[m] {
            c.tick();
        }
    }
    let mut slots: Vec<Option<Clock>> = universe.into_iter().map(Some).collect();
    (0..groups)
        .map(|g| {
            let members = take_group(&mut slots, &plan.label, g);
            members
                .reduce(|mut acc, c| {
                    acc.join(c)
                        .map_err(|_| ())
                        .expect("universe members are pairwise disjoint");
                    acc
                })
                .expect("every group has at least one member")
        })
        .collect()
}

/// The oracle counterpart of [`impl_clocks`].
pub fn oracle_clocks(plan: &Plan, groups: u8) -> Vec<oracle::Clock> {
    let mut universe = vec![oracle::Clock::seed()];
    for &i in &plan.schedule {
        let child = universe[i].fork();
        universe.push(child);
    }
    for (m, c) in universe.iter_mut().enumerate() {
        for _ in 0..plan.ticks[m] {
            c.tick();
        }
    }
    let mut slots: Vec<Option<oracle::Clock>> = universe.into_iter().map(Some).collect();
    (0..groups)
        .map(|g| {
            let members = take_group(&mut slots, &plan.label, g);
            members
                .reduce(|mut acc, c| {
                    acc.join(c)
                        .map_err(|_| ())
                        .expect("universe members are pairwise disjoint");
                    acc
                })
                .expect("every group has at least one member")
        })
        .collect()
}

// ───────────────────────────── version corpora ─────────────────────────────

/// The versions of the preserved [`impl_clocks`] — randomized event trees with history.
pub fn impl_versions(plan: &Plan, groups: u8) -> Vec<Version> {
    impl_clocks(plan, groups)
        .into_iter()
        .map(|c| c.version())
        .collect()
}

/// The oracle counterpart of [`impl_versions`].
pub fn oracle_versions(plan: &Plan, groups: u8) -> Vec<oracle::Version> {
    oracle_clocks(plan, groups)
        .into_iter()
        .map(|c| c.version())
        .collect()
}

/// Move every member labelled `g` out of `slots`, in ascending member order (the same
/// order for impl and oracle, so the fold builds identical trees on both sides).
fn take_group<T>(slots: &mut [Option<T>], label: &[u8], g: u8) -> std::vec::IntoIter<T> {
    let taken: Vec<T> = (0..slots.len())
        .filter(|&m| label[m] == g)
        .filter_map(|m| slots[m].take())
        .collect();
    taken.into_iter()
}
