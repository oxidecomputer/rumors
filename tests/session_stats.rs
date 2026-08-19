//! Public-surface pins for [`rumors::SessionStats`]: the per-session
//! counters carried by [`rumors::Gossiped`].
//!
//! The walk-tier suite (`src/tree/mirror/streaming/tests/stats.rs`) pins
//! the counters against an in-memory dispute oracle; here the same
//! counters are checked where an application reads them (one-shot
//! [`Rumors::gossip`], the [`Rumors::gossip_when`] stream), plus the
//! wire-only claims that need a real link: the byte counters against an
//! independent transport-level tally, and the conservation law
//! `len_after = len_before + gained - shed` over real sessions.

mod common;

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures::StreamExt;
use proptest::prelude::*;
use rumors::link::{Connector, Done, Link, LinkParts, MemoryLink};
use rumors::{Gossiped, Led, Peer, Rumors, SessionStats};
use tokio::io::AsyncWrite;

use crate::common::wire::{LINK_BUF, assert_control_drained, block_on, bootstrap_fork_async};

/// Run one gossip session between two handles over an in-memory link,
/// returning both sides' [`Gossiped`].
async fn gossip_pair<T>(a: &Rumors<T>, b: &Rumors<T>) -> (Gossiped, Gossiped)
where
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
    let pair = (a_out.expect("gossip A"), b_out.expect("gossip B"));
    assert_control_drained(a_link, b_link);
    pair
}

/// A one-shot session reports its datum on both ends, and a catch-up
/// session disputes nothing.
///
/// An established but empty replica catching up gains exactly the
/// provider's live count with zero disputes on both sides (supplies are
/// not disputes), the provider moves nothing, both report `Led::Local`,
/// and both converge on one version.
#[test]
fn catchup_gains_the_providers_count_without_disputes() {
    block_on(async {
        let provider: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        // Fork while empty: the newcomer holds an identity but no content.
        let empty = bootstrap_fork_async(&provider).await;
        provider.batch().send(1).send(2).send(3);

        let (p, e) = gossip_pair(&provider, &empty).await;
        assert_eq!(p.led, Led::Local, "a one-shot call is this side's trigger");
        assert_eq!(e.led, Led::Local);
        assert_eq!(p.converged, e.converged, "one session, one frontier");

        assert_eq!(e.stats.messages_gained, 3, "the catch-up learns everything");
        assert_eq!(e.stats.messages_shed, 0);
        assert_eq!(p.stats.messages_gained, 0);
        assert_eq!(p.stats.messages_shed, 0);
        assert_eq!(p.stats.disputed_scopes, 0, "supplies are not disputes");
        assert_eq!(e.stats.disputed_scopes, 0);
        assert!(p.stats.bytes_sent > 0, "the content crossed the codec seam");
        assert_eq!(empty.snapshot().len(), 3);
    });
}

/// Converged replicas report zero in every field: equal greeting versions
/// end the session at the greeting, so no window is derived, no data
/// stream opens, and nothing is counted.
#[test]
fn converged_replicas_report_zero_stats() {
    block_on(async {
        let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork_async(&a).await;
        a.batch().send(7);
        let _ = gossip_pair(&a, &b).await;

        let (a_g, b_g) = gossip_pair(&a, &b).await;
        assert_eq!(a_g.stats, SessionStats::default());
        assert_eq!(b_g.stats, SessionStats::default());
    });
}

/// A redaction is honored as a shed: the peer still holding the message
/// drops its copy (one shed, nothing gained on either side), and the pair
/// converges on the smaller set.
#[test]
fn honored_redaction_counts_as_shed() {
    block_on(async {
        let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        a.batch().send(10).send(20);
        let b = bootstrap_fork_async(&a).await;
        let version = a
            .snapshot()
            .iter()
            .find(|(_, value)| ***value == 10)
            .map(|(version, _)| version.clone())
            .expect("the sent message is live");
        a.redact(&version);

        let (a_g, b_g) = gossip_pair(&a, &b).await;
        assert_eq!(b_g.stats.messages_shed, 1, "b honors a's deletion");
        assert_eq!(b_g.stats.messages_gained, 0);
        assert_eq!(a_g.stats.messages_shed, 0);
        assert_eq!(a_g.stats.messages_gained, 0);
        assert_eq!(a.snapshot().len(), 1);
        assert_eq!(b.snapshot().len(), 1);
    });
}

/// The floor window grants exactly one scope at its widest stage; the
/// value a session reports is the resolved window's widest capacity.
#[test]
fn floor_window_reports_one_granted_scope() {
    block_on(async {
        let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork_async(&a).await;
        a.batch().send(1);
        b.batch().send(2);

        let (a_g, b_g) = gossip_pair(&a, &b).await;
        assert_eq!(a_g.stats.window_granted, 1);
        assert_eq!(b_g.stats.window_granted, 1);
    });
}

/// The `gossip_when` stream carries the same per-session stats: a served
/// remote push reports what the session gained.
#[test]
fn gossip_when_reports_session_stats() {
    block_on(async {
        let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork_async(&a).await;
        a.batch().send(42);

        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let mut a_drive = a.gossip_when(a.changes(), &mut a_link);
        // The serving side's policy stream stays quiet, so its session is
        // remote-led (a `changes()` stream's first tick would initiate).
        let mut b_drive = b.gossip_when(futures::stream::pending(), &mut b_link);
        let (pushed, served) = tokio::join!(a_drive.next(), b_drive.next());
        let pushed = pushed.expect("driver running").expect("push succeeds");
        let served = served.expect("driver running").expect("serve succeeds");

        assert_eq!(served.led, Led::Remote);
        assert_eq!(served.stats.messages_gained, 1);
        assert_eq!(pushed.stats.messages_gained, 0);
        assert_eq!(pushed.stats.bytes_sent, served.stats.bytes_received);
    });
}

// ---- the byte counters against an independent transport tally -------------

/// An `AsyncWrite` that tallies every byte accepted by the inner writer.
struct CountingWrite<W> {
    inner: W,
    written: Arc<AtomicUsize>,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CountingWrite<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(accepted)) = &poll {
            self.written.fetch_add(*accepted, Ordering::Relaxed);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A [`Connector`] that tallies data-stream bytes and counts stream opens.
#[derive(Clone)]
struct CountingConnector<C> {
    inner: C,
    written: Arc<AtomicUsize>,
    opens: Arc<AtomicUsize>,
}

impl<C: Connector> Connector for CountingConnector<C> {
    type Tx = CountingWrite<C::Tx>;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        self.opens.fetch_add(1, Ordering::Relaxed);
        let (inner, _) = self.inner.connect().await?;
        Ok((
            CountingWrite {
                inner,
                written: self.written.clone(),
            },
            Done::discard(),
        ))
    }
}

/// One link end whose data-stream writes are tallied independently of the
/// crate's own counters, with the opened-stream count alongside.
#[allow(clippy::type_complexity)]
fn counting_link(
    link: MemoryLink,
) -> (
    Link<
        tokio::io::DuplexStream,
        tokio::io::DuplexStream,
        CountingConnector<rumors::link::MemoryConnector>,
        rumors::link::MemoryAcceptor,
    >,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
) {
    let written = Arc::new(AtomicUsize::new(0));
    let opens = Arc::new(AtomicUsize::new(0));
    let parts = link.into_parts();
    let link = LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: CountingConnector {
            inner: parts.connector,
            written: written.clone(),
            opens: opens.clone(),
        },
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link();
    (link, written, opens)
}

/// Bytes of the label a sender writes before its first frame; the codec
/// seam's counters exclude it, so the transport tally exceeds
/// `bytes_sent` by exactly this much per opened stream.
const LABEL_LEN: usize = 2;

/// The byte counters measure exactly the codec seam.
///
/// Each side's transport-level data-stream tally equals its reported
/// `bytes_sent` plus one label per opened stream, and over the lossless
/// in-memory link one side's `bytes_sent` is the other's
/// `bytes_received`.
#[test]
fn byte_counters_match_the_transport_tally() {
    block_on(async {
        let a: Rumors<Vec<u8>> = Peer::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork_async(&a).await;
        {
            let mut batch = a.batch();
            for i in 0u8..20 {
                batch.send(vec![i; 64]);
            }
        }
        {
            let mut batch = b.batch();
            for i in 0u8..20 {
                batch.send(vec![0xa0 | (i & 0x0f); 96]);
            }
        }

        let (a_raw, b_raw) = rumors::link::memory_with_capacity(LINK_BUF);
        let (mut a_link, a_written, a_opens) = counting_link(a_raw);
        let (mut b_link, b_written, b_opens) = counting_link(b_raw);
        let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
        let a_g = a_out.expect("gossip A");
        let b_g = b_out.expect("gossip B");

        let a_written = a_written.load(Ordering::Relaxed) as u64;
        let b_written = b_written.load(Ordering::Relaxed) as u64;
        let a_labels = (a_opens.load(Ordering::Relaxed) * LABEL_LEN) as u64;
        let b_labels = (b_opens.load(Ordering::Relaxed) * LABEL_LEN) as u64;

        assert!(
            a_g.stats.bytes_sent > 0,
            "content crossed in both directions"
        );
        assert!(b_g.stats.bytes_sent > 0);
        assert_eq!(a_written, a_g.stats.bytes_sent + a_labels);
        assert_eq!(b_written, b_g.stats.bytes_sent + b_labels);
        assert_eq!(a_g.stats.bytes_sent, b_g.stats.bytes_received);
        assert_eq!(b_g.stats.bytes_sent, a_g.stats.bytes_received);
    });
}

/// A V1 session reports zero in every field: the alternating
/// implementation computes none of the counters, and each field
/// documents the zero. The session itself still converges the pair.
#[cfg(feature = "protocol-v1")]
#[test]
fn v1_sessions_report_zero_stats() {
    use rumors::Protocol;

    use crate::common::wire::bootstrap_fork_async_with_protocol;

    block_on(async {
        let a: Rumors<u64> = Peer::seed()
            .sync_window_floor()
            .protocol(Protocol::V1)
            .into_rumors();
        let b = bootstrap_fork_async_with_protocol(&a, Protocol::V1).await;
        a.batch().send(5);
        b.batch().send(6);

        let (a_g, b_g) = gossip_pair(&a, &b).await;
        assert_eq!(a_g.stats, SessionStats::default());
        assert_eq!(b_g.stats, SessionStats::default());
        // The session was not a no-op: the transfer happened, uncounted.
        assert_eq!(a.snapshot().len(), 2);
        assert_eq!(b.snapshot().len(), 2);
    });
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Over random two-sided divergence, every real session conserves the
    /// live count on each side (`after = before + gained - shed`), and
    /// the two ends' byte counters mirror each other over the lossless
    /// in-memory link.
    #[test]
    fn sessions_conserve_the_live_count(
        a_sends in proptest::collection::vec(any::<u64>(), 0..24),
        b_sends in proptest::collection::vec(any::<u64>(), 0..24),
    ) {
        block_on(async {
            let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
            let b = bootstrap_fork_async(&a).await;
            {
                let mut batch = a.batch();
                for send in &a_sends {
                    batch.send(*send);
                }
            }
            {
                let mut batch = b.batch();
                for send in &b_sends {
                    batch.send(*send);
                }
            }
            let a_before = a.snapshot().len() as u64;
            let b_before = b.snapshot().len() as u64;

            let (a_g, b_g) = gossip_pair(&a, &b).await;

            prop_assert_eq!(
                a.snapshot().len() as u64,
                a_before + a_g.stats.messages_gained - a_g.stats.messages_shed,
            );
            prop_assert_eq!(
                b.snapshot().len() as u64,
                b_before + b_g.stats.messages_gained - b_g.stats.messages_shed,
            );
            prop_assert_eq!(a_g.stats.bytes_sent, b_g.stats.bytes_received);
            prop_assert_eq!(b_g.stats.bytes_sent, a_g.stats.bytes_received);
            prop_assert_eq!(a.snapshot().hash(), b.snapshot().hash());
            Ok(())
        })?;
    }
}
