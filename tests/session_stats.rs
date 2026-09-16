//! Session statistics match the messages reconciled and the bytes carried.
//!
//! Message counts are checked against recorded sends and redactions, including
//! both peers shedding messages in one session. Byte counts are checked against
//! captured data streams, excluding their parsed labels, across reused links.
//! Point tests cover catch-up, one-sided redaction, no-op sessions, the minimum
//! window, and the continuous gossip driver's results.

mod common;

use std::collections::BTreeMap;

use futures::StreamExt;
use proptest::prelude::*;
use rumors::testing::stream_label;
use rumors::{Led, Peer, Rumors, SessionStats};

use crate::common::gossip_snapshot::{CaptureLink, capture_sides};
use crate::common::oracle::readout;
use crate::common::wire::{
    LINK_BUF, assert_control_drained, block_on, bootstrap_fork_async, gossip_pair_async,
};

/// An empty replica learns the provider's messages without resolving disputes.
/// Both one-shot calls report local initiation and the same final frontier.
#[test]
fn catchup_gains_the_providers_count_without_disputes() {
    block_on(async {
        let provider: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        // Join before any sends so the next session transfers the whole set.
        let empty = bootstrap_fork_async(&provider).await;
        provider.send_all([1, 2, 3]).unwrap();

        let (p, e) = gossip_pair_async(&provider, &empty).await;
        assert_eq!(p.led, Led::Local, "a one-shot call is this side's trigger");
        assert_eq!(e.led, Led::Local);
        assert_eq!(p.converged, e.converged, "one session, one frontier");

        assert_eq!(e.stats.messages_gained, 3, "the catch-up learns everything");
        assert_eq!(e.stats.messages_shed, 0);
        assert_eq!(p.stats.messages_gained, 0);
        assert_eq!(p.stats.messages_shed, 0);
        assert_eq!(p.stats.disputed_scopes, 0, "supplies are not disputes");
        assert_eq!(e.stats.disputed_scopes, 0);
        assert!(
            p.stats.bytes_sent > 0,
            "the session sent reconciliation frames"
        );
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
        a.send_all([7]).unwrap();
        let _ = gossip_pair_async(&a, &b).await;

        let (a_g, b_g) = gossip_pair_async(&a, &b).await;
        assert_eq!(a_g.stats, SessionStats::default());
        assert_eq!(b_g.stats, SessionStats::default());
    });
}

/// Honoring a remote redaction counts as one shed; the redactor counts no changes.
#[test]
fn honored_redaction_counts_as_shed() {
    block_on(async {
        let a = Peer::<u64>::seed().sync_window_floor().into_rumors();
        a.send_all([10, 20]).unwrap();
        let b = bootstrap_fork_async(&a).await;
        let version = a.snapshot().iter().next().unwrap().0.clone();
        a.redact(&version);

        let (a_g, b_g) = gossip_pair_async(&a, &b).await;
        assert_eq!(b_g.stats.messages_shed, 1);
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
        a.send_all([1]).unwrap();
        b.send_all([2]).unwrap();

        let (a_g, b_g) = gossip_pair_async(&a, &b).await;
        assert_eq!(a_g.stats.window_granted, 1);
        assert_eq!(b_g.stats.window_granted, 1);
    });
}

/// The `gossip` stream carries the same per-session stats: a served
/// remote push reports what the session gained.
#[test]
fn gossip_stream_reports_session_stats() {
    block_on(async {
        let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        let b = bootstrap_fork_async(&a).await;
        a.send_all([42]).unwrap();

        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let mut a_drive = a.gossip(&mut a_link);
        // The serving side's policy stream stays quiet, so its session is
        // remote-led (a `changes()` stream's first tick would initiate).
        let b = b
            .try_into_peer()
            .await
            .unwrap()
            .gossip_when(|_| futures::stream::pending::<()>())
            .into_rumors();
        let mut b_drive = b.gossip(&mut b_link);
        let (pushed, served) = tokio::join!(a_drive.next(), b_drive.next());
        let pushed = pushed.expect("driver running").expect("push succeeds");
        let served = served.expect("driver running").expect("serve succeeds");

        assert_eq!(served.led, Led::Remote);
        assert_eq!(served.stats.messages_gained, 1);
        assert_eq!(pushed.stats.messages_gained, 0);
        assert_eq!(pushed.stats.bytes_sent, served.stats.bytes_received);
        drop((a_drive, b_drive));
        assert_control_drained(a_link, b_link);
    });
}

/// Capture three divergent sessions after advancing an empty link to `initial_epoch`.
async fn record_sessions(
    peer: &Rumors<Vec<u8>>,
    mut link: CaptureLink,
    initial_epoch: u8,
    sends: &[Vec<u8>],
    stats: &mut Vec<SessionStats>,
) {
    // Converged sessions advance the epoch without opening data streams,
    // so the capture contains only the divergent sessions measured below.
    for _ in 0..initial_epoch {
        let result = peer.gossip_once(&mut link).await.expect("empty session");
        assert_eq!(result.stats, SessionStats::default());
    }
    for _ in 0..3 {
        peer.send_all(sends.iter().cloned()).unwrap();
        stats.push(peer.gossip_once(&mut link).await.expect("gossip").stats);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Each session counts every data-frame byte exactly once, excluding stream labels.
    ///
    /// Reuse the link across consecutive epochs, favoring the CBOR length boundary
    /// and the epoch wrap. Read label lengths from the capture instead of assuming
    /// their encoded size. The sender's count must also match its peer's receiver.
    #[test]
    fn byte_counters_match_the_transport_tally(
        initial_epoch in prop_oneof![Just(23u8), Just(255u8), any::<u8>()],
        a_sends in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..512), 1..8),
        b_sends in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..512), 1..8),
    ) {
        let a = Peer::seed().sync_window_floor().into_rumors();
        let b = block_on(bootstrap_fork_async(&a));
        let (mut a_stats, mut b_stats) = (Vec::new(), Vec::new());
        let (a_capture, b_capture) = capture_sides(
            |link| record_sessions(&a, link, initial_epoch, &a_sends, &mut a_stats),
            |link| record_sessions(&b, link, initial_epoch, &b_sends, &mut b_stats),
        );

        for (capture, sessions) in [(a_capture, &a_stats), (b_capture, &b_stats)] {
            let mut sent = BTreeMap::<u8, u64>::new();
            for stream in capture.streams {
                let ((epoch, _), label_len) = stream_label(&stream);
                *sent.entry(epoch).or_default() += (stream.len() - label_len) as u64;
            }
            for (round, stats) in sessions.iter().enumerate() {
                let epoch = initial_epoch.wrapping_add(round as u8);
                prop_assert!(stats.bytes_sent > 0, "each session sends new messages");
                prop_assert_eq!(sent.remove(&epoch), Some(stats.bytes_sent), "epoch {}", epoch);
            }
            prop_assert!(sent.is_empty(), "every captured stream belongs to a measured session");
        }
        for (a, b) in a_stats.iter().zip(&b_stats) {
            prop_assert_eq!(a.bytes_sent, b.bytes_received);
            prop_assert_eq!(b.bytes_sent, a.bytes_received);
        }
    }

    /// Session gains and sheds count the exact message versions added and removed.
    ///
    /// Both replicas must learn the other's redactions in every case. Additional
    /// shared redactions and local sends vary independently; equal payloads still
    /// count separately because messages are keyed by version.
    #[test]
    fn sessions_report_each_gain_and_shed(
        shared in prop::collection::vec(any::<u64>(), 2..24),
        a_sends in prop::collection::vec(any::<u64>(), 0..24),
        b_sends in prop::collection::vec(any::<u64>(), 0..24),
        redactions in prop::collection::vec((any::<bool>(), any::<prop::sample::Index>()), 0..24),
    ) {
        block_on(async {
            let a = Peer::seed().sync_window_floor().into_rumors();
            a.send_all(shared).unwrap();
            let b = bootstrap_fork_async(&a).await;
            let shared_versions: Vec<_> = a.snapshot().iter()
                .map(|(version, _)| version.clone()).collect();
            let mut expected = readout(&a.snapshot());

            // Redact distinct shared messages at opposite peers. Exclude these
            // two from the optional redactions so both peers must shed at least
            // one message during gossip, even after shrinking the generated case.
            for (peer, version) in [(&a, &shared_versions[0]), (&b, &shared_versions[1])] {
                peer.redact(version);
                expected.remove(version.as_bytes());
            }
            let remaining = &shared_versions[2..];
            for (at_b, index) in redactions {
                if !remaining.is_empty() {
                    let version = &remaining[index.index(remaining.len())];
                    let peer = if at_b { &b } else { &a };
                    peer.redact(version);
                    expected.remove(version.as_bytes());
                }
            }
            for (peer, values) in [(&a, a_sends), (&b, b_sends)] {
                for value in values {
                    let version = peer.send(value).unwrap();
                    expected.insert(version.as_bytes().to_vec(), value);
                }
            }
            let (a_before, b_before) = (readout(&a.snapshot()), readout(&b.snapshot()));
            let (a_g, b_g) = gossip_pair_async(&a, &b).await;

            prop_assert_eq!(readout(&a.snapshot()), expected.clone());
            prop_assert_eq!(readout(&b.snapshot()), expected.clone());
            for (before, stats) in [(a_before, a_g.stats), (b_before, b_g.stats)] {
                let gained = expected.keys().filter(|v| !before.contains_key(*v)).count() as u64;
                let shed = before.keys().filter(|v| !expected.contains_key(*v)).count() as u64;
                prop_assert!(shed > 0);
                // A net-length check alone misses equal errors in both counters.
                prop_assert_eq!(stats.messages_gained, gained);
                prop_assert_eq!(stats.messages_shed, shed);
            }
            prop_assert_eq!(a_g.stats.bytes_sent, b_g.stats.bytes_received);
            prop_assert_eq!(b_g.stats.bytes_sent, a_g.stats.bytes_received);
            prop_assert_eq!(a_g.converged, b_g.converged);
            Ok(())
        })?;
    }
}
