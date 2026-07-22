//! Gate-permanent identity invariants: party disjointness, conservation,
//! donated-exactly-once, and the fragmentation bound, over the peer
//! lifecycle.
//!
//! Motivation: the withdrawn version-hop attempt
//! passed every identity test it carried, yet was rejected in design review
//! because its speculative pre-forks would fragment a bootstrap provider's
//! id tree into non-contiguous shards under contention — a party
//! *size/shape* regression no test then modeled. These tests pin the
//! conservation algebra and that shape bound on mainline, so a future
//! identity-touching change fails the gate instead of relying on a reviewer
//! to spot the hazard.
//!
//! Four invariant families, each observed through the accounting-only alias
//! (`Rumors::dangerously_alias_party`; aliases here are compared and folded,
//! never ticked or treated as live):
//!
//! 1. **Disjointness** (the Law of Disjointness): all live parties are
//!    pairwise disjoint after every step of an arbitrary lifecycle
//!    schedule.
//! 2. **Conservation**: the fold-join of all live parties is invariant
//!    along the same schedules — it reconstitutes exactly [`Party::seed`]'s
//!    whole interval, the baseline established when the universe is seeded.
//!    Identity is moved and split, never created or destroyed; because each
//!    step is one clean session driven to completion, nothing is in flight
//!    between steps, and the fold covering the whole interval proves it.
//! 3. **Donated exactly once**: a bootstrap moves one fork (the newcomer's
//!    party rejoins the provider's remainder to exactly the provider's
//!    pre-session party), and a retirement moves one whole party (the
//!    absorber's post-party is exactly the join of the two pre-parties).
//! 4. **The fragmentation bound**: bootstrap-then-retire cycles return the
//!    provider's party bit-for-bit to its baseline — sequential and
//!    interleaved alike — so the ITC id tree's encoded size is bounded
//!    independent of lifecycle churn.
//!
//! The concurrent counterpart — disjointness probed mid-flight, under
//! injected faults — lives in `disruption.rs` via `common::sim`. These
//! schedules instead run one clean session at a time, which is what lets
//! the sharper equalities (2)–(4) hold after *every* step.

mod common;

use before::Party;
use proptest::prelude::*;
use rumors::{Peer, Retire, Rumors};

use crate::common::action::{arb_local_actions, build_local};
use crate::common::wire::{assert_control_drained, block_on, bootstrap_fork, wire_gossip};

/// Capacity for each in-memory link stream on the retirement path: a
/// divergent retiree's session moves content through its gossip round, so
/// keep `retire.rs`'s headroom.
const LINK_BUF: usize = 64 * 1024;

// ---- inspection helpers ---------------------------------------------------

/// Alias a live handle's party for accounting.
///
/// The alias is compared, folded, and dropped — never ticked, and never
/// joined into any live peer's state — per the
/// [`dangerously_alias_party`](Rumors::dangerously_alias_party) contract.
fn alias(handle: &Rumors<u64>) -> Party {
    handle
        .dangerously_alias_party()
        .expect("no retirement is in flight between schedule steps")
}

/// Assert every pair of live parties disjoint: the Law of Disjointness,
/// checked directly rather than via a fold, so a violation names the pair.
fn assert_pairwise_disjoint(fleet: &[Rumors<u64>]) {
    let parties: Vec<Party> = fleet.iter().map(alias).collect();
    for (i, pi) in parties.iter().enumerate() {
        for (j, pj) in parties.iter().enumerate().skip(i + 1) {
            assert!(
                pi.is_disjoint(pj),
                "live parties must be pairwise disjoint after every step: \
                 peers {i} and {j} overlap ({pi:?} vs {pj:?})"
            );
        }
    }
}

/// Assert the fold-join of all live parties reconstitutes exactly the
/// seed's whole interval: identity is conserved, and none is in flight.
fn assert_seed_conserved(fleet: &[Rumors<u64>]) {
    let mut parties = fleet.iter().map(alias);
    let mut whole = parties
        .next()
        .expect("the schedule never empties the fleet");
    whole
        .join_all(parties)
        .expect("live parties are pairwise disjoint, so the fold cannot overlap");
    assert!(
        whole == Party::seed(),
        "the join of all live parties must be invariant — exactly the seed's \
         whole interval — after every step; got {whole:?}"
    );
}

// ---- session drivers ------------------------------------------------------

/// Retire `retiree` into `absorber` over a clean in-memory link, requiring
/// the retirement to complete (a clean wire and a gossiping counterparty
/// admit no other outcome).
fn retire_into(retiree: Rumors<u64>, absorber: &Rumors<u64>) {
    let outcome = block_on(async move {
        let retiree = retiree
            .try_into_peer()
            .await
            .expect("the fleet holds each set's sole handle");
        let (mut r_link, mut a_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (retired, gossiped) =
            tokio::join!(retiree.retire(&mut r_link), absorber.gossip(&mut a_link));
        gossiped.expect("absorber gossip");
        assert_control_drained(r_link, a_link);
        retired
    });
    assert!(
        matches!(outcome, Retire::Retired),
        "a clean-wire retirement into a gossiping peer must complete: {outcome:?}"
    );
}

// ---- lifecycle schedules --------------------------------------------------

/// One abstract lifecycle step. Peer indices are drawn from all of `usize`
/// and resolved modulo the live fleet at execution time (the `sim.rs`
/// idiom), so every generated schedule is valid at every fleet size and the
/// shrinker can simplify steps independently.
#[derive(Debug, Clone, Copy)]
enum Op {
    /// One peer originates a message, advancing its version with its party.
    Send { peer: usize, value: u64 },
    /// Plain gossip between two distinct live peers: no identity moves.
    Gossip { a: usize, off: usize },
    /// A brand-new peer bootstraps off a live provider, which donates a
    /// fork of its party; the newcomer joins the fleet.
    Bootstrap { provider: usize },
    /// A live peer retires into another (distinct) live peer, donating its
    /// whole party; the retiree leaves the fleet.
    Retire { retiree: usize, off: usize },
}

fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => (any::<usize>(), any::<u64>()).prop_map(|(peer, value)| Op::Send { peer, value }),
        3 => (any::<usize>(), any::<usize>()).prop_map(|(a, off)| Op::Gossip { a, off }),
        2 => any::<usize>().prop_map(|provider| Op::Bootstrap { provider }),
        2 => (any::<usize>(), any::<usize>()).prop_map(|(retiree, off)| Op::Retire { retiree, off }),
    ]
}

/// Execute one step against the live fleet. Steps whose lifecycle
/// preconditions cannot be met at the current fleet size — gossip or
/// retirement with no distinct counterparty — are skipped, so the fleet
/// never empties and every executed session respects the API's contract.
fn apply(fleet: &mut Vec<Rumors<u64>>, op: Op) {
    let n = fleet.len();
    match op {
        Op::Send { peer, value } => {
            fleet[peer % n].send(value);
        }
        Op::Gossip { a, off } if n >= 2 => {
            let a = a % n;
            let b = (a + 1 + off % (n - 1)) % n;
            wire_gossip(&fleet[a], &fleet[b]);
        }
        Op::Bootstrap { provider } => {
            let newcomer = bootstrap_fork(&fleet[provider % n]);
            fleet.push(newcomer);
        }
        Op::Retire { retiree, off } if n >= 2 => {
            let r = retiree % n;
            let a = (r + 1 + off % (n - 1)) % n;
            let retiring = fleet.remove(r);
            // Removing index `r` shifted every index above it down by one.
            let a = if a > r { a - 1 } else { a };
            retire_into(retiring, &fleet[a]);
        }
        // A lone peer has no distinct counterparty to gossip with or retire
        // into.
        Op::Gossip { .. } | Op::Retire { .. } => {}
    }
}

/// Run `ops` from a freshly seeded universe, invoking `check` on the live
/// fleet at the baseline and after every step.
fn run_schedule(ops: &[Op], check: impl Fn(&[Rumors<u64>])) {
    let mut fleet = vec![Peer::<u64>::seed().into_rumors()];
    check(&fleet);
    for &op in ops {
        apply(&mut fleet, op);
        check(&fleet);
    }
}

proptest! {
    /// The Law of Disjointness survives arbitrary lifecycle schedules:
    /// after every step of any mix of sends, plain gossip, bootstraps of
    /// new peers off arbitrary members, and retirements of arbitrary
    /// members into others, all live parties are pairwise disjoint.
    #[test]
    fn parties_stay_pairwise_disjoint_under_lifecycle_schedules(
        ops in prop::collection::vec(arb_op(), 1..=20),
    ) {
        run_schedule(&ops, assert_pairwise_disjoint);
    }

    /// Identity is conserved along arbitrary lifecycle schedules: the join
    /// of all live parties equals the baseline established at seed — the
    /// whole `[0, 1)` interval — after every step. Bootstrap and retirement
    /// move and split identity but never create or destroy it, and clean
    /// sessions leave no donation in flight between steps.
    #[test]
    fn identity_is_conserved_under_lifecycle_schedules(
        ops in prop::collection::vec(arb_op(), 1..=20),
    ) {
        run_schedule(&ops, assert_seed_conserved);
    }
}

// ---- donated exactly once -------------------------------------------------

proptest! {
    /// A bootstrap donates exactly one fork: the newcomer's party is
    /// disjoint from the provider's remainder, and joining the two
    /// reconstitutes exactly the provider's pre-session party — no other
    /// identity moved, none was minted, none was lost.
    #[test]
    fn bootstrap_donates_exactly_once(actions in arb_local_actions()) {
        let seed = Peer::<u64>::seed().into_rumors();
        // A provider holding a proper sub-share (not the whole seed), with
        // arbitrary content: the general donation case.
        let provider = build_local(bootstrap_fork(&seed), &actions);

        let pre = alias(&provider);
        let newcomer = bootstrap_fork(&provider);
        let remainder = alias(&provider);
        let minted = alias(&newcomer);

        prop_assert!(
            remainder.is_disjoint(&minted),
            "the newcomer's party must be disjoint from the provider's \
             remainder ({minted:?} vs {remainder:?})"
        );
        let mut rejoined = remainder;
        rejoined
            .join(minted)
            .expect("disjoint parties always join");
        prop_assert!(
            rejoined == pre,
            "remainder ⊔ newcomer must equal the provider's pre-session \
             party: {rejoined:?} vs {pre:?}"
        );
    }

    /// A retirement donates exactly one whole party: the absorber's
    /// post-session party equals its pre-session party joined with the
    /// retiree's pre-session party — the retiree's region moved whole, and
    /// nothing else changed hands.
    #[test]
    fn retire_donates_exactly_once(
        retiree_actions in arb_local_actions(),
        absorber_actions in arb_local_actions(),
    ) {
        let seed = Peer::<u64>::seed().into_rumors();
        let absorber = build_local(bootstrap_fork(&seed), &absorber_actions);
        let retiree = build_local(bootstrap_fork(&seed), &retiree_actions);

        let absorber_pre = alias(&absorber);
        let retiree_pre = alias(&retiree);
        retire_into(retiree, &absorber);

        let mut expected = absorber_pre;
        expected
            .join(retiree_pre)
            .expect("distinct bootstrap forks are disjoint");
        let absorber_post = alias(&absorber);
        prop_assert!(
            absorber_post == expected,
            "the absorber's post-party must be absorber-pre ⊔ retiree-pre: \
             {absorber_post:?} vs {expected:?}"
        );
    }
}

// ---- the fragmentation bound ----------------------------------------------

proptest! {
    /// Sequential bootstrap/retire cycles cannot fragment the provider's id
    /// tree: after each of `k` cycles of (bootstrap a newcomer off `P`,
    /// retire it back into `P`), `P`'s party is bit-for-bit its pre-cycle
    /// baseline — the ITC join renormalizes the returned fork — so its
    /// encoded size is constant, independent of `k`. This is the shape
    /// regression that sank the withdrawn version-hop design: its
    /// speculative forks would have left non-contiguous shards that grow
    /// this measure permanently.
    #[test]
    fn party_returns_to_baseline_under_sequential_cycles(k in 1usize..=12) {
        let seed = Peer::<u64>::seed().into_rumors();
        // The provider under test holds a proper sub-share, the general
        // case; the seed handle stays live holding the complement.
        let p = bootstrap_fork(&seed);
        let baseline = alias(&p);
        let baseline_bits = baseline.encoded_bits();

        for cycle in 0..k {
            let newcomer = bootstrap_fork(&p);
            // Both sides originate mid-cycle: versions advance under the
            // forked and remainder parties, which must not disturb the
            // identity algebra.
            newcomer.send(cycle as u64);
            p.send(u64::MAX - cycle as u64);
            retire_into(newcomer, &p);

            let now = alias(&p);
            prop_assert!(
                now == baseline,
                "cycle {cycle}: the provider's party must return to its \
                 baseline, got {now:?} vs {baseline:?}"
            );
            prop_assert_eq!(
                now.encoded_bits(), baseline_bits,
                "cycle {}: the party's encoded size must not grow with the \
                 cycle count", cycle
            );
        }
    }

    /// Out-of-order (interleaved) bootstrap/retire cycles renormalize too:
    /// bootstrap `m` newcomers off `P`, then retire them back into `P` in
    /// an arbitrary order — the ITC join still collapses the id tree, and
    /// `P`'s party returns bit-for-bit to its baseline. Retirement order
    /// does not determine the join's result; the region algebra does.
    #[test]
    fn party_returns_to_baseline_under_interleaved_cycles(
        order in (2usize..=4)
            .prop_flat_map(|m| Just((0..m).collect::<Vec<usize>>()).prop_shuffle()),
    ) {
        let seed = Peer::<u64>::seed().into_rumors();
        let p = bootstrap_fork(&seed);
        let baseline = alias(&p);

        // Bootstrap all newcomers first (in creation order), then retire
        // them in the shuffled order: reabsorption never matches the order
        // the forks were split off in.
        let mut newcomers: Vec<Option<Rumors<u64>>> = (0..order.len())
            .map(|i| {
                let newcomer = bootstrap_fork(&p);
                newcomer.send(i as u64);
                Some(newcomer)
            })
            .collect();
        for &i in &order {
            let retiring = newcomers[i].take().expect("each newcomer retires once");
            retire_into(retiring, &p);
        }

        let now = alias(&p);
        prop_assert!(
            now == baseline,
            "after all interleaved retirements the provider's party must \
             return to its baseline, got {now:?} vs {baseline:?}"
        );
        prop_assert_eq!(
            now.encoded_bits(), baseline.encoded_bits(),
            "the party's encoded size must return to its baseline under \
             out-of-order reabsorption"
        );
    }
}
