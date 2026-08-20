//! The symmetric payload nesting-depth limit, exercised peer-to-peer.
//!
//! Send-side admission ([`Rumors::send`]) and decode-side wire ingress
//! judge the same bound, so a payload admitted anywhere is transferable
//! everywhere: the boundary pins here send a payload at exactly the
//! default limit through a real gossip session, reject one scope past it
//! at its author, and carry deep content across a fleet whose limit was
//! raised in concert.

mod common;

use ciborium::Value;
use rumors::{DEFAULT_PAYLOAD_DEPTH_LIMIT, PayloadDepthLimit, Peer, Rumors};

use crate::common::wire::{bootstrap_fork, wire_gossip};

/// A payload nested in exactly `depth` array scopes around one integer.
fn nested(depth: u64) -> Value {
    (0..depth).fold(Value::Integer(0.into()), |value, _| {
        Value::Array(vec![value])
    })
}

/// A payload at exactly the default depth limit is admitted at send and
/// round-trips peer-to-peer over an in-memory link: the boundary holds
/// end to end.
#[test]
fn a_payload_at_the_default_depth_round_trips() {
    let at_limit = nested(DEFAULT_PAYLOAD_DEPTH_LIMIT.get());
    let a: Rumors<Value> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    a.send(at_limit.clone())
        .expect("a payload at exactly the limit is admitted");
    wire_gossip(&a, &b);

    let snapshot = b.snapshot();
    let (_, received) = snapshot.iter().next().expect("the payload arrived");
    assert_eq!(
        *received, at_limit,
        "the payload survives the transfer intact"
    );
}

/// One scope past the limit is rejected at send with the typed error —
/// at the author, at the moment of choice — and nothing is stored, so no
/// session can ever wedge on it.
#[test]
fn one_scope_past_the_limit_is_rejected_at_send() {
    let rumors: Rumors<Value> = Peer::seed().sync_window_floor().into_rumors();
    let error = rumors
        .send(nested(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 1))
        .expect_err("one scope past the limit is rejected");
    assert_eq!(error.limit, DEFAULT_PAYLOAD_DEPTH_LIMIT);
    assert_eq!(rumors.snapshot().len(), 0, "a rejected send stores nothing");
}

/// A fleet whose limit was raised in concert gossips content past the
/// default depth clean: the knob threads through send admission, the
/// bootstrap builder, and wire ingress alike.
#[test]
fn equal_raised_limits_gossip_deep_content_clean() {
    let raised = PayloadDepthLimit::new(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 64);
    let deep = nested(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 32);

    let a: Rumors<Value> = Peer::seed()
        .payload_depth_limit(raised)
        .sync_window_floor()
        .into_rumors();
    // The fork selects the same raised limit one session early, on the
    // bootstrap builder, exactly as a real fleet member would.
    let b = crate::common::wire::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory();
        let (served, joined) = tokio::join!(
            a.gossip(&mut provider),
            Peer::<Value>::bootstrap()
                .payload_depth_limit(raised)
                .join(&mut newcomer),
        );
        served.expect("the seed serves the bootstrap");
        joined
            .expect("the bootstrap session completes")
            .expect("the seed is established")
            .sync_window_floor()
            .into_rumors()
    });

    a.send(deep.clone()).expect("the raised limit admits it");
    wire_gossip(&a, &b);

    let snapshot = b.snapshot();
    let (_, received) = snapshot.iter().next().expect("the deep payload arrived");
    assert_eq!(*received, deep);
}

/// A recording observer for the mismatch pin: retains whether a session
/// elected roles and how many *data* streams it opened.
///
/// The control stream's two directions open with the session itself, so
/// they are not counted; what this makes observable is the abort's
/// placement — after the greetings, before the election and any data
/// stream.
#[derive(Default)]
struct SessionShape {
    elected: std::sync::Mutex<bool>,
    streams: std::sync::Mutex<usize>,
}

struct ShapeObserver(std::sync::Arc<SessionShape>);

struct ShapeSession(std::sync::Arc<SessionShape>);

impl rumors::observe::Observer for ShapeObserver {
    fn session(
        &self,
        _session: &rumors::observe::SessionInfo,
    ) -> Option<Box<dyn rumors::observe::SessionObserver>> {
        Some(Box::new(ShapeSession(self.0.clone())))
    }
}

impl rumors::observe::SessionObserver for ShapeSession {
    fn elected(&self, _role: rumors::observe::Role) {
        *self.0.elected.lock().unwrap() = true;
    }

    fn stream(
        &self,
        stream: &rumors::observe::StreamInfo,
    ) -> Option<Box<dyn rumors::observe::StreamObserver>> {
        // The control stream's two directions open with the session
        // itself; only data streams witness the descent starting.
        if matches!(stream.id, rumors::observe::StreamId::Data { .. }) {
            *self.0.streams.lock().unwrap() += 1;
        }
        None
    }
}

/// Peers with different payload depth limits abort on both sides with
/// the typed mismatch, each error naming both limits from its own
/// vantage.
///
/// The session opens no data stream and elects no role: the check runs
/// after the greetings and before anything else.
///
/// The pair is *converged* (freshly forked, no divergence), so the pin
/// also proves the check precedes the equal-versions short-circuit:
/// a mixed configuration is caught even on a no-op session.
#[test]
fn mismatched_limits_abort_both_sides_at_the_handshake() {
    use rumors::Error;
    let a: Rumors<Value> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    // Re-key one side's limit through the Peer knob: the fork was minted
    // at the default, and the setting follows the peer through reunion.
    let raised = PayloadDepthLimit::new(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 1);
    let b = crate::common::wire::block_on(async {
        let peer = b.try_into_peer().await.expect("the sole handle reunites");
        peer.payload_depth_limit(raised).into_rumors()
    });

    let a_shape = std::sync::Arc::new(SessionShape::default());
    let b_shape = std::sync::Arc::new(SessionShape::default());
    let a = crate::common::wire::block_on(async {
        let peer = a.try_into_peer().await.expect("the sole handle reunites");
        peer.observe(std::sync::Arc::new(ShapeObserver(a_shape.clone())))
            .into_rumors()
    });
    let b = crate::common::wire::block_on(async {
        let peer = b.try_into_peer().await.expect("the sole handle reunites");
        peer.observe(std::sync::Arc::new(ShapeObserver(b_shape.clone())))
            .into_rumors()
    });

    let (a_err, b_err) = crate::common::wire::block_on(async {
        let (mut a_link, mut b_link) = rumors::link::memory();
        let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
        (
            a_out.expect_err("mismatched limits must abort"),
            b_out.expect_err("mismatched limits must abort"),
        )
    });

    let Error::PayloadDepthMismatch { local, remote } = a_err else {
        panic!("a's error must be the typed mismatch: {a_err:?}");
    };
    assert_eq!((local, remote), (DEFAULT_PAYLOAD_DEPTH_LIMIT, raised));
    let Error::PayloadDepthMismatch { local, remote } = b_err else {
        panic!("b's error must be the typed mismatch: {b_err:?}");
    };
    assert_eq!((local, remote), (raised, DEFAULT_PAYLOAD_DEPTH_LIMIT));

    for shape in [&a_shape, &b_shape] {
        assert!(
            !*shape.elected.lock().unwrap(),
            "the mismatch precedes the role election"
        );
        assert_eq!(
            *shape.streams.lock().unwrap(),
            0,
            "the mismatch opens no data stream"
        );
    }
}
