//! End-to-end proof that the adapter bridges real sessions faithfully.
//!
//! Driven through the public API only: a bootstrapped-then-gossiping
//! peer under a capturing subscriber yields the documented span tree,
//! and every item the hook delivers surfaces as exactly one `message`
//! event (held against a counting wrapper around the adapter itself).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rumors::Peer;
use rumors::observe::{Observer, Role, SessionInfo, SessionObserver, StreamInfo, StreamObserver};
use rumors_tracing::TracingObserver;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// One captured span: its name, recorded fields, and explicit parent.
#[derive(Debug)]
struct SpanRecord {
    name: String,
    fields: BTreeMap<String, String>,
    parent: Option<u64>,
}

/// One captured event: its recorded fields and explicit parent span.
#[derive(Debug)]
struct EventRecord {
    fields: BTreeMap<String, String>,
    parent: Option<u64>,
}

#[derive(Default)]
struct State {
    spans: BTreeMap<u64, SpanRecord>,
    events: Vec<EventRecord>,
}

/// A minimal capturing subscriber: retains every span and event with
/// its fields, resolving parents from the explicit parent the adapter
/// always passes.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<State>>);

/// Collects a span's or event's fields into string form via their
/// debug rendering.
struct Fields<'a>(&'a mut BTreeMap<String, String>);

impl tracing::field::Visit for Fields<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

impl Subscriber for Capture {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut state = self.0.lock().unwrap();
        let id = state.spans.len() as u64 + 1;
        let mut fields = BTreeMap::new();
        span.record(&mut Fields(&mut fields));
        state.spans.insert(
            id,
            SpanRecord {
                name: span.metadata().name().to_string(),
                fields,
                parent: span.parent().map(Id::into_u64),
            },
        );
        Id::from_u64(id)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut state = self.0.lock().unwrap();
        let record = state.spans.get_mut(&span.into_u64()).expect("known span");
        values.record(&mut Fields(&mut record.fields));
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = BTreeMap::new();
        event.record(&mut Fields(&mut fields));
        self.0.lock().unwrap().events.push(EventRecord {
            fields,
            parent: event.parent().map(Id::into_u64),
        });
    }

    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

/// Wraps the adapter and counts every item delivery, so the captured
/// events can be held against the hook's own invocation count.
struct Counting {
    inner: TracingObserver,
    messages: Arc<AtomicU64>,
}

impl Observer for Counting {
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        let inner = self.inner.session(session)?;
        Some(Box::new(CountingSession {
            inner,
            messages: Arc::clone(&self.messages),
        }))
    }
}

struct CountingSession {
    inner: Box<dyn SessionObserver>,
    messages: Arc<AtomicU64>,
}

impl SessionObserver for CountingSession {
    fn elected(&self, role: Role) {
        self.inner.elected(role);
    }

    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        let inner = self.inner.stream(stream)?;
        Some(Box::new(CountingStream {
            inner,
            messages: Arc::clone(&self.messages),
        }))
    }
}

struct CountingStream {
    inner: Box<dyn StreamObserver>,
    messages: Arc<AtomicU64>,
}

impl StreamObserver for CountingStream {
    fn message(&mut self, bytes: &[u8]) {
        self.messages.fetch_add(1, Ordering::Relaxed);
        self.inner.message(bytes);
    }
}

/// The event's `message` text, as the debug visitor records it.
fn message_text(event: &EventRecord) -> Option<&str> {
    event.fields.get("message").map(String::as_str)
}

/// A bootstrapped-then-gossiping peer emits the documented span tree,
/// with exactly one `message` event per item the hook delivered.
///
/// The tree: a `session` span per session with kind and ordinal,
/// `stream` spans beneath it, and a `role elected` event once the
/// election is decided. Every `message` event is stamped with a
/// session-dense ordinal and carries a rendering free of defect notes.
#[test]
fn adapter_bridges_real_sessions() {
    let capture = Capture::default();
    let messages = Arc::new(AtomicU64::new(0));
    let counting = Arc::new(Counting {
        inner: TracingObserver::new(),
        messages: Arc::clone(&messages),
    });

    tracing::subscriber::with_default(capture.clone(), || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime");
        runtime.block_on(async {
            let alice = Peer::<String>::seed().into_rumors();
            alice
                .send("from the seed".to_string())
                .expect("a flat string admits");

            // Bob's own bootstrap session is observed through the
            // builder; the attachment then follows the joined peer.
            let (mut near, mut far) = rumors::link::memory();
            let (served, joined) = tokio::join!(
                alice.gossip(&mut far),
                Peer::<String>::bootstrap()
                    .observe(counting.clone())
                    .join(&mut near),
            );
            served.expect("provider session");
            let bob = joined
                .expect("bootstrap session")
                .expect("alice is established, not herself bootstrapping")
                .into_rumors();

            // Diverge both replicas so the follow-up gossip elects a
            // role and moves data-stream frames both ways.
            bob.send("from bob".to_string())
                .expect("a flat string admits");
            alice
                .send("from alice".to_string())
                .expect("a flat string admits");
            let (mut near, mut far) = rumors::link::memory();
            let (a, b) = tokio::join!(alice.gossip(&mut far), bob.gossip(&mut near));
            a.expect("alice's gossip session");
            b.expect("bob's gossip session");
        });
    });

    let hook_items = messages.load(Ordering::Relaxed);
    assert!(hook_items > 0, "the sessions moved wire items");

    let state = capture.0.lock().unwrap();

    // The span tree: one session span per observed session, in entry
    // order, with stream spans parented beneath.
    let sessions: Vec<(&u64, &SpanRecord)> = state
        .spans
        .iter()
        .filter(|(_, s)| s.name == "session")
        .collect();
    assert_eq!(sessions.len(), 2, "bootstrap then gossip: {sessions:?}");
    assert!(sessions[0].1.fields["kind"].contains("Bootstrap"));
    assert_eq!(sessions[0].1.fields["ordinal"], "0");
    assert!(sessions[1].1.fields["kind"].contains("Gossip"));
    assert_eq!(sessions[1].1.fields["ordinal"], "1");
    for (_, session) in &sessions {
        assert!(session.fields["protocol"].contains("V2"));
        assert_eq!(session.parent, None);
    }

    for (id, span) in state.spans.iter().filter(|(_, s)| s.name == "stream") {
        let parent = span.parent.expect("stream spans sit under a session");
        assert_eq!(
            state.spans[&parent].name, "session",
            "stream span {id} parents a session"
        );
    }
    let gossip_id = *sessions[1].0;
    let gossip_streams: Vec<&SpanRecord> = state
        .spans
        .values()
        .filter(|s| s.name == "stream" && s.parent == Some(gossip_id))
        .collect();
    assert!(
        gossip_streams
            .iter()
            .any(|s| s.fields["kind"] == "\"control\""),
        "the gossip session's control stream is observed"
    );
    assert!(
        gossip_streams
            .iter()
            .any(|s| s.fields["kind"] == "\"data\""),
        "divergent gossip opens observed data streams"
    );

    // The election is reported into the diverged gossip session.
    assert!(
        state
            .events
            .iter()
            .any(|e| { message_text(e) == Some("role elected") && e.parent == Some(gossip_id) }),
        "the diverged gossip session elects a role"
    );

    // Every hook delivery surfaced as exactly one message event, and
    // every rendering is a clean single item.
    let message_events: Vec<&EventRecord> = state
        .events
        .iter()
        .filter(|e| message_text(e) == Some("message"))
        .collect();
    assert_eq!(message_events.len() as u64, hook_items);
    for event in &message_events {
        let item = &event.fields["item"];
        assert!(!item.is_empty());
        assert!(!item.contains("unrenderable"), "clean item: {item}");
    }

    // Sorting one session's events by ordinal reconstructs the
    // observed interleaving: per session, the stamps are exactly
    // 0..n, each used once.
    for (session_id, _) in &sessions {
        let mut ordinals: Vec<u64> = message_events
            .iter()
            .filter(|e| {
                let stream = e.parent.expect("message events sit in stream spans");
                state.spans[&stream].parent == Some(**session_id)
            })
            .map(|e| e.fields["ordinal"].parse().expect("ordinal is a number"))
            .collect();
        ordinals.sort_unstable();
        let expected: Vec<u64> = (0..ordinals.len() as u64).collect();
        assert_eq!(
            ordinals, expected,
            "session {session_id} ordinals are dense"
        );
    }
}

/// Concurrent sessions through clones of one observed handle number
/// cleanly: the two session spans carry the ordinals {0, 1} as a set,
/// and each session's message ordinals are dense from 0.
///
/// The ordinals are held as a set because order between concurrent
/// sessions is unspecified.
/// Two counterparties gossip with the observed peer inside one
/// `tokio::join!` on a current-thread runtime, so the sessions
/// interleave at await points — the re-entrancy of the adapter's
/// `session` and the sharing of its counters is the property under
/// test, not thread parallelism.
#[test]
fn concurrent_sessions_number_cleanly() {
    let capture = Capture::default();

    tracing::subscriber::with_default(capture.clone(), || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime");
        runtime.block_on(async {
            let alice = Peer::<String>::seed().into_rumors();
            alice
                .send("from the seed".to_string())
                .expect("a flat string admits");

            // Bootstrap both counterparties unobserved, and attach the
            // adapter to bob only after his join: the two concurrent
            // gossip sessions are then the first sessions the adapter
            // sees.
            let (mut near, mut far) = rumors::link::memory();
            let (served, joined) = tokio::join!(
                alice.gossip(&mut far),
                Peer::<String>::bootstrap().join(&mut near),
            );
            served.expect("provider session");
            let bob = joined
                .expect("bootstrap session")
                .expect("alice is established, not herself bootstrapping")
                .observe(Arc::new(TracingObserver::new()))
                .into_rumors();

            let (mut near, mut far) = rumors::link::memory();
            let (served, joined) = tokio::join!(
                alice.gossip(&mut far),
                Peer::<String>::bootstrap().join(&mut near),
            );
            served.expect("provider session");
            let carol = joined
                .expect("bootstrap session")
                .expect("alice is established, not herself bootstrapping")
                .into_rumors();

            // Diverge all three replicas so both sessions elect roles
            // and move data-stream frames.
            alice
                .send("from alice".to_string())
                .expect("a flat string admits");
            bob.send("from bob".to_string())
                .expect("a flat string admits");
            carol
                .send("from carol".to_string())
                .expect("a flat string admits");

            // Both of bob's sessions run inside one join, through the
            // one shared observer, over separate links.
            let bob_too = bob.clone();
            let (mut near_a, mut far_a) = rumors::link::memory();
            let (mut near_c, mut far_c) = rumors::link::memory();
            let (a, b1, b2, c) = tokio::join!(
                alice.gossip(&mut far_a),
                bob.gossip(&mut near_a),
                bob_too.gossip(&mut near_c),
                carol.gossip(&mut far_c),
            );
            a.expect("alice's session");
            b1.expect("bob's session with alice");
            b2.expect("bob's session with carol");
            c.expect("carol's session");
        });
    });

    let state = capture.0.lock().unwrap();

    // Exactly the two concurrent gossip sessions were observed, and
    // their ordinals are {0, 1} as a set: each session numbered once,
    // no collision and no gap, whichever entered first.
    let sessions: Vec<(&u64, &SpanRecord)> = state
        .spans
        .iter()
        .filter(|(_, s)| s.name == "session")
        .collect();
    assert_eq!(sessions.len(), 2, "two concurrent sessions: {sessions:?}");
    let mut ordinals: Vec<&str> = sessions
        .iter()
        .map(|(_, s)| s.fields["ordinal"].as_str())
        .collect();
    ordinals.sort_unstable();
    assert_eq!(
        ordinals,
        ["0", "1"],
        "session ordinals are the set {{0, 1}}"
    );
    for (_, session) in &sessions {
        assert!(session.fields["kind"].contains("Gossip"));
    }

    // Each session's message ordinals are dense from 0: the sessions
    // share the adapter but not their message counters.
    let message_events: Vec<&EventRecord> = state
        .events
        .iter()
        .filter(|e| message_text(e) == Some("message"))
        .collect();
    for (session_id, _) in &sessions {
        let mut ordinals: Vec<u64> = message_events
            .iter()
            .filter(|e| {
                let stream = e.parent.expect("message events sit in stream spans");
                state.spans[&stream].parent == Some(**session_id)
            })
            .map(|e| e.fields["ordinal"].parse().expect("ordinal is a number"))
            .collect();
        assert!(
            !ordinals.is_empty(),
            "session {session_id} moved wire items"
        );
        ordinals.sort_unstable();
        let expected: Vec<u64> = (0..ordinals.len() as u64).collect();
        assert_eq!(
            ordinals, expected,
            "session {session_id} ordinals are dense"
        );
    }
}
