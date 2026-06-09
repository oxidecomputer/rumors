//! Crate-level unit tests for party mechanics that the public integration
//! tests can't reach: they need either a *forged* `Known` (private fields) or
//! to read a `Known`'s [`Party`] and compare it to [`Party::seed`]. Both
//! require in-crate access, so they live here rather than in `tests/`.

use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

use before::Party;

use crate::tree::{Root, Tree};
use crate::{Error, Known};

/// Capacity for the in-memory duplex pipe; retire/absorb move no content, so the
/// exact size is immaterial.
const DUPLEX_BUF: usize = 64 * 1024;

/// Insert each of `vals` into `k`, driving the async inserts to completion.
fn with_messages(mut k: Known<u64>, vals: &[u64]) -> Known<u64> {
    pollster::block_on(async move {
        for &v in vals {
            k.message([v]).await;
        }
        k
    })
}

/// Drive `child.retire` against `survivor.gossip` over a duplex pipe, asserting
/// the child retired (`Ok(None)`), and return the (party-grown) survivor.
fn retire_child_into(survivor: Known<u64>, child: Known<u64>) -> Known<u64> {
    pollster::block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (child_out, survivor_out) = tokio::join!(
            child.retire(&mut a_r, &mut a_w),
            survivor.gossip(&mut b_r, &mut b_w),
        );
        assert!(
            child_out.expect("child retire").is_none(),
            "a dominating survivor absorbs the child"
        );
        survivor_out.expect("survivor gossip")
    })
}

/// Drive `provider.gossip` against a fresh `bootstrap`, returning the post-serve
/// provider and the bootstrapped peer.
fn bootstrap_from(provider: Known<u64>) -> (Known<u64>, Known<u64>) {
    pollster::block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (provider_out, boot_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Known::<u64>::bootstrap(&mut b_r, &mut b_w),
        );
        (
            provider_out.expect("provider gossip"),
            boot_out
                .expect("bootstrap")
                .expect("provider served the bootstrap"),
        )
    })
}

/// A peer that absorbs a retiree whose party **overlaps** its own rejects it
/// with [`Error::PartyOverlap`] rather than corrupting its clock. A correct
/// universe never produces this (live parties are always disjoint); we forge it
/// with [`Party::dangerously_alias`] — a copy of the absorber's *exact* region —
/// to model a buggy or malicious peer. The overlap is detected by the absorbing
/// `party.join`, the only place it can arise.
#[test]
fn overlapping_retiree_party_is_rejected() {
    let survivor = Known::<u64>::seed();

    // Forge a retiree sharing the survivor's network and its *exact* party
    // region (not a disjoint fork), with an empty tree so its version equals the
    // survivor's and the survivor takes the absorb branch.
    let forged = Known::<u64> {
        network: survivor.network,
        party: Arc::new(RwLock::new(
            survivor.party.read().unwrap().dangerously_alias(),
        )),
        tree: Tree {
            root: Root::default(),
        },
        canonical: PhantomData,
    };

    let (_retire_out, survivor_out) = pollster::block_on(async {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        tokio::join!(
            forged.retire(&mut a_r, &mut a_w),
            survivor.gossip(&mut b_r, &mut b_w),
        )
    });

    assert!(
        matches!(survivor_out, Err(Error::PartyOverlap)),
        "absorbing an overlapping party must surface PartyOverlap, got {survivor_out:?}"
    );
}

/// Retiring every fork back into the peer they descended from reclaims the whole
/// id-space with no leak: the survivor's party normalizes back to exactly
/// [`Party::seed`] (`"1"`, the whole interval). Each bootstrap hands a child a
/// disjoint slice of the seed's region; each `retire` hands a slice back, and a
/// leak anywhere would leave the reunited party short of the whole.
#[test]
fn retiring_all_forks_reconstitutes_the_seed_party() {
    let survivor = Known::<u64>::seed();
    // Each child is a genuine party-disjoint fork, minted by serving a bootstrap.
    // All are empty, so they share the seed's version, are reflexively dominated,
    // and retire with no prior gossip.
    let (survivor, c1) = bootstrap_from(survivor);
    let (survivor, c2) = bootstrap_from(survivor);
    let (survivor, c3) = bootstrap_from(survivor);

    let survivor = retire_child_into(survivor, c3);
    let survivor = retire_child_into(survivor, c2);
    let survivor = retire_child_into(survivor, c1);

    assert_eq!(
        &*survivor.party.read().unwrap(),
        &Party::seed(),
        "retiring all forks back must reconstitute the whole id-space",
    );
}

/// Bootstrap mints a fresh party by forking the provider's; retiring that peer
/// back must reclaim exactly that minted region. Provider with real content,
/// bootstrap (a wire fork), then retire the newcomer home: the provider's party
/// normalizes back to [`Party::seed`], proving the bootstrap hand-off and the
/// retire commit are jointly leak-free.
#[test]
fn bootstrap_then_retire_reconstitutes_the_seed_party() {
    let provider = with_messages(Known::<u64>::seed(), &[1, 2, 3]);

    let (provider, newcomer) = bootstrap_from(provider);
    // The newcomer pulled all content and is a causal fork (equal version), so
    // the provider reflexively dominates it and absorbs it on retire.
    let provider = retire_child_into(provider, newcomer);

    assert_eq!(
        &*provider.party.read().unwrap(),
        &Party::seed(),
        "retiring a bootstrapped peer back must reconstitute the whole id-space",
    );
}
