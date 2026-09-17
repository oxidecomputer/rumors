# rumors

<!-- cargo-rdme start -->

Lightspeed causal gossip for high-bandwidth networks.

`rumors` replicates a set of messages across a fleet of peers with no
coordination: every peer holds a full replica, changes it locally (inserting
*or* removing messages), and reconciles pairwise with whichever peer(s) it
can reach. Replicas which transitively gossip eventually converge on the
same set of messages; `rumors` works hard to turn "eventually" into "ASAP".

Unlike many gossip protocols, `rumors` features **redaction**. When any peer
redacts a message, the deletion propagates through gossip and purges that
message from every replica. A redacted message leaves no tombstone, so local
memory can shrink with the live set. Gossip still carries the causal history
needed to honor the deletion.

## When *should* you use it?

**If bandwidth is abundant and latency matters.**

Most gossip protocols are designed to be thrifty with bandwidth, trading
increased rounds of communication for smaller metadata overhead. However,
bandwidth is only getting cheaper and more plentiful, whereas *latency* is
capped by the laws of physics. `rumors` is designed for today and tomorrow;
it optimizes for extremely fast convergence when bandwidth is not a primary
constraint.

## When *shouldn't* you use it?

- **If the set of live messages outgrows its smallest peer.** Every peer
  replicates the whole set; sharding is not supported.
- **If you need a consistently ordered, durable history.** A replicated log
  gives you sequencing; `rumors` only gives you causal ordering, which may
  be linearized differently between peers.
- **If you don't control the peers.** An authorized member can follow the
  protocol while arbitrarily changing the gossip set: every member can
  write and redact messages. Completing synchronization requires finishing
  the expected protocol exchange; it does not make those updates trustworthy.
  A non-conforming peer can leave synchronization waiting indefinitely.
  Applications choose deadlines when they need to bound those waits.
  Authenticating peers and securing the transport are the application's job;
  the `link` module lists exactly what the protocol asks of the transport.
- **If bandwidth is your scarce resource.** `rumors` buys low latency with
  bandwidth. Small reconciliations can spend several kilobytes of protocol
  traffic per message. Metered, narrow, or high-loss links may call for a
  different design.

## Joining and leaving a network

`Peer::seed` creates a new gossip network. Other peers join through
`Peer::bootstrap`, synchronizing with any established member. Peers
created by independent calls to `seed` belong to separate networks and
cannot gossip with each other.

`Peer::retire` leaves the network after a final synchronization with
another member. Retiring when possible and reusing a `Bookmark` across
restarts help keep message versions compact as peers come and go.

## The shape of the API

`Peer` manages a replica's lifecycle: creating a network, joining one,
attaching a bookmark, and retiring. It cannot be cloned.

`Peer::into_rumors` returns a `Rumors` handle for everyday use. Clone
that handle to `send`, `redact`, observe
`messages`, and `gossip`
concurrently. All handles share the same replica.

`Rumors::gossip` keeps a connection synchronized, initiating on local
changes by default. Configure `Peer::gossip_when` to choose another
initiation policy and `Peer::session_deadline` to limit active exchanges
without timing idle waits. Use `Rumors::gossip_once` to force one exchange.

When the other handles have been dropped, `Rumors::try_into_peer`
recovers the `Peer`. This ensures retirement cannot happen while another
handle still uses the replica.

The `Peer` docs walk the full lifecycle as one runnable example,
including every retirement outcome and bootstrapping a universe without
a distinguished first peer. For a guided first encounter (two peers from
an empty project through send, gossip, and redaction, with the output each
step prints), start at `tutorial`. For how a session actually reconciles
two replicas (the descent, the disjoint frontier, why deletion needs no
tombstones, and how the design compares to its neighbors), read
`reconciliation`.

## Example

Two peers, one universe, one message, one gossip session. Shown whole,
nothing hidden, as it would sit in a `main.rs` (the async runtime here is
Tokio for convenience; see [Runtime independence](#runtime-independence)):

```rust
use rumors::Peer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The universe's first peer creates it; every later peer bootstraps in.
    let alice = Peer::<String>::seed().into_rumors();

    // A send commits right here.
    alice.send("the meeting is at noon".to_string())?;

    // A session runs over a `Link`: a control byte stream plus a supply
    // of independent data streams (see the `link` module); here, the
    // in-memory pair. Alice serves one gossip session...
    let (mut near, mut far) = rumors::link::memory();
    let serve = alice.clone();
    tokio::spawn(async move {
        serve.gossip_once(&mut far).await.unwrap();
    });

    // ...and Bob joins the universe through it, arriving as a full replica.
    let rumors::Joined::Joined { peer: bob } =
        Peer::<String>::bootstrap().join(&mut near).await
    else {
        panic!("Alice must serve the bootstrap");
    };
    let bob = bob.into_rumors();

    // Convergence: Bob holds the message Alice sent before they ever met.
    let snapshot = bob.snapshot();
    let (_version, message) = snapshot.iter().next().expect("one live message");
    println!("bob heard: {message}");
    // Prints exactly:
    //     bob heard: the meeting is at noon
    assert_eq!(message.as_str(), "the meeting is at noon");
    Ok(())
}
```

## How should you observe messages?

- `Snapshot` (`Rumors::snapshot`) is a **point-in-time value**:
  iterate it, look up a message by its `Version` (`Snapshot::get`),
  or slice it by causal range (`Snapshot::range`). Taking one is
  cheap and never waits.
- `UnorderedMessages` (`Rumors::unordered_messages`) is the **live stream, arbitrary
  order**: everything not already inside your starting checkpoint, then
  everything learned afterwards, at the lowest cost. Use it by default.
- `CausalMessages` (`Rumors::causal_messages`) is the **live
  stream, causal order**: a message arrives only after everything it
  causally depends on, for an amortized logarithmic surcharge with
  bursts up to the size of the set. Use it only when consumers require
  causal delivery.
- `Changes` (`Rumors::changes`) is the **live signal, no content**:
  one coalesced `()` per observed advance of the set, for waking work
  that reacts to change without consuming it: gossip drivers,
  persist-on-change, UI refresh. It is not delivery; pair it with a
  checkpoint-bearing observer for that.

The live message observers expose a `checkpoint`:
the sound resume point for delivery across restarts. Its docs state exactly
what a resume re-observes, and why folding the yielded versions yourself is
not a substitute.

**Deletions are never delivered as events.** A redacted message simply
stops being live, and no redaction object exists anywhere for an observer
to yield (`Rumors::redact` explains why none is needed); an application
that needs deletion events sends them as ordinary messages of its own.

All of the above observe the *set*. To watch the *wire* instead — every
protocol message of a live session, as raw CBOR items, for debuggers,
recorders, and tracing adapters — attach a handler from the `observe`
module (`Peer::observe`).

## Transport: bring a `Link`

A session's transport is a `Link`: one persistent bidirectional
*control stream* plus a supply of independent, individually
flow-controlled unidirectional *data streams*, opened lazily as
reconciliation needs them. The `link` module states what an
implementation must guarantee, ships the in-memory instantiation
(`link::memory`), and documents how to bind a real transport (QUIC
connections map streams one to one; TCP can carry one stream per
connection behind a routing listener). A conformance suite (the
`conformance::link` module, unlocked by the `conformance` cargo feature)
checks those guarantees on a caller-built link. `Link`'s docs state
what a session promises on `Ok`, `Err`,
and cancellation.

## Runtime independence

Sessions and observers are plain futures and streams, driven entirely by
the caller. The I/O traits are Tokio's runtime-independent
[`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html)
and [`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html);
no Tokio runtime, spawning, sockets, or timers are required by this crate.

## Choosing a payload type

Your message type `T` needs `serde::Serialize`,
`serde::de::DeserializeOwned`, `Eq`, `Send`, `Sync`, and
`'static`, all demanded once, at peer construction. Payloads are
serialized as CBOR ([RFC 8949](https://www.rfc-editor.org/rfc/rfc8949)).
Each bound guards replication:

- **`Serialize` must succeed on every value you send.** CBOR itself
  imposes no format-driven failures, so a `Serialize` error is a bug
  in the payload type: sending panics. Avoid types whose `Serialize`
  is data-dependently fallible (for example `std::path::PathBuf`,
  which errors on non-UTF-8 paths).
- **Every encoding must decode back equal to the value sent.** Each
  send re-decodes its own encoding with the exact decoder receivers
  run and compares by `Eq`; a lossy encoding (for example
  `Some(None)` in a nested `Option`, which decodes as `None`) is the
  typed `EncodeError`, rejected at the author rather than silently
  diverging at every replica. The bound is `Eq` rather than
  `PartialEq` so the check is never spurious; this excludes
  `f32`/`f64` fields (NaN compares unequal to itself).
- **Nesting depth is bounded.** Decoding a payload may recurse at
  most `Peer::payload_depth_limit` steps (256 by default, ample
  for ordinary types); an over-deep value is rejected at send. The
  limit is held to exact equality fleet-wide at every handshake, so
  an admitted payload is transferable everywhere; the knob's docs
  carry the full contract.

On compatibility across versions of your own type: because CBOR
carries field and variant *names*, reordering `struct` fields or
`enum` variants does not break compatibility with prior versions of
your type `T`; however, *renaming breaks compatibility*. It is worth
designing around this from the get-go: consider an outer `enum`
indicating the version of your application-level message type, even
if it starts out only having one variant, `V1`.

## Cargo features

Every feature is off by default.

- `conformance`: the public validation suite for caller-built `link`
  instantiations (the `conformance::link` module). Enable it from a
  dev-dependency; it is safe, though pointless, in an application.
- `fs`: `FileBookmark` and its caller-driven blocking-I/O bridge.
- `test-internals`: this crate's own test scaffolding, enabled through
  its self-referential dev-dependency. Never enable it in an application.

## Stability and testing

The wire format is steady by design: each `Protocol` is pinned
byte-for-byte by snapshot tests, and once a version has shipped, a wire
change introduces a new protocol version.

The crate is validated by property tests stating the model's invariants
(convergence under arbitrary gossip schedules, deletion honoring, observer
soundness) and by the wire-format snapshots. Found a gap? An issue or a
test is very welcome.

<!-- cargo-rdme end -->
