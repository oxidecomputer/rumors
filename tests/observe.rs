//! Check the wire-observation contract against independent transport captures.
//!
//! Each callback must contain one complete CBOR item. Within each directed
//! stream, those items must reproduce the transport bytes, excluding the
//! stream-open label. The handler set must match the streams actually used.
//!
//! Generated gossip, bootstrap, and retirement sessions check this contract
//! alongside session kinds and role elections. A separate property compares
//! observed and unobserved sessions to ensure attachment leaves the wire intact.

mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use ciborium::value::Value;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::observe::{
    Direction, Observer, Role, SessionInfo, SessionKind, SessionObserver, StreamId, StreamInfo,
    StreamObserver,
};
use rumors::testing::stream_label;
use rumors::{Peer, Retire, Rumors};

use crate::common::gossip_snapshot::{CapturedLink, capture_sides, corpora, payloads};
use crate::common::window::WindowChoice;
use crate::common::wire::{block_on, bootstrap_fork_configured};

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
    /// Require exactly the captured streams and bytes in one direction.
    fn assert_capture(&self, side: &str, direction: Direction, capture: &CapturedLink) {
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

        let mut expected = BTreeMap::from([(None, capture.control.clone())]);
        for blob in &capture.streams {
            let ((_, index), label_len) = stream_label(blob);
            assert!(
                expected
                    .insert(Some(index), blob[label_len..].to_vec())
                    .is_none(),
                "one transport stream per index in this session"
            );
        }
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

/// Check both peers' sent and received streams, and complementary elections.
fn assert_pair(a: &SessionRecord, b: &SessionRecord, a_wire: &CapturedLink, b_wire: &CapturedLink) {
    a.assert_capture("A", Direction::Sent, a_wire);
    a.assert_capture("A", Direction::Received, b_wire);
    b.assert_capture("B", Direction::Sent, b_wire);
    b.assert_capture("B", Direction::Received, a_wire);

    // Use the transport to decide whether reconciliation occurred, so a missing
    // observation callback cannot also suppress the election check.
    let a_role = *a.elected.lock().unwrap();
    let b_role = *b.elected.lock().unwrap();
    if a_wire.streams.is_empty() && b_wire.streams.is_empty() {
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

/// Seed a network with the supplied messages and optional observer.
fn seeded(observer: Option<&Arc<Recording>>, payloads: &[Vec<u8>]) -> Rumors<Vec<u8>> {
    // The wire-neutrality test runs two separate networks. A fixed network ID
    // makes their captures comparable; peers from those runs never interact.
    let mut peer = Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).sync_window_floor();
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
        WindowChoice::Floor,
    ))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 24,
        ..ProptestConfig::default()
    })]

    /// Gossip reports each session and directed stream exactly once, with the
    /// wire's complete items and an election consistent with both speakers.
    #[test]
    fn hook_mirrors_the_wire_exactly(
        (shared, only_a, only_b) in corpora(),
        add_messages in any::<bool>(),
    ) {
        let rec_a = Arc::new(Recording::default());
        let rec_b = Arc::new(Recording::default());
        let a = seeded(Some(&rec_a), &shared);
        let b = forked(Some(&rec_b), &a);
        rec_a.take_session(SessionKind::Gossip);
        rec_b.take_session(SessionKind::Bootstrap);
        if add_messages {
            a.send_all(only_a).unwrap();
            b.send_all(only_b).unwrap();
        }
        let (a_capture, b_capture) = capture_sides(
            |mut link| async move { a.gossip_once(&mut link).await.expect("gossip A"); },
            |mut link| async move { b.gossip_once(&mut link).await.expect("gossip B"); },
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
    ) {
        let captures = [true, false].map(|observed| {
            let rec_a = Arc::new(Recording::default());
            let rec_b = Arc::new(Recording::default());
            let a = seeded(observed.then_some(&rec_a), &shared);
            let b = forked(observed.then_some(&rec_b), &a);
            if add_messages {
                a.send_all(only_a.iter().cloned()).unwrap();
                b.send_all(only_b.iter().cloned()).unwrap();
            }
            capture_sides(
                |mut link| async move { a.gossip_once(&mut link).await.expect("gossip A"); },
                |mut link| async move { b.gossip_once(&mut link).await.expect("gossip B"); },
            )
        });
        let [(a_observed, b_observed), (a_plain, b_plain)] = captures;
        prop_assert_eq!(a_observed.control, a_plain.control);
        prop_assert_eq!(a_observed.streams, a_plain.streams);
        prop_assert_eq!(b_observed.control, b_plain.control);
        prop_assert_eq!(b_observed.streams, b_plain.streams);
    }

    /// Joining observes the full bootstrap and serving sessions, including
    /// stream metadata and elections for empty and populated networks.
    #[test]
    fn bootstrap_sessions_are_observed(payloads in payloads()) {
        let rec_provider = Arc::new(Recording::default());
        let rec_newcomer = Arc::new(Recording::default());
        let provider = seeded(Some(&rec_provider), &payloads);
        let bootstrap = Peer::<Vec<u8>>::bootstrap().observe(rec_newcomer.clone());
        let (provider_capture, newcomer_capture) = capture_sides(
            |mut link| async move {
                provider.gossip_once(&mut link).await.expect("provider gossip");
            },
            |mut link| async move {
                assert!(matches!(bootstrap.join(&mut link).await, rumors::Joined::Joined { .. }));
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
    ) {
        let rec_absorber = Arc::new(Recording::default());
        let rec_retiree = Arc::new(Recording::default());
        let absorber = seeded(Some(&rec_absorber), &shared);
        let retiree = forked(Some(&rec_retiree), &absorber);
        rec_absorber.take_session(SessionKind::Gossip);
        rec_retiree.take_session(SessionKind::Bootstrap);
        if add_messages {
            absorber.send_all(only_a).unwrap();
            retiree.send_all(only_b).unwrap();
        }
        let retiree = block_on(retiree.try_into_peer()).expect("the retiree handle is unique");
        let (absorber_capture, retiree_capture) = capture_sides(
            |mut link| async move {
                absorber.gossip_once(&mut link).await.expect("absorber gossip");
            },
            |mut link| async move {
                assert!(matches!(retiree.retire(&mut link).await, Retire::Retired));
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
