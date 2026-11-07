//! Crate-level tests that need private internals.
//!
//! These cover party ownership across joining and leaving, failure at exact
//! retirement wire boundaries, protocol rejection through a peer session,
//! the network secret's delivery and redaction, and root-hash metering.

use std::sync::Arc;

use before::Party;
use tokio::io::{BufReader, DuplexStream};
use tokio::sync::watch;

use crate::bookmark::{Bookmark, BookmarkHandle, NoBookmark};
use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::network::NetworkSecret;
use crate::observe::{
    Direction, Observer, SessionInfo, SessionObserver, StreamId, StreamInfo, StreamObserver,
};
use crate::peer::SessionSettings;
use crate::testing::property::{
    FLEET_DIVISOR, LOCAL_DIVISOR, NetworkSeed, SESSION_DIVISOR, distinct_network_seeds,
    divided_proptest, network_seed,
};
use crate::testing::{
    AdversarialRead, AdversarialWrite, IoFault, IoFaultUnit, IoOperation, IoPlan, IoSide,
    MEMORY_CAPACITY, PREAMBLE_LEN, memory_connection, run_to_quiescence, wrap_io,
};
use crate::tree::{Root, Tree};
use crate::{Connection, Error, Inner, Peer, Retire};

/// Insert each of `vals` into `k` as one committed batch.
fn with_messages(k: Peer<u64>, vals: &[u64]) -> Peer<u64> {
    k.send_all(vals.iter().copied())
        .expect("flat test payloads are within any depth limit");
    k
}

/// Read a `Peer`'s party for assertions.
fn party_of<B: Bookmark>(k: &Peer<u64, B>) -> Party {
    k.inner.borrow().party.dangerously_alias()
}

/// Drive `child.retire` against `survivor.gossip` over a memory connection, asserting
/// the child retired, and return the (party-grown) survivor.
fn retire_child_into(survivor: Peer<u64>, child: Peer<u64>) -> Peer<u64> {
    pollster::block_on(async {
        let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        let (child_out, survivor_out) = tokio::join!(
            child.retire(&mut a_end.read, &mut a_end.write),
            survivor.gossip_once(&mut b_end.read, &mut b_end.write),
        );
        assert!(
            matches!(
                child_out,
                Retire::Retired {
                    connection: Connection::Reusable { .. }
                }
            ),
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
        let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        let (provider_out, boot_out) = tokio::join!(
            provider.gossip_once(&mut a_end.read, &mut a_end.write),
            Peer::<u64>::bootstrap().join(&mut b_end.read, &mut b_end.write),
        );
        provider_out.expect("provider gossip");
        (
            provider,
            (match boot_out {
                crate::Joined::Joined {
                    peer,
                    connection: crate::Connection::Reusable { .. },
                } => peer,
                _ => panic!("provider served the bootstrap"),
            }),
        )
    })
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A peer that absorbs a retiree whose party **overlaps** its own rejects it
    /// with [`Error::Protocol`] rather than corrupting its clock.
    ///
    /// A correct universe never produces this (live parties are always disjoint);
    /// we forge it with [`Party::dangerously_alias`] to check that absorption
    /// detects a peer's violation of party disjointness.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn overlapping_retiree_party_is_rejected(network in network_seed()) {
        let survivor = network.peer::<u64>();

        // Forge a retiree sharing the survivor's network and its *exact* party
        // region (not a disjoint fork), with an empty tree so its version equals the
        // survivor's and the survivor takes the absorb branch.
        let forged = Peer::<u64> {
            codec: PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
            keys: survivor.keys.clone(),
            settings: SessionSettings {
                sizing: survivor.settings.sizing,
                ..SessionSettings::default()
            },
            inner: watch::Sender::new(Inner::new(
                party_of(&survivor),
                Tree::from_root(survivor.keys.clone(), Root::default()),
            )),
            bookmark: BookmarkHandle::new(NoBookmark),
        };

        // Each side's future owns its halves: the absorber rejects the overlap
        // *before* writing its completion marker, so only its halves' drop lets the
        // forged retiree's marker read observe the abort as EOF.
        let (retire_out, survivor_out) = pollster::block_on(async {
            let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
            tokio::join!(
                async move { forged.retire(a_end.read, a_end.write).await },
                async move { survivor.gossip_once(b_end.read, b_end.write).await },
            )
        });

        assert!(
            matches!(survivor_out, Err(Error::Protocol(_))),
            "absorbing an overlapping party must surface PartyOverlap, got {survivor_out:?}"
        );
        // The absorber aborted pre-marker, so the forged retiree's party is in
        // limbo: the retiree is consumed and its connection lost, never
        // confirmed reusable.
        assert!(
            matches!(
                retire_out,
                Retire::Retired {
                    connection: Connection::Lost { .. }
                }
            ),
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
    ///
    /// The claim holds for every network seed.
    #[test]
    fn retiring_all_forks_reconstitutes_the_seed_party(network in network_seed()) {
        let survivor = network.peer::<u64>();
        // Each child is a party-disjoint fork created by serving a bootstrap.
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

    /// Bootstrap creates a fresh party by forking the provider's; retiring that
    /// peer back must reclaim exactly that forked region.
    ///
    /// Provider with real content, bootstrap (a wire fork), then retire the
    /// newcomer home: the provider's party normalizes back to [`Party::seed`],
    /// proving the bootstrap hand-off and the retire commit are jointly leak-free.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bootstrap_then_retire_reconstitutes_the_seed_party(network in network_seed()) {
        let provider = with_messages(network.peer::<u64>(), &[1, 2, 3]);

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
    ///
    /// The claim holds for every network seed.
    #[test]
    fn mutual_retire_declines_both(network in network_seed()) {
        let survivor = network.peer::<u64>();
        let (survivor, child) = bootstrap_from(survivor);

        let (a_out, b_out) = pollster::block_on(async {
            let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
            tokio::join!(
                survivor.retire(a_end.read, a_end.write),
                child.retire(b_end.read, b_end.write)
            )
        });
        let (Retire::Declined { peer: survivor, .. }, Retire::Declined { peer: child, .. }) =
            (a_out, b_out)
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
}

// ---- fault injection: severing the wire mid-retire ------------------------

/// The wire length of `retiree`'s greeting item, used to place an exact cut
/// after the handshake.
fn greeting_frame_len(retiree: &Peer<u64>) -> usize {
    use crate::tree::mirror::streaming::{self, Local, materialized, message::Greeting};
    use crate::tree::typed::Prefix;

    let root: streaming::Root<Local> = retiree.inner.borrow().tree.clone().root.into();
    let fan = pollster::block_on(materialized::greeting_fan(&Local, root.root.clone()))
        .expect("the in-memory backend lists exactly the children it reports");
    // Reassemble the greeting from the same fields as the session, then
    // measure its single wire item.
    let greeting = Greeting {
        version: retiree.snapshot().latest().clone(),
        set_len: root.len(),
        grandchildren: root
            .root
            .as_ref()
            .map_or(0, |node| node.grandchild_count() as u64),
        max_version_bytes: root.max_version_bytes(),
        payload_depth_limit: retiree.codec.limit().get(),
        target_message_size: retiree.settings.sizing.run_budget.bytes() as u64,
        listing: materialized::fan_listing(
            Prefix::new().erase(),
            &fan,
            Some(retiree.keys.as_ref()),
        )
        .expect("the in-memory backend keeps every count in range"),
    };
    crate::tree::mirror::streaming::remote::codec::greeting::encode_greeting(&greeting).len()
}

/// The wire length of `retiree`'s trailing party frame, so a write budget
/// can land on an exact protocol boundary.
fn party_frame_len(retiree: &Peer<u64>) -> usize {
    // The hand-off is the party-atom tag wrapping a byte string of the
    // canonical party encoding.
    use crate::tree::mirror::cbor::head_len;
    let party = party_of(retiree).as_bytes().len();
    head_len(crate::tags::PARTY_TAG) + head_len(party as u64) + party
}

/// Drive `retiree.retire` against `peer.gossip` over a connection whose
/// retiree-side write half takes `budget` bytes, then fails every write.
///
/// That severs the connection at that point of the retiree's outgoing
/// bytes, whatever item the next byte belongs to.
///
/// Each side's halves are owned by its future, so the failing side's drop
/// surfaces as end-of-stream to the other rather than deadlocking the join.
/// Returns both outcomes.
#[allow(clippy::type_complexity)]
fn severed_retire(
    retiree: Peer<u64>,
    peer: &mut Peer<u64>,
    budget: usize,
) -> (
    Retire<u64, BufReader<AdversarialRead<DuplexStream>>, AdversarialWrite<DuplexStream>>,
    Result<(), Error>,
) {
    pollster::block_on(async move {
        let (a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        tokio::join!(
            async move {
                let plan = IoPlan {
                    fault: Some(IoFault {
                        operation: IoOperation::Write,
                        after: budget,
                        unit: IoFaultUnit::Bytes,
                    }),
                    ..IoPlan::default()
                };
                let (read, write, _) =
                    wrap_io(IoSide::Left, plan, a_end.read.into_inner(), a_end.write);
                retiree.retire(read, write).await
            },
            async move {
                peer.gossip_once(&mut b_end.read, &mut b_end.write)
                    .await
                    .map(|_| ())
            },
        )
    })
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A session severed during the reconciliation descent costs nothing.
    ///
    /// The trailing party frame was provably never sent, so the retiree comes back
    /// intact ([`Retire::Recovered`]) — same content, still-live party — and a
    /// subsequent clean retire of the recovered set reconstitutes the seed's whole
    /// identity space. This pins retire's fork-last ordering: the identity is
    /// never in limbo during the descent.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn severed_descent_recovers_the_retiree(network in network_seed()) {
        let survivor = network.peer::<u64>();
        let (mut survivor, child) = bootstrap_from(survivor);
        // Diverge: the child holds content the survivor lacks, so the retire
        // session must descend, and its frames overflow the budget's slack.
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
        // gossip round re-carries the divergent content) succeeds. The survivor's
        // party then normalizes back to the whole identity space.
        let survivor = retire_child_into(survivor, child);
        assert_eq!(
            party_of(&survivor),
            Party::seed(),
            "a severed-then-retried retire must reconstitute the whole id-space",
        );
    }

    /// A session severed as the retiree writes its completion marker still consumes
    /// the retiree.
    ///
    /// The party frame was delivered whole, so the absorber may well hold the
    /// identity: [`Retire::Recovered`] here would let the same identity live
    /// twice, and a reusable connection would overstate (the peer's commit
    /// was never confirmed). The only safe outcome is [`Retire::Retired`]
    /// with a [`Connection::Lost`](crate::Connection::Lost), whose error
    /// identifies [`Phase::Completion`](crate::error::Phase::Completion).
    /// Failure at this point must consume the retiree; the donated identity
    /// cannot be recovered safely.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn severed_completion_marker_consumes_the_retiree(network in network_seed()) {
        let survivor = network.peer::<u64>();
        let (mut survivor, child) = bootstrap_from(survivor);

        // Both peers are empty and converged. This budget admits the preamble,
        // greeting, and party item, then fails the write of the completion marker.
        let budget = PREAMBLE_LEN + greeting_frame_len(&child) + party_frame_len(&child);
        let (child_out, peer_out) = severed_retire(child, &mut survivor, budget);

        let Retire::Retired {
            connection: Connection::Lost { error },
        } = child_out
        else {
            panic!("a failure on the completion marker must consume the retiree, got {child_out:?}");
        };
        assert!(
            matches!(error, Error::Transport(ref error) if error.context.phase == crate::error::Phase::Completion),
            "the post-hand-off failure is the distinguished post-commit error, got {error:?}"
        );
        // The absorber committed the party before its own epilogue read hit the
        // severed wire: it reports the same post-commit residue.
        assert!(
            matches!(peer_out, Err(Error::Transport(ref error)) if error.context.phase == crate::error::Phase::Completion),
            "the absorber's confirmation of the retiree's completion fails, got {peer_out:?}"
        );
        drop(survivor);
    }

    /// A session severed on the trailing party frame itself leaves delivery
    /// irreducibly uncertain.
    ///
    /// The retiree cannot know whether the peer received its party, so it
    /// is consumed ([`Retire::Retired`] with a
    /// [`Connection::Lost`](crate::Connection::Lost)) rather than risk
    /// duplicating the region by surviving alongside a delivered copy.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn severed_party_frame_consumes_the_retiree(network in network_seed()) {
        let survivor = network.peer::<u64>();
        let (mut survivor, child) = bootstrap_from(survivor);

        // Both empty and converged, so the retiree's outgoing bytes are exactly
        // preamble + greeting + party frame + completion marker. The budget admits
        // the first two to the byte, so the party frame is the write that fails
        // (and the trailing marker is never reached).
        let budget = PREAMBLE_LEN + greeting_frame_len(&child);
        let (child_out, peer_out) = severed_retire(child, &mut survivor, budget);

        assert!(
            matches!(
                child_out,
                Retire::Retired {
                    connection: Connection::Lost { .. }
                }
            ),
            "a failure on the party frame itself must consume the retiree, got {child_out:?}"
        );
        assert!(
            peer_out.is_err(),
            "the absorber never receives the promised party frame"
        );
        drop(survivor);
    }
}

// ---- fault injection: cutting the wire across a bootstrap hand-off -------

/// The joiner's party, if a bootstrap returned a peer.
fn joined_party<R, W>(joined: crate::Joined<u64, R, W>) -> Option<Party> {
    match joined {
        crate::Joined::Joined { peer, .. } => Some(party_of(&peer)),
        crate::Joined::Failed { .. } => None,
        other => panic!("a provider serves the bootstrap or the session fails, got {other:?}"),
    }
}

/// One bootstrap from a fresh peer of `network` holding three messages, over
/// a connection whose joiner-side halves fail as `fault` says.
///
/// Returns the provider after its session, the joiner's party if the join
/// returned a peer, and the joiner's transport report: how many of the
/// provider's bytes it read, and how many of its own it wrote.
fn cut_bootstrap(
    network: NetworkSeed,
    fault: Option<IoFault>,
) -> (Peer<u64>, Option<Party>, crate::testing::IoReport) {
    let provider = with_messages(network.peer::<u64>(), &[1, 2, 3]);
    pollster::block_on(async move {
        let (mut a_end, b_end) = memory_connection(MEMORY_CAPACITY);
        let plan = IoPlan {
            fault,
            ..IoPlan::default()
        };
        let (read, write, report) =
            wrap_io(IoSide::Right, plan, b_end.read.into_inner(), b_end.write);
        // The join owns its halves, so a failed joiner's drop reaches the
        // provider as end of stream rather than leaving it waiting.
        let (_, joined) = tokio::join!(
            provider.gossip_once(&mut a_end.read, &mut a_end.write),
            Peer::<u64>::bootstrap().join(read, write),
        );
        (provider, joined_party(joined), report.snapshot())
    })
}

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// Cutting the wire at any point of a bootstrap never gives one identity to
    /// both sides, and a joiner that has received its whole party keeps it.
    ///
    /// The cut falls after every prefix of the provider's bytes as the joiner
    /// reads them, and after every prefix of the joiner's own bytes as it writes
    /// them. At every cut the provider's party and the joiner's are disjoint.
    /// Once the joiner has read the whole party item, which is everything the
    /// provider sends but its two-byte completion marker, it returns its peer,
    /// and the two parties together are the seed's whole identity: nothing is
    /// stranded. A cut inside the hand-off window, after the provider began
    /// sending the party and before the joiner holds all of it, can strand the
    /// fork, but never duplicates it.
    ///
    /// The claim holds for every network seed. Each case runs one bootstrap
    /// for every byte the two sides exchange, so it takes the fleet divisor.
    #[test]
    fn cut_bootstrap_never_duplicates_and_keeps_a_received_party(network in network_seed()) {
        let (_, clean, report) = cut_bootstrap(network, None);
        assert!(clean.is_some(), "an uncut bootstrap joins");
        let party_end = report.read_bytes - crate::peer::COMPLETION_MARKER.len();
        let cuts = (0..=report.read_bytes)
            .map(|after| (IoOperation::Read, after))
            .chain((0..=report.write_bytes).map(|after| (IoOperation::Write, after)));
        let mut kept_after_failure = 0;
        for (operation, after) in cuts {
            let fault = IoFault {
                operation,
                after,
                unit: IoFaultUnit::Bytes,
            };
            let (provider, joiner, cut) = cut_bootstrap(network, Some(fault));
            let mut whole = party_of(&provider);
            match joiner {
                Some(joiner) => {
                    assert!(
                        whole.join(joiner).is_ok(),
                        "a {operation:?} cut after {after} bytes left one identity on both sides",
                    );
                    assert_eq!(
                        whole,
                        Party::seed(),
                        "a {operation:?} cut after {after} bytes stranded identity the joiner holds a peer for",
                    );
                    kept_after_failure += usize::from(cut.injected.is_some());
                }
                None => assert!(
                    cut.read_bytes < party_end,
                    "a {operation:?} cut after {after} bytes dropped a party the joiner had read whole",
                ),
            }
        }
        assert!(
            kept_after_failure > 0,
            "some cut falls after the party arrived, so the kept-party arm is exercised",
        );
    }
}

/// A session deadline that expires on its `expires_at`-th poll, counting
/// every poll in `polls`.
///
/// A session polls its deadline only after the session itself returns
/// `Pending`, so the deadline's polls count the session's suspensions, and
/// expiring on the `k`-th cancels the session at its `k`-th suspension.
struct ExpireAtPoll {
    /// Polls of this session's deadline so far.
    polls: Arc<std::sync::atomic::AtomicUsize>,
    /// The poll on which the deadline expires; `usize::MAX` never does.
    expires_at: usize,
}

impl Future for ExpireAtPoll {
    type Output = ();

    /// Count the poll, and expire on the chosen one. A pending deadline
    /// registers no waker: the session it races has registered its own.
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        let polls = self
            .polls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        if polls >= self.expires_at {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    }
}

/// A deadline factory whose deadline expires on its `expires_at`-th poll,
/// counting polls in `polls`.
fn expire_at_poll(
    polls: &Arc<std::sync::atomic::AtomicUsize>,
    expires_at: usize,
) -> impl Fn() -> ExpireAtPoll + Send + Sync + 'static {
    let polls = Arc::clone(polls);
    move || ExpireAtPoll {
        polls: Arc::clone(&polls),
        expires_at,
    }
}

/// What one cancelled bootstrap left on each side, and how often each
/// side's session suspended.
struct Cancelled {
    /// The provider's party after its session.
    provider: Party,
    /// The phase the provider's session failed in, if it failed with a
    /// transport failure or a protocol violation.
    provider_failed_in: Option<crate::error::Phase>,
    /// The joiner's party, if the join returned a peer.
    joiner: Option<Party>,
    /// Whether the join returned a peer whose session then failed, so that
    /// its provider never confirmed completion.
    connection_lost: bool,
    /// Suspensions of the joiner's session, then of the provider's.
    suspensions: [usize; 2],
}

/// One bootstrap from a fresh peer of `network` holding three messages,
/// whose joiner's session is cancelled at its `joiner_at`-th suspension and
/// whose provider's at its `provider_at`-th (`usize::MAX` for never).
///
/// Each side owns its halves, so a cancelled side's drop reaches the other
/// as end of stream rather than leaving it waiting.
fn cancelled_bootstrap(network: NetworkSeed, joiner_at: usize, provider_at: usize) -> Cancelled {
    let polls = [(); 2].map(|()| Arc::new(std::sync::atomic::AtomicUsize::new(0)));
    let provider = with_messages(
        network
            .peer::<u64>()
            .session_deadline(expire_at_poll(&polls[1], provider_at)),
        &[1, 2, 3],
    );
    let (served, joined) = pollster::block_on(async {
        let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
        let (served, joined) = tokio::join!(
            provider.gossip_once(a_end.read, a_end.write),
            Peer::<u64>::bootstrap()
                .session_deadline(expire_at_poll(&polls[0], joiner_at))
                .join(b_end.read, b_end.write),
        );
        (served.map(drop), joined)
    });
    let connection_lost = matches!(
        joined,
        crate::Joined::Joined {
            connection: crate::Connection::Lost { .. },
            ..
        }
    );
    Cancelled {
        provider: party_of(&provider),
        provider_failed_in: match served {
            Err(Error::Transport(failure)) => Some(failure.context.phase),
            Err(Error::Protocol(violation)) => Some(violation.context.phase),
            _ => None,
        },
        joiner: joined_party(joined),
        connection_lost,
        suspensions: polls.map(|polls| polls.load(std::sync::atomic::Ordering::Relaxed)),
    }
}

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// Cancelling either side of a bootstrap at any suspension, or both at
    /// any pair of them, never gives one identity to both sides, and a joiner
    /// that returns a peer holds exactly what the provider gave up.
    ///
    /// A session deadline that expires on its `k`-th poll cancels its session
    /// at the `k`-th suspension ([`ExpireAtPoll`]). An uncancelled bootstrap
    /// counts each side's suspensions; the joiner's cancellation then ranges
    /// over each of its suspensions, one past its last, and never, and so does
    /// the provider's, in every combination. At every combination the two
    /// parties are disjoint, and when the join returns a peer the two
    /// together are the seed's whole identity.
    ///
    /// Two floors show both custody paths ran. The provider reserves its fork
    /// before reconciliation, so a provider whose session failed in
    /// reconciliation, holding the whole seed with no peer joined, had its
    /// unsent fork come back to it; some cancellation does that. And some join
    /// returns a peer beside a
    /// [`Connection::Lost`](crate::Connection::Lost), keeping a party whose
    /// completion the cancelled provider never confirmed.
    ///
    /// A cancellation can strand the fork, which neither side then holds: a
    /// joiner that leaves after its last reconciliation bytes leaves the
    /// provider to give the fork up into a closed connection. That loses an
    /// identity but never duplicates one, so it is permitted here.
    ///
    /// The claim holds for every network seed. Each case runs a bootstrap per
    /// pair of suspensions, so it takes the fleet divisor.
    #[test]
    fn cancelled_bootstrap_never_duplicates_and_keeps_a_received_party(network in network_seed()) {
        let clean = cancelled_bootstrap(network, usize::MAX, usize::MAX);
        assert!(clean.joiner.is_some(), "an uncancelled bootstrap joins");
        let points = |suspensions: usize| (1..=suspensions + 1).chain([usize::MAX]);
        let (mut restored, mut lost) = (0, 0);
        for joiner_at in points(clean.suspensions[0]) {
            for provider_at in points(clean.suspensions[1]) {
                let cancelled = cancelled_bootstrap(network, joiner_at, provider_at);
                let at = format!("joiner at {joiner_at}, provider at {provider_at}");
                let mut whole = cancelled.provider;
                match cancelled.joiner {
                    Some(joiner) => {
                        assert!(whole.join(joiner).is_ok(), "{at}: one identity on both sides");
                        assert_eq!(whole, Party::seed(), "{at}: a joined peer, yet identity stranded");
                        lost += usize::from(cancelled.connection_lost);
                    }
                    None => {
                        restored += usize::from(
                            whole == Party::seed()
                                && cancelled.provider_failed_in
                                    == Some(crate::error::Phase::Reconciliation),
                        );
                    }
                }
            }
        }
        assert!(restored > 0, "some cancellation returns a reserved, unsent fork to the provider");
        assert!(lost > 0, "some join keeps a party its provider never confirmed");
    }
}

/// What one cancelled retirement returned to the retiree.
enum RetireOutcome {
    /// The retiree left with its connection reusable: the absorber
    /// confirmed the retirement.
    Confirmed,
    /// The retirement failed before the handoff began, returning the peer.
    Recovered(Peer<u64>),
    /// The handoff began, and the retiree left with its connection lost,
    /// unconfirmed.
    Lost,
}

/// What one cancelled retirement left on each side, and how often each
/// side's session suspended.
struct CancelledRetire {
    /// The absorber, after its session.
    absorber: Peer<u64>,
    /// The absorber's party before the session.
    absorber_before: Party,
    /// What the retirement returned.
    outcome: RetireOutcome,
    /// Suspensions of the retiree's session, then of the absorber's.
    suspensions: [usize; 2],
}

/// Retire a peer joined from a fresh peer of `network` into the peer it
/// joined from, cancelling the retiree's session at its `retiree_at`-th
/// suspension and the absorber's at its `absorber_at`-th.
///
/// `usize::MAX` cancels never. Each side holds messages the other lacks.
///
/// Each side owns its halves, so a cancelled side's drop reaches the other
/// as end of stream rather than leaving it waiting.
fn cancelled_retire(
    network: NetworkSeed,
    retiree_at: usize,
    absorber_at: usize,
) -> CancelledRetire {
    let polls = [(); 2].map(|()| Arc::new(std::sync::atomic::AtomicUsize::new(0)));
    let (absorber, retiree) = bootstrap_from(with_messages(network.peer::<u64>(), &[1, 2, 3]));
    let retiree =
        with_messages(retiree, &[4, 5]).session_deadline(expire_at_poll(&polls[0], retiree_at));
    let absorber =
        with_messages(absorber, &[6]).session_deadline(expire_at_poll(&polls[1], absorber_at));
    let absorber_before = party_of(&absorber);
    let retired = pollster::block_on(async {
        let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
        let (retired, _) = tokio::join!(
            retiree.retire(a_end.read, a_end.write),
            absorber.gossip_once(b_end.read, b_end.write),
        );
        retired
    });
    let outcome = match retired {
        Retire::Retired {
            connection: Connection::Reusable { .. },
        } => RetireOutcome::Confirmed,
        Retire::Retired {
            connection: Connection::Lost { .. },
        } => RetireOutcome::Lost,
        Retire::Recovered { peer, .. } => RetireOutcome::Recovered(peer),
        Retire::Declined { .. } => panic!("the absorber is not retiring"),
    };
    CancelledRetire {
        absorber,
        absorber_before,
        outcome,
        suspensions: polls.map(|polls| polls.load(std::sync::atomic::Ordering::Relaxed)),
    }
}

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// Cancelling either side of a retirement at any suspension, or both at
    /// any pair of them, never gives one identity to both sides, and a
    /// retiree returned for retry still retires.
    ///
    /// A session deadline that expires on its `k`-th poll cancels its session
    /// at the `k`-th suspension ([`ExpireAtPoll`]). An uncancelled retirement
    /// counts each side's suspensions; the retiree's cancellation then ranges
    /// over each of its suspensions, one past its last, and never, and so does
    /// the absorber's, in every combination. A retirement whose connection
    /// stays reusable was confirmed, and leaves the absorber holding the
    /// seed's whole identity and the retiree's messages. A recovered retiree holds exactly the identity it had, the
    /// absorber exactly its own, and the two together the seed's whole
    /// identity; with the deadlines lifted, the retiree then retires into the
    /// absorber. A retirement whose connection is lost leaves the absorber
    /// holding either the seed's whole identity or exactly its own: the
    /// retiree's identity is absorbed whole or stranded, never split.
    ///
    /// Two floors show both custody paths ran: some cancellation returns the
    /// retiree for retry, and some consumes it after its handoff began.
    ///
    /// The claim holds for every network seed. Each case runs a retirement
    /// per pair of suspensions, so it takes the fleet divisor.
    #[test]
    fn cancelled_retire_never_duplicates_and_recovers_a_usable_peer(network in network_seed()) {
        let clean = cancelled_retire(network, usize::MAX, usize::MAX);
        assert!(matches!(clean.outcome, RetireOutcome::Confirmed), "an uncancelled retirement completes");
        let points = |suspensions: usize| (1..=suspensions + 1).chain([usize::MAX]);
        let (mut recovered, mut lost) = (0, 0);
        for retiree_at in points(clean.suspensions[0]) {
            for absorber_at in points(clean.suspensions[1]) {
                let at = format!("retiree at {retiree_at}, absorber at {absorber_at}");
                let cancelled = cancelled_retire(network, retiree_at, absorber_at);
                let absorber = party_of(&cancelled.absorber);
                match cancelled.outcome {
                    RetireOutcome::Confirmed => {
                        assert_eq!(absorber, Party::seed(), "{at}: a confirmed retirement left identity behind");
                        let held: Vec<u64> = cancelled.absorber.snapshot().iter().map(|(_, value)| *value).collect();
                        assert!(held.contains(&4) && held.contains(&5), "{at}: the absorber lacks the retiree's messages");
                    }
                    RetireOutcome::Recovered(retiree) => {
                        recovered += 1;
                        assert_eq!(absorber, cancelled.absorber_before, "{at}: a recovered retiree's identity was absorbed");
                        let mut whole = party_of(&retiree);
                        assert!(whole.join(absorber).is_ok(), "{at}: one identity on both sides");
                        assert_eq!(whole, Party::seed(), "{at}: a recovered retiree stranded identity");
                        let survivor = retire_child_into(
                            cancelled.absorber.session_deadline(std::future::pending::<()>),
                            retiree.session_deadline(std::future::pending::<()>),
                        );
                        assert_eq!(party_of(&survivor), Party::seed(), "{at}: the retried retirement left identity behind");
                    }
                    RetireOutcome::Lost => {
                        lost += 1;
                        assert!(
                            absorber == Party::seed() || absorber == cancelled.absorber_before,
                            "{at}: the absorber holds part of the retiree's identity",
                        );
                    }
                }
            }
        }
        assert!(recovered > 0, "some cancellation returns the retiree for retry");
        assert!(lost > 0, "some cancellation consumes the retiree after its handoff began");
    }
}

/// The payloads a peer holds live, sorted.
fn live_values(peer: &Peer<u64>) -> Vec<u64> {
    let mut values: Vec<u64> = peer.snapshot().iter().map(|(_, value)| *value).collect();
    values.sort_unstable();
    values
}

/// What one cancelled gossip left on one side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GossipSide {
    /// The session completed.
    Completed,
    /// The session failed after publishing the reconciled set.
    FailedPublished,
    /// The session failed with the side's set unchanged.
    FailedUnchanged,
}

/// Gossip between a fresh peer of `network` and a peer joined from it,
/// cancelling the first's session at its `a_at`-th suspension and the
/// second's at its `b_at`-th.
///
/// `usize::MAX` cancels never. Each side holds messages the other lacks,
/// and the first has redacted a message both held.
///
/// Checks that each side ends holding its own set or the reconciled one,
/// and that with the deadlines lifted the two then converge on the
/// reconciled set. Returns each side's outcome and suspensions.
fn cancelled_gossip(
    network: NetworkSeed,
    a_at: usize,
    b_at: usize,
) -> ([GossipSide; 2], [usize; 2]) {
    let polls = [(); 2].map(|()| Arc::new(std::sync::atomic::AtomicUsize::new(0)));
    let (a, b) = bootstrap_from(with_messages(network.peer::<u64>(), &[1, 2, 3]));
    let a = with_messages(a, &[6]);
    let one = a
        .snapshot()
        .iter()
        .find(|(_, value)| **value == 1)
        .map(|(version, _)| version.clone());
    assert!(a.redact(&one.expect("the first side holds message 1")));
    let a = a.session_deadline(expire_at_poll(&polls[0], a_at));
    let b = with_messages(b, &[4, 5]).session_deadline(expire_at_poll(&polls[1], b_at));
    let before = [live_values(&a), live_values(&b)];
    let reconciled = vec![2, 3, 4, 5, 6];
    let results = pollster::block_on(async {
        let (a_end, b_end) = memory_connection(MEMORY_CAPACITY);
        let (a_out, b_out) = tokio::join!(
            a.gossip_once(a_end.read, a_end.write),
            b.gossip_once(b_end.read, b_end.write),
        );
        [a_out.is_ok(), b_out.is_ok()]
    });
    let at = format!("first at {a_at}, second at {b_at}");
    let mut sides = [GossipSide::Completed; 2];
    for (index, peer) in [&a, &b].into_iter().enumerate() {
        let after = live_values(peer);
        sides[index] = if results[index] {
            assert_eq!(
                after, reconciled,
                "{at}: a completed session left side {index} unreconciled"
            );
            GossipSide::Completed
        } else if after == reconciled {
            GossipSide::FailedPublished
        } else {
            assert_eq!(
                after, before[index],
                "{at}: side {index} published part of a session"
            );
            GossipSide::FailedUnchanged
        };
    }
    let a = a.session_deadline(std::future::pending::<()>);
    let b = b.session_deadline(std::future::pending::<()>);
    pollster::block_on(async {
        let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        let (a_out, b_out) = tokio::join!(
            a.gossip_once(&mut a_end.read, &mut a_end.write),
            b.gossip_once(&mut b_end.read, &mut b_end.write),
        );
        a_out.expect("the first side gossips after the cancellation");
        b_out.expect("the second side gossips after the cancellation");
    });
    assert_eq!(
        live_values(&a),
        reconciled,
        "{at}: the first side did not converge afterwards"
    );
    assert_eq!(
        live_values(&b),
        reconciled,
        "{at}: the second side did not converge afterwards"
    );
    (
        sides,
        polls.map(|polls| polls.load(std::sync::atomic::Ordering::Relaxed)),
    )
}

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// Cancelling either side of a gossip session at any suspension, or both
    /// at any pair of them, publishes all of the session's result or none of
    /// it, and leaves both peers able to gossip.
    ///
    /// A session deadline that expires on its `k`-th poll cancels its session
    /// at the `k`-th suspension ([`ExpireAtPoll`]). An uncancelled session
    /// counts each side's suspensions; each side's cancellation then ranges
    /// over each of its suspensions, one past its last, and never, in every
    /// combination. Each side holds messages the other lacks, and one side
    /// has redacted a message both held, so the reconciled set differs from
    /// both. After every combination each side holds its own set or the
    /// reconciled one, a side whose session completed holds the reconciled
    /// one, and an uncancelled session between the two then reconciles both.
    ///
    /// A floor shows that cancellation reaches both sides of publication:
    /// some failed session leaves its side unchanged, and some publishes
    /// the reconciled set before failing in its completion exchange.
    ///
    /// The claim holds for every network seed. Each case runs two sessions
    /// per pair of suspensions, so it takes the fleet divisor.
    #[test]
    fn cancelled_gossip_publishes_all_or_nothing(network in network_seed()) {
        let (clean, suspensions) = cancelled_gossip(network, usize::MAX, usize::MAX);
        assert_eq!(clean, [GossipSide::Completed; 2], "an uncancelled session completes");
        let points = |suspensions: usize| (1..=suspensions + 1).chain([usize::MAX]);
        let (mut unchanged, mut published) = (0, 0);
        for a_at in points(suspensions[0]) {
            for b_at in points(suspensions[1]) {
                let (sides, _) = cancelled_gossip(network, a_at, b_at);
                for side in sides {
                    unchanged += usize::from(side == GossipSide::FailedUnchanged);
                    published += usize::from(side == GossipSide::FailedPublished);
                }
            }
        }
        assert!(unchanged > 0, "some cancelled session leaves its side unchanged");
        assert!(published > 0, "some cancelled session publishes before failing");
    }
}

/// A writer that passes `budget` bytes through to `inner`, then accepts no
/// more: every later write stays pending, as a stalled connection's does.
struct StallAfter<W> {
    /// The writer the budget's bytes go to.
    inner: W,
    /// Bytes still to pass through before the stall.
    budget: usize,
}

impl<W: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for StallAfter<W> {
    /// Pass bytes through until the budget is spent, then stay pending.
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if self.budget == 0 {
            return std::task::Poll::Pending;
        }
        let take = buf.len().min(self.budget);
        let poll = std::pin::Pin::new(&mut self.inner).poll_write(cx, &buf[..take]);
        if let std::task::Poll::Ready(Ok(written)) = poll {
            self.budget -= written;
        }
        poll
    }

    /// Flush the inner writer.
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// Shut the inner writer down.
    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Bookmark storage whose committed record the test can read back.
#[derive(Clone, Default)]
struct SharedRecord(Arc<std::sync::Mutex<Option<Vec<u8>>>>);

/// Model atomic replacement of an in-memory record that cannot fail.
impl Bookmark for SharedRecord {
    /// Replacing an in-memory buffer cannot fail.
    type Error = std::convert::Infallible;
    /// An owned snapshot of the committed bytes.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Return a snapshot of the committed bytes.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(self.0.lock().unwrap().clone().map(std::io::Cursor::new))
    }

    /// Replace the complete record.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        *self.0.lock().unwrap() = Some(bytes);
        Ok(())
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A joiner whose session deadline expires after its whole party has
    /// arrived keeps the party, and attaches its selected bookmark.
    ///
    /// The deadline expires while the joiner waits for the provider's completion
    /// marker, and the join returns a peer with the provider's content beside
    /// a [`Connection::Lost`](crate::Connection::Lost) carrying the deadline's
    /// error. The peer comes back in [`Joined::Joined`](crate::Joined::Joined),
    /// which carries a peer only once its bookmark is attached, and the
    /// bookmark holds a persisted record.
    ///
    /// A clean bootstrap measures how many bytes the provider writes. A second
    /// provider, built identically from the same network seed, then serves
    /// through a writer that passes every byte before its completion marker
    /// and stalls on the marker. Under a paused clock the joiner's deadline
    /// fires exactly when nothing else can run: after the party arrived.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn deadline_after_the_party_arrives_keeps_it(network in network_seed()) {
        let provider = || with_messages(network.peer::<u64>(), &[1, 2, 3]);
        let clean = provider();
        let provider_bytes = pollster::block_on(async {
            let (a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
            let (mut read, mut write, report) = wrap_io(
                IoSide::Left,
                IoPlan::default(),
                a_end.read.into_inner(),
                a_end.write,
            );
            let (served, joined) = tokio::join!(
                clean.gossip_once(&mut read, &mut write),
                Peer::<u64>::bootstrap().join(&mut b_end.read, &mut b_end.write),
            );
            served.expect("the measured bootstrap completes");
            assert!(
                joined_party(joined).is_some(),
                "the measured bootstrap joins"
            );
            report.snapshot().write_bytes
        });

        let provider = provider();
        let record = SharedRecord::default();
        let joined = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()
            .unwrap()
            .block_on(async {
                let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
                let mut stalled = StallAfter {
                    inner: &mut a_end.write,
                    budget: provider_bytes - crate::peer::COMPLETION_MARKER.len(),
                };
                let serve = provider.gossip_once(&mut a_end.read, &mut stalled);
                let join = Peer::<u64>::bootstrap()
                    .session_deadline(|| tokio::time::sleep(std::time::Duration::from_secs(1)))
                    .bookmark(record.clone())
                    .join(&mut b_end.read, &mut b_end.write);
                let joined = tokio::select! {
                    joined = join => joined,
                    _ = serve => panic!("the stalled provider cannot finish"),
                };
                match joined {
                    crate::Joined::Joined {
                        peer,
                        connection: crate::Connection::Lost { error },
                    } => (peer, error),
                    other => panic!("a deadline after the party arrives keeps the peer, got {other:?}"),
                }
            });
        let (peer, error) = joined;
        assert!(matches!(error, Error::DeadlineExceeded), "got {error:?}");
        assert!(
            record.0.lock().unwrap().is_some(),
            "attachment persists a record despite the lost connection"
        );
        assert_eq!(
            peer.snapshot().len(),
            3,
            "the joiner holds the provider's content"
        );
        let mut whole = party_of(&provider);
        assert!(
            whole.join(party_of(&peer)).is_ok(),
            "the parties are disjoint"
        );
        assert_eq!(
            whole,
            Party::seed(),
            "the provider and joiner hold the whole identity"
        );
    }
}

// ---- the network secret --------------------------------------------------

/// Every item observed across a peer's sessions, in observation order, with
/// its stream and direction.
#[derive(Default)]
struct ObservedItems(std::sync::Mutex<Vec<(StreamId, Direction, Vec<u8>)>>);

/// A peer observer recording every item of every session into one log.
struct RecordItems(Arc<ObservedItems>);

impl Observer for RecordItems {
    /// Record every session.
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        Some(Box::new(RecordSession(self.0.clone())))
    }
}

/// A session observer recording every stream into the shared log.
struct RecordSession(Arc<ObservedItems>);

impl SessionObserver for RecordSession {
    /// Record every stream, tagged with its identity and direction.
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        Some(Box::new(RecordStream(
            self.0.clone(),
            stream.id,
            stream.direction,
        )))
    }
}

/// A stream observer appending each item to the shared log.
struct RecordStream(Arc<ObservedItems>, StreamId, Direction);

impl StreamObserver for RecordStream {
    /// Append the item with its stream and direction.
    fn message(&mut self, bytes: &[u8]) {
        self.0
            .0
            .lock()
            .unwrap()
            .push((self.1, self.2, bytes.to_vec()));
    }
}

/// Every rendering of `secret`'s bytes a log line could carry: lowercase
/// and uppercase hex, and Rust's array `Debug` form.
fn renderings(secret: &NetworkSecret) -> [String; 3] {
    let bytes = secret.as_bytes();
    [
        hex::encode(bytes),
        hex::encode_upper(bytes),
        format!("{bytes:?}"),
    ]
}

/// Whether `haystack` holds `needle` as a contiguous run.
fn contains_run(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A bootstrap delivers the network's secret, sealed, before any supply,
    /// and no observation, on either side, carries it in the clear.
    ///
    /// The joiner ends up holding the provider's secret. The provider's second
    /// received control item is the joiner's key item; the joiner's second is
    /// the sealed secret item, which it observes before any received data
    /// frame. No item either side observed, and so no byte that crossed the
    /// wire, contains the secret's sixteen bytes.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bootstrap_delivers_the_secret_first_and_observers_never_see_it(network in network_seed()) {
        let provider_items = Arc::new(ObservedItems::default());
        let joiner_items = Arc::new(ObservedItems::default());
        let provider = with_messages(
            network.peer::<u64>().observe(Arc::new(RecordItems(provider_items.clone()))),
            &[1, 2, 3],
        );
        let joiner = pollster::block_on(async {
            let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
            let (served, joined) = tokio::join!(
                provider.gossip_once(&mut a_end.read, &mut a_end.write),
                Peer::<u64>::bootstrap()
                    .observe(Arc::new(RecordItems(joiner_items.clone())))
                    .join(&mut b_end.read, &mut b_end.write),
            );
            served.expect("the provider serves the bootstrap");
            match joined {
                crate::Joined::Joined { peer, connection: crate::Connection::Reusable { .. } } => peer,
                other => panic!("the bootstrap joins, got {other:?}"),
            }
        });
        assert!(
            joiner.keys == provider.keys,
            "the joiner holds the network's secret"
        );

        let joiner_items = joiner_items.0.lock().unwrap();
        let received: Vec<(usize, StreamId, &[u8])> = joiner_items
            .iter()
            .enumerate()
            .filter(|(_, (_, direction, _))| *direction == Direction::Received)
            .map(|(position, (stream, _, bytes))| (position, *stream, bytes.as_slice()))
            .collect();
        let control: Vec<_> = received
            .iter()
            .filter(|(_, stream, _)| *stream == StreamId::Control)
            .collect();
        // An item's tag, read from its head.
        let tag_of = |item: &[u8]| {
            let mut input = item;
            crate::tree::mirror::cbor::read_head(&mut input)
                .expect("a control item opens with a canonical head")
                .value
        };
        let (secret_position, _, secret_item) = control[1];
        assert_eq!(
            tag_of(secret_item),
            crate::tags::NETWORK_SECRET_TAG,
            "the joiner's second control item is the sealed secret item"
        );
        let provider_control: Vec<Vec<u8>> = provider_items
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|(stream, direction, _)| {
                *stream == StreamId::Control && *direction == Direction::Received
            })
            .map(|(_, _, bytes)| bytes.clone())
            .collect();
        assert_eq!(
            tag_of(&provider_control[1]),
            crate::tags::BOOTSTRAP_KEY_TAG,
            "the provider's second control item is the joiner's key item"
        );
        let first_data = received
            .iter()
            .find(|(_, stream, _)| matches!(stream, StreamId::Data { .. }))
            .map(|(position, _, _)| *position)
            .expect("the provider supplies its messages");
        assert!(
            secret_position < &first_data,
            "the secret arrives before the first supply"
        );

        for (side, items) in [
            ("joiner", &*joiner_items),
            ("provider", &*provider_items.0.lock().unwrap()),
        ] {
            for (stream, direction, bytes) in items {
                assert!(
                    !contains_run(bytes, provider.keys.secret().as_bytes()),
                    "the {side}'s {direction:?} {stream:?} observation carries the secret",
                );
            }
        }
    }

    /// No `Debug` rendering of a peer, a handle, a snapshot, or a network, nor
    /// any rendering of a network-mismatch error, carries a network's secret.
    ///
    /// Two independently seeded peers gossip, so the session fails with
    /// [`Mismatch::Network`](crate::error::Mismatch::Network), which names both
    /// networks; every rendering is searched for each secret in hex and in
    /// Rust's array form.
    ///
    /// The claim holds for every pair of distinct network seeds.
    #[test]
    fn no_rendering_carries_a_network_secret(
        (network, other_network) in distinct_network_seeds(),
    ) {
        let left = with_messages(network.peer::<u64>(), &[1]);
        let right = other_network.peer::<u64>();
        let keys = [left.keys.clone(), right.keys.clone()];
        let error = pollster::block_on(async {
            let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
            let (left_out, _) = tokio::join!(
                left.gossip_once(&mut a_end.read, &mut a_end.write),
                right.gossip_once(&mut b_end.read, &mut b_end.write),
            );
            left_out.expect_err("independently seeded peers refuse to gossip")
        });
        assert!(
            matches!(
                error,
                Error::Mismatch(crate::error::Mismatch::Network { .. })
            ),
            "got {error:?}",
        );
        let rendered = format!(
            "{left:?} {right:?} {:?} {:?} {} {error:?} {error} {:?} {:#?}",
            left.snapshot(),
            left.network(),
            left.network(),
            left.keys,
            right.keys,
        );
        for keys in &keys {
            for rendering in renderings(keys.secret()) {
                assert!(
                    !rendered.contains(&rendering),
                    "a rendering carries a secret: {rendered}"
                );
            }
        }
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// An uncontained supply crossing a real peer session surfaces through
    /// [`Rumors::gossip`](crate::Rumors::gossip) as its typed violation.
    ///
    /// The error is [`Error::Protocol`] carrying the wire decoder's
    /// `UncontainedSupply`, naming the escaped leaf's keyed path, and the
    /// replica's content is untouched.
    ///
    /// This observes through [`Rumors::gossip`](crate::Rumors::gossip) the same
    /// rejection the wire tests exercise directly. A local [`Tree::join`]
    /// builds the invalid store because session validation cannot guard an
    /// in-process join.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn uncontained_supply_fails_gossip(network in network_seed()) {
        use std::convert::Infallible;

        use crate::message::Message;
        use crate::tree::mirror::streaming::remote::{DecodeError, Error as ProxyError};
        use crate::tree::typed::Path;

        let survivor = network.peer::<u64>();
        let (survivor, child) = bootstrap_from(survivor);

        // Causally concurrent divergence on both sides makes the session
        // descends instead of short-circuiting on equal declared versions.
        let receiver = with_messages(survivor, &[1]);
        let poisoned = with_messages(child, &[2]);

        // Poison the serving peer's store: the escaped leaf plants above the
        // declared ceiling while leaving that ceiling unchanged, creating the
        // store a nonconforming implementation would serve.
        let base = poisoned.inner.borrow().tree.latest().clone();
        let (escaped_root, _, escaped) = crate::tree::arb::poisoned_root(
            poisoned.inner.borrow().tree.path_key(),
            &party_of(&poisoned),
            &base,
            Message::new(0u64),
        );
        poisoned.inner.send_modify(|inner| {
            inner
                .tree
                .join(Tree::from_root(poisoned.keys.clone(), escaped_root));
        });
        assert!(
            !crate::tree::mirror::contained(&escaped, poisoned.inner.borrow().tree.latest()),
            "the planted leaf's version escapes the declared ceiling",
        );
        // Both peers share the network, so either one's path key places the
        // escaped leaf where the receiver's decoder names it.
        let path = <[u8; 32]>::from(Path::for_leaf(
            poisoned.inner.borrow().tree.path_key(),
            &escaped,
        ));

        let receiver = receiver.into_rumors();
        let before = receiver.snapshot().hash();

        // The receiving side's decoder diagnoses the violation mid-session, so the poisoned counterparty never reaches a
        // session boundary; racing the two sides lets the receiver's typed
        // error surface without waiting on the abandoned peer.
        let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
        let receiver_out = run_to_quiescence(async {
            tokio::select! {
                out = receiver.gossip_once(&mut a_end.read, &mut a_end.write) => out,
                out = poisoned.gossip_once(&mut b_end.read, &mut b_end.write) => {
                    panic!("the poisoned side must not complete a session, got {out:?}")
                }
            }
        })
        .expect("the rejecting session becomes quiescent");
        assert!(
            matches!(
                receiver_out,
                Err(Error::Protocol(ref error))
                    if matches!(
                        error.source.downcast_ref::<ProxyError<Infallible>>(),
                        Some(ProxyError::Decode(DecodeError::UncontainedSupply { path: actual }))
                            if *actual == path
                    ),
            ),
            "the receiving peer rejects the escaped leaf with its typed violation, got {receiver_out:?}",
        );

        // Nothing of the failed session reached the replica.
        assert_eq!(
            receiver.snapshot().hash(),
            before,
            "the replica's content is unchanged by the rejected session",
        );
    }
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// The root-hash meter is alive: a root-hash read through the public
    /// snapshot surface moves the per-thread counter by exactly one.
    ///
    /// This makes the counters used by `batch_commit_root_hash_reads` and
    /// `gossip_session_root_hash_reads` fail if root-hash reads stop registering.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn root_hash_read_meter_is_live(network in network_seed()) {
        let peer = with_messages(network.peer::<u64>(), &[1]);
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
    /// [`Tree::act`] returns, so no root hash is read — and none *forced* over
    /// the freshly rebuilt, memo-less copy-on-write spine — inside the watch
    /// critical section. Both the batch build and the commit run synchronously
    /// on this thread, so the bracketed count is exact.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn batch_commit_root_hash_reads(network in network_seed()) {
        let peer = with_messages(network.peer::<u64>(), &[1, 2]);
        let before = crate::tree::meter::root_hash_reads();
        peer.batch(|batch| batch.send(3))
            .expect("a flat payload is within any depth limit");
        assert_eq!(
            crate::tree::meter::root_hash_reads() - before,
            0,
            "a batch commit reads no root hash in its critical section",
        );
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

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
    ///
    /// The claim holds for every network seed.
    #[test]
    fn gossip_session_root_hash_reads(network in network_seed()) {
        let provider = with_messages(network.peer::<u64>(), &[1, 2, 3]);
        let (provider, joiner) = bootstrap_from(provider);

        // Divergence on both sides gives the session real work: each
        // side both provides and absorbs content.
        let provider = with_messages(provider, &[10]);
        let joiner = with_messages(joiner, &[20]);

        let before = crate::tree::meter::root_hash_reads();
        pollster::block_on(async {
            let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
            let (provider_out, joiner_out) = tokio::join!(
                provider.gossip_once(&mut a_end.read, &mut a_end.write),
                joiner.gossip_once(&mut b_end.read, &mut b_end.write)
            );
            provider_out.expect("provider gossip");
            joiner_out.expect("joiner gossip");
        });
        assert_eq!(
            crate::tree::meter::root_hash_reads() - before,
            0,
            "a gossip session reads no root hash in either side's commit",
        );
    }
}
