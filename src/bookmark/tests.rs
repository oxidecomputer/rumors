//! Tests for [`Bookmark`] and [`Known::bookmark`].
//!
//! A [`Bookmark`] is a persistent checkpoint of a party's identity: the minimal
//! state needed to recover a [`Known`] after an ungraceful restart without
//! leaking its slice of the ITC id-space. These tests assert on the bookmark's
//! private `inner` map and on a `Known`'s private [`Party`], so they live
//! in-crate rather than in `tests/`.
//!
//! The behaviors under test:
//!
//! - *Fidelity* — a bookmark records an alias of the live party at its latest
//!   version (the thing a restore would resurrect).
//! - *Collapse* — re-checkpointing one lineage never grows the clock vector; a
//!   bookmark is a snapshot, not an append-only log.
//! - *Reclamation* — a fork that is checkpointed and then lost is reabsorbed by
//!   any party that comes to dominate it, so its id-region is never leaked.
//! - *Retention* — a still-outstanding overlapping region is kept, never
//!   silently dropped.
//! - *Persistence* — the whole structure round-trips through borsh unchanged.

use before::{Clock, Party, Version};
use proptest::prelude::*;

use crate::{Bookmark, Known, Network};

/// Observe a single `value` on `k`, driving the async insert to completion.
fn message(k: &mut Known<u64>, value: u64) {
    pollster::block_on(k.message([value]));
}

/// The clocks a bookmark stores for `network`, or an empty slice if it tracks
/// nothing there.
fn clocks_for(bookmark: &Bookmark, network: Network) -> &[Clock] {
    bookmark
        .inner
        .get(&network)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Both constructors produce the same empty bookmark: no networks, no clocks.
#[test]
fn new_bookmark_is_empty() {
    assert_eq!(Bookmark::new(), Bookmark::default());
    assert!(Bookmark::new().inner.is_empty());
}

/// Bookmarking a seed records exactly one clock under that seed's network: an
/// alias of the live party at its latest version. The checkpoint is a faithful
/// copy of the identity a restore would resurrect.
#[test]
fn bookmark_records_current_party_and_version() {
    let mut k = Known::<u64>::seed();
    message(&mut k, 1);
    message(&mut k, 2);

    let mut bookmark = Bookmark::new();
    k.bookmark(&mut bookmark);

    let clocks = clocks_for(&bookmark, k.network());
    assert_eq!(clocks.len(), 1, "exactly one clock for the network");
    assert_eq!(clocks[0].party(), &k.party, "records the live party");
    assert_eq!(
        clocks[0].version(),
        k.latest(),
        "records the latest version"
    );
}

/// Re-checkpointing one lineage — advancing it in between — never accumulates
/// clocks. Each bookmark dominates and discards the prior self-alias (the party
/// is byte-identical to itself), leaving exactly one clock whose version tracks
/// the newest observation. A bookmark is a snapshot, not an append-only log.
#[test]
fn rebookmarking_a_lineage_collapses_to_one_clock() {
    let mut k = Known::<u64>::seed();
    let mut bookmark = Bookmark::new();

    let mut previous: Option<Version> = None;
    for value in 0..5u64 {
        message(&mut k, value);
        k.bookmark(&mut bookmark);

        let clocks = clocks_for(&bookmark, k.network());
        assert_eq!(clocks.len(), 1, "collapses to a single clock");
        assert_eq!(clocks[0].version(), k.latest());
        if let Some(previous) = previous.replace(k.latest().clone()) {
            assert!(&previous < clocks[0].version(), "version strictly advances");
        }
    }
}

/// A redaction ticks the party, so re-bookmarking after a [`redact`] records a
/// strictly newer version: the checkpoint tracks deletions, not just inserts.
/// This is the same strict version tick that drives deletion-honoring during
/// reconciliation, observed through the bookmark.
///
/// [`redact`]: Known::redact
#[test]
fn redaction_advances_the_bookmarked_version() {
    let mut k = Known::<u64>::seed();
    let mut keys = Vec::new();
    pollster::block_on(k.message_then([1, 2], |key, _, _| {
        keys.push(key);
        std::future::ready(())
    }));

    let mut bookmark = Bookmark::new();
    k.bookmark(&mut bookmark);
    let before = clocks_for(&bookmark, k.network())[0].version().clone();

    k.redact([keys[0]]);
    k.bookmark(&mut bookmark);
    let after = clocks_for(&bookmark, k.network())[0].version().clone();

    assert!(
        before < after,
        "redaction advanced the checkpointed version"
    );
}

/// The headline anti-leak behavior: a fork that is checkpointed and then lost
/// (an ungraceful exit) does not leak its id-region. The parent forks a child
/// at the shared empty version, bookmarks and drops it, then bookmarks itself —
/// and its party normalizes back to the whole [`Party::seed`], exactly as if the
/// child had [`retire`](Known::retire)d home. The reclaimed region is what keeps
/// the version-vector space from leaking across un-graceful restarts.
#[test]
fn dominating_party_reclaims_a_discarded_fork() {
    let mut parent = Known::<u64>::seed();
    let mut bookmark = Bookmark::new();

    {
        let mut child = parent.fork();
        child.bookmark(&mut bookmark); // checkpoint the child ...
    } // ... then lose it ungracefully.

    parent.bookmark(&mut bookmark);

    assert_eq!(
        parent.party,
        Party::seed(),
        "the discarded fork's region is reclaimed into the whole",
    );
    let clocks = clocks_for(&bookmark, parent.network());
    assert_eq!(clocks.len(), 1, "and the checkpoint collapses to the whole");
    assert_eq!(clocks[0].party(), &Party::seed());
}

/// The same reclamation, but with the whole checkpointed *before* the fork —
/// the interleaving that exposes the absorb loop's order-sensitivity. The
/// bookmark first stores an alias of the whole `1`; the fork then leaves behind
/// the disjoint half `(0, 1)`; and the trailing self-checkpoint extracts both
/// at once. The dominated clocks are visited in vector order — the stored whole
/// `1` first, the discarded half `(0, 1)` second:
///
///   - joining `1` into the *partial* party `(1, 0)` fails (the whole strictly
///     contains us), so a single pass retains it;
///   - joining `(0, 1)` then grows the party to the whole `1`.
///
/// Now the retained `1` is *exactly* the party — a redundant duplicate of the
/// alias we are about to push — but a single in-loop pass never revisits it.
/// The checkpoint must still collapse to one clock: reclamation has to classify
/// each retained clock against the *fully-grown* party, dropping any the party
/// has come to cover.
#[test]
fn reclamation_collapses_a_dominated_superset() {
    let mut parent = Known::<u64>::seed();
    let mut bookmark = Bookmark::new();

    parent.bookmark(&mut bookmark); // checkpoint the whole `1` ...

    {
        let mut child = parent.fork(); // ... split off `(0, 1)` ...
        child.bookmark(&mut bookmark); // ... checkpoint it ...
    } // ... then lose it ungracefully.

    parent.bookmark(&mut bookmark); // the trailing checkpoint must collapse.

    assert_eq!(
        parent.party,
        Party::seed(),
        "the discarded fork's region is reclaimed into the whole",
    );
    let clocks = clocks_for(&bookmark, parent.network());
    assert_eq!(
        clocks.len(),
        1,
        "the now-dominated stored whole is dropped, not left as a duplicate",
    );
    assert_eq!(clocks[0].party(), &Party::seed());
}

/// A still-outstanding overlapping region is retained, never dropped. Bookmark
/// the whole, then fork off a child and bookmark the *child*: the child is a
/// strict sub-region, so it cannot absorb the stored whole (they are not
/// disjoint), and the whole is not the child's exact party, so it must be kept.
/// Dropping it would leak the parent's still-live region from the checkpoint.
#[test]
fn overlapping_outstanding_region_is_retained() {
    let mut parent = Known::<u64>::seed();
    let network = parent.network();

    let mut bookmark = Bookmark::new();
    parent.bookmark(&mut bookmark); // stores the whole

    let mut child = parent.fork(); // parent and child are now disjoint halves
    child.bookmark(&mut bookmark);

    assert_eq!(
        clocks_for(&bookmark, network).len(),
        2,
        "the outstanding whole is retained alongside the child's own clock",
    );
}

/// One bookmark can checkpoint several universes at once; each network's clocks
/// live in their own slot and never interfere.
#[test]
fn distinct_universes_are_tracked_separately() {
    let mut a = Known::<u64>::seed();
    let mut b = Known::<u64>::seed();
    assert_ne!(
        a.network(),
        b.network(),
        "independent seeds, distinct networks"
    );

    let mut bookmark = Bookmark::new();
    a.bookmark(&mut bookmark);
    b.bookmark(&mut bookmark);

    assert_eq!(bookmark.inner.len(), 2, "one slot per universe");
    assert_eq!(clocks_for(&bookmark, a.network())[0].party(), &a.party);
    assert_eq!(clocks_for(&bookmark, b.network())[0].party(), &b.party);
}

/// A bookmark survives serialization unchanged: persisting and reloading it —
/// the entire point of a checkpoint — is the identity function. The fixture
/// holds a retained overlap so the round-trip exercises a multi-clock vector,
/// not just the trivial single-clock case.
#[test]
fn borsh_roundtrip_is_identity() {
    let mut parent = Known::<u64>::seed();
    message(&mut parent, 7);

    let mut bookmark = Bookmark::new();
    parent.bookmark(&mut bookmark);
    let mut child = parent.fork();
    child.bookmark(&mut bookmark); // retained overlap: a two-clock vector

    let bytes = borsh::to_vec(&bookmark).expect("serialize");
    let restored: Bookmark = borsh::from_slice(&bytes).expect("deserialize");

    assert_eq!(bookmark, restored, "round-trip preserves the bookmark");
}

/// An action a single lineage can take between checkpoints.
#[derive(Clone, Debug)]
enum Op {
    /// Observe a new message, advancing the version.
    Message(u64),
    /// Fork a child, checkpoint it into the bookmark, then immediately lose it.
    ForkDiscard,
    /// Checkpoint the live lineage.
    Bookmark,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        any::<u64>().prop_map(Op::Message),
        Just(Op::ForkDiscard),
        Just(Op::Bookmark),
    ]
}

proptest! {
    /// No id-space leaks through a checkpoint, under *any* interleaving. A fork
    /// that is bookmarked and discarded is always reclaimable by the dominating
    /// lineage: after an arbitrary sequence of messages and discarded forks, a
    /// trailing bookmark normalizes the live party back to the whole
    /// [`Party::seed`]. Discarded forks never advance past the lineage, so the
    /// lineage always dominates them and reabsorbs every region it spun off —
    /// the order in which they are reclaimed does not matter.
    #[test]
    fn discarded_forks_are_always_reclaimed(
        ops in proptest::collection::vec(op(), 0..40),
    ) {
        let mut k = Known::<u64>::seed();
        let mut bookmark = Bookmark::new();

        for op in ops {
            match op {
                Op::Message(value) => message(&mut k, value),
                Op::ForkDiscard => {
                    let mut child = k.fork();
                    child.bookmark(&mut bookmark);
                }
                Op::Bookmark => k.bookmark(&mut bookmark),
            }
        }

        // A trailing checkpoint reclaims every region the lineage ever spun off.
        k.bookmark(&mut bookmark);

        prop_assert!(
            k.party == Party::seed(),
            "the lineage reclaims the whole id-space, got {}",
            k.party,
        );
        // Reclaiming the whole leaves nothing outstanding, so a minimal
        // checkpoint is *exactly* one clock: an alias of the reclaimed whole.
        // Every stored clock is a sub-region of `seed`, so it is either joined
        // back in or dropped as now-covered by the fully-grown party; none may
        // survive as a redundant duplicate, whatever order they were extracted
        // in.
        let clocks = clocks_for(&bookmark, k.network());
        prop_assert_eq!(
            clocks.len(),
            1,
            "the checkpoint collapses to a single clock, got {:?}",
            clocks,
        );
        prop_assert!(
            clocks[0].party() == &Party::seed(),
            "and that clock is an alias of the reclaimed whole",
        );
    }

    /// Fidelity and collapse hold across an arbitrary fork-free history: after
    /// every checkpoint of a single advancing lineage, the bookmark holds
    /// exactly one clock for that network, and it is an alias of the live party
    /// at its latest version. Repeated checkpoints never accumulate.
    #[test]
    fn fork_free_checkpoints_stay_a_faithful_singleton(
        values in proptest::collection::vec(any::<u64>(), 1..30),
    ) {
        let mut k = Known::<u64>::seed();
        let mut bookmark = Bookmark::new();

        for value in values {
            message(&mut k, value);
            k.bookmark(&mut bookmark);

            let clocks = clocks_for(&bookmark, k.network());
            prop_assert_eq!(clocks.len(), 1, "a fork-free lineage stays a singleton");
            prop_assert!(clocks[0].party() == &k.party, "records the live party");
            prop_assert!(clocks[0].version() == k.latest(), "records the latest version");
        }
    }
}
