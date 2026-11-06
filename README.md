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
  [what securing the transport means](#what-securing-the-transport-means)
  lists exactly what the protocol asks of it.
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

    // A session runs over one connection, taken as its two halves (see
    // Transport below); here, an in-memory pair. Alice serves one gossip
    // session...
    let (near, far) = tokio::io::duplex(64 * 1024);
    let serve = alice.clone();
    tokio::spawn(async move {
        let (read, write) = tokio::io::split(far);
        let read = tokio::io::BufReader::new(read);
        serve.gossip_once(read, write).await.unwrap();
    });

    // ...and Bob joins the universe through it, arriving as a full replica.
    let (read, write) = tokio::io::split(near);
    let read = tokio::io::BufReader::new(read);
    let rumors::Joined::Joined { peer: bob, .. } =
        Peer::<String>::bootstrap().join(read, write).await
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

## Transport

A session runs over one connection to its peer, which it takes as two
halves: a buffered read half implementing
[`AsyncBufRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufRead.html)
and a write half implementing
[`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html).
Every session method takes the halves by value. The single-session
methods (`Rumors::gossip_once`, `Bootstrap::join`, and
`Peer::retire`) return them when the session completes; the
`Rumors::gossip` driver owns them for its whole lifetime and drops them
when it ends. Tokio implements both traits for mutable references, so
pass `&mut read, &mut write` to keep halves you hold elsewhere, such as a
connection you mean to reuse after a driver ends.

The connection must be:

- **one reliable, ordered duplex byte stream**: every byte written
  arrives, once, in the order written;
- **independent in its two directions**: a read proceeds while the same
  side's write is blocked, so a carrier that couples them (a half-duplex
  turn protocol, or a lock held across a read and a write) can deadlock a
  session;
- **paced by its receiver**: a writer that runs ahead of its reader
  waits, and any positive buffer capacity suffices; sessions stay live
  down to a one-byte buffer;
- **closed visibly**: when the peer departs, the read half reports end of
  stream or an error, and every write, pending or later, eventually
  completes or fails.

A TCP connection split with `TcpStream::into_split` meets every clause,
as does one bidirectional QUIC stream, or a `tokio::io::duplex` pair for
an in-process peer. The crate opens no connections itself.

Wrap the read half in `tokio::io::BufReader`: a TCP read half, a TLS
stream's read half, and a duplex pipe implement only `AsyncRead`. A
session reads its items from the buffer and consumes exactly the bytes
that are its own, so whatever follows a session stays in your buffer:
keep the `BufReader` for the next session on the same connection, and
never read the connection around it. The write half needs no wrapper: a
session gathers the messages it writes into *batches*, in a buffer of
its own of about 63 KiB, and writes and flushes each batch at once, so
each batch reaches the write half as one write: one system call on a TCP
socket. A batch ends when the session has nothing more ready to send, or
when it reaches the bounds `Peer::write_batch_range` sets. The default
bounds suit TLS 1.3 over TCP: a batch of small messages leaves as one
record that fills at most one TCP segment, and any other batch as one
record of at most TLS's largest plaintext, unless one message alone
outgrows that record and goes out by itself (`default::tls`). A session
writes a message at least the buffer's size directly, and continues any
write the transport takes only part of. A `BufWriter` around the write
half is safe, but only copies again what the session has already
gathered.

Over TCP, disable Nagle's algorithm on both ends with
`TcpStream::set_nodelay(true)` before splitting the stream. A session
often writes a short message and then waits for the peer's answer to it.
With Nagle's algorithm on, the kernel holds a short write back until the
peer acknowledges the bytes already in flight, and the peer's TCP stack
may delay that acknowledgement, commonly by tens to hundreds of
milliseconds, because it has nothing of its own to send yet. Each such
exchange can then stall for the delayed-acknowledgement interval, and a
session makes many in sequence as it descends the tree.

A departed peer that never closes the connection leaves a session
waiting, and so does a transport whose writes never complete; configure
`Peer::session_deadline` to bound such waits.

### What a session promises

Sessions synchronize message sets and may also join or retire a peer.
Their outcomes determine whether the replica and the connection can be
reused:

- **Success:** both sides confirmed completion. The returned halves rest
  exactly at the session boundary, ready for another session. Gossip
  returns `Gossiped`; bootstrap and retirement report their results
  through `Joined` and `Retire`. A bootstrap that joined returns the
  halves in a `Connection::Reusable`.
- **Failure:** the connection's position is unknown: discard it and
  reconnect. A later session on it gets no typed warning; it can fail in
  any way, or misread the stale bytes. Failure does not always mean that
  nothing changed: a failure in
  `Phase::Completion` can follow a
  local commit. Accepting a retirement can also commit content before a
  bookmark write fails; see `Error::Bookmark`. A bootstrap that fails
  after this side has joined reports the failure as a
  `Connection::Lost` beside the new peer.
- **Cancellation:** dropping an active session future also leaves the
  connection at an unknown position; discard it. No partial
  reconciliation is published, but a completed local commit is not
  undone. Cancelling bootstrap or retirement also drops the builder or
  peer it consumed; no outcome is returned for retry. See `Bookmark`
  for the effects on restart bookkeeping.

A bootstrap whose session fails after its whole identity arrived keeps
its live peer, beside a `Connection::Lost`. Bootstrap attaches a
selected bookmark once its wire session has ended with this side holding
its identity, whether or not the session completed. Failing or
cancelling that attachment leaves the connection where the session left
it; `Joined::Unbookmarked` preserves the joined peer after an
attachment failure.

Configure `Peer::session_deadline` to bound gossip, bootstrap, and
retirement exchanges while leaving idle waits untimed. Post-join bookmark
attachment is outside this deadline. Discard the connection on expiry.
`Gossip::Unconditionally` can initiate liveness probes on an otherwise
idle connection.

### What securing the transport means

The application authenticates peers and secures the transport. Protocol
checks help diagnose incompatible or buggy peers; they do not make a
completed session's updates trustworthy. An authorized peer can follow
the protocol while arbitrarily changing the gossip set. A non-conforming
peer can leave synchronization waiting indefinitely, so applications
choose deadlines when they need to bound those waits.

The transport has these responsibilities:

- **Authentication is authorization.** Any counterparty that can
  complete a session holds full write authority over the set; the
  protocol makes no finer-grained access decision.
- **Integrity.** The wire decoders detect malformed frames, not
  tampering: a transport that lets a third party modify bytes in flight
  has granted that party the session. In a bootstrap it has also granted
  the network's secret: the third party can replace the joining peer's
  public key with its own and open the secret the provider seals to it.
  A leaked secret stays useful for the network's whole life, since it
  cannot be rotated (see
  keyed addresses).
- **Confidentiality.** Session traffic (message bodies, set sizes,
  version structure, the greeting's metadata) crosses the connection in
  cleartext; the protocol adds no encryption of its own. The one exception
  is the network's secret, which a provider seals with HPKE for the
  joining peer it serves. The set's integrity
  does not rest on confidentiality: the digests and address bytes an
  observer reads on the wire do not let it compute those of versions it
  chooses (see keyed addresses).
- **Freshness.** The protocol does not authenticate a connection or a
  session as new; a transport that could replay recorded traffic into a
  live connection must rule that out itself.

## Runtime independence

Sessions and observers are plain futures and streams, driven entirely by
the caller. The I/O traits are Tokio's runtime-independent
[`AsyncBufRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufRead.html)
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
  an admitted payload is transferable everywhere; the setting's docs
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

- `conformance`: the public validation suite for caller-built bookmark
  storage (the `conformance::bookmark` module). Enable it from a
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
