//! Check the wire-observation contract against independent transport captures.
//!
//! Each callback must contain one complete CBOR item. Grouped by directed
//! stream, those items must reproduce the connection's bytes: a generic CBOR
//! parse of each direction separates its control items from its frames, and
//! its frames by the logical stream index each names. The handler set must
//! match the streams actually used.
//!
//! Generated gossip, bootstrap, and retirement sessions check this contract
//! alongside session kinds and role elections. A separate property compares
//! observed and unobserved sessions to ensure attachment leaves the wire intact.

use rumors_testkit::common;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use ciborium::value::Value;
use futures::FutureExt;
use proptest::prelude::*;
use rumors::observe::{
    Direction, Observer, Role, SessionErrorKind, SessionInfo, SessionKind, SessionObserver,
    SessionOutcome, StreamId, StreamInfo, StreamObserver,
};
use rumors::testing::PipeCapture;
use rumors::{Peer, Retire, Rumors, SessionStats, Version};

use crate::common::budget::BudgetChoice;
use crate::common::gossip_snapshot::{capture_sides, corpora, payloads};
use crate::common::wire::{block_on, bootstrap_fork_configured};
use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::divided_proptest;

/// Retain every observation callback for comparison with the transport.
#[derive(Default)]
struct Recording {
    /// Sessions not yet checked by the test.
    sessions: Mutex<Vec<Arc<SessionRecord>>>,
}

/// One session's metadata, election, and directed streams.
struct SessionRecord {
    /// Lifecycle operation reported at session creation.
    info: SessionInfo,
    /// Role reported before any data handler opens.
    elected: Mutex<Option<Role>>,
    /// Every stream handler created, including ones receiving no messages.
    streams: Mutex<Vec<Arc<StreamRecord>>>,
    /// The one terminal callback, copied out of its synchronous borrow.
    outcomes: Mutex<Vec<RecordedOutcome>>,
}

/// An observed terminal outcome retained by the test.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordedOutcome {
    /// A confirmed session with its frontier and statistics.
    Completed {
        /// The frontier copied from the callback.
        converged: Version,
        /// The session's measured work.
        stats: SessionStats,
    },
    /// A session method returned this failure category.
    Failed(SessionErrorKind),
    /// The session future was dropped.
    Cancelled,
}

/// One directed stream's metadata and ordered message callbacks.
struct StreamRecord {
    /// Stream index, speaker, and direction reported at creation.
    info: StreamInfo,
    /// Ordered callbacks, preserving their boundaries.
    items: Mutex<Vec<Vec<u8>>>,
}

/// Retrieve records at known session boundaries.
impl Recording {
    /// Require exactly one session of the expected kind since the last check.
    fn take_session(&self, kind: SessionKind) -> Arc<SessionRecord> {
        let mut sessions = self.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1, "one observation handler per session");
        let session = sessions.pop().unwrap();
        assert_eq!(session.info.kind, kind);
        session
    }
}

/// Record every session without filtering.
impl Observer for Recording {
    /// Retain the session record and return its callback handler.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        let record = Arc::new(SessionRecord {
            info: *session,
            elected: Mutex::new(None),
            streams: Mutex::new(Vec::new()),
            outcomes: Mutex::new(Vec::new()),
        });
        self.sessions.lock().unwrap().push(record.clone());
        Some(Box::new(RecordSession(record)))
    }
}

/// A session callback handler sharing its record with the test.
struct RecordSession(Arc<SessionRecord>);

/// Record stream creation and check election ordering as callbacks arrive.
impl SessionObserver for RecordSession {
    /// Require a single election result.
    fn elected(&self, role: Role) {
        let previous = self.0.elected.lock().unwrap().replace(role);
        assert!(previous.is_none(), "the election is decided at most once");
    }

    /// Retain the terminal callback in owned form.
    fn finished(&self, outcome: SessionOutcome<'_>) {
        let outcome = match outcome {
            SessionOutcome::Completed { converged, stats } => RecordedOutcome::Completed {
                converged: converged.clone(),
                stats,
            },
            SessionOutcome::Failed(failure) => RecordedOutcome::Failed(failure),
            SessionOutcome::Cancelled => RecordedOutcome::Cancelled,
            other => panic!("unknown session outcome: {other:?}"),
        };
        self.0.outcomes.lock().unwrap().push(outcome);
    }

    /// Check the speaker against the prior election, then record the stream.
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        if let StreamId::Data { speaker, .. } = stream.id {
            let role = self
                .0
                .elected
                .lock()
                .unwrap()
                .expect("election precedes data streams");
            match stream.direction {
                Direction::Sent => assert_eq!(speaker, role, "local peer speaks sent data"),
                Direction::Received => {
                    assert_ne!(speaker, role, "remote peer speaks received data")
                }
            }
        }
        let record = Arc::new(StreamRecord {
            info: *stream,
            items: Mutex::new(Vec::new()),
        });
        self.0.streams.lock().unwrap().push(record.clone());
        Some(Box::new(RecordStream(record)))
    }
}

/// A stream callback handler sharing its record with the test.
struct RecordStream(Arc<StreamRecord>);

/// Preserve callback boundaries as well as bytes.
impl StreamObserver for RecordStream {
    /// Append one callback's bytes in delivery order.
    fn message(&mut self, bytes: &[u8]) {
        self.0.items.lock().unwrap().push(bytes.to_vec());
    }
}

/// Compare observations with bytes captured independently at the transport.
impl SessionRecord {
    /// Return the session's sole terminal callback.
    fn outcome(&self) -> RecordedOutcome {
        let outcomes = self.outcomes.lock().unwrap();
        assert_eq!(outcomes.len(), 1, "one terminal callback per session");
        outcomes[0].clone()
    }

    /// Count observed data frames in one direction.
    fn data_frames(&self, direction: Direction) -> u64 {
        self.streams
            .lock()
            .unwrap()
            .iter()
            .filter(|stream| {
                stream.info.direction == direction
                    && matches!(stream.info.id, StreamId::Data { .. })
            })
            .map(|stream| stream.items.lock().unwrap().len() as u64)
            .sum()
    }

    /// Require exactly the captured streams and bytes in one direction.
    fn assert_capture(&self, side: &str, direction: Direction, capture: &PipeCapture) {
        // None names the control stream; Some(index) names a data stream.
        // Keeping even empty handlers in the map catches unused handlers, and
        // checking each insertion catches duplicates that concatenation hides.
        let mut observed = BTreeMap::new();
        for stream in self.streams.lock().unwrap().iter() {
            if stream.info.direction != direction {
                continue;
            }
            let index = match stream.info.id {
                StreamId::Control => None,
                StreamId::Data { index, .. } => Some(index),
                other => panic!("unhandled stream kind: {other:?}"),
            };
            let items = stream.items.lock().unwrap();
            for item in items.iter() {
                assert_one_item(item);
            }
            assert!(
                observed.insert(index, items.concat()).is_none(),
                "{side} {direction:?}: duplicate handler for {index:?}"
            );
        }

        let expected = by_stream(&capture.sent);
        assert_eq!(
            observed, expected,
            "{side} {direction:?}: handlers and bytes match the wire"
        );
    }
}

/// Require one complete CBOR item, parsed without the Rumors codec.
fn assert_one_item(bytes: &[u8]) {
    let mut input = bytes;
    let _: Value = ciborium::de::from_reader(&mut input)
        .unwrap_or_else(|e| panic!("an observed callback must be one CBOR item: {e}"));
    assert!(
        input.is_empty(),
        "a callback must not include a second item"
    );
}

/// Split one direction's bytes into its CBOR items, parsed without the
/// Rumors codec, and concatenate them per directed stream: `None` for the
/// control items, `Some(index)` for the frames naming logical stream
/// `index`.
///
/// A frame is an untagged array whose first element is its stream index;
/// every control item is tagged or a text string.
fn by_stream(bytes: &[u8]) -> BTreeMap<Option<u8>, Vec<u8>> {
    let mut streams = BTreeMap::from([(None, Vec::new())]);
    let mut rest = bytes;
    while !rest.is_empty() {
        let mut input = rest;
        let item: Value = ciborium::de::from_reader(&mut input)
            .unwrap_or_else(|e| panic!("the connection carries whole CBOR items: {e}"));
        let len = rest.len() - input.len();
        let stream = match &item {
            Value::Array(elements) => {
                let Some(Value::Integer(index)) = elements.first() else {
                    panic!("a frame opens with its stream index: {item:?}");
                };
                Some(u8::try_from(*index).expect("stream indices fit a byte"))
            }
            _ => None,
        };
        streams
            .entry(stream)
            .or_default()
            .extend_from_slice(&rest[..len]);
        rest = input;
    }
    streams
}

/// Whether one direction's bytes carry any frame.
fn carries_frames(bytes: &[u8]) -> bool {
    by_stream(bytes).keys().any(Option::is_some)
}

/// Check both peers' sent and received streams, and complementary elections.
fn assert_pair(a: &SessionRecord, b: &SessionRecord, a_wire: &PipeCapture, b_wire: &PipeCapture) {
    a.assert_capture("A", Direction::Sent, a_wire);
    a.assert_capture("A", Direction::Received, b_wire);
    b.assert_capture("B", Direction::Sent, b_wire);
    b.assert_capture("B", Direction::Received, a_wire);

    let completed = |record: &SessionRecord| match record.outcome() {
        RecordedOutcome::Completed { converged, stats } => (converged, stats),
        outcome => panic!("session did not complete: {outcome:?}"),
    };
    let (a_converged, a_stats) = completed(a);
    let (b_converged, b_stats) = completed(b);
    assert_eq!(
        a_converged, b_converged,
        "both callbacks report one frontier"
    );
    for (record, stats) in [(a, a_stats), (b, b_stats)] {
        assert_eq!(stats.frames_sent, record.data_frames(Direction::Sent));
        assert_eq!(
            stats.frames_received,
            record.data_frames(Direction::Received)
        );
    }

    // Use the transport to decide whether reconciliation occurred, so a missing
    // observation callback cannot also suppress the election check.
    let a_role = *a.elected.lock().unwrap();
    let b_role = *b.elected.lock().unwrap();
    if !carries_frames(&a_wire.sent) && !carries_frames(&b_wire.sent) {
        assert_eq!(a_role, None, "no election in a control-only session");
        assert_eq!(b_role, None, "no election in a control-only session");
    } else {
        assert_ne!(
            a_role.expect("A elected a role"),
            b_role.expect("B elected a role"),
            "the two roles are complementary"
        );
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A returned transport error produces one failed terminal callback.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn failed_session_reports_its_error_category_once(network in network_seed()) {
        let observer = Arc::new(Recording::default());
        let peer = network.peer::<u64>().observe(observer.clone()).into_rumors();
        let (mut end, remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
        drop(remote);

        let result = block_on(peer.gossip_once(&mut end.read, &mut end.write));
        assert!(matches!(result, Err(rumors::Error::Transport(_))));
        let session = observer.take_session(SessionKind::Gossip);
        assert_eq!(
            session.outcome(),
            RecordedOutcome::Failed(SessionErrorKind::Transport)
        );
    }

    /// Deadline expiry is reported as a failure rather than cancellation.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn deadline_reports_its_error_category_once(network in network_seed()) {
        let observer = Arc::new(Recording::default());
        let peer = network.peer::<u64>()
            .session_deadline(|| std::future::ready(()))
            .observe(observer.clone())
            .into_rumors();
        let (mut end, _remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);

        assert!(matches!(
            block_on(peer.gossip_once(&mut end.read, &mut end.write)),
            Err(rumors::Error::DeadlineExceeded)
        ));
        let session = observer.take_session(SessionKind::Gossip);
        assert_eq!(
            session.outcome(),
            RecordedOutcome::Failed(SessionErrorKind::DeadlineExceeded)
        );
    }

    /// Dropping an active session produces one cancelled terminal callback.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn cancelled_session_reports_cancellation_once(network in network_seed()) {
        let observer = Arc::new(Recording::default());
        let peer = network.peer::<u64>().observe(observer.clone()).into_rumors();
        let (mut end, _remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
        let mut gossip = Box::pin(peer.gossip_once(&mut end.read, &mut end.write));
        assert!(gossip.as_mut().now_or_never().is_none());
        drop(gossip);

        let session = observer.take_session(SessionKind::Gossip);
        assert_eq!(session.outcome(), RecordedOutcome::Cancelled);
    }
}

/// Found the network `network` names, holding the supplied messages, with an
/// optional observer.
fn seeded(
    network: NetworkSeed,
    observer: Option<&Arc<Recording>>,
    payloads: &[Vec<u8>],
) -> Rumors<Vec<u8>> {
    // The wire-neutrality test runs two separate networks from one seed, so
    // their captures are comparable; peers from those runs never interact.
    let mut peer = network.peer().sync_memory_budget(0);
    if let Some(observer) = observer {
        peer = peer.observe(observer.clone());
    }
    let peer = peer.into_rumors();
    peer.send_all(payloads.iter().cloned()).unwrap();
    peer
}

/// Join the parent's network, attaching the observer before bootstrap begins.
fn forked(observer: Option<&Arc<Recording>>, parent: &Rumors<Vec<u8>>) -> Rumors<Vec<u8>> {
    let mut bootstrap = Peer::bootstrap();
    if let Some(observer) = observer {
        bootstrap = bootstrap.observe(observer.clone());
    }
    block_on(bootstrap_fork_configured(
        parent,
        bootstrap,
        BudgetChoice::Zero,
    ))
}

proptest! {
    /// Gossip reports each session and directed stream exactly once, with the
    /// wire's complete items and an election consistent with both speakers.
    #[test]
    fn hook_mirrors_the_wire_exactly(
        (shared, only_a, only_b) in corpora(),
        add_messages in any::<bool>(),
        network in network_seed(),
    ) {
        let rec_a = Arc::new(Recording::default());
        let rec_b = Arc::new(Recording::default());
        let a = seeded(network, Some(&rec_a), &shared);
        let b = forked(Some(&rec_b), &a);
        rec_a.take_session(SessionKind::Gossip);
        rec_b.take_session(SessionKind::Bootstrap);
        if add_messages {
            a.send_all(only_a).unwrap();
            b.send_all(only_b).unwrap();
        }
        let (a_capture, b_capture) = capture_sides(
            |mut end| async move { a.gossip_once(&mut end.read, &mut end.write).await.expect("gossip A"); },
            |mut end| async move { b.gossip_once(&mut end.read, &mut end.write).await.expect("gossip B"); },
        );
        assert_pair(
            &rec_a.take_session(SessionKind::Gossip),
            &rec_b.take_session(SessionKind::Gossip),
            &a_capture,
            &b_capture,
        );
    }

    /// Attaching an observer leaves every transport byte unchanged.
    #[test]
    fn observation_never_changes_the_wire(
        (shared, only_a, only_b) in corpora(),
        add_messages in any::<bool>(),
        network in network_seed(),
    ) {
        let captures = [true, false].map(|observed| {
            let rec_a = Arc::new(Recording::default());
            let rec_b = Arc::new(Recording::default());
            let a = seeded(network, observed.then_some(&rec_a), &shared);
            let b = forked(observed.then_some(&rec_b), &a);
            if add_messages {
                a.send_all(only_a.iter().cloned()).unwrap();
                b.send_all(only_b.iter().cloned()).unwrap();
            }
            capture_sides(
                |mut end| async move { a.gossip_once(&mut end.read, &mut end.write).await.expect("gossip A"); },
                |mut end| async move { b.gossip_once(&mut end.read, &mut end.write).await.expect("gossip B"); },
            )
        });
        let [(a_observed, b_observed), (a_plain, b_plain)] = captures;
        prop_assert_eq!(a_observed.sent, a_plain.sent);
        prop_assert_eq!(b_observed.sent, b_plain.sent);
    }

    /// Joining observes the full bootstrap and serving sessions, including
    /// stream metadata and elections for empty and populated networks.
    #[test]
    fn bootstrap_sessions_are_observed(payloads in payloads(), network in network_seed()) {
        let rec_provider = Arc::new(Recording::default());
        let rec_newcomer = Arc::new(Recording::default());
        let provider = seeded(network, Some(&rec_provider), &payloads);
        let bootstrap = Peer::<Vec<u8>>::bootstrap().observe(rec_newcomer.clone());
        let (provider_capture, newcomer_capture) = capture_sides(
            |mut end| async move {
                provider.gossip_once(&mut end.read, &mut end.write).await.expect("provider gossip");
            },
            |mut end| async move {
                assert!(matches!(bootstrap.join(&mut end.read, &mut end.write).await, rumors::Joined::Joined { .. }));
            },
        );
        assert_pair(
            &rec_provider.take_session(SessionKind::Gossip),
            &rec_newcomer.take_session(SessionKind::Bootstrap),
            &provider_capture,
            &newcomer_capture,
        );
    }

    /// Retirement observes the final reconciliation and handoff on both peers,
    /// whether their contents match or either peer has added messages.
    #[test]
    fn retire_sessions_are_observed(
        (shared, only_a, only_b) in corpora(),
        add_messages in any::<bool>(),
        network in network_seed(),
    ) {
        let rec_absorber = Arc::new(Recording::default());
        let rec_retiree = Arc::new(Recording::default());
        let absorber = seeded(network, Some(&rec_absorber), &shared);
        let retiree = forked(Some(&rec_retiree), &absorber);
        rec_absorber.take_session(SessionKind::Gossip);
        rec_retiree.take_session(SessionKind::Bootstrap);
        if add_messages {
            absorber.send_all(only_a).unwrap();
            retiree.send_all(only_b).unwrap();
        }
        let retiree = block_on(retiree.try_into_peer()).expect("the retiree handle is unique");
        let (absorber_capture, retiree_capture) = capture_sides(
            |mut end| async move {
                absorber.gossip_once(&mut end.read, &mut end.write).await.expect("absorber gossip");
            },
            |mut end| async move {
                assert!(matches!(retiree.retire(&mut end.read, &mut end.write).await, Retire::Retired { .. }));
            },
        );
        assert_pair(
            &rec_absorber.take_session(SessionKind::Gossip),
            &rec_retiree.take_session(SessionKind::Retire),
            &absorber_capture,
            &retiree_capture,
        );
    }
}
