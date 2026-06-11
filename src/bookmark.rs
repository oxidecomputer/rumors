// Under construction
#![allow(dead_code)]

//! Identity checkpoints that survive an ungraceful restart.
//!
//! A [`Bookmark`] persists *who* a [`Known`](crate::Known) is and how far it
//! has advanced, so a peer that crashed can recover its identity instead of
//! leaking it. See the [`Bookmark`] type for the recovery model and the one
//! rule that keeps it sound.
//!
//! This module is not yet wired to the public surface: the recovery API is
//! being redesigned, and until it lands nothing outside the crate can drive
//! [`Bookmark::update`].

use std::collections::BTreeMap;

use before::{Clock, Party, Version};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::Network;

/// A persistable checkpoint of a [`Known`](crate::Known)'s identity, used to
/// recover that identity after an ungraceful restart instead of leaking it.
///
/// A bookmark records *who* a [`Known`](crate::Known) is and how far it has
/// causally advanced — its identity and its [`Version`](crate::Version) — but
/// none of *what* it knows. The content is left to be recovered the same way
/// any peer gets it: by [`gossip`](crate::Known::gossip)ing. One bookmark can
/// checkpoint several identities together without confusing them.
///
/// # Identities, and why leaking one is costly
///
/// Every peer in a universe holds a distinct *identity*: its share of a single
/// space of identities, first minted whole by [`seed`](crate::Known::seed).
/// [`bootstrap`](crate::Known::bootstrap) splits a share off for a new peer so
/// it can act independently; [`retire`](crate::Known::retire) hands a share
/// back, reuniting it. A [`Version`](crate::Version) is a timestamp expressed
/// relative to these shares, and a share split off through more bootstraps is
/// more finely subdivided, so its versions cost more bits to represent.
/// Retiring coalesces shares again, shrinking that cost back down.
///
/// A peer that simply dies takes its share with it: nothing remains to
/// [`retire`](crate::Known::retire) it back, so the subdivision it represents
/// becomes permanent and every peer's versions stay larger than they need to be
/// from then on. A graceful [`retire`](crate::Known::retire) avoids this by
/// handing the share back before leaving, but a crash gives no chance to
/// retire. A bookmark is the durable record that closes that gap: it lets the
/// restarted peer recover its old share and fold it back in, rather than
/// stranding it.
///
/// # Recovering an identity automatically once you have caught up to it
///
/// A crashed peer recovers by [`bootstrap`](crate::Known::bootstrap)ping a
/// fresh identity from the network, re-learning the content it lost, and then
/// reclaiming a bookmarked identity as its own. Reclaiming an identity means
/// resuming as the very peer that wrote the bookmark, which is sound only once
/// the recovering peer's [`Version`](crate::Version) is **at least as advanced
/// as** the one stored in the bookmark.
///
/// The bookmarked version is a high-water mark: it names everything that
/// identity had already done before the crash. Resume as that identity while
/// still behind the mark and you would, as that identity, go on to do things it
/// has by its own record already done, two different actions claiming one
/// identity's same place in history, which corrupts causal order irreparably.
/// Being caught up — your version equal to or beyond the mark — is the proof
/// that you already know everything that identity ever did, so stepping into it
/// is indistinguishable from having been it all along. This is the same
/// condition [`retire`](crate::Known::retire) establishes from the other side:
/// its in-session round of gossip catches the absorbing peer up to the retiree
/// before the party changes hands.
///
/// A freshly-recovered peer might have caught up to only *some* of the
/// identities a bookmark holds, so recovery is incremental: reclaim the ones
/// you have caught up to, [`gossip`](crate::Known::gossip) to advance, and
/// reclaim more as you do. An identity you have not yet caught up to is simply
/// kept for a later attempt.
///
/// # One bookmark per process, handled linearly
///
/// A bookmark is the durable identity of a *single* peer across *its own*
/// restarts: write it, crash, then recover by reclaiming from it. Reusing one
/// bookmark for that same peer's successive lives is the whole point. Sharing
/// one bookmark between *distinct, concurrently-live* peers is the one misuse
/// that turns this tool against itself.
///
/// Reclamation folds back every stored identity the live party has *caught up
/// to*. But two live peers in one universe [`gossip`](crate::Known::gossip)
/// with one another, so each comes to hold the other's content and thereby to
/// satisfy that catch-up test for the other's share. If they share a bookmark,
/// peer `A` — having gossiped `B`'s messages — meets the test for still-live
/// `B`'s share and absorbs it, and the same identity is now claimed by two live
/// parties at once.
///
/// So a bookmark, like the identity it records, must be handled **linearly**
/// and **persisted atomically**. A duplicated bookmark is as dangerous as a
/// duplicated party, because that is precisely what reclaiming from a shared
/// one can produce.
#[derive(Debug, Eq, PartialEq, Hash, BorshDeserialize, BorshSerialize, Default)]
pub struct Bookmark {
    inner: BTreeMap<Network, Vec<Clock>>,
}

impl Bookmark {
    /// Create a new, empty bookmark.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold the live `party` and `version` into the bookmark, reclaiming every
    /// stored identity that `version` has caught up to.
    ///
    /// An identity is caught up to when `version` is at least as advanced as
    /// that identity's *own contribution* — its stored version restricted to
    /// its share, `clock.own_version()`. Because a share's id region is
    /// advanced only by that share (regions are disjoint among live peers),
    /// this projection is the share's complete authored history, and dominating
    /// it is all that licenses reabsorbing the share: what the identity knew
    /// *outside* its region is authored by others and irrelevant to becoming
    /// it. This is strictly weaker than dominating the stored version in full,
    /// so it reclaims more, no less safely. Those are exactly the identities it
    /// is causally honest to reabsorb, and `party` grows in place to take them
    /// back. An identity `version` has not yet caught up to is left untouched,
    /// so calling this repeatedly as `version` advances reclaims more each
    /// time. A reclaimed identity whose share `party` now wholly holds is
    /// redundant and dropped; one that still covers a share held elsewhere is
    /// kept until that share, too, comes back. Finally `party` is recorded at
    /// `version` as the bookmark's latest record of this identity.
    pub(crate) fn update(&mut self, network: Network, party: &mut Party, version: &Version) {
        // Get the clocks for this network
        let clocks = self.inner.entry(network).or_default();

        // Reclaim every dominated region disjoint from our party by joining it
        // back in, setting aside any that overlap (a nested region we cannot
        // absorb in place). Disjointness is preserved as the party grows, so
        // every disjoint clock is absorbed in this single pass regardless of
        // the order they are visited: by the end, `party` has grown to its
        // final, fully-reclaimed value.
        let mut overlapping = Vec::new();
        for clock in clocks.extract_if(.., |clock| clock.own_version() <= *version) {
            let (p, v) = clock.into_parts();
            if let Err(p) = party.join(p) {
                overlapping.push(Clock::from_parts(p, v));
            }
        }

        // Retain only the overlapping clocks the *fully-grown* party does not
        // already cover: regions still outstanding *above* us (a strict
        // superset of our party), which we must never drop on the floor. A
        // clock the party now covers — equal to it, or a sub-region it has
        // reabsorbed — is redundant and dropped. Classifying against the final
        // party rather than the partial one mid-absorb is what makes this
        // order-insensitive: a single in-loop pass would retain a superset that
        // a later join turns into a now-covered duplicate.
        clocks.extend(
            overlapping
                .into_iter()
                .filter(|clock| !party.covers(clock.party())),
        );

        // Store an alias of our party at its current version
        clocks.push(Clock::from_parts(
            party.dangerously_alias(),
            version.clone(),
        ))
    }
}
