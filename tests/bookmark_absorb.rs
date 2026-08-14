//! Boundary pins for the absorb-persist loss window.
//!
//! The session contract's third qualified exception to "`Err` means
//! unchanged": absorbing a retiring peer commits the reconciled content and
//! the absorbed identity first, then persists the bookmark, so a persist
//! failing on exactly that write leaves the session `Err` with the
//! absorption live in memory but not yet crash-safe — and a retried gossip
//! re-runs the persist, exactly as [`rumors::Error::Bookmark`]'s docs
//! promise. This window is denominated in *persist steps*, not wire bytes:
//! the boundary unit is which bookmark write fails. The two tests here sit
//! one persist step apart — the write *before* the absorption (everything
//! unchanged on both sides) and the write *after* it (the documented
//! residue) — so a drift in where the session commits relative to its
//! persist schedule fails one leg's assertion class.

mod common;

use std::sync::{Arc, Mutex};

use rumors::{Bookmark, Error, Peer, Retire, Rumors};

use crate::common::flaky::{DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::wire::{bootstrap_fork_async, tokio_block_on as block_on, wire_gossip_async};

/// The message payload; one marker message proves content moved (or did not).
type Msg = u64;

/// Capacity for every in-memory link stream, as in the sibling bookmark
/// suites.
const LINK_BUF: usize = 8 * 1024;

/// The retiree's marker message: absorbed content the boundary assertions
/// look for.
const MARKER: Msg = 41;

/// Drive `retiree.retire` against `absorber.gossip` over a clean in-memory
/// link, returning both outcomes. Each side's future owns its link, so an
/// aborting side surfaces EOF to the other.
async fn retire_into(
    retiree: Rumors<Msg>,
    absorber: &Rumors<Msg, FlakyInMemoryBookmark>,
) -> (
    Retire<Msg>,
    Result<rumors::Gossiped, Error<FlakyInMemoryBookmark>>,
) {
    let absorber = absorber.clone();
    let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);
    let retire = tokio::spawn(async move {
        let mut link = ret_side;
        let peer = retiree
            .try_into_peer()
            .await
            .expect("the test holds the sole handle to the retiring set");
        peer.retire(&mut link).await
    });
    let absorb = tokio::spawn(async move {
        let mut link = abs_side;
        absorber.gossip(&mut link).await
    });
    let (retire_out, absorb_out) = tokio::join!(retire, absorb);
    (
        retire_out.expect("retire task"),
        absorb_out.expect("absorb task"),
    )
}

/// Whether `rumors` holds a live message with payload [`MARKER`].
fn holds_marker<B: Bookmark>(rumors: &Rumors<Msg, B>) -> bool {
    rumors
        .snapshot()
        .iter()
        .any(|(_key, _version, value)| **value == MARKER)
}

/// The absorber fleet: A (flaky bookmark, `writes` schedule), a retiree
/// holding [`MARKER`], and D, a third replica for post-failure sessions.
///
/// The retire session's boundary writes are the next two decisions after the
/// fleet is built — its pre-absorb update, then the post-absorb persist —
/// and that alignment is asserted here mechanically (the feed's consulted
/// count), never narrated: a change in how many writes setup consumes fails
/// this assertion instead of silently shifting the boundary.
fn fleet(
    writes: Vec<bool>,
) -> (
    DurableStore,
    Rumors<Msg, FlakyInMemoryBookmark>,
    Rumors<Msg>,
    Rumors<Msg>,
) {
    let store: DurableStore = Arc::new(Mutex::new(None));
    let faults = Arc::new(Mutex::new(FaultFeed::new(Vec::new(), writes)));
    let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), 0);
    block_on(async {
        let a = Peer::<Msg>::seed()
            .sync_window_floor()
            .bookmark(bookmark)
            .await
            .expect("a pristine seed attaches its bookmark without touching storage")
            .into_rumors();
        let r = bootstrap_fork_async(&a).await;
        r.send(MARKER);
        // D boots from the retiree, so its arrival consumes none of A's
        // bookmark writes and the boundary alignment below holds.
        let d = bootstrap_fork_async(&r).await;
        let consulted = faults.lock().unwrap().writes_consulted();
        assert_eq!(
            consulted, 2,
            "fleet setup must consume exactly two of A's write decisions \
             (the update and the donation slice serving the retiree's \
             bootstrap), so the retire session's boundary writes come next",
        );
        (store, a, r, d)
    })
}

/// One persist step *inside* the window: the bookmark write after the
/// absorption fails, and the session ends in the documented residue.
///
/// The absorber returns [`Error::Bookmark`] with the absorption live in
/// memory — its party covers the retiree's region and the retiree's message
/// is committed — while the durable record still lacks both, so a crash
/// here would strand the identity. The retiree, whose party crossed before
/// the failure, is consumed as [`Retire::Uncertain`]. A retried gossip on a
/// fresh link then re-runs the persist successfully, making the absorption
/// crash-safe: the recovery [`Error::Bookmark`]'s docs promise.
#[test]
fn a_persist_failing_after_the_absorption_leaves_it_live_but_not_durable() {
    let (store, a, r, d) = fleet(vec![false, false, false, true]);
    let r_party = r
        .dangerously_alias_party()
        .expect("a live retiree holds its party");

    let (retire_out, absorb_out) = block_on(retire_into(r, &a));
    assert!(
        matches!(absorb_out, Err(Error::Bookmark(_))),
        "the failed post-absorb persist must surface as Error::Bookmark, got {absorb_out:?}"
    );
    assert!(
        matches!(retire_out, Retire::Uncertain { .. }),
        "a retiree whose party crossed before the abort is consumed, got {retire_out:?}"
    );

    // Live in memory: the absorption committed before the persist failed.
    let a_party = a
        .dangerously_alias_party()
        .expect("a live absorber holds its party");
    assert!(
        a_party.covers(&r_party),
        "the absorber's live party must cover the absorbed region"
    );
    assert!(
        holds_marker(&a),
        "the reconciled content must be committed on the absorber"
    );

    // Absent from the durable record: a crash here would strand the region.
    assert!(
        persisted_record(&store)
            .into_values()
            .flatten()
            .all(|clock| clock.into_parts().0.is_disjoint(&r_party)),
        "the durable record must not yet cover the absorbed region"
    );

    // The promised recovery: a retried gossip on a fresh link re-runs the
    // persist, and the absorption becomes crash-safe. The shared driver
    // asserts both sides' `Ok` and the drained control streams.
    block_on(wire_gossip_async(&a, &d));
    assert!(
        persisted_record(&store)
            .into_values()
            .flatten()
            .any(|clock| clock.into_parts().0.covers(&r_party)),
        "after the retried session, the durable record must cover the absorbed region"
    );
}

/// One persist step *outside* the window: the bookmark write before the
/// absorption fails, and nothing moves on either side.
///
/// The absorber's session-opening update fails, aborting before any
/// reconciliation: the absorber's party, content, and durable record are
/// all byte-unchanged, and the retiree comes back intact
/// ([`Retire::Recovered`]) with its party and message, so a clean retry
/// hands everything over. Paired one persist step below
/// [`a_persist_failing_after_the_absorption_leaves_it_live_but_not_durable`],
/// this pins the window's near edge in persist steps.
#[test]
fn a_persist_failing_before_the_absorption_leaves_both_sides_unchanged() {
    let (store, a, r, _d) = fleet(vec![false, false, true]);
    let r_party = r
        .dangerously_alias_party()
        .expect("a live retiree holds its party");
    let a_party_before = a
        .dangerously_alias_party()
        .expect("a live absorber holds its party");
    let record_before = store.lock().unwrap().clone();

    let (retire_out, absorb_out) = block_on(retire_into(r, &a));
    assert!(
        matches!(absorb_out, Err(Error::Bookmark(_))),
        "the failed pre-absorb persist must surface as Error::Bookmark, got {absorb_out:?}"
    );
    let Retire::Recovered { peer, .. } = retire_out else {
        panic!("a pre-hand-off failure must recover the retiree, got {retire_out:?}");
    };
    let recovered = peer.into_rumors();

    // Unchanged, on every axis: live party, content, and the durable bytes.
    let a_party = a
        .dangerously_alias_party()
        .expect("a live absorber holds its party");
    assert_eq!(
        a_party, a_party_before,
        "the absorber's live party must be unchanged"
    );
    assert!(
        !holds_marker(&a),
        "no content may move in a session that failed before reconciliation"
    );
    assert_eq!(
        *store.lock().unwrap(),
        record_before,
        "the durable record must be byte-unchanged"
    );
    assert_eq!(
        recovered
            .dangerously_alias_party()
            .expect("the recovered retiree holds its party"),
        r_party,
        "the recovered retiree's party must be intact"
    );
    assert!(
        holds_marker(&recovered),
        "the recovered retiree still holds its message"
    );

    // Nothing was lost: a clean retry hands the identity and content over.
    let (retire_out, absorb_out) = block_on(retire_into(recovered, &a));
    assert!(
        matches!(retire_out, Retire::Retired),
        "the retried retire must complete, got {retire_out:?}"
    );
    absorb_out.expect("the retried absorption is clean");
    let a_party = a
        .dangerously_alias_party()
        .expect("a live absorber holds its party");
    assert!(
        a_party.covers(&r_party) && holds_marker(&a),
        "the retried session must hand over the identity and the content"
    );
}
