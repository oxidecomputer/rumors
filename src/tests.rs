//! Crate-level unit tests for party mechanics that the public integration tests
//! can't reach.
//!
//! They need either a *forged* `Peer` (private fields) or to read a `Peer`'s
//! [`Party`] and compare it to [`Party::seed`]. Both require in-crate access,
//! so they live here rather than in `tests/`.

use std::pin::Pin;
use std::task::{Context, Poll};

use before::Party;
use std::sync::Arc;
use tokio::io::AsyncWrite;

use tokio::sync::{Mutex, watch};

use crate::bookmark::{Bookmarked, NoBookmark};
use crate::tree::{Root, Tree};
use crate::{Error, Inner, Peer, Retire};

/// Capacity for the in-memory duplex pipe; every retiree here is already
/// converged with its absorber, so the sessions move no content and the exact
/// size is immaterial.
const DUPLEX_BUF: usize = 64 * 1024;

/// The preamble frame's wire length: length prefix(4) + magic(6) +
/// proto_version(2) + network(16) + intent(1). The fault-injection budgets
/// below land cuts on exact protocol boundaries relative to this.
const PREAMBLE_LEN: usize = 4 + 25;

/// Insert each of `vals` into `k` as one committed batch.
fn with_messages(k: Peer<u64>, vals: &[u64]) -> Peer<u64> {
    let mut batch = k.batch();
    for &v in vals {
        batch.send(v);
    }
    drop(batch);
    k
}

/// Read a `Peer`'s party for assertions.
fn party_of(k: &Peer<u64>) -> Party {
    k.inner
        .borrow()
        .party
        .as_ref()
        .expect("a live Peer holds its party")
        .dangerously_alias()
}

/// Drive `child.retire` against `survivor.gossip` over a duplex pipe, asserting
/// the child retired, and return the (party-grown) survivor.
fn retire_child_into(survivor: Peer<u64>, child: Peer<u64>) -> Peer<u64> {
    pollster::block_on(async {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (child_out, survivor_out) = tokio::join!(
            child.retire(&mut a_r, &mut a_w),
            survivor.gossip(&mut b_r, &mut b_w),
        );
        assert!(
            matches!(child_out, Retire::Retired),
            "the survivor absorbs the child",
        );
        survivor_out.expect("survivor gossip");
        survivor
    })
}

/// Drive `provider.gossip` against a fresh `bootstrap`, returning the
/// post-serve provider and the bootstrapped peer.
fn bootstrap_from(provider: Peer<u64>) -> (Peer<u64>, Peer<u64>) {
    pollster::block_on(async {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (provider_out, boot_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Peer::<u64>::bootstrap(&mut b_r, &mut b_w),
        );
        provider_out.expect("provider gossip");
        (
            provider,
            boot_out
                .expect("bootstrap")
                .expect("provider served the bootstrap"),
        )
    })
}

/// A peer that absorbs a retiree whose party **overlaps** its own rejects it
/// with [`Error::PartyOverlap`] rather than corrupting its clock.
///
/// A correct universe never produces this (live parties are always disjoint);
/// we forge it with [`Party::dangerously_alias`] — a copy of the absorber's
/// *exact* region — to model a buggy or malicious peer. The overlap is detected
/// by the absorbing `party.join`, the only place it can arise.
#[test]
fn overlapping_retiree_party_is_rejected() {
    let survivor = Peer::<u64>::seed();

    // Forge a retiree sharing the survivor's network and its *exact* party
    // region (not a disjoint fork), with an empty tree so its version equals the
    // survivor's and the survivor takes the absorb branch.
    let forged = Peer::<u64> {
        network: survivor.network,
        inner: watch::Sender::new(Inner {
            party: Some(party_of(&survivor)),
            tree: Tree {
                root: Root::default(),
            },
        }),
        bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
        marker: std::marker::PhantomData,
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
/// [`Party::seed`] (`"1"`, the whole interval).
///
/// Each bootstrap hands a child a disjoint slice of the seed's region; each
/// `retire` hands a slice back, and a leak anywhere would leave the reunited
/// party short of the whole.
#[test]
fn retiring_all_forks_reconstitutes_the_seed_party() {
    let survivor = Peer::<u64>::seed();
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
        party_of(&survivor),
        Party::seed(),
        "retiring all forks back must reconstitute the whole id-space",
    );
}

/// Bootstrap mints a fresh party by forking the provider's; retiring that peer
/// back must reclaim exactly that minted region.
///
/// Provider with real content, bootstrap (a wire fork), then retire the
/// newcomer home: the provider's party normalizes back to [`Party::seed`],
/// proving the bootstrap hand-off and the retire commit are jointly leak-free.
#[test]
fn bootstrap_then_retire_reconstitutes_the_seed_party() {
    let provider = with_messages(Peer::<u64>::seed(), &[1, 2, 3]);

    let (provider, newcomer) = bootstrap_from(provider);
    // The newcomer pulled all content and is a causal fork (equal version), so
    // the provider reflexively dominates it and absorbs it on retire.
    let provider = retire_child_into(provider, newcomer);

    assert_eq!(
        party_of(&provider),
        Party::seed(),
        "retiring a bootstrapped peer back must reconstitute the whole id-space",
    );
}

/// A retiree whose counterparty is *also* retiring is declined cleanly after
/// the preamble.
///
/// Both come back intact, parties untouched, and a clean retire of one into the
/// other afterwards still reconstitutes the whole id-space.
#[test]
fn mutual_retire_declines_both() {
    let survivor = Peer::<u64>::seed();
    let (survivor, child) = bootstrap_from(survivor);

    let (a_out, b_out) = pollster::block_on(async {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        tokio::join!(
            survivor.retire(&mut a_r, &mut a_w),
            child.retire(&mut b_r, &mut b_w),
        )
    });
    let (Retire::Declined { peer: survivor }, Retire::Declined { peer: child }) = (a_out, b_out)
    else {
        panic!("mutual retirement must decline both sides intact");
    };

    let survivor = retire_child_into(survivor, child);
    assert_eq!(
        party_of(&survivor),
        Party::seed(),
        "a declined retire must leave both parties whole",
    );
}

// ---- fault injection: severing the wire mid-retire ------------------------

/// An [`AsyncWrite`] wrapper that forwards writes until a byte budget is
/// exhausted, then fails every write with [`BrokenPipe`]: a deterministic
/// stand-in for a connection severed at a chosen point in the session.
///
/// Reads are not budgeted; the counterparty observes the cut as EOF once the
/// session's halves drop.
///
/// [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
struct Fuse<W> {
    inner: W,
    remaining: usize,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        if this.remaining == 0 {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "fuse blown",
            )));
        }
        // Admit at most the remaining budget; the writer's retry of the
        // unwritten tail then trips the exhausted fuse above.
        let admitted = buf.len().min(this.remaining);
        match Pin::new(&mut this.inner).poll_write(cx, &buf[..admitted]) {
            Poll::Ready(Ok(n)) => {
                this.remaining -= n;
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

/// The wire length of `retiree`'s greeting frame, so a [`Fuse`] budget can
/// land on an exact protocol boundary.
///
/// The frame is a 4-byte length prefix + borsh-encoded
/// [`Handshake`](crate::tree::mirror::message::Handshake) body — since the
/// preamble rework, the version alone.
fn greeting_frame_len(retiree: &Peer<u64>) -> usize {
    let greeting = crate::tree::mirror::alternating::message::Handshake {
        version: retiree.snapshot().latest().clone(),
    };
    4 + borsh::to_vec(&greeting).expect("serialize greeting").len()
}

/// Drive `retiree.retire` against `peer.gossip` over a duplex whose
/// retiree-side writer is fused to `budget` bytes.
///
/// Each side's I/O halves are owned by its future, so the failing side's drop
/// surfaces as EOF to the other rather than deadlocking the join. Returns both
/// outcomes.
fn severed_retire(
    retiree: Peer<u64>,
    peer: &mut Peer<u64>,
    budget: usize,
) -> (Retire<u64>, Result<(), Error>) {
    pollster::block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (a_r, a_w) = tokio::io::split(a_side);
        let (b_r, b_w) = tokio::io::split(b_side);
        tokio::join!(
            async move {
                let mut a_r = a_r;
                let mut a_w = Fuse {
                    inner: a_w,
                    remaining: budget,
                };
                retiree.retire(&mut a_r, &mut a_w).await
            },
            async move {
                let (mut b_r, mut b_w) = (b_r, b_w);
                peer.gossip(&mut b_r, &mut b_w).await
            },
        )
    })
}

/// A session severed during the reconciliation descent costs nothing.
///
/// The trailing party frame was provably never sent, so the retiree comes back
/// intact ([`Retire::Recovered`]) — same content, still-live party — and a
/// subsequent clean retire of the recovered set reconstitutes the seed's whole
/// id-space. This pins retire's fork-last ordering: the id-region is never in
/// limbo during the descent.
#[test]
fn severed_descent_recovers_the_retiree() {
    let survivor = Peer::<u64>::seed();
    let (mut survivor, child) = bootstrap_from(survivor);
    // Diverge: the child holds content the survivor lacks, so the retire
    // session must descend, and its frames overflow the fuse's slack.
    let child = with_messages(child, &(0..32).collect::<Vec<u64>>());
    let hash = child.snapshot().hash();

    // Admit exactly the preamble and greeting, plus a slack smaller than any
    // descent frame: the cut provably lands after the handshake completes and
    // before the party hand-off.
    let budget = PREAMBLE_LEN + greeting_frame_len(&child) + 16;
    let (child_out, peer_out) = severed_retire(child, &mut survivor, budget);

    assert!(
        peer_out.is_err(),
        "the severed wire fails the absorbing side too"
    );
    let Retire::Recovered { peer: child, .. } = child_out else {
        panic!("a pre-hand-off failure must recover the retiree, got {child_out:?}");
    };
    assert_eq!(
        child.snapshot().hash(),
        hash,
        "the recovered retiree's content is intact"
    );

    // The recovered retiree still owns its live region: a clean retire (whose
    // gossip round re-carries the divergent content) succeeds, and the
    // survivor's party normalizes back to the whole id-space — nothing leaked.
    let survivor = retire_child_into(survivor, child);
    assert_eq!(
        party_of(&survivor),
        Party::seed(),
        "a severed-then-retried retire must reconstitute the whole id-space",
    );
}

/// A session severed on the trailing party frame itself is the irreducible
/// two-generals window.
///
/// The retiree cannot know whether the peer received its
/// party, so it is consumed ([`Retire::Uncertain`]) rather than risk
/// duplicating the region by surviving alongside a delivered copy.
#[test]
fn severed_party_frame_is_uncertain() {
    let survivor = Peer::<u64>::seed();
    let (mut survivor, child) = bootstrap_from(survivor);

    // Both empty and converged, so the session is exactly preamble + greeting
    // + party frame. The fuse admits the first two to the byte, so the party
    // frame is the write that fails.
    let budget = PREAMBLE_LEN + greeting_frame_len(&child);
    let (child_out, peer_out) = severed_retire(child, &mut survivor, budget);

    assert!(
        matches!(child_out, Retire::Uncertain { .. }),
        "a failure on the party frame itself must consume the retiree, got {child_out:?}"
    );
    assert!(
        peer_out.is_err(),
        "the absorber never receives the promised party frame"
    );
    drop(survivor);
}
