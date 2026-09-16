use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use tokio::io::AsyncReadExt;

use super::*;

/// A handler that counts what it is asked for and records stream
/// identities, so the plumbing tests can see exactly which levels were
/// created.
#[derive(Default)]
struct Counting {
    sessions: AtomicUsize,
    infos: Mutex<Vec<SessionInfo>>,
    streams: Arc<Mutex<Vec<StreamInfo>>>,
}

impl Observer for Counting {
    /// Count the session and create its stream-counting handler.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        self.sessions.fetch_add(1, Ordering::Relaxed);
        self.infos.lock().unwrap().push(*session);
        Some(Box::new(CountingSession {
            streams: Arc::clone(&self.streams),
        }))
    }
}

/// A session handler that records every stream offered to it.
struct CountingSession {
    streams: Arc<Mutex<Vec<StreamInfo>>>,
}

/// Record each stream and accept it with a sink.
impl SessionObserver for CountingSession {
    /// Retain the stream metadata and return an inert handler.
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        self.streams.lock().unwrap().push(*stream);
        Some(Box::new(Sink))
    }
}

/// A stream handler that accepts and discards every message.
struct Sink;

/// Discard each delivered message.
impl StreamObserver for Sink {
    /// Accept the message without retaining it.
    fn message(&mut self, _: &[u8]) {}
}

/// An observer that records its registration position at every level.
struct Ordered {
    /// This observer's registration position.
    index: usize,
    /// The shared call log.
    calls: Arc<Mutex<Vec<usize>>>,
}

/// Record the peer callback and accept the session.
impl Observer for Ordered {
    /// Record the call and return the next-level handler.
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        self.calls.lock().unwrap().push(self.index);
        Some(Box::new(Ordered {
            index: self.index,
            calls: Arc::clone(&self.calls),
        }))
    }
}

/// Record session callbacks and accept each stream.
impl SessionObserver for Ordered {
    /// Record the election callback.
    fn elected(&self, _: Role) {
        self.calls.lock().unwrap().push(self.index);
    }

    /// Record the stream callback and return the next-level handler.
    fn stream(&self, _: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        self.calls.lock().unwrap().push(self.index);
        Some(Box::new(Ordered {
            index: self.index,
            calls: Arc::clone(&self.calls),
        }))
    }
}

/// Record stream-message callbacks.
impl StreamObserver for Ordered {
    /// Record delivery to this handler.
    fn message(&mut self, _: &[u8]) {
        self.calls.lock().unwrap().push(self.index);
    }
}

/// An unattached peer's session handle is inert: nothing is created and
/// every invocation is a no-op, whatever the session kind.
#[test]
fn unattached_handles_are_inert() {
    let attachment = Attachment::default();
    let handle = attachment.begin(SessionKind::Gossip);
    assert!(!handle.attached());
    handle.control_sent(b"x");
    handle.control_received(b"x");
    handle.elected(Role::Initiator);
    assert!(handle.data(Role::Initiator, 0, Direction::Sent).is_none());
}

/// Beginning an observed V2 session creates the control stream's two
/// directed handlers immediately, ahead of any wire traffic.
#[test]
fn begin_creates_the_control_handlers() {
    let observer = Arc::new(Counting::default());
    let mut attachment = Attachment::default();
    attachment.attach(observer.clone());

    let handle = attachment.begin(SessionKind::Bootstrap);
    assert!(handle.attached());
    assert_eq!(observer.sessions.load(Ordering::Relaxed), 1);
    let infos = observer.infos.lock().unwrap();
    assert_eq!(
        *infos,
        vec![SessionInfo {
            kind: SessionKind::Bootstrap,
            protocol: Protocol::V2,
        }]
    );
    let streams = observer.streams.lock().unwrap();
    assert_eq!(
        *streams,
        vec![
            StreamInfo {
                id: StreamId::Control,
                direction: Direction::Sent,
            },
            StreamInfo {
                id: StreamId::Control,
                direction: Direction::Received,
            },
        ]
    );
}

/// A bootstrap's debug summary reports how many observers it carries.
#[test]
fn bootstrap_debug_reports_observer_count() {
    let observer = Arc::new(Counting::default());
    let bootstrap = crate::Peer::<()>::bootstrap().observe(observer);
    let debug = format!("{bootstrap:?}");
    assert!(
        debug.contains("observe: Attachment { observers: 1 }"),
        "{debug}"
    );
}

proptest! {
    /// Every observation level visits all handlers in registration order.
    #[test]
    fn repeated_attachment_preserves_every_callback(observer_count in 0usize..8) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut attachment = Attachment::default();
        for index in 0..observer_count {
            attachment.attach(Arc::new(Ordered {
                index,
                calls: Arc::clone(&calls),
            }));
        }

        let handle = attachment.begin(SessionKind::Gossip);
        handle.elected(Role::Initiator);
        handle.control_sent(b"sent");
        handle.control_received(b"received");
        if let Some(mut data) = handle.data(Role::Initiator, 3, Direction::Sent) {
            data.message(b"data");
        }

        // `begin` makes three ordered passes: sessions, then both control
        // directions. Election, both control messages, opening the data stream,
        // and its message add five more.
        let expected = std::iter::repeat_n(0..observer_count, 8)
            .flatten()
            .collect::<Vec<_>>();

        prop_assert_eq!(&*calls.lock().unwrap(), &expected);
        prop_assert_eq!(handle.attached(), observer_count != 0);
    }
}

/// The capture adapter retains exactly the bytes it delivered, across
/// split reads, so an observed exact read hands its handler the true
/// wire bytes.
#[tokio::test]
async fn capture_read_retains_delivered_bytes() {
    let mut source: &[u8] = b"one item";
    let mut capture = CaptureRead::new(&mut source);
    let mut first = [0u8; 3];
    capture.read_exact(&mut first).await.unwrap();
    let mut rest = Vec::new();
    capture.read_to_end(&mut rest).await.unwrap();
    assert_eq!(capture.bytes(), b"one item");
}
