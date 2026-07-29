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
use crate::link::{Connector, Link, MemoryAcceptor, MemoryConnector, MemoryLink, memory};
use crate::testing::{Quiescence, run_to_quiescence};
use crate::tree::backend::Local;
use crate::tree::{Root, Tree};
use crate::{Error, Inner, Peer, Protocol, Retire};

/// The preamble's wire length: magic(6) + proto_version(2) + network(16) +
/// intent(1). The fault-injection budgets
/// below land cuts on exact protocol boundaries relative to this.
const PREAMBLE_LEN: usize = 25;

/// Insert each of `vals` into `k` as one committed batch.
fn with_messages(k: Peer<u64>, vals: &[u64]) -> Peer<u64> {
    crate::testing::commit(vals.iter().fold(k.batch(), |batch, &v| batch.send(v)));
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

/// Drive `child.retire` against `survivor.gossip` over a memory link, asserting
/// the child retired, and return the (party-grown) survivor.
fn retire_child_into(survivor: Peer<u64>, child: Peer<u64>) -> Peer<u64> {
    pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        let (child_out, survivor_out) =
            tokio::join!(child.retire(&mut a_link), survivor.gossip(&mut b_link),);
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
        let (mut a_link, mut b_link) = memory();
        let (provider_out, boot_out) = tokio::join!(
            provider.gossip(&mut a_link),
            Peer::<u64>::bootstrap().join(&mut b_link),
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
        protocol: survivor.protocol,
        window: survivor.window,
        run_budget: survivor.run_budget,
        inner: watch::Sender::new(Inner {
            party: Some(party_of(&survivor)),
            tree: Tree {
                backend: Local,
                root: Root::default(),
            },
        }),
        bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
        commit: Arc::new(Mutex::new(())),
    };

    // Each side's future owns its link: the absorber rejects the overlap
    // *before* writing its epilogue marker, so only its link drop lets the
    // forged retiree's marker read observe the abort as EOF.
    let (retire_out, survivor_out) = pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        tokio::join!(
            async move { forged.retire(&mut a_link).await },
            async move { survivor.gossip(&mut b_link).await },
        )
    });

    assert!(
        matches!(survivor_out, Err(Error::PartyOverlap)),
        "absorbing an overlapping party must surface PartyOverlap, got {survivor_out:?}"
    );
    // The absorber aborted pre-marker, so the forged retiree's party is in
    // limbo: `Uncertain`, never a false `Retired`.
    assert!(
        matches!(retire_out, Retire::Uncertain { .. }),
        "an unconfirmed in-flight party must consume the retiree, got {retire_out:?}"
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
        let (mut a_link, mut b_link) = memory();
        tokio::join!(survivor.retire(&mut a_link), child.retire(&mut b_link))
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
/// The budget is shared across every fused writer of one link — the control
/// half and each data stream — so the cut lands at a chosen point in the
/// session's total outgoing byte count, wherever that byte travels. Reads
/// are not budgeted; the counterparty observes the cut as EOF once the
/// session's link drops.
///
/// [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
struct Fuse<W> {
    inner: W,
    remaining: Arc<std::sync::Mutex<usize>>,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let mut remaining = this.remaining.lock().expect("fuse budget lock");
        if *remaining == 0 {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "fuse blown",
            )));
        }
        // Admit at most the remaining budget; the writer's retry of the
        // unwritten tail then trips the exhausted fuse above.
        let admitted = buf.len().min(*remaining);
        match Pin::new(&mut this.inner).poll_write(cx, &buf[..admitted]) {
            Poll::Ready(Ok(n)) => {
                *remaining -= n;
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

/// The wire length of `retiree`'s complete greeting — the causal-version
/// frame plus the root-fan listing frame — so a [`Fuse`] budget can land on
/// an exact protocol boundary.
fn greeting_frame_len(retiree: &Peer<u64>) -> usize {
    use crate::tree::mirror::streaming::{self, Local, materialized};

    let root: streaming::Root<Local, u64> = retiree.inner.borrow().tree.clone().root;
    let fan = pollster::block_on(materialized::greeting_fan(&Local, root.root))
        .unwrap_or_else(|never| match never {});
    let listing = borsh::to_vec(&materialized::fan_listing(&fan)).expect("a listing serializes");
    crate::tree::mirror::framing::LENGTH_HEADER_LEN
        + crate::tree::mirror::framing::GREETING_SIZE_WORDS_LEN
        + retiree.snapshot().latest().as_bytes().len()
        + crate::tree::mirror::framing::LENGTH_HEADER_LEN
        + listing.len()
}

/// The wire length of `retiree`'s trailing party frame, so a [`Fuse`] budget
/// can land on an exact protocol boundary.
fn party_frame_len(retiree: &Peer<u64>) -> usize {
    crate::tree::mirror::framing::LENGTH_HEADER_LEN
        + borsh::to_vec(&party_of(retiree))
            .expect("a party serializes")
            .len()
}

/// A connector whose opened streams draw on the link's shared fuse budget.
#[derive(Clone)]
struct FusedConnector {
    inner: MemoryConnector,
    remaining: Arc<std::sync::Mutex<usize>>,
}

impl Connector for FusedConnector {
    type Tx = Fuse<tokio::io::DuplexStream>;

    async fn connect(&self) -> std::io::Result<Self::Tx> {
        let inner = self.inner.connect().await?;
        Ok(Fuse {
            inner,
            remaining: Arc::clone(&self.remaining),
        })
    }
}

/// Fuse one link's whole outgoing side to a shared byte budget.
fn fused_link(
    link: MemoryLink,
    budget: usize,
) -> Link<tokio::io::DuplexStream, Fuse<tokio::io::DuplexStream>, FusedConnector, MemoryAcceptor> {
    let remaining = Arc::new(std::sync::Mutex::new(budget));
    let parts = link.into_parts();
    crate::link::LinkParts {
        control_read: parts.control_read,
        control_write: Fuse {
            inner: parts.control_write,
            remaining: Arc::clone(&remaining),
        },
        connector: FusedConnector {
            inner: parts.connector,
            remaining,
        },
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

/// Drive `retiree.retire` against `peer.gossip` over a link whose
/// retiree-side outgoing bytes are fused to `budget`.
///
/// Each side's link is owned by its future, so the failing side's drop
/// surfaces as EOF to the other rather than deadlocking the join. Returns both
/// outcomes.
fn severed_retire(
    retiree: Peer<u64>,
    peer: &mut Peer<u64>,
    budget: usize,
) -> (Retire<u64>, Result<(), Error>) {
    pollster::block_on(async move {
        let (a_link, mut b_link) = memory();
        tokio::join!(
            async move {
                let mut a_link = fused_link(a_link, budget);
                retiree.retire(&mut a_link).await
            },
            async move { peer.gossip(&mut b_link).await.map(|_gossiped| ()) },
        )
    })
}

/// A session severed during the reconciliation descent costs nothing.
///
/// The trailing party frame was provably never sent, so the retiree comes back
/// intact ([`Retire::Recovered`]) — same content, still-live party — and a
/// subsequent clean retire of the recovered set reconstitutes the seed's whole
/// identity space. This pins retire's fork-last ordering: the identity is
/// never in limbo during the descent.
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

/// A session severed on the retiree's epilogue marker itself — the last byte
/// of a clean retire session — still consumes the retiree.
///
/// The party frame was delivered whole, so the absorber may well hold the
/// identity: [`Retire::Recovered`] here would let the same identity live
/// twice, and [`Retire::Retired`] would overstate (the peer's commit was
/// never confirmed). The only sound outcome is [`Retire::Uncertain`], and
/// its error is the distinguished post-commit [`Error::Epilogue`]: the
/// epilogue's failure return must preserve the retire outcome rather than
/// mapping back to a recovery.
#[test]
fn severed_epilogue_marker_is_uncertain() {
    let survivor = Peer::<u64>::seed();
    let (mut survivor, child) = bootstrap_from(survivor);

    // Both empty and converged, so the retiree's outgoing bytes are exactly
    // preamble + greeting + party frame + epilogue marker. The budget is
    // that full clean session minus one byte: everything through the party
    // frame is delivered, and the marker write is the write that fails.
    let budget = PREAMBLE_LEN + greeting_frame_len(&child) + party_frame_len(&child);
    let (child_out, peer_out) = severed_retire(child, &mut survivor, budget);

    let Retire::Uncertain { error } = child_out else {
        panic!("a failure on the epilogue marker must consume the retiree, got {child_out:?}");
    };
    assert!(
        matches!(error, Error::Epilogue(_)),
        "the post-hand-off failure is the distinguished post-commit error, got {error:?}"
    );
    // The absorber committed the party before its own epilogue read hit the
    // severed wire: it reports the same post-commit residue.
    assert!(
        matches!(peer_out, Err(Error::Epilogue(_))),
        "the absorber's confirmation of the retiree's completion fails, got {peer_out:?}"
    );
    drop(survivor);
}

/// A gossip session cancelled mid-flight poisons the link: the next session
/// on it fails fast with [`Error::LinkPoisoned`], before any wire traffic,
/// instead of misreading the interrupted session's leftover control bytes.
#[test]
fn a_cancelled_session_poisons_the_link_for_gossip() {
    let survivor = Peer::<u64>::seed();
    let (_survivor, child) = bootstrap_from(survivor);
    let (mut a_link, _b_link) = memory();

    // Cancel a session mid-flight, deterministically: the counterparty
    // never drives its end, so the session stalls awaiting the peer's
    // preamble and the bounded-poll harness reports the stall — dropping
    // (cancelling) the session future on its way out.
    assert_eq!(
        run_to_quiescence(child.gossip(&mut a_link)).err(),
        Some(Quiescence::Stalled),
    );

    // The fail-fast needs no counterparty at all: it resolves before any
    // wire traffic, which the closed-world harness itself proves.
    let retry = run_to_quiescence(child.gossip(&mut a_link)).expect("fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "a poisoned link must fail the next gossip fast, got {retry:?}"
    );
}

/// A retire attempted on a poisoned link recovers the peer intact with
/// [`Error::LinkPoisoned`]: the fail-fast happens before any wire traffic,
/// so the identity was never at risk and remains free to retire elsewhere.
#[test]
fn retire_on_a_poisoned_link_recovers_the_peer() {
    let survivor = Peer::<u64>::seed();
    let (survivor, child) = bootstrap_from(survivor);
    let (mut a_link, _b_link) = memory();

    // Poison the link: cancel a gossip session stalled on its silent peer.
    assert_eq!(
        run_to_quiescence(child.gossip(&mut a_link)).err(),
        Some(Quiescence::Stalled),
    );

    let out = run_to_quiescence(child.retire(&mut a_link)).expect("fail-fast needs no peer");
    let Retire::Recovered { peer, error } = out else {
        panic!("retire on a poisoned link must recover the peer, got {out:?}");
    };
    assert!(
        matches!(error, Error::LinkPoisoned),
        "the fail-fast error is the poison diagnosis, got {error:?}"
    );

    // The recovered peer is genuinely intact: over a fresh link it retires
    // cleanly, reconstituting the seed's whole id-space in the survivor.
    let survivor = retire_child_into(survivor, peer);
    assert_eq!(
        party_of(&survivor),
        Party::seed(),
        "the recovered peer's clean retire must reconstitute the whole id-space",
    );
}

/// A session severed on the trailing party frame itself leaves delivery
/// irreducibly uncertain.
///
/// The retiree cannot know whether the peer received its
/// party, so it is consumed ([`Retire::Uncertain`]) rather than risk
/// duplicating the region by surviving alongside a delivered copy.
#[test]
fn severed_party_frame_is_uncertain() {
    let survivor = Peer::<u64>::seed();
    let (mut survivor, child) = bootstrap_from(survivor);

    // Both empty and converged, so the retiree's outgoing bytes are exactly
    // preamble + greeting + party frame + epilogue marker. The fuse admits
    // the first two to the byte, so the party frame is the write that fails
    // (and the trailing marker is never reached).
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

/// An uncontained supply crossing a real peer session surfaces through
/// [`Rumors::gossip`](crate::Rumors::gossip) as its typed violation.
///
/// The error is [`Error::Mirror`] carrying `UncontainedSupply`; the
/// replica's content is untouched, and the link is poisoned so the next
/// session on it fails fast with [`Error::LinkPoisoned`].
///
/// The peer-tier parity leg of the containment tripwires: the same
/// rejection the mirror tiers pin in process and over their wires, here
/// observed at the public API. The poisoned store is forged through the
/// local `Tree::join` seam — no session tripwire guards an in-memory join —
/// the residency mechanism the tree tier pins in
/// `escaped_version_defeats_redaction_in_a_poisoned_store`.
#[test]
fn uncontained_supply_fails_gossip_and_poisons_the_link() {
    use crate::error::{MaterializedError, MaterializedViolation};
    use crate::message::Message;
    use crate::tree::mirror::Error as MirrorError;

    let survivor = Peer::<u64>::seed();
    let (survivor, child) = bootstrap_from(survivor);

    // Honest, causally concurrent divergence on both sides, so the session
    // descends instead of short-circuiting on equal declared versions.
    let receiver = with_messages(survivor, &[1]);
    let poisoned = with_messages(child, &[2]);

    // Poison the serving peer's store: the escaped leaf plants above the
    // declared ceiling, which stays honest — the store an authorized but
    // nonconforming implementation would then serve.
    let base = poisoned.inner.borrow().tree.latest().clone();
    let (escaped_root, _, escaped) =
        crate::tree::arb::poisoned_root(&party_of(&poisoned), &base, Message::new(0u64));
    poisoned.inner.send_modify(|inner| {
        inner.tree.join_now(Tree {
            backend: Local,
            root: escaped_root,
        });
    });
    assert!(
        !crate::tree::mirror::contained(&escaped, poisoned.inner.borrow().tree.latest()),
        "the planted leaf's version escapes the declared ceiling",
    );

    let receiver = receiver.into_rumors();
    let before = receiver.snapshot().hash();

    // The receiving side's own materialized participant diagnoses the
    // violation mid-session, so the poisoned counterparty never reaches a
    // session boundary; racing the two sides lets the receiver's typed
    // error surface without waiting on the abandoned peer.
    let (mut a_link, mut b_link) = memory();
    let receiver_out = run_to_quiescence(async {
        tokio::select! {
            out = receiver.gossip(&mut a_link) => out,
            out = poisoned.gossip(&mut b_link) => {
                panic!("the poisoned side must not complete a session, got {out:?}")
            }
        }
    })
    .expect("the rejecting session becomes quiescent");
    assert!(
        matches!(
            receiver_out,
            Err(Error::Mirror(MirrorError::Client(
                MaterializedError::Violation(MaterializedViolation::UncontainedSupply)
            ))),
        ),
        "the receiving peer rejects the escaped leaf with its typed violation, got {receiver_out:?}",
    );

    // Nothing of the failed session reached the replica.
    assert_eq!(
        receiver.snapshot().hash(),
        before,
        "the replica's content is unchanged by the rejected session",
    );

    // The failed session poisoned the link: the next session on it fails
    // fast, before any wire traffic — no counterparty is even present.
    let retry = run_to_quiescence(receiver.gossip(&mut a_link)).expect("fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "a poisoned link must fail the next gossip fast, got {retry:?}",
    );
}

/// The root-hash meter is alive: a root-hash read through the public
/// snapshot surface moves the per-thread counter by exactly one.
///
/// The liveness leg for the two commit-path pins below — a ceiling asserted
/// over a counter that stopped counting would pass vacuously.
#[test]
fn root_hash_read_meter_is_live() {
    let peer = with_messages(Peer::<u64>::seed(), &[1]);
    let before = crate::tree::meter::root_hash_reads();
    let _ = peer.snapshot().hash();
    assert_eq!(
        crate::tree::meter::root_hash_reads() - before,
        1,
        "one snapshot hash is exactly one root-hash read",
    );
}

/// Pins the root-hash reads a batch commit performs: zero.
///
/// The commit decides "did the tree change?" from the changed flag
/// [`Tree::react`] returns, so no root hash is read — and none *forced*
/// over the freshly rebuilt, memo-less copy-on-write spine — anywhere in
/// the commit's phases. The commit future is driven to completion on this
/// thread (`testing::commit` uses `pollster`, no spawns), so the bracketed
/// count is exact.
#[test]
fn batch_commit_root_hash_reads() {
    let peer = with_messages(Peer::<u64>::seed(), &[1, 2]);
    let before = crate::tree::meter::root_hash_reads();
    crate::testing::commit(peer.batch().send(3));
    assert_eq!(
        crate::tree::meter::root_hash_reads() - before,
        0,
        "a batch commit reads no root hash in its critical section",
    );
}

/// Pins the root-hash reads a plain gossip session performs, across both
/// sides: zero.
///
/// Each side's write-back commit decides "did the tree change?" from the
/// changed flag [`Tree::join`] returns, so no root hash is read — and none
/// *forced* over the freshly merged, memo-less spine — inside the watch
/// critical section; the mirror exchange itself hashes nodes, never the
/// root through [`Tree::hash`]. Both peers run on this thread (`pollster`
/// drives the joined futures with no spawns), so the bracketed count is
/// exact.
#[test]
fn gossip_session_root_hash_reads() {
    let provider = with_messages(Peer::<u64>::seed(), &[1, 2, 3]);
    let (provider, joiner) = bootstrap_from(provider);

    // Honest divergence on both sides, so the session has real work: each
    // side both provides and absorbs content.
    let provider = with_messages(provider, &[10]);
    let joiner = with_messages(joiner, &[20]);

    let before = crate::tree::meter::root_hash_reads();
    pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        let (provider_out, joiner_out) =
            tokio::join!(provider.gossip(&mut a_link), joiner.gossip(&mut b_link));
        provider_out.expect("provider gossip");
        joiner_out.expect("joiner gossip");
    });
    assert_eq!(
        crate::tree::meter::root_hash_reads() - before,
        0,
        "a gossip session reads no root hash in either side's commit",
    );
}

/// The gossip fork section waits on the commit lock.
///
/// Party linearity's first leg: a committer that stamped its actions from
/// the pre-fork party but has not yet published must exclude any party
/// *shrink*, or the donated fork's new owner could mint the coordinates
/// the in-flight commit is about to publish (the classification lives at
/// `Peer::commit`). This pins the mechanism, not just the stall: while
/// the commit lock is parked — exactly a stalled `Batch::commit` — a
/// session serving a bootstrap must not have *forked the party yet* (the
/// donation happens inside the lock's critical section), and releasing
/// the lock lets the donation and the session complete.
#[tokio::test]
async fn fork_section_waits_on_the_commit_lock() {
    use futures::FutureExt as _;

    let provider = Peer::<u64>::seed();
    let parked = Arc::clone(&provider.commit);
    let provider = provider.into_rumors();
    crate::testing::commit(provider.batch().send(7));

    // Park a committer: hold the commit lock as a commit stalled between
    // its prep and publish would.
    let guard = parked.lock_owned().await;
    let before = provider
        .dangerously_alias_party()
        .expect("a live set holds its party");

    // A newcomer bootstraps from the provider: serving the join *forks*
    // the provider's party, and that fork happens inside the commit
    // lock's critical section.
    let (mut near, mut far) = memory();
    let session = async {
        tokio::join!(
            Peer::<u64>::bootstrap().join(&mut near),
            provider.gossip(&mut far),
        )
    };
    let mut session = std::pin::pin!(session);

    // Drive the joint session: it must park at the provider's fork
    // section with the party still whole — the donation must not be
    // sliced out from under the stalled committer.
    for _ in 0..256 {
        assert!(
            session.as_mut().now_or_never().is_none(),
            "the fork section must wait for the parked commit lock",
        );
    }
    let during = provider
        .dangerously_alias_party()
        .expect("a live set holds its party");
    assert_eq!(
        before, during,
        "no fork leaves the party while the commit lock is held",
    );

    // Release the committer; the donation proceeds and the session
    // completes, narrowing the provider's party.
    drop(guard);
    let (joined, served) = session.await;
    served.expect("the provider serves the join");
    joined
        .expect("the join session completes")
        .expect("the provider is established");
    let after = provider
        .dangerously_alias_party()
        .expect("a live set holds its party");
    assert_ne!(before, after, "the released session donated a fork");
}

/// A V1 session on a storage-owning backend declines before any wire
/// traffic.
///
/// The frozen alternating protocol runs on resident nodes, so a peer whose
/// store owns its nodes must fail [`Protocol::V1`] sessions with
/// [`Error::ProtocolUnsupported`] — and fail them *pre-wire*. One-sided
/// completion is the pre-wire witness (an error that needed any round trip
/// would park awaiting the absent counterparty), and a V1 counterparty
/// that later drives the far end finds nothing to read.
#[tokio::test]
async fn v1_declines_a_storage_backed_peer_before_the_wire() {
    let peer = Peer::<u64, NoBookmark, crate::conformance::backend::tests::Materializing>::seed_in(
        crate::conformance::backend::tests::Materializing,
    )
    .protocol(Protocol::V1)
    .into_rumors();
    let (mut near, mut far) = memory();

    let declined = run_to_quiescence(peer.gossip(&mut near)).expect("declines without a peer");
    assert!(
        matches!(
            declined,
            Err(Error::ProtocolUnsupported {
                protocol: Protocol::V1
            })
        ),
        "a storage-backed V1 session must decline as unsupported, got {declined:?}",
    );

    // Nothing crossed: a V1 counterparty on the far end writes its own
    // preamble and then parks reading one that never arrived.
    let counterparty = Peer::<u64>::seed().protocol(Protocol::V1).into_rumors();
    let stalled = run_to_quiescence(counterparty.gossip(&mut far));
    assert!(
        matches!(stalled, Err(Quiescence::Stalled)),
        "the declined side must have written nothing",
    );
}

/// A V1 bootstrap on a storage-owning backend declines before any wire
/// traffic.
///
/// The claimant-side twin of
/// [`v1_declines_a_storage_backed_peer_before_the_wire`]: the entry gate
/// fires in `bootstrap` exactly as in `gossip`, one-sided, before the
/// preamble.
#[tokio::test]
async fn v1_declines_a_storage_backed_bootstrap_before_the_wire() {
    let (mut near, _far) = memory();
    let declined = run_to_quiescence(
        Peer::<u64>::bootstrap()
            .backend(crate::conformance::backend::tests::Materializing)
            .protocol(Protocol::V1)
            .join(&mut near),
    )
    .expect("declines without a peer");
    assert!(
        matches!(
            declined,
            Err(Error::ProtocolUnsupported {
                protocol: Protocol::V1
            })
        ),
        "a storage-backed V1 bootstrap must decline as unsupported",
    );
}

/// A cancelled absorb recovers the retiree's donated identity.
///
/// The window: the retiring peer's party has crossed the wire and rides
/// the absorber's install-or-recover guard while the session parks at the
/// commit lock behind a stalled committer. Cancelling the session there
/// must not strand the donation: the guard's drop joins it into the
/// replica's party (pure growth, lock-free by the classification at
/// `Peer::commit`).
#[tokio::test]
async fn cancelled_absorb_recovers_the_retirees_identity() {
    use futures::FutureExt as _;

    let absorber = Peer::<u64>::seed();
    let lock = Arc::clone(&absorber.commit);
    let absorber = absorber.into_rumors();

    // The retiree joins the absorber's universe first: only a fellow
    // member can retire into it.
    let (mut a, mut b) = memory();
    let (joined, served) = tokio::join!(
        Peer::<u64>::bootstrap().join(&mut a),
        absorber.gossip(&mut b),
    );
    served.expect("the absorber serves the join");
    let retiree = joined
        .expect("the join session completes")
        .expect("the retiree is established");

    let before = absorber
        .dangerously_alias_party()
        .expect("a live set holds its party");

    // Park the commit lock before the session starts: the absorber's fork
    // section queues on it first.
    let guard = lock.clone().lock_owned().await;

    let (mut far, mut near) = memory();
    // The block scopes the session future: `pin!` pins it in a hidden
    // local, so only leaving the block genuinely drops (cancels) it —
    // dropping the `Pin<&mut _>` alone would not.
    {
        // `unconstrained`: the drive below hand-polls far past tokio's
        // cooperative budget, which would otherwise freeze every tokio
        // primitive mid-session and fake a park.
        let session = tokio::task::unconstrained(async {
            tokio::join!(absorber.gossip(&mut far), retiree.retire(&mut near))
        });
        let mut session = std::pin::pin!(session);

        // Drive to the absorber's fork section, parked on the held lock.
        for _ in 0..64 {
            assert!(
                session.as_mut().now_or_never().is_none(),
                "the session must still be in flight",
            );
        }

        // Queue a reclaim behind the parked fork section *before* releasing
        // the guard: the mutex is queue-fair, so the lock passes fork section
        // → reclaim, and the absorber's *install* — many wire round trips
        // later — can only queue behind the reclaim. A single session poll
        // can cross the whole wire exchange, so the reclaim must already be
        // in the queue when the fork section releases.
        let mut reclaim = Box::pin(lock.clone().lock_owned());
        assert!(
            (&mut reclaim).now_or_never().is_none(),
            "the reclaim parks behind the held guard",
        );
        drop(guard);
        let mut reclaimed = None;
        for _ in 0..64 {
            assert!(
                session.as_mut().now_or_never().is_none(),
                "the install queues behind the reclaim, never completing first",
            );
            if let Some(guard) = (&mut reclaim).now_or_never() {
                reclaimed = Some(guard);
                break;
            }
        }
        let _guard = reclaimed.expect("the fork section releases the lock");

        // Drive until the retiree's party has crossed and the absorber parks
        // at the install's lock acquisition, donation riding the guard. The
        // count is deliberately generous: every poll past the park is a cheap
        // no-op, and the session must stay pending under all of them.
        for _ in 0..65_536 {
            assert!(
                session.as_mut().now_or_never().is_none(),
                "the install must wait for the reclaimed commit lock",
            );
        }
        let during = absorber
            .dangerously_alias_party()
            .expect("a live set holds its party");
        assert_eq!(
            before, during,
            "no donation installs while the lock is held"
        );

        // Leaving the block cancels the session with the donation in
        // flight: the guard must recover it into the replica rather than
        // strand it.
    }
    let after = absorber
        .dangerously_alias_party()
        .expect("a live set holds its party");
    assert_ne!(
        before, after,
        "the cancelled absorb must recover the donated identity",
    );
}
