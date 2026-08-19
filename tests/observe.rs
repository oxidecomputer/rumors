//! The internal-capture half of the wire-capture validity contract.
//!
//! The transport-captured render property (`tests/wire_legibility.rs`)
//! proves *external* capture: a third party tapping the wire reads
//! valid CBOR. This suite proves *internal* capture: the observation
//! hook hands its handlers exactly one CBOR item per invocation, and
//! concatenating one directed stream's invocations reproduces that
//! stream's transport bytes, byte for byte. The whole-stream property
//! implies the per-item property only if the hook is bug-free; the
//! differential here tests exactly that implication, against the same
//! recording-link oracle the snapshot suites trust. Neither test
//! supplants the other.
//!
//! The claims are families over sessions, so they are proptests:
//! randomized peer contents drive real gossip sessions (with the
//! joining side's own bootstrap observed through the builder), and
//! fixed pairings cover the bootstrap and retire session kinds
//! end to end.

mod common;

use std::sync::{Arc, Mutex};

use ciborium::value::Value;
use proptest::collection::vec;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::observe::{
    Direction, Observer, Role, SessionInfo, SessionKind, SessionObserver, StreamId, StreamInfo,
    StreamObserver,
};
use rumors::{Peer, Retire, Rumors};

use crate::common::gossip_snapshot::{CapturedLink, capture_sides};
use crate::common::wire::block_on;

/// A recording observer: retains every session, stream, and item it is
/// shown, so tests can hold the hook's view against the transport's.
#[derive(Default)]
struct Recording {
    sessions: Mutex<Vec<Arc<SessionRecord>>>,
}

struct SessionRecord {
    info: SessionInfo,
    elected: Mutex<Option<Role>>,
    streams: Mutex<Vec<Arc<StreamRecord>>>,
}

struct StreamRecord {
    info: StreamInfo,
    items: Mutex<Vec<Vec<u8>>>,
}

impl Recording {
    /// The record of the peer's most recent session.
    fn last_session(&self) -> Arc<SessionRecord> {
        self.sessions
            .lock()
            .unwrap()
            .last()
            .expect("at least one session was observed")
            .clone()
    }
}

impl Observer for Recording {
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

struct RecordSession(Arc<SessionRecord>);

impl SessionObserver for RecordSession {
    fn elected(&self, role: Role) {
        let previous = self.0.elected.lock().unwrap().replace(role);
        assert!(previous.is_none(), "the election is decided at most once");
    }

    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        let record = Arc::new(StreamRecord {
            info: *stream,
            items: Mutex::new(Vec::new()),
        });
        self.0.streams.lock().unwrap().push(record.clone());
        Some(Box::new(RecordStream(record)))
    }
}

struct RecordStream(Arc<StreamRecord>);

impl StreamObserver for RecordStream {
    fn message(&mut self, bytes: &[u8]) {
        self.0.items.lock().unwrap().push(bytes.to_vec());
    }
}

impl SessionRecord {
    /// Concatenate one direction's control-stream items.
    fn control(&self, direction: Direction) -> Vec<u8> {
        let mut bytes = Vec::new();
        for stream in self.streams.lock().unwrap().iter() {
            if stream.info.id == StreamId::Control && stream.info.direction == direction {
                for item in stream.items.lock().unwrap().iter() {
                    bytes.extend_from_slice(item);
                }
            }
        }
        bytes
    }

    /// One direction's data streams as `(index, speaker, concatenated
    /// bytes)`, asserting each index appears at most once.
    fn data(&self, direction: Direction) -> Vec<(u8, Role, Vec<u8>)> {
        let mut out: Vec<(u8, Role, Vec<u8>)> = Vec::new();
        for stream in self.streams.lock().unwrap().iter() {
            let StreamId::Data { speaker, index } = stream.info.id else {
                continue;
            };
            if stream.info.direction != direction {
                continue;
            }
            assert!(
                out.iter().all(|(seen, ..)| *seen != index),
                "one handler per directed data stream"
            );
            let mut bytes = Vec::new();
            for item in stream.items.lock().unwrap().iter() {
                bytes.extend_from_slice(item);
            }
            out.push((index, speaker, bytes));
        }
        out
    }

    /// Every observed invocation across every stream of this session.
    fn items(&self) -> Vec<Vec<u8>> {
        self.streams
            .lock()
            .unwrap()
            .iter()
            .flat_map(|stream| stream.items.lock().unwrap().clone())
            .collect()
    }
}

/// Assert `bytes` parse as exactly one CBOR item with nothing left over.
fn assert_one_item(bytes: &[u8]) {
    let mut input = bytes;
    let _: Value = ciborium::de::from_reader(&mut input)
        .unwrap_or_else(|e| panic!("an observed invocation must be one CBOR item: {e}"));
    assert!(
        input.is_empty(),
        "an observed invocation carries one item exactly, {} residue bytes found",
        input.len()
    );
}

/// Assert one side's hook view mirrors its transport capture: control
/// items concatenate to the control blob, each sent data stream's items
/// concatenate to its transport blob behind the two-byte open label,
/// and every invocation is one CBOR item.
fn assert_mirrors(side: &str, session: &SessionRecord, capture: &CapturedLink) {
    assert_eq!(
        session.control(Direction::Sent),
        capture.control,
        "{side}: sent control items must concatenate to the transport capture"
    );
    let sent = session.data(Direction::Sent);
    assert_eq!(
        sent.len(),
        capture.streams.len(),
        "{side}: one sent data handler per opened transport stream"
    );
    for blob in &capture.streams {
        assert!(
            blob.len() >= 2,
            "{side}: an opened stream carries its label"
        );
        let index = blob[1];
        let (_, _, bytes) = sent
            .iter()
            .find(|(sent_index, ..)| *sent_index == index)
            .unwrap_or_else(|| panic!("{side}: no sent handler for data stream {index}"));
        assert_eq!(
            bytes,
            &blob[2..],
            "{side}: data stream {index}'s items must concatenate to its capture"
        );
    }
    for item in session.items() {
        assert_one_item(&item);
    }
}

/// Assert the received side of `local` equals what `remote` put on the
/// wire, per directed stream: the lossless in-memory link delivers the
/// counterparty's sent bytes verbatim.
fn assert_received_mirrors_remote(
    side: &str,
    local: &SessionRecord,
    remote_capture: &CapturedLink,
) {
    assert_eq!(
        local.control(Direction::Received),
        remote_capture.control,
        "{side}: received control items must equal the peer's sent capture"
    );
    let received = local.data(Direction::Received);
    for blob in &remote_capture.streams {
        let index = blob[1];
        let Some((_, _, bytes)) = received.iter().find(|(i, ..)| *i == index) else {
            panic!("{side}: no received handler for the peer's data stream {index}");
        };
        assert_eq!(
            bytes,
            &blob[2..],
            "{side}: received data stream {index} must equal the peer's sent bytes"
        );
    }
}

/// Assert the two sides' elections are complementary, and each side's
/// data-stream speakers agree with its elected role: sent streams are
/// spoken by the local role, received streams by the counterparty's.
fn assert_election(a: &SessionRecord, b: &SessionRecord) {
    let a_role = *a.elected.lock().unwrap();
    let b_role = *b.elected.lock().unwrap();
    let opened = !a.data(Direction::Sent).is_empty()
        || !b.data(Direction::Sent).is_empty()
        || !a.data(Direction::Received).is_empty();
    if !opened {
        assert_eq!(a_role, None, "no election without data streams");
        assert_eq!(b_role, None, "no election without data streams");
        return;
    }
    let a_role = a_role.expect("a session with data streams elected a role");
    let b_role = b_role.expect("a session with data streams elected a role");
    assert_ne!(a_role, b_role, "the two roles are complementary");
    for (session, role) in [(a, a_role), (b, b_role)] {
        for (_, speaker, _) in session.data(Direction::Sent) {
            assert_eq!(speaker, role, "sent streams are spoken by the local role");
        }
        for (_, speaker, _) in session.data(Direction::Received) {
            assert_ne!(speaker, role, "received streams are spoken by the peer");
        }
    }
}

/// A freshly seeded peer loaded with `payloads`, observed when an
/// observer is given.
fn seeded(observer: Option<&Arc<Recording>>, payloads: &[Vec<u8>]) -> Rumors<Vec<u8>> {
    // A fixed-seed network id: the byte-identity property compares two
    // whole universes, so both must draw the same identity.
    let mut peer = Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).sync_window_floor();
    if let Some(observer) = observer {
        peer = peer.observe(observer.clone());
    }
    let peer = peer.into_rumors();
    for payload in payloads {
        peer.send(payload.clone());
    }
    peer
}

/// A peer bootstrapped from `parent` over an uncaptured in-memory
/// link; when an observer is given it attaches on the builder, so the
/// join session itself is observed.
fn forked(observer: Option<&Arc<Recording>>, parent: &Rumors<Vec<u8>>) -> Rumors<Vec<u8>> {
    let (mut near, mut far) = rumors::link::memory();
    let serve = parent.clone();
    let mut bootstrap = Peer::<Vec<u8>>::bootstrap();
    if let Some(observer) = observer {
        bootstrap = bootstrap.observe(observer.clone());
    }
    block_on(async {
        let (peer, served) = tokio::join!(bootstrap.join(&mut near), serve.gossip(&mut far),);
        served.expect("the parent serves the fork");
        peer.expect("the join session completes")
            .expect("the parent held a universe to share")
            .into_rumors()
    })
}

/// Arbitrary payload corpora, mirroring the wire-legibility suite's
/// shape: enough variety to drive matches, queries, empty queries, and
/// batched supply runs, small enough for many full sessions.
#[allow(clippy::type_complexity)]
fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    let payload = vec(any::<u8>(), 0..48);
    (
        vec(payload.clone(), 0..12),
        vec(payload.clone(), 0..12),
        vec(payload, 0..12),
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 24,
        ..ProptestConfig::default()
    })]

    /// For arbitrary gossip sessions, the hook's view mirrors the wire
    /// exactly: per directed stream (control and data, both
    /// directions), concatenating the observed invocations reproduces
    /// the transport capture byte for byte; every invocation is
    /// exactly one CBOR item; session identity carries the right kind,
    /// protocol, and ordinal; and the two sides' role elections are
    /// complementary and agree with every data stream's speaker.
    #[test]
    fn hook_mirrors_the_wire_exactly(
        (shared, only_a, only_b) in corpora(),
    ) {
        let rec_a = Arc::new(Recording::default());
        let rec_b = Arc::new(Recording::default());
        let a = seeded(Some(&rec_a), &shared);
        let b = forked(Some(&rec_b), &a);
        for payload in &only_a {
            a.send(payload.clone());
        }
        for payload in &only_b {
            b.send(payload.clone());
        }
        let (a_capture, b_capture) = capture_sides(
            {
                let a = a.clone();
                move |mut link| async move {
                    a.gossip(&mut link).await.expect("gossip A");
                }
            },
            {
                let b = b.clone();
                move |mut link| async move {
                    b.gossip(&mut link).await.expect("gossip B");
                }
            },
        );

        // The serve and the join were each side's session 0; the
        // captured gossip is session 1 on both.
        let a_session = rec_a.last_session();
        let b_session = rec_b.last_session();
        for (session, side) in [(&a_session, "A"), (&b_session, "B")] {
            prop_assert_eq!(session.info.kind, SessionKind::Gossip, "{}", side);
            prop_assert_eq!(session.info.protocol, rumors::Protocol::V2, "{}", side);
            prop_assert_eq!(session.info.ordinal, 1, "{}", side);
        }
        prop_assert_eq!(
            rec_b.sessions.lock().unwrap()[0].info.kind,
            SessionKind::Bootstrap,
            "the builder-attached observer saw the join itself"
        );

        assert_mirrors("A", &a_session, &a_capture);
        assert_mirrors("B", &b_session, &b_capture);
        assert_received_mirrors_remote("A", &a_session, &b_capture);
        assert_received_mirrors_remote("B", &b_session, &a_capture);
        assert_election(&a_session, &b_session);
    }

    /// An observed session's wire bytes are identical to an unobserved
    /// one's: attachment never changes what crosses the transport.
    #[test]
    fn observation_never_changes_the_wire(
        (shared, only_a, only_b) in corpora(),
    ) {
        let captures = [true, false].map(|observed| {
            let rec_a = Arc::new(Recording::default());
            let rec_b = Arc::new(Recording::default());
            let a = seeded(observed.then_some(&rec_a), &shared);
            let b = forked(observed.then_some(&rec_b), &a);
            for payload in &only_a {
                a.send(payload.clone());
            }
            for payload in &only_b {
                b.send(payload.clone());
            }
            capture_sides(
                {
                    let a = a.clone();
                    move |mut link| async move {
                        a.gossip(&mut link).await.expect("gossip A");
                    }
                },
                {
                    let b = b.clone();
                    move |mut link| async move {
                        b.gossip(&mut link).await.expect("gossip B");
                    }
                },
            )
        });
        let [(a_observed, b_observed), (a_plain, b_plain)] = captures;
        prop_assert_eq!(a_observed.control, a_plain.control);
        prop_assert_eq!(a_observed.streams, a_plain.streams);
        prop_assert_eq!(b_observed.control, b_plain.control);
        prop_assert_eq!(b_observed.streams, b_plain.streams);
    }
}

/// A bootstrap pairing is observed end to end: the newcomer's session
/// carries the `Bootstrap` kind at ordinal zero, the provider observes
/// an ordinary gossip serve, both sides' hook views mirror their
/// transport captures (the party hand-off and epilogue included), and
/// every invocation is one CBOR item.
#[test]
fn bootstrap_sessions_are_observed() {
    let rec_provider = Arc::new(Recording::default());
    let rec_newcomer = Arc::new(Recording::default());
    let provider = seeded(Some(&rec_provider), &[b"seeded".to_vec()]);
    let (provider_capture, newcomer_capture) = capture_sides(
        {
            let provider = provider.clone();
            move |mut link| async move {
                provider.gossip(&mut link).await.expect("provider gossip");
            }
        },
        {
            let rec_newcomer = rec_newcomer.clone();
            move |mut link| async move {
                Peer::<Vec<u8>>::bootstrap()
                    .observe(rec_newcomer)
                    .join(&mut link)
                    .await
                    .expect("bootstrap handshake")
                    .expect("provider served the bootstrap");
            }
        },
    );
    let provider_session = rec_provider.last_session();
    let newcomer_session = rec_newcomer.last_session();
    assert_eq!(provider_session.info.kind, SessionKind::Gossip);
    assert_eq!(newcomer_session.info.kind, SessionKind::Bootstrap);
    assert_eq!(newcomer_session.info.ordinal, 0);
    assert_mirrors("provider", &provider_session, &provider_capture);
    assert_mirrors("newcomer", &newcomer_session, &newcomer_capture);
    assert_received_mirrors_remote("provider", &provider_session, &newcomer_capture);
    assert_received_mirrors_remote("newcomer", &newcomer_session, &provider_capture);
}

/// A retire pairing is observed end to end: the retiree's session
/// carries the `Retire` kind, the absorber observes an ordinary
/// gossip, both hook views mirror their transport captures (the
/// retiree's party hand-off included), and every invocation is one
/// CBOR item.
#[test]
fn retire_sessions_are_observed() {
    let rec_absorber = Arc::new(Recording::default());
    let rec_retiree = Arc::new(Recording::default());
    let absorber = seeded(Some(&rec_absorber), &[b"kept".to_vec()]);
    let retiree = forked(Some(&rec_retiree), &absorber);
    retiree.send(b"handed off".to_vec());
    let retiree = block_on(retiree.try_into_peer()).expect("the retiree handle is unique");
    let (absorber_capture, retiree_capture) = capture_sides(
        {
            let absorber = absorber.clone();
            move |mut link| async move {
                absorber.gossip(&mut link).await.expect("absorber gossip");
            }
        },
        move |mut link| async move {
            match retiree.retire(&mut link).await {
                Retire::Retired => {}
                other => panic!("the retiree must retire cleanly, got {other:?}"),
            }
        },
    );
    let absorber_session = rec_absorber.last_session();
    let retiree_session = rec_retiree.last_session();
    assert_eq!(absorber_session.info.kind, SessionKind::Gossip);
    assert_eq!(retiree_session.info.kind, SessionKind::Retire);
    assert_mirrors("absorber", &absorber_session, &absorber_capture);
    assert_mirrors("retiree", &retiree_session, &retiree_capture);
    assert_received_mirrors_remote("absorber", &absorber_session, &retiree_capture);
    assert_received_mirrors_remote("retiree", &retiree_session, &absorber_capture);
    assert_election(&absorber_session, &retiree_session);
}
