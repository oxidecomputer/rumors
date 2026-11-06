//! Session ingress checks: invalid completion markers are protocol violations,
//! premature closes are transport failures, and bootstrap claimants must have
//! no causal history.
//!
//! A rejection leaves the connection at an unknown position. Existing-replica
//! completion failures happen after commit; bootstrap constructs no peer.

mod notifications;
// The session futures hold pointers; their baselines are 64-bit sizes.
#[cfg(target_pointer_width = "64")]
mod session_size;

use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::testing::run_to_quiescence;
use before::Party;
use futures::future::BoxFuture;
use proptest::prelude::*;
use std::sync::Arc;
use tokio::io::{BufReader, duplex, split};

use super::{COMPLETION_MARKER, SessionDefect, epilogue};
use crate::error::Phase;
use crate::network::NetworkKeys;
use crate::observe::SessionHandle;
use crate::testing::property::{NetworkSeed, SESSION_DIVISOR, divided_proptest, network_seed};
use crate::testing::{MEMORY_CAPACITY, MemoryEnd, memory_connection};
use crate::tree::mirror::{
    party,
    preamble::{self, Intent},
    secret,
    streaming::{self, Local, materialized, remote as streaming_remote},
};
use crate::tree::{self, Tree};
use crate::{Error, Network, Peer, default};

/// Both sides exchange markers over a one-byte transport without deadlock.
///
/// Each side writes and flushes before its read resolves, so the exchange
/// completes even when the transport holds a single byte in flight; both
/// sides return `Ok`, the mutual completion certificate.
#[test]
fn concurrent_exchange_is_symmetric() {
    let (left_io, right_io) = duplex(1);
    let (mut left_read, mut left_write) = split(left_io);
    let (mut right_read, mut right_write) = split(right_io);

    let (left, right) = run_to_quiescence(async {
        let observe = SessionHandle::default();
        tokio::join!(
            epilogue(&mut left_read, &mut left_write, &observe),
            epilogue(&mut right_read, &mut right_write, &observe),
        )
    })
    .expect("completion exchange must not deadlock");
    left.expect("left epilogue completes");
    right.expect("right epilogue completes");
}

proptest! {
    /// Complete marker bytes either confirm completion or report the exact
    /// invalid bytes as a protocol violation in the completion phase.
    #[test]
    fn complete_markers_are_classified(
        bytes in prop_oneof![Just(COMPLETION_MARKER), any::<[u8; 2]>()],
    ) {
        let mut reader = &bytes[..];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(epilogue(&mut reader, &mut writer, &SessionHandle::default()));
        if bytes == COMPLETION_MARKER {
            prop_assert!(result.is_ok());
        } else {
            let Err(Error::Protocol(error)) = result else { prop_assert!(false, "invalid marker accepted: {bytes:?}"); return Ok(()); };
            prop_assert_eq!(error.context.phase, crate::error::Phase::Completion);
            let Some(SessionDefect::CompletionMarker(actual)) = error.source.downcast_ref::<SessionDefect>() else { prop_assert!(false, "missing invalid-marker diagnostic"); return Ok(()); };
            prop_assert_eq!(*actual, bytes);
        }
    }

    /// Closing at any point before the complete marker is a transport EOF.
    #[test]
    fn incomplete_markers_are_transport_failures(cut in 0..COMPLETION_MARKER.len()) {
        let mut reader = &COMPLETION_MARKER[..cut];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(epilogue(&mut reader, &mut writer, &SessionHandle::default()));
        let Err(Error::Transport(error)) = result else { prop_assert!(false, "close was not a transport failure: {result:?}"); return Ok(()); };
        prop_assert_eq!(error.context.phase, crate::error::Phase::Completion);
        prop_assert_eq!(error.source.kind(), std::io::ErrorKind::UnexpectedEof);
    }
}

/// Reading the marker consumes exactly the marker's bytes, leaving later
/// bytes untouched.
///
/// A next session's preamble may already sit behind the marker on a reused
/// connection; the epilogue must not slurp it. After a clean exchange the
/// following bytes remain unread in the transport.
#[test]
fn bytes_after_the_marker_stay_untouched() {
    let bytes = [COMPLETION_MARKER[0], COMPLETION_MARKER[1], b'R', b'U'];
    let mut reader = &bytes[..];
    let mut writer = tokio::io::sink();
    pollster::block_on(epilogue(
        &mut reader,
        &mut writer,
        &SessionHandle::default(),
    ))
    .expect("the marker completes");
    assert_eq!(reader, b"RU", "the next session's bytes were consumed");
}

// ---- the greeting version of a bootstrap claimant --------------------------

/// An empty tree root whose ceiling records `events` committed-then-redacted
/// events: what a misdeclaring claimant presents, nothing to provide plus a
/// version dominating any replica with fewer events.
///
/// The fixture uses the normal send and redact paths: redaction advances the
/// frontier and leaves no tombstone, so sending and then redacting `events`
/// messages leaves an empty root with an `events`-tick version. The donor
/// that builds it founds the network `network` names.
fn redacted_history_root(network: NetworkSeed, events: u64) -> tree::Root {
    let donor = network.peer::<u64>();
    donor
        .send_all(0..events)
        .expect("flat test payloads are within any depth limit");
    let versions: Vec<_> = donor.snapshot().versions().cloned().collect();
    donor.redact_all(&versions);
    let snapshot = donor.snapshot();
    assert!(snapshot.is_empty(), "every message was redacted");
    assert!(
        !snapshot.latest().is_empty(),
        "the redactions advanced the ceiling"
    );
    donor.inner.borrow().tree.clone().root
}

/// Drive one session as a bootstrap claimant whose greeting version comes
/// from `root` instead of a newborn's empty version.
///
/// This is the bootstrap join's wire flow with the one deviation under test:
/// the preamble declares [`Network::BOOTSTRAP`] while the greeting declares
/// `root`'s causal frontier. Returns the donated party and the reconciled
/// tree if the counterparty serves the session to completion.
async fn claim_bootstrap_with_root(
    end: &mut MemoryEnd,
    root: tree::Root,
) -> Result<(Party, Tree<u64>), Error> {
    // Our key item follows our preamble, as a conforming joiner's does.
    let joining = secret::Joining::new();
    let mut staged = preamble::Staged::new();
    let counterparty = preamble::preamble(
        Network::BOOTSTRAP,
        Intent::Remain,
        &joining.item(),
        &mut staged,
        &mut end.read,
        &mut end.write,
        &SessionHandle::default(),
    )
    .await
    .map_err(Error::from)?;
    // A provider seals its network's secret for our key; a bootstrapping
    // counterparty sends its own key item instead, which we discard.
    let keys = match counterparty {
        preamble::Counterparty::Bootstrapping(item) => {
            secret::decode_key(&item)?;
            None
        }
        preamble::Counterparty::Established { network, .. } => {
            let secret = joining
                .receive(&mut end.read, network, &SessionHandle::default())
                .await?;
            Some(Arc::new(NetworkKeys::new(secret)))
        }
    };
    let local_root: streaming::Root<Local> = root.into();
    // The claimed root's leaves sit at the fixture secret's paths, so its
    // digests are listed under the fixture's keys.
    let local = materialized::Handshaking::start(
        Local,
        local_root,
        Some(NetworkKeys::fixture()),
        default::TARGET_MESSAGE_SIZE as u64,
    );
    let proxy = streaming_remote::Handshaking::start(
        Local,
        &mut end.read,
        &mut end.write,
        PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
        keys.clone(),
    );
    let handshaken = streaming::handshake(local, proxy)
        .await
        .map_err(streaming_remote::streaming_error(Phase::Greeting))?;
    let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
    let (root, (mut read, mut write)) = descent
        .await
        .map_err(streaming_remote::streaming_error(Phase::Reconciliation))?;
    let party = party::receive(&mut read, &SessionHandle::default()).await?;
    epilogue(&mut read, &mut write, &SessionHandle::default()).await?;
    let keys = keys.expect("only a provider donates a party, and it sends its secret first");
    Ok((party, Tree::from_root(keys, root.into())))
}

/// A provider founding the network `network` names, holding `values`.
fn provider_with(network: NetworkSeed, values: &[u64]) -> Peer<u64> {
    let provider = network.peer::<u64>();
    provider
        .send_all(values.iter().copied())
        .expect("flat test payloads are within any depth limit");
    provider
}

/// Read a provider's live party for before/after comparison.
fn party_of(provider: &Peer<u64>) -> Party {
    provider.inner.borrow().party.dangerously_alias()
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A provider serving a bootstrap rejects a claimant whose greeting declares
    /// causal history: [`Error::Protocol`], nothing moved.
    ///
    /// The declared version would otherwise drive the deletion-honoring filter
    /// as the claimant's causal frontier, making every dominated local subtree
    /// read as deleted-there — a claimant providing nothing while declaring a
    /// dominating version would empty the provider's replica. The rejection
    /// runs after the greeting and before the descent: the provider's content,
    /// version, and party are unchanged, the speculative bootstrap fork is
    /// re-joined, and the error names the claimed history's event floor.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bootstrap_claimant_declaring_history_is_rejected(network in network_seed()) {
        let provider = provider_with(network, &[1, 2, 3]);
        let hash_before = provider.snapshot().hash();
        let party_before = party_of(&provider);
        let claimant_tree = Tree::<()>::from_root(NetworkKeys::fixture(), redacted_history_root(network, 8));
        let claimed_min_events = claimant_tree.latest().min_ticks();
        assert!(
            provider.snapshot().latest() < claimant_tree.latest(),
            "the claimed version dominates the provider's frontier",
        );

        let provider_ref = &provider;
        let (claim_out, first) = pollster::block_on(async {
            let (mut a_end, b_end) = memory_connection(MEMORY_CAPACITY);
            tokio::join!(
                async move { claim_bootstrap_with_root(&mut a_end, claimant_tree.root).await },
                async move { provider_ref.gossip_once(b_end.read, b_end.write).await },
            )
        });

        match first {
            Err(Error::Protocol(error)) => {
                let Some(SessionDefect::BootstrapHistory {
                    claimed_min_events: reported,
                }) = error.source.downcast_ref::<SessionDefect>()
                else {
                    panic!("missing claimant diagnostic: {error:?}");
                };
                assert_eq!(*reported, claimed_min_events);
            }
            other => panic!("the provider rejects the claimant, got {other:?}"),
        }
        assert!(
            claim_out.is_err(),
            "the rejected claimant's session fails, got {claim_out:?}",
        );
        assert_eq!(
            provider.snapshot().hash(),
            hash_before,
            "the provider's content is unchanged",
        );
        assert_eq!(
            party_of(&provider),
            party_before,
            "the speculative bootstrap fork snapped back in place",
        );
    }

    /// A joining peer rejects a mutual-bootstrap counterparty whose greeting
    /// declares causal history, instead of bailing as if the encounter were two
    /// newborns.
    ///
    /// The newborn requirement binds every bootstrap claimant, and in a
    /// mutual-bootstrap encounter the joining side is the side facing one: it
    /// surfaces the same [`Error::Protocol`] a serving provider would, rather than
    /// certifying a clean mutual bail against a counterparty that is neither
    /// newborn nor a provider.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn mutual_bootstrap_counterparty_with_history_is_rejected(network in network_seed()) {
        let (join_out, claim_out) = pollster::block_on(async {
            let (a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
            tokio::join!(
                async move { Peer::<u64>::bootstrap().join(a_end.read, a_end.write).await },
                async move { claim_bootstrap_with_root(&mut b_end, redacted_history_root(network, 8)).await },
            )
        });

        assert!(
            matches!(
                join_out,
                crate::Joined::Failed {
                    error: Error::Protocol(_),
                    ..
                }
            ),
            "the joining side rejects the counterparty's claimed history, got {join_out:?}",
        );
        assert!(
            claim_out.is_err(),
            "the rejected counterparty's session fails, got {claim_out:?}",
        );
    }

    /// A rejected claimant costs the provider nothing: a newborn claimant
    /// over a fresh connection afterwards succeeds and replicates the full
    /// content.
    ///
    /// The rejection's recovery contract is the claimant's alone — the
    /// provider needs no repair beyond a fresh connection, and a newborn
    /// claimant (empty greeting version) is served exactly as before.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn rejected_claimant_leaves_the_provider_serviceable(network in network_seed()) {
        let provider = provider_with(network, &[1, 2, 3]);

        let provider_ref = &provider;
        pollster::block_on(async {
            let (mut a_end, b_end) = memory_connection(MEMORY_CAPACITY);
            let (claim_out, provider_out) = tokio::join!(
                async move { claim_bootstrap_with_root(&mut a_end, redacted_history_root(network, 8)).await },
                async move { provider_ref.gossip_once(b_end.read, b_end.write).await },
            );
            assert!(
                matches!(provider_out, Err(Error::Protocol(_))),
                "the provider rejects the claimant, got {provider_out:?}",
            );
            assert!(claim_out.is_err());
        });

        let witness = pollster::block_on(async {
            let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);
            let (witness_out, provider_out) = tokio::join!(
                Peer::<u64>::bootstrap().join(&mut a_end.read, &mut a_end.write),
                provider.gossip_once(&mut b_end.read, &mut b_end.write),
            );
            provider_out.expect("the provider serves the newborn claimant");
            match witness_out {
                crate::Joined::Joined { peer, connection: crate::Connection::Reusable { .. } } => peer,
                _ => panic!("the provider donates"),
            }
        });
        assert_eq!(
            witness.snapshot().len(),
            3,
            "the newborn claimant replicates the provider's full content",
        );
    }
}

proptest! {
    /// Identity transfer and completion decode through the pipe's read half
    /// however their bytes arrive, leaving a following session's bytes
    /// unconsumed for the next reader.
    ///
    /// Half the cases first peek at the closing items' first byte, as the
    /// demultiplexer does when it stops: the peek fills the read half's
    /// buffer with as many bytes as the transport has ready, which its
    /// capacity bounds, and consumes none of them.
    #[test]
    fn trailing_items_leave_the_next_session_unread(
        forks in 0usize..32,
        donation in any::<bool>(),
        split_at in any::<usize>(),
        peeked in any::<bool>(),
        capacity in 1usize..=4,
        following in prop::collection::vec(any::<u8>(), 1..32),
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        run_to_quiescence(async {
            let observe = SessionHandle::default();
            let mut identity = Party::seed();
            for _ in 0..forks {
                let _ = identity.fork();
            }
            let expected = identity.as_bytes().to_vec();
            let mut wire = Vec::new();
            if donation {
                party::send(&identity, &mut wire, &observe).await.unwrap();
            }
            wire.extend_from_slice(&COMPLETION_MARKER);
            let cut = split_at % (wire.len() + 1);
            let (local, remote) = duplex(capacity);
            let (local_read, mut local_write) = split(local);
            let (mut remote_read, mut remote_write) = split(remote);
            let mut read = streaming_remote::PipeRead::new(BufReader::new(local_read));
            let receive = async {
                if peeked {
                    let first = read.peek().await.unwrap();
                    assert_eq!(first, Some(wire[0]), "the peek shows the first closing byte");
                }
                if donation {
                    let received = party::receive(&mut read, &observe).await.unwrap();
                    assert_eq!(received.as_bytes(), expected);
                }
                super::finish_session(&mut read, &mut local_write, &observe).await.unwrap();
                let mut next = vec![0; following.len()];
                read.read_exact(&mut next).await.unwrap();
                assert_eq!(next, following);
            };
            let send = async {
                remote_write.write_all(&wire[..cut]).await.unwrap();
                // Read our marker concurrently with sending the suffix, so
                // neither waits for room only the other can free in so small
                // a duplex.
                let (_, marker) = tokio::join!(
                    async { remote_write.write_all(&wire[cut..]).await.unwrap(); },
                    async {
                        let mut marker = [0; COMPLETION_MARKER.len()];
                        remote_read.read_exact(&mut marker).await.unwrap();
                        marker
                    },
                );
                assert_eq!(marker, COMPLETION_MARKER);
                // A peer may begin the next session only after our marker.
                remote_write.write_all(&following).await.unwrap();
            };
            tokio::join!(receive, send);
        }).expect("trailing items must decode without consuming the next session");
    }
}

proptest! {
    /// A following session's bytes that arrive together with the closing
    /// items stay in the read half's buffer, unconsumed, once the closing
    /// exchange completes.
    ///
    /// The peer writes its closing items and the following bytes in one
    /// write before this side reads anything, so the read half's first fill
    /// takes all of them; afterwards the buffer holds exactly the following
    /// bytes.
    #[test]
    fn following_bytes_stay_buffered_after_the_closing_items(
        forks in 0usize..32,
        donation in any::<bool>(),
        peeked in any::<bool>(),
        following in prop::collection::vec(any::<u8>(), 1..32),
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let buffered = run_to_quiescence(async {
            let observe = SessionHandle::default();
            let mut identity = Party::seed();
            for _ in 0..forks {
                let _ = identity.fork();
            }
            let mut wire = Vec::new();
            if donation {
                party::send(&identity, &mut wire, &observe).await.unwrap();
            }
            wire.extend_from_slice(&COMPLETION_MARKER);
            wire.extend_from_slice(&following);
            let (local, remote) = duplex(MEMORY_CAPACITY);
            let (local_read, mut local_write) = split(local);
            let (mut remote_read, mut remote_write) = split(remote);
            remote_write.write_all(&wire).await.unwrap();
            let mut read = streaming_remote::PipeRead::new(BufReader::new(local_read));
            if peeked {
                let first = read.peek().await.unwrap();
                assert_eq!(first, Some(wire[0]), "the peek shows the first closing byte");
            }
            if donation {
                let received = party::receive(&mut read, &observe).await.unwrap();
                assert_eq!(received.as_bytes(), identity.as_bytes());
            }
            super::finish_session(&mut read, &mut local_write, &observe).await.unwrap();
            let mut marker = [0; COMPLETION_MARKER.len()];
            remote_read.read_exact(&mut marker).await.unwrap();
            assert_eq!(marker, COMPLETION_MARKER);
            read.into_inner().buffer().to_vec()
        })
        .expect("the closing exchange completes over a roomy duplex");
        prop_assert_eq!(buffered, following);
    }
}

/// Withhold the completion marker while allowing all reconciliation traffic.
struct HoldCompletion<W> {
    /// The real writer used before the completion exchange.
    inner: W,
    /// Signals that reconciliation committed and the marker is now withheld.
    reached: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Pause only at completion so the test can expire an already-committed session.
impl<W: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for HoldCompletion<W> {
    /// Write normally until the session tries to send its completion marker.
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if bytes == COMPLETION_MARKER {
            self.reached
                .store(true, std::sync::atomic::Ordering::SeqCst);
            // The test drops this writer after expiring the other side's
            // deadline; it deliberately never resumes this write.
            std::task::Poll::Pending
        } else {
            std::pin::Pin::new(&mut self.inner).poll_write(cx, bytes)
        }
    }

    /// Flush every byte accepted before the held marker.
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// Delegate shutdown to the underlying writer.
    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Deadline expiry at completion reports failure without undoing the
    /// complete local commit. A fresh connection can then confirm convergence.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn completion_deadline_preserves_the_local_commit(network in network_seed()) {
        use futures::FutureExt;
        use std::future::{Future, poll_fn};
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        };
        use std::task::Poll;

        let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
        let a = network.peer::<u64>().sync_memory_budget(0).into_rumors();
        let (served, joined) = run_to_quiescence(async {
            futures::join!(
                a.gossip_once(&mut near.read, &mut near.write),
                Peer::<u64>::bootstrap().join(&mut far.read, &mut far.write)
            )
        })
        .unwrap();
        served.unwrap();
        let crate::Joined::Joined { peer, connection: crate::Connection::Reusable { .. } } = joined else {
            panic!("bootstrap succeeds")
        };
        let b = peer.into_rumors();
        a.send(1).unwrap();
        b.send(2).unwrap();

        let (expire, deadline) = futures::channel::oneshot::channel();
        let deadline = Mutex::new(Some(deadline));
        let a = pollster::block_on(a.try_into_peer())
            .unwrap()
            .session_deadline(move || deadline.lock().unwrap().take().unwrap().map(|r| r.unwrap()))
            .into_rumors();
        let reached = Arc::new(AtomicBool::new(false));
        let mut far_write = HoldCompletion {
            inner: far.write,
            reached: reached.clone(),
        };
        let mut left = Box::pin(a.gossip_once(&mut near.read, &mut near.write));
        let mut right = Box::pin(b.gossip_once(&mut far.read, &mut far_write));
        run_to_quiescence(poll_fn(|cx| {
            assert!(left.as_mut().poll(cx).is_pending());
            assert!(right.as_mut().poll(cx).is_pending());
            if reached.load(Ordering::SeqCst) && a.snapshot().len() == 2 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }))
        .expect("reconciliation reaches the held completion marker");
        assert_eq!(a.snapshot().hash(), b.snapshot().hash());
        assert_eq!(a.snapshot().len(), 2);
        expire.send(()).unwrap();
        assert!(matches!(
            run_to_quiescence(left).unwrap(),
            Err(Error::DeadlineExceeded)
        ));
        drop(right);

        // The peer's content is intact; remove the test's single-use deadline
        // factory before checking ordinary gossip over a fresh connection.
        let a = pollster::block_on(a.try_into_peer())
            .unwrap()
            .session_deadline(std::future::pending)
            .into_rumors();
        let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
        let (left, right) = run_to_quiescence(async {
            futures::join!(
                a.gossip_once(&mut near.read, &mut near.write),
                b.gossip_once(&mut far.read, &mut far.write)
            )
        })
        .unwrap();
        left.unwrap();
        right.unwrap();
    }
}
