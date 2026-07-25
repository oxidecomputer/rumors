# rumors

<!-- cargo-rdme start -->

Lightspeed causal gossip for high-bandwidth networks.

`rumors` replicates a set of messages across a fleet of peers with no
coordination: every peer holds a full replica, changes it locally (inserting
*or* removing messages), and reconciles pairwise with whichever peer(s) it
can reach. Replicas which transitively gossip eventually converge on the
same set of messages; `rumors` works hard to turn "eventually" into "ASAP".

Unlike many gossip protocols, `rumors` features **redaction**. When any peer
redacts a message, it is contagiously purged from every peer's memory,
allowing superseded messages to be garbage-collected without global
coordination. Redaction is effectively free along every axis: conveying an
arbitrary quantity of redactions costs little additional communication, and
a redacted message leaves zero residual local bookkeeping. Memory usage
therefore scales up *and down* with the live set of messages, and bandwidth
up *and down* with the quantity of previously-unknown messages.

## When *should* you use it?

**If bandwidth is abundant and latency matters.**

Most gossip protocols are designed to be thrifty with bandwidth, trading
increased rounds of communication for smaller metadata overhead. However,
bandwidth is only getting cheaper and more plentiful, whereas *latency* is
capped by the laws of physics. `rumors` is designed for today and tomorrow;
it optimizes for extremely fast convergence when bandwidth is not a primary
constraint.

**`rumors` could be a particularly excellent fit if:**

- peers produce in total **less than 10,000 messages/second**, and
- each peer-to-peer link offers **1 Gb/s or better**.

In this regime, every change propagates at the pace of a few network round
trips per gossip hop, for any message set size that fits in memory. Required
bandwidth scales linearly down with message rate (for example, 100
messages/s at 10 Mb/s), and total set size increases cost only by a (very
slow-growing) logarithmic factor. These figures price `rumors`' own metadata
overhead; message bodies ride on top at their raw byte rate (at 10,000
messages/s, about 80 Mb/s per KB of mean body size). That term is a rounding
error for sub-KB bodies, and overtakes the metadata around 10 KB; past that,
you are paying to move your data, not to coordinate it, a cost no
replication scheme escapes.

**At the limits:** Up to roughly an order of magnitude past these bounds
(a thinner link, or a faster message rate), peers degrade gracefully
rather than failing outright: they may still converge, but may run stale
in proportion to roughly the square of the bandwidth shortfall (derived: a
session's metadata amortizes, falling roughly as the inverse square root
of the backlog at realistic scales, so equilibrium repays a bandwidth
deficit with its square in staleness). Past that, they will likely fall
behind regardless of gossip frequency. In the other direction, past
~10 Gb/s the network ceases to be the limit at all: CPU caps message
rate, and RAM caps set size.

## When *shouldn't* you use it?

- **If the set of live messages outgrows its smallest peer.** Every peer
  replicates the whole set; sharding is not supported.
- **If you need a consistently ordered, durable history.** A replicated log
  gives you sequencing; `rumors` only gives you causal ordering, which may
  be linearized differently between peers.
- **If you don't control the peers.** Peers trust one another: the protocol
  rejects malformed and mismatched sessions, but it is not Byzantine-tolerant.
  A compromised member can fabricate, redact, deny service, and violate the
  linearity of peer identities that everything rests on (a violation the
  model assumes absent, mostly undetectable, and unrecoverable).
  Authenticating peers and securing the transport are the application's job;
  the `link` module lists exactly what the protocol asks of the transport.
- **If bandwidth is your scarce resource.** `rumors` buys low latency with
  bandwidth: when reconciling small divergences, payloads under ~10 KB use
  more bandwidth for metadata than for messages. Reconciling larger
  divergences amortizes much of this cost, but on metered, narrow, or
  high-loss links, this crate strikes the wrong balance.

One boundary of the trust model is worth naming precisely. Message keys
and subtree digests are BLAKE3: a `Key` binds a message's send version to
its content at the full 32-byte width, while the reconciliation tree
compares 16-byte truncated digests to save wire bytes. A *peer* never needs
a hash collision (it holds write authority outright), so the residual
adversary is a content author outside the peer set, who controls message
content but never the version. Every send is stamped with a fresh causal
version, unpredictable under concurrent gossip churn, so mining a
collision against the truncated digest would require predicting the exact
insertion version fast; the version stamp is what denies precomputation.

## Network membership is identity custody

No global shared secret initiates a peer into a gossip network. Instead,
membership in the network is contagious, just like messages. A single call
to `Peer::seed` creates a new gossip network, and every other member
joins by `Peer::bootstrap`ping from some already-bootstrapped peer, back
along a chain of introductions that ends at the seed.

Peers may also `Peer::retire` from the network, donating their identity
to an arbitrary recipient. Identities are returned to circulation rather
than discarded because peer identity consumes *identity space*: a peer's
identity is a point in that shared space (a set of non-overlapping
intervals), and every message's `Version` is expressed in terms of the
tree of bootstrapped identities. Each `Peer::bootstrap` therefore widens
timestamps a little, and each `Peer::retire` narrows them again. A peer
that drops off without retiring strands its identity, and the universe's
timestamps stay a little wider forever: a few wasted bits, nothing
corrupted. (The identity machinery is `before`'s interval tree clocks;
see its docs for the model and for the [paper it
implements](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf).)

At fleet scale, identity-space hygiene is worth a moment of design. Every
bootstrap forks the provider's own identity, so the topology of
introductions is the shape of the resulting identities: large fleets
should bootstrap with some kind of fan-out, neither one seed serving every
joiner nor each arrival introducing the next in a chain. Retire when you
can, and give churny peers a `Bookmark` so a crashed peer's identity is
reclaimed rather than stranded. Identity space may still strand and
fragment over a long life; where the application can arrange it, the full
remedy is a roll call: atomically have every participant retire its
identity, then reissue identities along a good topology. That solves both
fragmentation and loss, but only if the application can guarantee that no
peer left out of the roll call will ever resurrect: an application-level
obligation, not always possible.

## The shape of the API

One replica has two faces, split by functionality. `Peer` is the unique
`!Clone` anchor that holds the peer's identity; it appears only at the edges
of a replica's life, where identity can move between peers: seeding a
universe (`Peer::seed`), joining one (`Peer::bootstrap`), leaving it
(`Peer::retire`).

Trading the anchor away (`Peer::into_rumors`) opens the working state:
`Rumors` clones freely, and cloned handles may `send`,
`redact`, observe `messages`,
and `gossip` (among other operations) concurrently
with one another. When all other clones are gone, `Rumors::try_into_peer`
recovers the anchor. This temporal partitioning lets the compiler guarantee
that your whole peer identity moves in or out only when you own it
exclusively.

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
async fn main() -> Result<(), rumors::Error> {
    // The universe's first peer creates it; every later peer bootstraps in.
    let alice = Peer::<String>::seed().into_rumors();

    // A bare `send` statement commits when its `Batch` drops, right here.
    alice.send("the meeting is at noon".to_string());

    // A session runs over a `Link`: a control byte stream plus a supply
    // of independent data streams (see the `link` module); here, the
    // in-memory pair. Alice serves one gossip session...
    let (mut near, mut far) = rumors::link::memory();
    let serve = alice.clone();
    tokio::spawn(async move {
        serve.gossip(&mut far).await.unwrap();
    });

    // ...and Bob joins the universe through it, arriving as a full replica.
    let bob = Peer::<String>::bootstrap()
        .join(&mut near)
        .await?
        .expect("alice is established, not herself bootstrapping");
    let bob = bob.into_rumors();

    // Convergence: Bob holds the message Alice sent before they ever met.
    let snapshot = bob.snapshot();
    let (_key, _version, message) = snapshot.iter().next().expect("one live message");
    println!("bob heard: {message}");
    // Prints exactly:
    //     bob heard: the meeting is at noon
    assert_eq!(message.as_str(), "the meeting is at noon");
    Ok(())
}
```

## How should you observe messages?

- `Snapshot` (`Rumors::snapshot`) is a **point-in-time value**:
  iterate it, look up a `Key` (`Snapshot::get`), or slice it by
  causal range (`Snapshot::range`). Taking one is cheap and never
  waits.
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
`AsyncRead` and `AsyncWrite`;
no Tokio runtime, spawning, sockets, or timers are required by this crate.

## Wire compatibility

Every session opens with a fixed 25-byte preamble carrying
`PROTOCOL_MAGIC`, the selected `Protocol`'s version, the network, and
session intent.
A counterparty that is not speaking `rumors`, or speaks an incompatible
version, is rejected before any peer-declared frame length is trusted
(`Error::MagicMismatch`, `Error::VersionMismatch`).
`Protocol::V2` is the default; `Protocol::V1` (behind the `protocol-v1`
cargo feature) can be selected on an established `Peer` with
`Peer::protocol`, or while joining with
`Bootstrap::protocol`. Both endpoints must select the same
protocol.

## Cargo features

Every feature is off by default.

- `conformance`: the public validation suite for caller-built `link`
  instantiations (the `conformance::link` module). Enable it from a
  dev-dependency; it is safe, though pointless, in an application.
- `protocol-v1`: the strictly alternating `Protocol::V1`, kept for wire
  compatibility with V1 peers and comparative measurement. Enabling it
  compiles a large per-height state-machine surface into the binary,
  which is why it is off by default.
- `test-internals`: this crate's own test scaffolding, enabled through
  its self-referential dev-dependency. Never enable it in an
  application.

## Stability and testing

The wire format is steady by design: each `Protocol` is pinned
byte-for-byte by snapshot tests, and a wire change introduces a new
protocol version rather than silently changing an existing one.

The crate is validated by property tests stating the model's invariants
(convergence under arbitrary gossip schedules, deletion honoring, observer
soundness) and by the wire-format snapshots. Found a gap? An issue or a
test is very welcome.

<!-- cargo-rdme end -->
