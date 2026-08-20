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
/// round-trips peer-to-peer over an in-memory link: the boundary is
/// usable end to end, not merely admitted locally.
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
