//! Session statistics match the messages reconciled and the bytes carried.
//!
//! Message counts are checked against recorded sends and redactions, including
//! both peers shedding messages in one session. Byte counts are checked against
//! the frames captured on one reused connection. Point tests cover catch-up,
//! one-sided redaction, no-op sessions, and the continuous gossip driver's
//! results. Budget waits are checked here at an unlimited budget, where none
//! can occur, and against the ledger's own count in the session-budget suite,
//! where the budget binds.

use rumors_testkit::common;

use ciborium::value::Value;
use futures::StreamExt;
use proptest::prelude::*;
use rumors::{Led, Rumors, SessionStats};

use crate::common::budget::BudgetChoice;
use crate::common::gossip_snapshot::{CaptureEnd, capture_sides};
use crate::common::oracle::readout;
use crate::common::wire::{
    assert_connection_drained, block_on, bootstrap_fork_async, bootstrap_fork_with_budget_async,
    gossip_pair_async,
};
use rumors::testing::MEMORY_CAPACITY;
use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

/// One session's frames in a captured direction: their total bytes and their
/// count, parsed without the Rumors codec.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Frames {
    /// Bytes of every frame.
    bytes: u64,
    /// Number of frames.
    count: u64,
}

/// Split one direction's captured bytes into sessions and tally each session's
/// frames.
///
/// Every session opens with its preamble, a tagged item, and a frame is an
/// untagged array, so a generic CBOR parse finds both without the Rumors codec.
fn frames_per_session(bytes: &[u8]) -> Vec<Frames> {
    let mut sessions = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let mut input = rest;
        let item: Value =
            ciborium::de::from_reader(&mut input).expect("captured items are valid CBOR");
        let len = (rest.len() - input.len()) as u64;
        match item {
            Value::Tag(PREAMBLE_TAG, _) => sessions.push(Frames::default()),
            Value::Array(_) => {
                let session = sessions.last_mut().expect("a preamble opens every session");
                session.bytes += len;
                session.count += 1;
            }
            _ => {}
        }
        rest = input;
    }
    sessions
}

/// The self-described CBOR tag that opens every session's preamble.
const PREAMBLE_TAG: u64 = 55799;

/// Clear the timing measurement when asserting that a session did no work.
fn without_elapsed(mut stats: SessionStats) -> SessionStats {
    stats.elapsed = std::time::Duration::ZERO;
    stats
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// An empty replica learns the provider's messages without resolving disputes.
    /// Both one-shot calls report local initiation and the same final frontier.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn catchup_gains_the_providers_count_without_disputes(network in network_seed()) {
        block_on(async {
            let provider: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
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
    /// end the session at the greeting, so no data stream carries a frame and
    /// nothing is counted.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn converged_replicas_report_zero_stats(network in network_seed()) {
        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let b = bootstrap_fork_async(&a).await;
            a.send_all([7]).unwrap();
            let _ = gossip_pair_async(&a, &b).await;

            let (a_g, b_g) = gossip_pair_async(&a, &b).await;
            assert_eq!(without_elapsed(a_g.stats), SessionStats::default());
            assert_eq!(without_elapsed(b_g.stats), SessionStats::default());
        });
    }

    /// Honoring a remote redaction counts as one shed; the redactor counts no changes.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn honored_redaction_counts_as_shed(network in network_seed()) {
        block_on(async {
            let a = network.peer::<u64>().sync_memory_budget(0).into_rumors();
            a.send_all([10, 20]).unwrap();
            let b = bootstrap_fork_async(&a).await;
            let version = a.snapshot().versions().next().unwrap().clone();
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

    /// The `gossip` stream carries the same per-session stats: a served
    /// remote push reports what the session gained.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn gossip_stream_reports_session_stats(network in network_seed()) {
        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let b = bootstrap_fork_async(&a).await;
            a.send_all([42]).unwrap();

            let (mut a_end, mut b_end) = rumors::testing::memory_connection(MEMORY_CAPACITY);
            let mut a_drive = a.gossip_stream(&mut a_end.read, &mut a_end.write);
            // The serving side's policy stream stays quiet, so its session is
            // remote-led (a `changes()` stream's first tick would initiate).
            let b = b
                .try_into_peer()
                .await
                .unwrap()
                .gossip_when(|_| futures::stream::pending::<()>())
                .into_rumors();
            let mut b_drive = b.gossip_stream(&mut b_end.read, &mut b_end.write);
            let (pushed, served) = tokio::join!(a_drive.next(), b_drive.next());
            let pushed = pushed.expect("driver running").expect("push succeeds");
            let served = served.expect("driver running").expect("serve succeeds");

            assert_eq!(served.led, Led::Remote);
            assert_eq!(served.stats.messages_gained, 1);
            assert_eq!(pushed.stats.messages_gained, 0);
            assert_eq!(pushed.stats.bytes_sent, served.stats.bytes_received);
            drop((a_drive, b_drive));
            assert_connection_drained(a_end, b_end);
        });
    }
}

/// Capture three divergent sessions over one reused connection.
async fn record_sessions(
    peer: &Rumors<Vec<u8>>,
    mut end: CaptureEnd,
    sends: &[Vec<u8>],
    stats: &mut Vec<SessionStats>,
) {
    for _ in 0..3 {
        peer.send_all(sends.iter().cloned()).unwrap();
        let (gossiped, ..) = peer
            .gossip_once(&mut end.read, &mut end.write)
            .await
            .expect("gossip");
        stats.push(gossiped.stats);
    }
}

proptest! {
    /// Each session counts every frame byte exactly once, and only frame
    /// bytes: the preamble, greeting, and closing items that share the
    /// connection are excluded.
    ///
    /// Three sessions reuse one connection; the capture's bytes split into
    /// sessions at each preamble. The sender's count must also match its
    /// peer's receiver.
    #[test]
    fn byte_counters_match_the_transport_tally(
        a_sends in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..512), 1..8),
        b_sends in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..512), 1..8),
        network in network_seed(),
    ) {
        let a = network.peer().sync_memory_budget(0).into_rumors();
        let b = block_on(bootstrap_fork_async(&a));
        let (mut a_stats, mut b_stats) = (Vec::new(), Vec::new());
        let (a_capture, b_capture) = capture_sides(
            |end| record_sessions(&a, end, &a_sends, &mut a_stats),
            |end| record_sessions(&b, end, &b_sends, &mut b_stats),
        );

        for (capture, sessions) in [(a_capture, &a_stats), (b_capture, &b_stats)] {
            let captured = frames_per_session(&capture.sent);
            prop_assert_eq!(captured.len(), sessions.len(), "one preamble per session");
            for (frames, stats) in captured.iter().zip(sessions.iter()) {
                prop_assert!(stats.bytes_sent > 0, "each session sends new messages");
                prop_assert_eq!(frames.bytes, stats.bytes_sent);
                prop_assert_eq!(frames.count, stats.frames_sent);
            }
        }
        for (a, b) in a_stats.iter().zip(&b_stats) {
            prop_assert_eq!(a.bytes_sent, b.bytes_received);
            prop_assert_eq!(b.bytes_sent, a.bytes_received);
            prop_assert_eq!(a.frames_sent, b.frames_received);
            prop_assert_eq!(b.frames_sent, a.frames_received);
        }
    }

    /// A session with an unlimited budget reports no budget waits on either
    /// side: admission then has room for every question and answer as soon
    /// as the session is ready to build it, whatever the two replicas hold.
    ///
    /// Both sides add independently drawn messages after the fork, so the
    /// sessions range from converged through disputes at several levels.
    #[test]
    fn an_unlimited_budget_reports_no_waits(
        shared in prop::collection::vec(any::<u64>(), 0..256),
        a_sends in prop::collection::vec(any::<u64>(), 0..256),
        b_sends in prop::collection::vec(any::<u64>(), 0..256),
        network in network_seed(),
    ) {
        block_on(async {
            let unlimited = BudgetChoice::Bytes(usize::MAX);
            let a = unlimited.apply(network.peer()).into_rumors();
            a.send_all(shared).unwrap();
            let b = bootstrap_fork_with_budget_async(&a, unlimited).await;
            a.send_all(a_sends).unwrap();
            b.send_all(b_sends).unwrap();
            let (a_g, b_g) = gossip_pair_async(&a, &b).await;
            prop_assert_eq!(a_g.stats.budget_waits, 0);
            prop_assert_eq!(b_g.stats.budget_waits, 0);
            Ok(())
        })?;
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
        network in network_seed(),
    ) {
        block_on(async {
            let a = network.peer().sync_memory_budget(0).into_rumors();
            a.send_all(shared).unwrap();
            let b = bootstrap_fork_async(&a).await;
            let shared_versions: Vec<_> = a.snapshot().versions().cloned().collect();
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
