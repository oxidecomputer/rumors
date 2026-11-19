//! The payload admission contract, exercised peer-to-peer.
//!
//! Send-side admission ([`Peer::send`]) runs the exact decode every
//! receiver's wire ingress runs — same payload type, same limit, same engine —
//! and requires the decoded value to equal the value sent, so a payload
//! admitted anywhere is transferable everywhere and reads back everywhere as
//! what its author meant. The boundary pins here send payloads at exactly the
//! default depth limit through real gossip sessions (the container spine and
//! the recommended versioning-`enum` shape, whose decode recursion is
//! type-dependent), reject one step past the limit at its author, reject a
//! lossy value (`Some(None)`) at its author, and carry deep content across a
//! fleet whose limit was raised in concert; the last pin holds that the sender
//! exits with an error when a counterparty aborts mid-session on a decode
//! failure.

use rumors_testkit::common;

use rumors::{Peer, default};

use crate::common::wire::{join_fork, wire_gossip};
use rumors_testkit::common::property::{LOCAL_DIVISOR, SESSION_DIVISOR};
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

/// Pure CBOR array nesting from a type meeting every message type
/// requirement: each layer serializes as a one-element array, the innermost
/// empty.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Arr(Vec<Arr>);

/// A payload of exactly `depth` nested array scopes (`depth` >= 1).
fn nested(depth: u64) -> Arr {
    (1..depth).fold(Arr(vec![]), |a, _| Arr(vec![a]))
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A payload at exactly the default depth limit is admitted at send and
    /// round-trips peer-to-peer over an in-memory connection: the boundary holds
    /// end to end.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_payload_at_the_default_depth_round_trips(network in network_seed()) {
        let at_limit = nested(default::MESSAGE_DEPTH_LIMIT);
        let a: Peer<Arr> = network.builder().sync_memory_budget(0).seed();
        let b = join_fork(&a);

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
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// One step past the limit is rejected at send with the depth
    /// error, at the author, at the moment of choice, and nothing is
    /// stored, so no session can ever wedge on it.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn one_step_past_the_limit_is_rejected_at_send(network in network_seed()) {
        let peer: Peer<Arr> = network.builder().sync_memory_budget(0).seed();
        let error = peer
            .send(nested(default::MESSAGE_DEPTH_LIMIT + 1))
            .expect_err("one step past the limit is rejected");
        assert!(
            matches!(error, rumors::error::EncodeError::TooDeep { limit } if limit == default::MESSAGE_DEPTH_LIMIT),
            "the rejection is the depth case naming the limit: {error:?}"
        );
        assert_eq!(peer.snapshot().len(), 0, "a rejected send stores nothing");
    }
}

/// A recursive enum in the crate docs' recommended versioning shape.
///
/// Each `N` wrapper is one map scope on the wire, and decoding it as `E` prices
/// the innermost unit variant one further recursion step — accounting only the
/// type's own decode can price, which is why admission runs that decode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum E {
    A,
    N(Box<E>),
}

/// `E::A` under `wrappers` layers of `E::N`.
fn nested_enum(wrappers: u64) -> E {
    (0..wrappers).fold(E::A, |e, _| E::N(Box::new(e)))
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// The deepest enum payload whose own decode fits the default limit is
    /// admitted at send and round-trips peer-to-peer at equal limits.
    ///
    /// The admission decode is the ingress decode, so what sends is exactly
    /// what lands, for the type-dependent enum accounting too.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn the_deepest_admissible_enum_round_trips(network in network_seed()) {
        // The innermost unit variant costs the step the wrappers don't.
        let at_limit = nested_enum(default::MESSAGE_DEPTH_LIMIT - 1);
        let a: Peer<E> = network.builder().sync_memory_budget(0).seed();
        let b = join_fork(&a);

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
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// An enum payload whose own decode needs one step past the limit is
    /// rejected at send with the depth error, and nothing is stored:
    /// the author learns at the moment of choice, and no receiver can ever
    /// see the value.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn an_enum_needing_one_step_past_the_limit_is_rejected_at_send(network in network_seed()) {
        let peer: Peer<E> = network.builder().sync_memory_budget(0).seed();
        let error = peer
            .send(nested_enum(default::MESSAGE_DEPTH_LIMIT))
            .expect_err("a decode needing limit + 1 is rejected");
        assert!(
            matches!(error, rumors::error::EncodeError::TooDeep { limit } if limit == default::MESSAGE_DEPTH_LIMIT),
            "the rejection is the depth case naming the limit: {error:?}"
        );
        assert_eq!(peer.snapshot().len(), 0, "a rejected send stores nothing");
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A fleet whose limit was raised in concert gossips content past the
    /// default depth clean: the knob threads through send admission, the
    /// join builder, and wire ingress alike.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn equal_raised_limits_gossip_deep_content_clean(network in network_seed()) {
        let raised = default::MESSAGE_DEPTH_LIMIT + 64;
        let deep = nested(default::MESSAGE_DEPTH_LIMIT + 32);

        let a: Peer<Arr> = network.builder()
            .message_depth_limit(raised)
            .sync_memory_budget(0).seed();
        // The fork selects the same raised limit one session early, on the
        // join builder, exactly as a real fleet member would.
        let b = crate::common::wire::block_on(async {
            let (mut provider, mut newcomer) =
                rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (served, joined) = tokio::join!(
                a.gossip_once(&mut provider.read, &mut provider.write),
                Peer::<Arr>::builder()
                    .message_depth_limit(raised)
                    .sync_memory_budget(0)
                    .join(&mut newcomer.read, &mut newcomer.write),
            );
            served.expect("the seed serves the join");
            match joined {
                rumors::Join::Connected { peer, connection: rumors::Connection::Reusable { .. } } => peer,
                _ => panic!("the seed is established"),
            }
        });

        a.send(deep.clone()).expect("the raised limit admits it");
        wire_gossip(&a, &b);

        let snapshot = b.snapshot();
        let (_, received) = snapshot.iter().next().expect("the deep payload arrived");
        assert_eq!(*received, deep);
    }
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// The canonical faithfulness regression: a payload whose encoding
    /// decodes to a different value is rejected at send, and nothing is
    /// stored.
    ///
    /// A nested `Option` holding `Some(None)` serializes to CBOR null and
    /// decodes as `None` — a silent divergence in value space that is
    /// invisible in byte space (re-serializing the decoded `None` yields
    /// the same null). Admission compares the decoded value against the
    /// value sent, so the lossy value dies typed at its author and no
    /// replica can ever read a message differently than it was meant.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_lossy_value_is_rejected_at_send(network in network_seed()) {
        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        struct P {
            field: Option<Option<u32>>,
        }
        let peer: Peer<P> = network.builder().sync_memory_budget(0).seed();
        let error = peer
            .send(P { field: Some(None) })
            .expect_err("a value whose encoding decodes differently is rejected");
        assert!(
            matches!(error, rumors::error::EncodeError::Roundtrip),
            "the rejection is the typed faithfulness case: {error:?}"
        );
        assert_eq!(peer.snapshot().len(), 0, "a rejected send stores nothing");

        // The faithful values of the same type pass: admission judges the
        // value, not the type.
        peer.send(P { field: None }).expect("None is faithful");
        peer
            .send(P {
                field: Some(Some(7)),
            })
            .expect("Some(Some(_)) is faithful");
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// When a counterparty aborts mid-session on a payload decode failure
    /// and discards its connection, the sender's own `gossip` completes
    /// with an error rather than hanging.
    ///
    /// The sender's error is a transport failure: the departure of its
    /// counterparty, met by whichever of its reads or writes comes first, in
    /// reconciliation or at completion depending on how far the sender got.
    /// It still holds its message.
    ///
    /// The decode failure is injected by pairing ends whose payload types
    /// disagree (the sender's set holds a CBOR integer; the receiver
    /// decodes payloads as `String`), the one shape that still fails at
    /// ingress between equal limits now that admission runs the receiving
    /// decode. The receiver's own exit is the typed decode error.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_sender_exits_typed_when_its_counterparty_aborts_on_decode(network in network_seed()) {
        let a: Peer<u64> = network.builder().sync_memory_budget(0).seed();
        let b = crate::common::wire::block_on(async {
            let (mut provider, mut newcomer) =
                rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (served, joined) = tokio::join!(
                a.gossip_once(&mut provider.read, &mut provider.write),
                Peer::<String>::builder()
                    .sync_memory_budget(0)
                    .join(&mut newcomer.read, &mut newcomer.write),
            );
            served.expect("the seed serves the join (no payloads to decode yet)");
            match joined {
                rumors::Join::Connected { peer, connection: rumors::Connection::Reusable { .. } } => peer,
                _ => panic!("the seed is established"),
            }
        });

        a.send(7u64).expect("a flat integer");

        // Production-shaped teardown: each side drives its own end, and the
        // aborting side's session owns its halves, so a failure drops them, as
        // every session `Err` instructs. `block_on` fails on quiescence without
        // completion, so a hung sender is a test failure here, never a parked
        // process.
        let (a_out, b_out) = crate::common::wire::block_on(async {
            let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            tokio::join!(
                a.gossip_once(a_end.read, a_end.write),
                b.gossip_once(b_end.read, b_end.write)
            )
        });

        let b_err = b_out.expect_err("the receiver aborts on the payload decode");
        assert!(
            matches!(b_err, rumors::Error::Protocol(_)),
            "the receiver's exit is the typed decode failure: {b_err:?}"
        );
        let a_err = a_out.expect_err("the sender's session cannot complete without its counterparty");
        assert!(
            matches!(
                a_err,
                rumors::Error::Transport(ref error) if matches!(
                    error.context.phase,
                    rumors::error::Phase::Reconciliation | rumors::error::Phase::Completion
                )
            ),
            "the sender meets its counterparty's departure as a transport failure, \
             in reconciliation or at completion: {a_err:?}"
        );
        assert_eq!(
            a.snapshot().len(),
            1,
            "the sender's replica still holds its message"
        );
    }
}
