use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::AsyncReadExt;

use super::*;

/// A handler that counts what it is asked for and records stream
/// identities, so the plumbing tests can see exactly which levels were
/// minted.
#[derive(Default)]
struct Counting {
    sessions: AtomicUsize,
    infos: Mutex<Vec<SessionInfo>>,
    streams: Arc<Mutex<Vec<StreamInfo>>>,
}

impl Observer for Counting {
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        self.sessions.fetch_add(1, Ordering::Relaxed);
        self.infos.lock().unwrap().push(*session);
        Some(Box::new(CountingSession {
            streams: Arc::clone(&self.streams),
        }))
    }
}

struct CountingSession {
    streams: Arc<Mutex<Vec<StreamInfo>>>,
}

impl SessionObserver for CountingSession {
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        self.streams.lock().unwrap().push(*stream);
        Some(Box::new(Sink))
    }
}

struct Sink;

impl StreamObserver for Sink {
    fn message(&mut self, _: &[u8]) {}
}

/// An unattached peer's session handle is inert: nothing is minted and
/// every invocation is a no-op, whatever the session kind.
#[test]
fn unattached_handles_are_inert() {
    let attachment = Attachment::default();
    let handle = attachment.begin(SessionKind::Gossip, Protocol::V2);
    assert!(!handle.attached());
    handle.control_sent(b"x");
    handle.control_received(b"x");
    handle.elected(Role::Initiator);
    assert!(handle.data(Role::Initiator, 0, Direction::Sent).is_none());
}

/// Beginning an observed V2 session mints the control stream's two
/// directed handlers immediately, ahead of any wire traffic.
#[test]
fn begin_mints_the_control_handlers() {
    let observer = Arc::new(Counting::default());
    let mut attachment = Attachment::default();
    attachment.attach(observer.clone());

    let handle = attachment.begin(SessionKind::Bootstrap, Protocol::V2);
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
