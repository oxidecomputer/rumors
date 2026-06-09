//! KNOWN BUG (reproducer, `#[ignore]`d until fixed): party regions can be
//! transferred without the matching causal floor.
//!
//! A [`rumors`](rumors::Known::rumors) snapshot shares its originator's
//! *party* through the `Arc`, but its *tree* — and therefore the version
//! floor it serves — is a clone frozen at snapshot time. When a stale
//! snapshot serves a bootstrap, the newcomer receives a fork of the *live*
//! party paired with the *stale* version floor. If the originator ticked
//! since the snapshot was taken, the newcomer's first mints can be causally
//! dominated by versions the originator already published — and a dominated
//! version is indistinguishable from a redacted one, so the newcomer's
//! fresh messages are silently and permanently dropped on the next gossip.
//!
//! The sound invariant (which `retire` establishes by reconciling before the
//! hand-off, and which canonical bootstrap-serving gets for free because the
//! served tree is the party's own) is: a party region may only be activated
//! under a tree whose version dominates every event ever minted in that
//! region. Snapshot hand-offs break the coupling because the region rides
//! the shared `Arc` while the floor rides the snapshot's frozen tree.
//!
//! The mirrored direction (a snapshot *absorbing* a retiree: the originator
//! gains the retiree's region immediately, but the retiree's event floor
//! stays in the snapshot until it is joined back — or forever, if the
//! snapshot is dropped) has the same shape, but needs a transitive carrier
//! (a peer holding both the retiree's events and the originator's) to
//! manifest as loss, and is not reproduced here.

mod common;

use common::wire::block_on;
use rumors::Known;

const DUPLEX_BUF: usize = 64 * 1024;

/// A message minted by a peer bootstrapped from a stale snapshot must survive
/// gossip. Today it does not: the newcomer's floor is the snapshot's frozen
/// version, its first mint is dominated by the originator's later ticks, and
/// the message vanishes from both peers in one reconciliation round.
#[test]
#[ignore = "known bug: stale snapshot serves a live party fork with a frozen version floor"]
fn message_minted_after_stale_snapshot_bootstrap_survives_gossip() {
    block_on(async {
        let mut f = Known::<u64>::seed();
        // Snapshot taken while F is empty: its tree floor is the zero version.
        let s = f.rumors();
        // F ticks well past the snapshot.
        for v in 0..16u64 {
            f.message([v]).await;
        }

        // The stale snapshot serves a bootstrapper, forking the LIVE party.
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (s_out, b_out) = tokio::join!(
            s.gossip(&mut a_r, &mut a_w),
            Known::<u64>::bootstrap(&mut b_r, &mut b_w),
        );
        s_out.expect("snapshot serves");
        let mut b = b_out.expect("bootstrap").expect("served");

        // B mints a brand-new message from its stale floor: its version comes
        // out strictly dominated by F's latest.
        b.message([100u64]).await;

        // Sync the two: a dominated version reads as "already forgotten", so
        // the fresh message is evicted from both sides.
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (f_out, b_out) =
            tokio::join!(f.gossip(&mut a_r, &mut a_w), b.gossip(&mut b_r, &mut b_w),);
        let f = f_out.expect("f gossip");
        let b = b_out.expect("b gossip");
        let f_has = f.iter().any(|(_, _, m)| **m == 100);
        let b_has = b.iter().any(|(_, _, m)| **m == 100);
        assert!(
            f_has && b_has,
            "message 100 must survive the sync: f_has={f_has} b_has={b_has}"
        );
    });
}
