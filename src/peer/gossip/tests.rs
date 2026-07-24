//! Session-level ingress validation in [`gossip_inner`]'s driver layer.
//!
//! Two ingress surfaces are exercised here, both peer-controlled and both
//! reachable only with in-crate access to the session internals:
//!
//! - The V2 session epilogue marker: the last wire ingress of every V2
//!   session, one byte read from the control stream after all session
//!   work. The suite exhausts that byte space and its truncation directly
//!   against [`epilogue`]: every non-marker byte is a typed protocol
//!   violation, an honest cut is a typed EOF, both as the distinguished
//!   post-commit [`Error::Epilogue`] — never a panic and never a hang —
//!   and a clean exchange leaves the next session's bytes untouched. The
//!   end-to-end commit-boundary consequences are pinned in
//!   `tests/lifecycle.rs` and `src/tests.rs`.
//!
//! - The greeting version of a bootstrap claimant: a peer whose preamble
//!   declares [`Network::BOOTSTRAP`] is definitionally a newborn replica
//!   with no causal history, so its greeting version must be empty. The
//!   deletion-honoring filter trusts the greeting version as the
//!   counterparty's causal frontier, so a claimant declaring history it
//!   cannot have would otherwise make the provider drop — as
//!   deleted-there — every subtree that version dominates. The claimant
//!   here is driven by the crate's own protocol machinery handed a
//!   non-newborn root, standing in for a misbehaving implementation; the
//!   suite pins the rejection ([`Error::BootstrapHistoryConflict`]) on
//!   whichever side faces the claimant — the serving provider, and the
//!   joining side of a mutual-bootstrap encounter — with the detecting
//!   replica unchanged and its link poisoned.
//!
//! [`gossip_inner`]: super::Peer::gossip_inner
//! [`Network::BOOTSTRAP`]: Network

use before::Party;
use futures::future::BoxFuture;
use tokio::io::{duplex, split};

use super::{EPILOGUE_MARKER, alternating_error, epilogue, erase, streaming_error};
use crate::link::{Link, MemoryLink, memory};
use crate::tree::mirror::{
    alternating::{self, local as alternating_local, remote as alternating_remote},
    framing::{FrameRead, FrameWrite},
    handshake::{self, Intent},
    party,
    streaming::{self, Local, materialized, remote as streaming_remote},
};
use crate::tree::{self, Tree};
use crate::{Error, Network, Peer, Protocol};

/// Unwrap the sole error variant the epilogue can produce.
fn epilogue_error(result: Result<(), Error>) -> std::io::Error {
    match result {
        Err(Error::Epilogue(error)) => error,
        other => panic!("the epilogue fails as Error::Epilogue, got {other:?}"),
    }
}

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

    let (left, right) = pollster::block_on(async {
        tokio::join!(
            epilogue(&mut left_read, &mut left_write),
            epilogue(&mut right_read, &mut right_write),
        )
    });
    left.expect("left epilogue completes");
    right.expect("right epilogue completes");
}

/// Marker decoding is exhaustive: exactly the one marker byte is accepted
/// and every other byte is a typed protocol violation.
///
/// A non-marker byte — a desynchronized peer's next preamble included —
/// must surface [`Error::Epilogue`] with `InvalidData`, distinguishing a
/// protocol violation from an honest wire cut; the marker itself completes
/// the session.
#[test]
fn marker_byte_space_is_exhaustive() {
    for byte in u8::MIN..=u8::MAX {
        let bytes = [byte];
        let mut reader = &bytes[..];
        let mut writer = tokio::io::sink();
        let result = pollster::block_on(epilogue(&mut reader, &mut writer));
        if byte == EPILOGUE_MARKER {
            result.expect("the marker byte completes the epilogue");
        } else {
            let error = epilogue_error(result);
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::InvalidData,
                "byte {byte:#04x} must be a typed protocol violation",
            );
        }
    }
}

/// A peer that closes before its marker is a typed EOF, not a hang.
///
/// The honest wire cut must surface [`Error::Epilogue`] with
/// `UnexpectedEof` — the arm the two-generals residue lands on — kept
/// distinct from the `InvalidData` violation above so operators can tell a
/// dead link from a desynchronized peer.
#[test]
fn close_before_the_marker_is_a_typed_eof() {
    let mut reader: &[u8] = &[];
    let mut writer = tokio::io::sink();
    let result = pollster::block_on(epilogue(&mut reader, &mut writer));
    let error = epilogue_error(result);
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// Reading the marker consumes exactly one byte, leaving later bytes
/// untouched.
///
/// A next session's preamble may already sit behind the marker on a reused
/// link; the epilogue must not slurp it. After a clean exchange the
/// following bytes remain unread in the transport.
#[test]
fn bytes_after_the_marker_stay_untouched() {
    let bytes = [EPILOGUE_MARKER, b'R', b'U'];
    let mut reader = &bytes[..];
    let mut writer = tokio::io::sink();
    pollster::block_on(epilogue(&mut reader, &mut writer)).expect("the marker completes");
    assert_eq!(reader, b"RU", "the next session's bytes were consumed");
}

// ---- the greeting version of a bootstrap claimant --------------------------

/// An empty tree root whose ceiling records `events` committed-then-redacted
/// events: the exact shape a misdeclaring bootstrap claimant would present,
/// nothing to provide plus a version that dominates any replica with fewer
/// events.
///
/// Built with real semantics rather than forged bytes: a redaction advances
/// the ceiling and leaves no tombstone, so committing and then redacting
/// `events` messages leaves an empty root carrying a genuine `events`-tick
/// version.
fn redacted_history_root(events: u64) -> tree::Root<u64> {
    let donor = Peer::<u64>::seed();
    {
        let mut batch = donor.batch();
        for v in 0..events {
            batch.send(v);
        }
    }
    let keys: Vec<_> = donor.snapshot().iter().map(|(key, _, _)| key).collect();
    {
        let mut batch = donor.batch();
        for key in keys {
            batch.redact(key);
        }
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
    root: tree::Root<u64>,
) -> Result<(Party, Tree<u64>), Error> {
    let (read, write, connector, acceptor, epoch) = erase(link)?;
    let mut staged = handshake::Staged::new();
    handshake::preamble(
        Protocol::V2,
        Network::BOOTSTRAP,
        Intent::Remain,
        &mut staged,
        read,
        write,
    )
    .await
    .map_err(Error::from)?;
    let local_root: streaming::Root<Local, u64> = root.into();
    let local = materialized::Handshaking::start(Local, local_root);
    let carrier = Link::for_session(read, write, connector, acceptor, epoch);
    let proxy = streaming_remote::Handshaking::start(Local, carrier);
    let handshaken = streaming::handshake(local, proxy)
        .await
        .map_err(streaming_error)?;
    let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
    let (root, (mut read, mut write)) = descent.await.map_err(streaming_error)?;
    let party = party::receive(&mut read).await?;
    epilogue(&mut read, &mut write).await?;
    Ok((party, Tree { root: root.into() }))
}

/// Drive one V1 session as a bootstrap claimant whose greeting version comes
/// from `root` instead of a newborn's empty version.
///
/// [`claim_bootstrap_v2`]'s alternating-protocol twin; the frozen V1 wire has
/// no epilogue.
async fn claim_bootstrap_v1(
    link: &mut MemoryLink,
    root: tree::Root<u64>,
) -> Result<(Party, Tree<u64>), Error> {
    let (read, write, _connector, _acceptor, _epoch) = erase(link)?;
    let mut staged = handshake::Staged::new();
    handshake::preamble(
        Protocol::V1,
        Network::BOOTSTRAP,
        Intent::Remain,
        &mut staged,
        read,
        write,
    )
    .await
    .map_err(Error::from)?;
    let local = alternating_local::Exchange::start(root);
    let proxy = alternating_remote::Exchange::start(FrameRead::new(read), FrameWrite::new(write));
    let handshaken = alternating::handshake(local, proxy)
        .await
        .map_err(alternating_error)?;
    let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
    let (root, (read, _write)) = descent.await.map_err(alternating_error)?;
    let mut read = read.into_inner();
    let party = party::receive(&mut read).await?;
    Ok((party, Tree { root }))
}

/// A provider holding `values`, plus its pre-session content hash.
fn provider_with(values: &[u64]) -> Peer<u64> {
    let provider = Peer::<u64>::seed();
    {
        let mut batch = provider.batch();
        for &v in values {
            batch.send(v);
        }
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
/// greeting declares causal history: [`Error::BootstrapHistoryConflict`],
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
    let claimant_tree = Tree {
        root: redacted_history_root(8),
    };
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
        Err(Error::BootstrapHistoryConflict {
            claimed_min_events: reported,
        }) => assert_eq!(
            reported, claimed_min_events,
            "the error carries the claimed history's event floor",
        ),
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

/// A provider serving a bootstrap rejects, under V1, a claimant whose
/// greeting declares causal history: [`Error::BootstrapHistoryConflict`],
/// nothing moved.
///
/// The alternating protocol's deletion-honoring filter trusts the greeting
/// version exactly as the streaming protocol's does, so the newborn
/// requirement is enforced protocol-independently, above both descents.
#[test]
fn v1_bootstrap_claimant_declaring_history_is_rejected() {
    let provider = provider_with(&[1, 2, 3]).protocol(Protocol::V1);
    let hash_before = provider.snapshot().hash();
    let party_before = party_of(&provider);
    let claimant_root = redacted_history_root(8);

    let provider_ref = &provider;
    let (claim_out, provider_out) = pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        tokio::join!(
            async move { claim_bootstrap_v1(&mut a_link, claimant_root).await },
            async move { provider_ref.gossip(&mut b_link).await },
        )
    });

    assert!(
        matches!(provider_out, Err(Error::BootstrapHistoryConflict { .. })),
        "the provider rejects the claimant, got {provider_out:?}",
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
/// surfaces the same [`Error::BootstrapHistoryConflict`] a serving
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
        matches!(join_out, Err(Error::BootstrapHistoryConflict { .. })),
        "the joining side rejects the counterparty's claimed history, got {join_out:?}",
    );
    assert!(
        claim_out.is_err(),
        "the rejected counterparty's session fails, got {claim_out:?}",
    );
}

/// A joining peer rejects, under V1, a mutual-bootstrap counterparty whose
/// greeting declares causal history.
///
/// [`v2_mutual_bootstrap_counterparty_with_history_is_rejected`]'s
/// alternating-protocol twin: the rejection replaces the mutual bail on
/// the frozen V1 wire too.
#[test]
fn v1_mutual_bootstrap_counterparty_with_history_is_rejected() {
    let (join_out, claim_out) = pollster::block_on(async {
        let (mut a_link, mut b_link) = memory();
        tokio::join!(
            async move {
                Peer::<u64>::bootstrap()
                    .protocol(Protocol::V1)
                    .join(&mut a_link)
                    .await
            },
            async move { claim_bootstrap_v1(&mut b_link, redacted_history_root(8)).await },
        )
    });

    assert!(
        matches!(join_out, Err(Error::BootstrapHistoryConflict { .. })),
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
            matches!(provider_out, Err(Error::BootstrapHistoryConflict { .. })),
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
