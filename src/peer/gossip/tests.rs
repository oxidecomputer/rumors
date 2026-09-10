//! Session ingress checks: invalid completion markers are protocol violations,
//! premature closes are transport failures, and bootstrap claimants must have
//! no causal history.
//!
//! Rejections leave the link poisoned. Existing-replica
//! completion failures happen after commit; bootstrap constructs no peer.

use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::testing::run_to_quiescence;
use before::Party;
use futures::future::BoxFuture;
use proptest::prelude::*;
use tokio::io::{duplex, split};

use super::{EPILOGUE_MARKER, SessionDefect, epilogue, erase};
use crate::link::{Link, MemoryLink, memory};
use crate::observe::SessionHandle;
use crate::tree::mirror::{
    handshake::{self, Intent},
    party,
    streaming::{self, Local, materialized, remote as streaming_remote},
};
use crate::tree::{self, Tree};
use crate::{Error, Network, Peer};

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
    fn complete_markers_are_classified(bytes in prop_oneof![Just(EPILOGUE_MARKER), any::<[u8; 2]>()]) {
        let mut reader = &bytes[..];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(epilogue(&mut reader, &mut writer, &SessionHandle::default()));
        if bytes == EPILOGUE_MARKER {
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
    fn incomplete_markers_are_transport_failures(cut in 0..EPILOGUE_MARKER.len()) {
        let mut reader = &EPILOGUE_MARKER[..cut];
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
/// link; the epilogue must not slurp it. After a clean exchange the
/// following bytes remain unread in the transport.
#[test]
fn bytes_after_the_marker_stay_untouched() {
    let bytes = [EPILOGUE_MARKER[0], EPILOGUE_MARKER[1], b'R', b'U'];
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
/// Built with real semantics, standing in for a misbehaving implementation:
/// a redaction advances
/// the ceiling and leaves no tombstone, so committing and then redacting
/// `events` messages leaves an empty root carrying a genuine `events`-tick
/// version.
fn redacted_history_root(events: u64) -> tree::Root {
    let donor = Peer::<u64>::seed();
    {
        donor
            .send_all(0..events)
            .expect("flat test payloads are within any depth limit");
    }
    let versions: Vec<_> = donor
        .snapshot()
        .iter()
        .map(|(version, _)| version.clone())
        .collect();
    {
        donor.redact_all(&versions);
    }
    let snapshot = donor.snapshot();
    assert!(snapshot.is_empty(), "every message was redacted");
    assert!(
        !snapshot.latest().is_empty(),
        "the redactions advanced the ceiling"
    );
    donor.inner.borrow().tree.clone().root
}

/// Drive one V2 session as a bootstrap claimant whose greeting version comes
/// from `root` instead of a newborn's empty version.
///
/// This is the bootstrap join's wire flow with the one deviation under test:
/// the preamble declares [`Network::BOOTSTRAP`] while the greeting declares
/// `root`'s causal frontier. Returns the donated party and the reconciled
/// tree if the counterparty serves the session to completion.
async fn claim_bootstrap_v2(
    link: &mut MemoryLink,
    root: tree::Root,
) -> Result<(Party, Tree<u64>), Error> {
    let (read, write, connector, acceptor, epoch) = erase(link)?;
    let mut staged = handshake::Staged::new();
    handshake::preamble(
        Network::BOOTSTRAP,
        Intent::Remain,
        &mut staged,
        read,
        write,
        &SessionHandle::default(),
    )
    .await
    .map_err(Error::from)?;
    let local_root: streaming::Root<Local> = root.into();
    let local = materialized::Handshaking::start(Local, local_root);
    let carrier = Link::for_session(read, write, connector, acceptor, epoch);
    let proxy = streaming_remote::Handshaking::start(
        Local,
        carrier,
        PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
    );
    let handshaken = streaming::handshake(local, proxy)
        .await
        .map_err(Error::from)?;
    let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
    let (root, (mut read, mut write)) = descent.await.map_err(Error::from)?;
    let party = party::receive(&mut read, &SessionHandle::default()).await?;
    epilogue(&mut read, &mut write, &SessionHandle::default()).await?;
    Ok((party, Tree::from_root(root.into())))
}

/// A provider holding `values`, plus its pre-session root hash.
fn provider_with(values: &[u64]) -> Peer<u64> {
    let provider = Peer::<u64>::seed();
    {
        provider
            .send_all(values.iter().copied())
            .expect("flat test payloads are within any depth limit");
    }
    provider
}

/// Read a provider's live party for before/after comparison.
fn party_of(provider: &Peer<u64>) -> Party {
    provider
        .inner
        .borrow()
        .party
        .as_ref()
        .expect("a live Peer holds its party")
        .dangerously_alias()
}

/// A provider serving a bootstrap rejects, under V2, a claimant whose
/// greeting declares causal history: [`Error::Protocol`],
/// nothing moved.
///
/// The declared version would otherwise drive the deletion-honoring filter
/// as the claimant's causal frontier, making every dominated local subtree
/// read as deleted-there — a claimant providing nothing while declaring a
/// dominating version would empty the provider's replica. The rejection
/// runs after the greeting and before the descent: the provider's content,
/// version, and party are unchanged, the speculative bootstrap fork is
/// re-joined, the error names the claimed history's event floor, and the
/// link is poisoned like any failed session's, so the next session on it
/// fails fast.
#[test]
fn v2_bootstrap_claimant_declaring_history_is_rejected() {
    let provider = provider_with(&[1, 2, 3]);
    let hash_before = provider.snapshot().hash();
    let party_before = party_of(&provider);
    let claimant_tree = Tree::<()>::from_root(redacted_history_root(8));
    let claimed_min_events = claimant_tree.latest().min_ticks();
    assert!(
        provider.snapshot().latest() < claimant_tree.latest(),
        "the claimed version dominates the provider's frontier",
    );

    let provider_ref = &provider;
    let (claim_out, (first, second)) = pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        tokio::join!(
            async move { claim_bootstrap_v2(&mut a_link, claimant_tree.root).await },
            async move {
                let first = provider_ref.gossip(&mut b_link).await;
                // The second session on the same link must fail fast,
                // before any wire traffic: the rejection poisoned it.
                let second = provider_ref.gossip(&mut b_link).await;
                (first, second)
            },
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
        matches!(second, Err(Error::LinkPoisoned)),
        "the failed session poisons the link, got {second:?}",
    );
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

/// A joining peer rejects, under V2, a mutual-bootstrap counterparty whose
/// greeting declares causal history, instead of bailing as if the
/// encounter were two honest newborns.
///
/// The newborn requirement binds every bootstrap claimant, and in a
/// mutual-bootstrap encounter the joining side is the side facing one: it
/// surfaces the same [`Error::Protocol`] a serving
/// provider would, rather than certifying a clean mutual bail against a
/// counterparty that is neither newborn nor a provider.
#[test]
fn v2_mutual_bootstrap_counterparty_with_history_is_rejected() {
    let (join_out, claim_out) = pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        tokio::join!(
            async move { Peer::<u64>::bootstrap().join(&mut a_link).await },
            async move { claim_bootstrap_v2(&mut b_link, redacted_history_root(8)).await },
        )
    });

    assert!(
        matches!(join_out, Err(Error::Protocol(_))),
        "the joining side rejects the counterparty's claimed history, got {join_out:?}",
    );
    assert!(
        claim_out.is_err(),
        "the rejected counterparty's session fails, got {claim_out:?}",
    );
}

/// A rejected claimant costs the provider nothing: an honest bootstrap
/// over a fresh link afterwards succeeds and replicates the full content.
///
/// The rejection's recovery contract is the claimant's alone — the
/// provider needs no repair beyond a fresh link, and a genuinely newborn
/// claimant (empty greeting version) is served exactly as before.
#[test]
fn rejected_claimant_leaves_the_provider_serviceable() {
    let provider = provider_with(&[1, 2, 3]);

    let provider_ref = &provider;
    pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        let (claim_out, provider_out) = tokio::join!(
            async move { claim_bootstrap_v2(&mut a_link, redacted_history_root(8)).await },
            async move { provider_ref.gossip(&mut b_link).await },
        );
        assert!(
            matches!(provider_out, Err(Error::Protocol(_))),
            "the provider rejects the claimant, got {provider_out:?}",
        );
        assert!(claim_out.is_err());
    });

    let witness = pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        let (witness_out, provider_out) = tokio::join!(
            Peer::<u64>::bootstrap().join(&mut a_link),
            provider.gossip(&mut b_link),
        );
        provider_out.expect("the provider serves the honest bootstrap");
        witness_out
            .expect("the honest bootstrap completes")
            .expect("the provider donates")
    });
    assert_eq!(
        witness.snapshot().len(),
        3,
        "the honest newcomer replicates the provider's full content",
    );
}
