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

- **If the set of live messages outgrows its smallest peer's storage.**
  Every peer replicates the whole set — in memory by default, or in a
  transactional store the peer brings ([Storage](#storage-in-memory-by-default-durable-by-choice));
  sharding is not supported. A store lifts a peer's RAM ceiling, never
  the full-replica model.
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

A note on identity-space hygiene: Every bootstrap forks the provider's own
identity, so the topology of introductions is the shape of the resulting
identity representations (which are, internally, trees): large fleets should
bootstrap with some kind of fan-out, neither one seed serving every joiner
nor each arrival introducing the next in a chain. Retire when you can, and
give peers a `Bookmark` so a crashed peer's identity is self-reclaimed
rather than stranded. Identity space may still strand and fragment over a
long life; where the application can arrange it, the full remedy is a roll
call: atomically have every participant retire its identity, then reissue
identities along a tree-shaped topology. That solves both fragmentation and
loss, but only if the application can guarantee that no peer left out of the
roll call will ever resurrect: an application-level obligation, not always
possible.

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

    // A `send` commits when awaited (the in-memory set cannot fail).
    alice.send("the meeting is at noon".to_string()).await?;

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
    use futures::TryStreamExt;
    let snapshot = bob.snapshot();
    let (_key, _version, message) =
        snapshot.iter().try_next().await?.expect("one live message");
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

## Storage: in memory by default, durable by choice

A replica's tree lives in process memory unless you say otherwise: every
handle carries a storage backend type parameter defaulting to the
in-memory backend, and nothing about it needs configuring.

To make a peer outlive its process — or outgrow its RAM — store the
replica in a transactional key-value store instead. You bring the store:
implement `Kv` (named byte tables with atomic, serializable
transactions; the `store` module documents the contract) for your
embedded engine of choice, and validate the implementation with the
`conformance::kv` suite exactly as you would validate a caller-built
`Link`. `Memory` ships as the reference implementation. Wrap the
store in a `KvBackend` and hand it to the same lifecycle calls a
memory-resident peer uses:

- `Peer::seed_in` seeds a universe with the replica in the store;
- `Bootstrap::backend` joins an existing universe into one;
- `Peer::open` resumes the replica a store already holds — network,
  content, and identity all reconstruct from the store's own records,
  with no separate `Bookmark` required: identity is recorded
  atomically with every commit and made durable before anything it
  stamped reaches a peer, so a restarted peer can never re-mint a
  version coordinate that survives anywhere — in its own store or at
  any other replica — whatever the store's durability policy.

```rust
use rumors::{KvBackend, Memory, Peer};
let kv = Memory::default();

// Seed a universe whose replica lives in the store...
let alice: Peer<String, _, _> = Peer::seed_in(KvBackend::new(kv.clone()));
let alice = alice.into_rumors();
alice.send("carved in stone".to_string()).await?;
drop(alice);

// ...and resume it, as a later process would: same network, same
// identity, same message.
let alice = Peer::<String, _, _>::open(kv).await?.into_rumors();
use futures::TryStreamExt;
let snapshot = alice.snapshot();
let (_key, _version, message) =
    snapshot.iter().try_next().await?.expect("one live message");
assert_eq!(message.as_str(), "carved in stone");
```

What persistence promises is split in two, deliberately:

- **Crash consistency is the crate's floor.** The store's canonical
  record only ever names a fully persisted tree, and recovery makes any
  interruption equivalent to dropping every in-process handle at once:
  a peer reopens to a consistent replica after any crash.
- **Durability of acknowledged writes is the store's documented
  policy — until they escape.** A store that acknowledges before its
  writes are durable (`Kv::sync` is its flush) trades crash-window
  durability for commit latency, and the crate preserves that trade
  for local work: a send never waits on the flush. What no store
  policy can weaken: **acknowledged and escaped implies durable.**
  Every session waits out `Kv::sync` before transmitting, covering
  each commit whose versions it could ship — at most one wait per
  commit-then-send window, and none after local-only quiet — so a
  commit a crash takes is one whose versions never crossed the wire:
  absent after restart exactly as if it had never been acknowledged,
  with no other replica holding coordinates the restarted peer could
  re-mint. Content learned *from* other peers flows back at the next
  gossip either way.

Three deployment obligations ride along. A store holds exactly one
replica, owned by one process at a time — the backend counts, caches,
and recovers on that assumption. Long-held `Snapshot`s (and
observers mid-walk) hold storage registrations, so space reclamation
for redacted content waits on their release; reclamation itself runs
incrementally inside ordinary commits, with
`KvBackend::vacuum` as the explicit drain for maintenance windows
(keep a clone of the `KvBackend` you seeded with to call it). And
the throughput figures under [When *should* you use
it?](#when-should-you-use-it) price the in-memory backend: a stored
replica's local commits and cold reads additionally ride the store's
transaction latency, while wire costs are unchanged. One protocol
restriction: `Protocol::V1` sessions require the in-memory backend
(`Error::ProtocolUnsupported`).

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

- `conformance`: the public validation suites for caller-built
  extension points — `link` instantiations (`conformance::link`)
  and `Kv` stores (`conformance::kv`). Enable it from a
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
