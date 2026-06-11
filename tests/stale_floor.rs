//! Regression pin for the (fixed) stale-floor family of bugs.
//!
//! Under the old API, a `rumors()` snapshot shared its originator's *party*
//! through an `Arc` while serving a *tree* frozen at snapshot time. A stale
//! snapshot serving a bootstrap therefore handed the newcomer a fork of the
//! live party paired with a stale version floor — and the newcomer's first
//! mints, causally dominated by versions the originator had already
//! published, were indistinguishable from redacted messages and silently
//! destroyed by the next gossip round.
//!
//! The shared-state `Known` makes that desynchronization unrepresentable:
//! there is no snapshot type that can serve a bootstrap, and
//! `Known::gossip` snapshots the served tree and forks the party in one
//! critical section, so the newcomer's floor always matches its region.
//! ([`rumors::Snapshot`] is data, not a peer; [`rumors::Broadcast`] clones
//! share one synchronized state rather than freezing one.)
//!
//! This test pins the sound invariant positively, in the shape that used to
//! fail: messages minted by a newcomer bootstrapped from a peer that ticked
//! heavily beforehand must survive reconciliation in both directions.

mod common;

use common::wire::{block_on, bootstrap_fork_async, wire_gossip_async};
use rumors::Known;

/// A message minted by a freshly-bootstrapped peer survives gossip, no
/// matter how far the provider had ticked before serving the bootstrap:
/// the served floor and the forked party region are paired atomically.
#[test]
fn message_minted_after_bootstrap_survives_gossip() {
    block_on(async {
        let mut f = Known::<u64>::seed();
        // F ticks well past genesis before serving anyone.
        {
            let mut batch = f.batch();
            for v in 0..16u64 {
                batch.send(v);
            }
        }

        // B bootstraps from F and mints a brand-new message: its version
        // must come out above (or concurrent to) everything F published
        // before the fork, never dominated.
        let mut b = bootstrap_fork_async(&mut f).await;
        b.send(100);

        // Sync the two: a dominated version would read as "already
        // forgotten" and evict the fresh message from both sides.
        wire_gossip_async(&mut f, &mut b).await;

        let f_has = f.snapshot().iter().any(|(_, _, m)| **m == 100);
        let b_has = b.snapshot().iter().any(|(_, _, m)| **m == 100);
        assert!(
            f_has && b_has,
            "message 100 must survive the sync: f_has={f_has} b_has={b_has}"
        );
    });
}
