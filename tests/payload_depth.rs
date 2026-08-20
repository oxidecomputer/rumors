//! The payload nesting-depth limit, exercised peer-to-peer.
//!
//! Send-side admission ([`Rumors::send`]) runs the exact decode every
//! receiver's wire ingress runs — same payload type, same limit, same
//! engine — so a payload admitted anywhere is transferable everywhere.
//! The boundary pins here send payloads at exactly the default limit
//! through real gossip sessions (the container spine and the
//! recommended versioning-`enum` shape, whose decode recursion is
//! type-dependent), reject one step past the limit at its author, and
//! carry deep content across a fleet whose limit was raised in concert;
//! the last pin holds the sender's exit typed when a counterparty
//! aborts mid-session on a decode failure.

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

/// One step past the limit is rejected at send with the typed depth
/// error — at the author, at the moment of choice — and nothing is
/// stored, so no session can ever wedge on it.
#[test]
fn one_step_past_the_limit_is_rejected_at_send() {
    let rumors: Rumors<Value> = Peer::seed().sync_window_floor().into_rumors();
    let error = rumors
        .send(nested(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() + 1))
        .expect_err("one step past the limit is rejected");
    assert!(
        matches!(error, rumors::EncodeError::Depth { limit } if limit == DEFAULT_PAYLOAD_DEPTH_LIMIT),
        "the rejection is the typed depth case naming the limit: {error:?}"
    );
    assert_eq!(rumors.snapshot().len(), 0, "a rejected send stores nothing");
}

/// A recursive enum in the crate docs' recommended versioning shape.
///
/// Each `N` wrapper is one map scope on the wire, and decoding it as
/// `E` prices the innermost unit variant one further recursion step —
/// accounting only the type's own decode can price, which is why
/// admission runs that decode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum E {
    A,
    N(Box<E>),
}

/// `E::A` under `wrappers` layers of `E::N`.
fn nested_enum(wrappers: u64) -> E {
    (0..wrappers).fold(E::A, |e, _| E::N(Box::new(e)))
}

/// The deepest enum payload whose own decode fits the default limit is
/// admitted at send and round-trips peer-to-peer at equal limits.
///
/// The admission decode is the ingress decode, so what sends is exactly
/// what lands, for the type-dependent enum accounting too.
#[test]
fn the_deepest_admissible_enum_round_trips() {
    // The innermost unit variant costs the step the wrappers don't.
    let at_limit = nested_enum(DEFAULT_PAYLOAD_DEPTH_LIMIT.get() - 1);
    let a: Rumors<E> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    a.send(at_limit.clone())
        .expect("a payload whose decode needs exactly the limit is admitted");
    wire_gossip(&a, &b);

    let snapshot = b.snapshot();
    let (_, received) = snapshot.iter().next().expect("the payload arrived");
    assert_eq!(
        *received, at_limit,
        "the payload survives the transfer intact"
    );
}

/// An enum payload whose own decode needs one step past the limit is
/// rejected at send with the typed depth error, and nothing is stored:
/// the author learns at the moment of choice, and no receiver can ever
/// see the value.
#[test]
fn an_enum_needing_one_step_past_the_limit_is_rejected_at_send() {
    let rumors: Rumors<E> = Peer::seed().sync_window_floor().into_rumors();
    let error = rumors
        .send(nested_enum(DEFAULT_PAYLOAD_DEPTH_LIMIT.get()))
        .expect_err("a decode needing limit + 1 is rejected");
    assert!(
        matches!(error, rumors::EncodeError::Depth { limit } if limit == DEFAULT_PAYLOAD_DEPTH_LIMIT),
        "the rejection is the typed depth case naming the limit: {error:?}"
    );
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

/// When a counterparty aborts mid-session on a payload decode failure
/// and discards its poisoned link, the sender's own `gossip` completes
/// with a typed error rather than hanging.
///
/// The sender's error is [`rumors::Error::Epilogue`]: its local session
/// work committed, and only the peer's confirmation was lost. The
/// sender's replica is unharmed and still holds its message; the
/// failure's blast radius is one aborted session on each side.
///
/// The decode failure is injected by pairing ends whose payload types
/// disagree (the sender's set holds a CBOR integer; the receiver
/// decodes payloads as `String`), the one shape that still fails at
/// ingress between equal limits now that admission runs the receiving
/// decode. The receiver's own exit is the typed decode error.
#[test]
fn a_sender_exits_typed_when_its_counterparty_aborts_on_decode() {
    let a: Rumors<Value> = Peer::seed().sync_window_floor().into_rumors();
    let b = crate::common::wire::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory();
        let (served, joined) = tokio::join!(
            a.gossip(&mut provider),
            Peer::<String>::bootstrap().join(&mut newcomer),
        );
        served.expect("the seed serves the bootstrap (no payloads to decode yet)");
        joined
            .expect("the bootstrap session completes")
            .expect("the seed is established")
            .sync_window_floor()
            .into_rumors()
    });

    a.send(Value::Integer(7.into())).expect("a flat integer");

    // Production-shaped teardown: each side drives its own end, and the
    // aborting side discards its link, as every session `Err` instructs.
    // `block_on` fails on quiescence without completion, so a hung
    // sender is a test failure here, never a parked process.
    let (a_out, b_out) = crate::common::wire::block_on(async {
        let (mut a_link, b_link) = rumors::link::memory();
        tokio::join!(a.gossip(&mut a_link), async move {
            let mut b_link = b_link;
            let out = b.gossip(&mut b_link).await;
            drop(b_link);
            out
        })
    });

    let b_err = b_out.expect_err("the receiver aborts on the payload decode");
    assert!(
        matches!(b_err, rumors::Error::Mirror(_)),
        "the receiver's exit is the typed decode failure: {b_err:?}"
    );
    let a_err = a_out.expect_err("the sender's session cannot confirm completion");
    assert!(
        matches!(a_err, rumors::Error::Epilogue(_)),
        "the sender committed locally and lost only the confirmation: {a_err:?}"
    );
    assert_eq!(
        a.snapshot().len(),
        1,
        "the sender's replica still holds its message"
    );
}
